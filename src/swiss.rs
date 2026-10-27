use crate::state::Match;
use crate::state::Round;
use crate::state::SCORE_WIN;
use crate::state::Score;
use std::cmp::Ordering;
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

pub fn init_next_round(rounds: &[Round]) -> Option<Round> {
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
    let (bye, score_groups) = pick_bye(score_groups, rounds, num_players);

    // Instead of two nexted loops I will turn this into a recoursive function that turns
    // `score_groups` into `matches`. It will take `mathces`, `score_groups` (and whatever else
    // that's captured) and return the same pair.
    let (matches, _) = pair_up(vec![], score_groups, &color_history, &have_faced_each_other);
    if matches.is_empty() {
        // Impossible to create a new round without repeating the pairings.
        None
    } else {
        Some(Round { matches, bye })
    }
}

fn pair_up(
    matches: Vec<Match>,
    mut score_groups: BTreeMap<Score, Vec<usize>>,
    color_history: &[ColorHistory],
    have_faced_each_other: &impl Fn(usize, usize) -> bool,
) -> (Vec<Match>, BTreeMap<Score, Vec<usize>>) {
    score_groups.retain(|_, group| !group.is_empty());
    let Some((top_score, mut top_group)) = score_groups.pop_last() else {
        return (matches, score_groups);
    };

    // 1. Pick a player - starting from the highest score group and highest index.
    let Some(player) = top_group.pop() else {
        return (matches, score_groups);
    };
    if !top_group.is_empty() {
        score_groups.insert(top_score, top_group);
    }

    // 2. Find an opponent for this player, with the same preference logic, but excluding the pairs
    //    that faced each other.
    let candidates: Vec<(Score, usize, usize)> = score_groups
        .iter()
        .rev()
        .flat_map(|(&score, group)| {
            group
                .iter()
                .enumerate()
                .rev()
                .map(move |(idx, &opponent)| (score, idx, opponent))
        })
        .filter(|&(_, _, opponent)| !have_faced_each_other(player, opponent))
        .collect();

    for (opp_score, opp_idx, opponent) in candidates {
        let mut next_groups = score_groups.clone();
        let group = next_groups.get_mut(&opp_score).unwrap();
        group.remove(opp_idx);
        if group.is_empty() {
            next_groups.remove(&opp_score);
        }

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

        // 3. Push the found pairing into the `matches`
        let mut next_matches = matches.clone();
        next_matches.push(new_match);

        // 4. Call the function recursively to fill up the rest of the matchups.
        let (res_matches, res_groups) = pair_up(
            next_matches,
            next_groups,
            color_history,
            have_faced_each_other,
        );
        if res_groups.is_empty() {
            // 4.2. If the function returns matches filled up with all the matches, then simply return that.
            return (res_matches, res_groups);
        }
        // 4.1. Otherwise continue and try another opponent.
    }

    score_groups.entry(top_score).or_default().push(player);
    (matches, score_groups)
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

/// In returned vector the index of the element is the ranking, so the final scoreboard must be
/// rendered in that order. The first variable in the tuple is the index (initial position number)
/// of the player, not the index of the tuple in the array.
pub fn score_and_rank(rounds: &[Round]) -> Vec<(usize, Score)> {
    let scores = score(rounds, None);
    let opponents = rounds.iter().flat_map(|round| round.matches.iter()).fold(
        HashMap::<usize, Vec<usize>>::new(),
        |mut acc, m| {
            acc.entry(m.p1).or_default().push(m.p2);
            acc.entry(m.p2).or_default().push(m.p1);
            acc
        },
    );
    let mut ranked: Vec<(usize, Score)> = scores.clone().into_iter().collect();
    ranked.sort_by(|a, b| buchholz_tiebreak(&scores, &opponents, a.0, b.0));
    ranked
}

fn buchholz_tiebreak(
    scores: &HashMap<usize, Score>,
    opponents: &HashMap<usize, Vec<usize>>,
    p1: usize,
    p2: usize,
) -> Ordering {
    let score_of = |p: usize| scores.get(&p).copied().unwrap_or_default();
    let buchholz_of = |p: usize| {
        opponents
            .get(&p)
            .into_iter()
            .flatten()
            .cloned()
            .map(score_of)
            .sum::<Score>()
    };

    score_of(p2)
        .cmp(&score_of(p1))
        .then_with(|| buchholz_of(p2).cmp(&buchholz_of(p1)))
        .then_with(|| p1.cmp(&p2))
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
        assert_eq!(next_round, None);
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

        let r2 = init_next_round(&[r1]).unwrap();
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

        let r2 = init_next_round(&[r1]).unwrap();
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

        let r2 = init_next_round(&[r1]).unwrap();
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

        let r3 = init_next_round(&[r1, r2]).unwrap();
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

    #[test]
    fn test_init_next_round_backtracking_required() {
        // 6 players after 2 rounds:
        // Scores after R2:
        //   2.0: [2]
        //   1.0: [0, 1, 3, 4]
        //   0.0: [5]
        // Pairs already played:
        //   R1: (0, 5), (2, 4), (3, 1)
        //   R2: (2, 3), (1, 0), (4, 5)
        // Greedy without backtracking:
        //   1. Pops 2 (from 2.0), skips 4 and 3 (already played), pairs (2, 1).
        //   2. Pops 4 (from 1.0), pairs (4, 3).
        //   3. Remaining players are 0 and 5, who already played in R1 -> deadlock!
        // With backtracking:
        //   Undoes (4, 3), pairs (4, 0), leaving (3, 5) which is valid.
        let r1 = Round {
            matches: vec![
                Match {
                    p1: 0,
                    p2: 5,
                    result: Some(MatchResult::WinP1),
                },
                Match {
                    p1: 2,
                    p2: 4,
                    result: Some(MatchResult::WinP1),
                },
                Match {
                    p1: 3,
                    p2: 1,
                    result: Some(MatchResult::WinP1),
                },
            ],
            bye: None,
        };
        let r2 = Round {
            matches: vec![
                Match {
                    p1: 2,
                    p2: 3,
                    result: Some(MatchResult::WinP1),
                },
                Match {
                    p1: 1,
                    p2: 0,
                    result: Some(MatchResult::WinP1),
                },
                Match {
                    p1: 4,
                    p2: 5,
                    result: Some(MatchResult::WinP1),
                },
            ],
            bye: None,
        };

        let r3 = init_next_round(&[r1, r2]).expect("Should find valid R3 pairing via backtracking");
        assert_eq!(r3.matches.len(), 3);
        assert_eq!(r3.bye, None);
    }

    #[test]
    fn test_init_next_round_impossible_returns_none() {
        // 2 players who already played each other in R1 cannot be paired again in R2.
        let r1 = Round {
            matches: vec![Match {
                p1: 0,
                p2: 1,
                result: Some(MatchResult::WinP1),
            }],
            bye: None,
        };

        assert_eq!(init_next_round(&[r1]), None);
    }

    #[test]
    fn test_buchholz_tiebreak_and_score_and_rank() {
        // R1: 0 beats 1, 2 beats 3
        // R2: 0 draws 2, 1 beats 3
        // Scores:
        //   0: 1.5 (opponents: 1 [1.0], 2 [1.5] -> Buchholz = 2.5)
        //   2: 1.5 (opponents: 3 [0.0], 0 [1.5] -> Buchholz = 1.5)
        //   1: 1.0 (opponents: 0 [1.5], 3 [0.0] -> Buchholz = 1.5)
        //   3: 0.0 (opponents: 2 [1.5], 1 [1.0] -> Buchholz = 2.5)
        // Expected ranking: 0 (1.5, BH 2.5), 2 (1.5, BH 1.5), 1 (1.0), 3 (0.0)
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

        let ranked = score_and_rank(&[r1, r2]);
        assert_eq!(
            ranked,
            vec![
                (0, SCORE_WIN + SCORE_DRAW),
                (2, SCORE_WIN + SCORE_DRAW),
                (1, SCORE_WIN),
                (3, SCORE_LOSS),
            ]
        );
    }

    #[test]
    fn test_buchholz_tiebreak_higher_buchholz_wins_even_with_higher_index() {
        // Suppose player 2 has higher Buchholz than player 0 while tied in score:
        // R1: 0 beats 1, 2 beats 3
        // R2: 0 draws 2, 3 beats 1
        // Scores:
        //   0: 1.5 (opponents: 1 [0.0], 2 [1.5] -> Buchholz = 1.5)
        //   2: 1.5 (opponents: 3 [1.0], 0 [1.5] -> Buchholz = 2.5)
        //   3: 1.0
        //   1: 0.0
        // Player 2 must rank ahead of Player 0 due to higher Buchholz.
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
                    result: Some(MatchResult::WinP2),
                },
            ],
            bye: None,
        };

        let ranked = score_and_rank(&[r1, r2]);
        assert_eq!(
            ranked,
            vec![
                (2, SCORE_WIN + SCORE_DRAW),
                (0, SCORE_WIN + SCORE_DRAW),
                (3, SCORE_WIN),
                (1, SCORE_LOSS),
            ]
        );
    }

    #[test]
    fn test_score_and_rank_empty() {
        assert_eq!(score_and_rank(&[]), vec![]);
    }

    #[test]
    fn test_buchholz_tiebreak_primary_score_comparison() {
        let mut scores = HashMap::new();
        scores.insert(0, SCORE_WIN);
        scores.insert(1, SCORE_DRAW);

        let opponents = HashMap::new();
        assert_eq!(buchholz_tiebreak(&scores, &opponents, 0, 1), Ordering::Less);
        assert_eq!(
            buchholz_tiebreak(&scores, &opponents, 1, 0),
            Ordering::Greater
        );
    }

    #[test]
    fn test_buchholz_tiebreak_secondary_buchholz_comparison() {
        // Players 0 and 1 both have 1.0 point, but 0's opponent (2) has 2.0 points
        // while 1's opponent (3) has 0.5 points.
        let mut scores = HashMap::new();
        scores.insert(0, SCORE_WIN);
        scores.insert(1, SCORE_WIN);
        scores.insert(2, SCORE_WIN + SCORE_WIN);
        scores.insert(3, SCORE_DRAW);

        let mut opponents = HashMap::new();
        opponents.insert(0, vec![2]);
        opponents.insert(1, vec![3]);

        assert_eq!(buchholz_tiebreak(&scores, &opponents, 0, 1), Ordering::Less);
        assert_eq!(
            buchholz_tiebreak(&scores, &opponents, 1, 0),
            Ordering::Greater
        );
    }

    #[test]
    fn test_buchholz_tiebreak_tertiary_initial_index_comparison() {
        // Players 0 and 2 have identical scores and identical Buchholz totals.
        // Player 0 (lower initial index) should rank before Player 2.
        let mut scores = HashMap::new();
        scores.insert(0, SCORE_WIN);
        scores.insert(2, SCORE_WIN);
        scores.insert(1, SCORE_DRAW);
        scores.insert(3, SCORE_DRAW);

        let mut opponents = HashMap::new();
        opponents.insert(0, vec![1]);
        opponents.insert(2, vec![3]);

        assert_eq!(buchholz_tiebreak(&scores, &opponents, 0, 2), Ordering::Less);
        assert_eq!(
            buchholz_tiebreak(&scores, &opponents, 2, 0),
            Ordering::Greater
        );
        assert_eq!(
            buchholz_tiebreak(&scores, &opponents, 0, 0),
            Ordering::Equal
        );
    }

    #[test]
    fn test_score_and_rank_with_byes_contributes_zero_to_buchholz() {
        // 3 players across 2 rounds:
        // R1: 0 beats 1, 2 gets bye
        // R2: 2 draws 0, 1 gets bye
        // Scores:
        //   0: 1.5 (opponents: 1 [1.0], 2 [1.5] -> Buchholz = 2.5)
        //   2: 1.5 (opponents: 0 [1.5], bye [0.0] -> Buchholz = 1.5)
        //   1: 1.0 (opponents: 0 [1.5], bye [0.0] -> Buchholz = 1.5)
        // Even though 0 and 2 both have 1.5, 0 played two real opponents while 2 had a bye,
        // so 0 has higher Buchholz (2.5 vs 1.5) and ranks first.
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
                p1: 2,
                p2: 0,
                result: Some(MatchResult::Draw),
            }],
            bye: Some(1),
        };

        let ranked = score_and_rank(&[r1, r2]);
        assert_eq!(
            ranked,
            vec![
                (0, SCORE_WIN + SCORE_DRAW),
                (2, SCORE_WIN + SCORE_DRAW),
                (1, SCORE_WIN),
            ]
        );
    }
}
