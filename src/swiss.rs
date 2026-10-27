use crate::state::Match;
use crate::state::Round;
use crate::state::SCORE_WIN;
use crate::state::Score;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;

pub fn init_first_round(num_players: usize) -> Round {
    let matches = (0..)
        .map(|n| Match {
            p1: n * 2,
            p2: n * 2 + 1,
            result: None,
        })
        .take(num_players / 2)
        .collect();
    let bye = if num_players % 2 == 0 {
        None
    } else {
        Some(num_players - 1)
    };
    Round { matches, bye }
}

pub fn init_next_round(rounds: &[Round]) -> Round {
    let players_scored = score(rounds, None);
    let num_players = players_scored.len();
    let color_history = color_history(rounds);
    let pairings: HashSet<(usize, usize)> = rounds
        .iter()
        .flat_map(|round| round.matches.iter())
        .map(|m| (m.p1, m.p2))
        .collect();
    let have_faced_each_other =
        |p1, p2| pairings.contains(&(p1, p2)) || pairings.contains(&(p2, p1));
    let score_groups: BTreeMap<Score, Vec<usize>> = players_scored
        .into_iter()
        .fold(BTreeMap::<Score, Vec<usize>>::new(), |mut acc, (k, v)| {
            acc.entry(v).or_default().push(k);
            acc
        })
        .into_iter()
        .map(|(k, mut v)| {
            v.sort();
            (k, v)
        })
        .collect();
    let (bye, mut score_groups) = pick_bye(score_groups, rounds, num_players);
    let mut matches = vec![];
    while let Some((_, mut group)) = score_groups.pop_last() {
        while let Some(player) = group.pop() {
            if let Some(opp_idx) = group
                .iter()
                .rposition(|opponent| !have_faced_each_other(player, *opponent))
            {
                let opponent = group.remove(opp_idx);
                let new_match = match (
                    color_history[player].white_last_time,
                    color_history[opponent].white_last_time,
                ) {
                    (true, false) => Match {
                        p1: opponent,
                        p2: player,
                        result: None,
                    },
                    (false, true) => Match {
                        p1: player,
                        p2: opponent,
                        result: None,
                    },
                    _ => {
                        // 1. If one had more blacks, they plays white.
                        // 2. If one had less whites, they play white.
                        // 3. Else, the one with lower index plays white.
                        if color_history[player].num_blacks > color_history[opponent].num_blacks
                            || color_history[player].num_whites < color_history[opponent].num_whites
                            || player < opponent
                        {
                            Match {
                                p1: player,
                                p2: opponent,
                                result: None,
                            }
                        } else {
                            Match {
                                p1: opponent,
                                p2: player,
                                result: None,
                            }
                        }
                    }
                };
                matches.push(new_match);
            } else {
                // We couldn't find an opponent for this player in thier group. Put them in the next group.
                if let Some(mut entry) = score_groups.last_entry() {
                    entry.get_mut().push(player);
                } else {
                    todo!("Implement backtracking for pairing.");
                }
            }
        }
    }
    Round { matches, bye }
}

pub fn score(rounds: &[Round], current_round: Option<&Round>) -> HashMap<usize, Score> {
    let rounds = rounds.iter().chain(current_round);
    let match_points = rounds
        .clone()
        .flat_map(|round| round.matches.iter())
        .filter_map(|m| Some([(m.p1, m.p1_score()?), (m.p2, m.p2_score()?)].into_iter()))
        .flatten();
    let bye_points = rounds.filter_map(|round| round.bye).map(|p| (p, SCORE_WIN));
    match_points
        .chain(bye_points)
        .fold(HashMap::<usize, Score>::new(), |mut acc, (k, v)| {
            *acc.entry(k).or_default() += v;
            acc
        })
}

#[derive(Clone, Copy, Default)]
struct ColorHistory {
    white_last_time: bool,
    num_whites: usize,
    num_blacks: usize,
}

fn color_history(rounds: &[Round]) -> Vec<ColorHistory> {
    if rounds.is_empty() {
        return vec![];
    }
    let num_players = rounds[0].matches.len() * 2 + if rounds[0].bye.is_some() { 1 } else { 0 };
    rounds.iter().flat_map(|round| round.matches.iter()).fold(
        vec![Default::default(); num_players],
        |mut color_history, mtch| {
            color_history[mtch.p1].white_last_time = true;
            color_history[mtch.p2].white_last_time = false;
            color_history[mtch.p1].num_whites += 1;
            color_history[mtch.p2].num_blacks += 1;
            color_history
        },
    )
}

fn pick_bye(
    score_groups: BTreeMap<Score, Vec<usize>>,
    rounds: &[Round],
    num_players: usize,
) -> (Option<usize>, BTreeMap<Score, Vec<usize>>) {
    if num_players % 2 == 0 {
        return (None, score_groups);
    }
    let byes_before = rounds
        .iter()
        .flat_map(|round| round.bye)
        .collect::<HashSet<_>>();
    let (bye, score_groups): (Option<usize>, BTreeMap<Score, Vec<usize>>) =
        score_groups.into_iter().fold(
            (None, BTreeMap::new()),
            |(bye, mut score_groups), (score, group)| {
                let (bye, group): (Option<usize>, Vec<usize>) =
                    group
                        .into_iter()
                        .fold((bye, Vec::new()), |(bye, mut group), player| {
                            if bye.is_some() || byes_before.contains(&player) {
                                group.push(player);
                                (bye, group)
                            } else {
                                (Some(player), group)
                            }
                        });
                score_groups.insert(score, group);
                (bye, score_groups)
            },
        );
    (bye, score_groups)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::MatchResult;
    use crate::state::SCORE_DRAW;
    use crate::state::SCORE_LOSS;

    #[test]
    fn test_score_empty() {
        let scores = score(&[], None);
        assert!(scores.is_empty());
    }

    #[test]
    fn test_score_single_round_outcomes() {
        let round = Round {
            matches: vec![
                Match {
                    p1: 0,
                    p2: 1,
                    result: Some(MatchResult::WinP1),
                },
                Match {
                    p1: 2,
                    p2: 3,
                    result: Some(MatchResult::Draw),
                },
                Match {
                    p1: 4,
                    p2: 5,
                    result: Some(MatchResult::WinP2),
                },
            ],
            bye: Some(6),
        };
        let scores = score(&[round], None);
        assert_eq!(scores.get(&0), Some(&SCORE_WIN));
        assert_eq!(scores.get(&1), Some(&SCORE_LOSS));
        assert_eq!(scores.get(&2), Some(&SCORE_DRAW));
        assert_eq!(scores.get(&3), Some(&SCORE_DRAW));
        assert_eq!(scores.get(&4), Some(&SCORE_LOSS));
        assert_eq!(scores.get(&5), Some(&SCORE_WIN));
        assert_eq!(scores.get(&6), Some(&SCORE_WIN));
    }

    #[test]
    fn test_score_unplayed_matches_ignored() {
        let round = Round {
            matches: vec![
                Match {
                    p1: 0,
                    p2: 1,
                    result: Some(MatchResult::WinP1),
                },
                Match {
                    p1: 2,
                    p2: 3,
                    result: None,
                },
            ],
            bye: None,
        };
        let scores = score(&[round], None);
        assert_eq!(scores.get(&0), Some(&SCORE_WIN));
        assert_eq!(scores.get(&1), Some(&SCORE_LOSS));
        assert_eq!(scores.get(&2), None);
        assert_eq!(scores.get(&3), None);
    }

    #[test]
    fn test_score_multi_round_accumulation_and_current_round() {
        let r1 = Round {
            matches: vec![Match {
                p1: 0,
                p2: 1,
                result: Some(MatchResult::WinP1),
            }],
            bye: Some(2),
        };
        let r2 = Round {
            matches: vec![Match {
                p1: 0,
                p2: 2,
                result: Some(MatchResult::Draw),
            }],
            bye: Some(1),
        };
        let current = Round {
            matches: vec![Match {
                p1: 1,
                p2: 2,
                result: Some(MatchResult::WinP1),
            }],
            bye: Some(0),
        };

        // Scores from r1 and r2 only:
        let scores = score(&[r1.clone(), r2.clone()], None);
        assert_eq!(scores.get(&0), Some(&(SCORE_WIN + SCORE_DRAW)));
        assert_eq!(scores.get(&1), Some(&SCORE_WIN));
        assert_eq!(scores.get(&2), Some(&(SCORE_WIN + SCORE_DRAW)));

        // With current_round included:
        let total_scores = score(&[r1, r2], Some(&current));
        assert_eq!(
            total_scores.get(&0),
            Some(&(SCORE_WIN + SCORE_DRAW + SCORE_WIN))
        );
        assert_eq!(total_scores.get(&1), Some(&(SCORE_WIN + SCORE_WIN)));
        assert_eq!(total_scores.get(&2), Some(&(SCORE_WIN + SCORE_DRAW)));
    }

    #[test]
    fn test_color_history_empty() {
        let history = color_history(&[]);
        assert!(history.is_empty());
    }

    #[test]
    fn test_color_history_single_round() {
        let r1 = Round {
            matches: vec![
                Match {
                    p1: 0,
                    p2: 1,
                    result: None,
                },
                Match {
                    p1: 2,
                    p2: 3,
                    result: None,
                },
            ],
            bye: None,
        };
        let history = color_history(&[r1]);
        assert_eq!(history.len(), 4);
        assert!(history[0].white_last_time);
        assert_eq!(history[0].num_whites, 1);
        assert_eq!(history[0].num_blacks, 0);

        assert!(!history[1].white_last_time);
        assert_eq!(history[1].num_whites, 0);
        assert_eq!(history[1].num_blacks, 1);

        assert!(history[2].white_last_time);
        assert_eq!(history[2].num_whites, 1);
        assert_eq!(history[2].num_blacks, 0);

        assert!(!history[3].white_last_time);
        assert_eq!(history[3].num_whites, 0);
        assert_eq!(history[3].num_blacks, 1);
    }

    #[test]
    fn test_color_history_alternation_across_rounds() {
        let r1 = Round {
            matches: vec![Match {
                p1: 0,
                p2: 1,
                result: None,
            }],
            bye: None,
        };
        let r2 = Round {
            matches: vec![Match {
                p1: 1,
                p2: 0,
                result: None,
            }],
            bye: None,
        };
        let history = color_history(&[r1, r2]);
        assert_eq!(history.len(), 2);
        assert!(!history[0].white_last_time);
        assert_eq!(history[0].num_whites, 1);
        assert_eq!(history[0].num_blacks, 1);

        assert!(history[1].white_last_time);
        assert_eq!(history[1].num_whites, 1);
        assert_eq!(history[1].num_blacks, 1);
    }

    #[test]
    fn test_color_history_with_bye() {
        let r1 = Round {
            matches: vec![Match {
                p1: 0,
                p2: 1,
                result: None,
            }],
            bye: Some(2),
        };
        let history = color_history(&[r1]);
        assert_eq!(history.len(), 3);
        assert!(!history[2].white_last_time);
        assert_eq!(history[2].num_whites, 0);
        assert_eq!(history[2].num_blacks, 0);
    }

    #[test]
    fn test_pick_bye_even_number_of_players() {
        let mut score_groups = BTreeMap::new();
        score_groups.insert(SCORE_LOSS, vec![1, 2]);
        score_groups.insert(SCORE_WIN, vec![0, 3]);

        let (bye, remaining) = pick_bye(score_groups.clone(), &[], 4);
        assert_eq!(bye, None);
        assert_eq!(remaining, score_groups);
    }

    #[test]
    fn test_pick_bye_odd_number_of_players_lowest_score() {
        let mut score_groups = BTreeMap::new();
        score_groups.insert(SCORE_LOSS, vec![1]);
        score_groups.insert(SCORE_WIN, vec![0, 2]);

        let (bye, remaining) = pick_bye(score_groups, &[], 3);
        assert_eq!(bye, Some(1));
        assert_eq!(remaining.get(&SCORE_LOSS), Some(&vec![]));
        assert_eq!(remaining.get(&SCORE_WIN), Some(&vec![0, 2]));
    }

    #[test]
    fn test_pick_bye_skips_previous_bye_holder() {
        let mut score_groups = BTreeMap::new();
        score_groups.insert(SCORE_LOSS, vec![1, 2]);
        score_groups.insert(SCORE_WIN, vec![0]);

        let r1 = Round {
            matches: vec![Match {
                p1: 0,
                p2: 2,
                result: None,
            }],
            bye: Some(1),
        };

        let (bye, remaining) = pick_bye(score_groups, &[r1], 3);
        assert_eq!(bye, Some(2));
        assert_eq!(remaining.get(&SCORE_LOSS), Some(&vec![1]));
        assert_eq!(remaining.get(&SCORE_WIN), Some(&vec![0]));
    }

    #[test]
    fn test_pick_bye_all_players_had_byes() {
        let mut score_groups = BTreeMap::new();
        score_groups.insert(SCORE_WIN, vec![0]);

        let r1 = Round {
            matches: vec![],
            bye: Some(0),
        };

        let (bye, remaining) = pick_bye(score_groups.clone(), &[r1], 1);
        assert_eq!(bye, None);
        assert_eq!(remaining, score_groups);
    }

    #[test]
    fn test_init_next_round_empty() {
        let next_round = init_next_round(&[]);
        assert_eq!(
            next_round,
            Round {
                matches: vec![],
                bye: None
            }
        );
    }

    #[test]
    fn test_init_next_round_four_players_round_two() {
        let r1 = Round {
            matches: vec![
                Match {
                    p1: 0,
                    p2: 1,
                    result: Some(MatchResult::WinP1),
                },
                Match {
                    p1: 3,
                    p2: 2,
                    result: Some(MatchResult::WinP1),
                },
            ],
            bye: None,
        };

        let r2 = init_next_round(&[r1]);
        assert_eq!(r2.bye, None);
        assert_eq!(r2.matches.len(), 2);

        // Winners 0 and 3 must face each other, losers 1 and 2 must face each other
        let match_top = r2
            .matches
            .iter()
            .find(|m| (m.p1 == 0 && m.p2 == 3) || (m.p1 == 3 && m.p2 == 0))
            .expect("Winners 0 and 3 should be paired");
        let match_bottom = r2
            .matches
            .iter()
            .find(|m| (m.p1 == 1 && m.p2 == 2) || (m.p1 == 2 && m.p2 == 1))
            .expect("Losers 1 and 2 should be paired");

        // Both 0 and 3 were White in R1.
        // Tiebreaker (player < opponent) gives Player 0 White, Player 3 Black.
        assert_eq!(match_top.p1, 0);
        assert_eq!(match_top.p2, 3);

        // Both 1 and 2 were Black in R1.
        // Tiebreaker gives Player 1 White, Player 2 Black.
        assert_eq!(match_bottom.p1, 1);
        assert_eq!(match_bottom.p2, 2);
    }

    #[test]
    fn test_init_next_round_three_players_odd_bye() {
        let r1 = Round {
            matches: vec![Match {
                p1: 0,
                p2: 1,
                result: Some(MatchResult::WinP1),
            }],
            bye: Some(2),
        };

        let r2 = init_next_round(&[r1]);
        // Player 1 has 0 points and has not had a bye, so player 1 gets the bye
        assert_eq!(r2.bye, Some(1));
        assert_eq!(r2.matches.len(), 1);

        // Players 0 and 2 should be paired
        let m = &r2.matches[0];
        assert!((m.p1 == 0 && m.p2 == 2) || (m.p1 == 2 && m.p2 == 0));

        // Player 0 was White in R1; Player 2 had a bye (white_last_time=false).
        // So Player 2 plays White, Player 0 plays Black.
        assert_eq!(m.p1, 2);
        assert_eq!(m.p2, 0);
    }

    #[test]
    fn test_init_next_round_avoids_rematches() {
        let r1 = Round {
            matches: vec![
                Match {
                    p1: 0,
                    p2: 1,
                    result: Some(MatchResult::Draw),
                },
                Match {
                    p1: 2,
                    p2: 3,
                    result: Some(MatchResult::Draw),
                },
            ],
            bye: None,
        };

        let r2 = init_next_round(&[r1]);
        assert_eq!(r2.matches.len(), 2);
        for m in &r2.matches {
            assert!(!((m.p1 == 0 && m.p2 == 1) || (m.p1 == 1 && m.p2 == 0)));
            assert!(!((m.p1 == 2 && m.p2 == 3) || (m.p1 == 3 && m.p2 == 2)));
        }
    }

    #[test]
    fn test_init_next_round_cross_group_float_down() {
        let r1 = Round {
            matches: vec![
                Match {
                    p1: 0,
                    p2: 1,
                    result: Some(MatchResult::WinP1),
                },
                Match {
                    p1: 2,
                    p2: 3,
                    result: Some(MatchResult::WinP1),
                },
            ],
            bye: None,
        };
        let r2 = Round {
            matches: vec![
                Match {
                    p1: 0,
                    p2: 2,
                    result: Some(MatchResult::Draw),
                },
                Match {
                    p1: 1,
                    p2: 3,
                    result: Some(MatchResult::WinP1),
                },
            ],
            bye: None,
        };

        let r3 = init_next_round(&[r1, r2]);
        assert_eq!(r3.matches.len(), 2);
        // 0 and 2 already played, 0 and 1 already played.
        // 0 must play 3.
        let m_for_0 = r3
            .matches
            .iter()
            .find(|m| m.p1 == 0 || m.p2 == 0)
            .expect("Player 0 should have a match");
        let opponent_of_0 = if m_for_0.p1 == 0 {
            m_for_0.p2
        } else {
            m_for_0.p1
        };
        assert_eq!(opponent_of_0, 3);
    }
}
