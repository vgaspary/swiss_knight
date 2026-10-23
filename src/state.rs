use serde::Deserialize;
use serde::Serialize;
use std::fmt::Display;
use std::ops::Add;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    // Navigation
    Up,
    Down,

    // Lobby
    AddPlayer,
    RemovePlayer,
    StartTournament,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    AddPlayer(String),
    RemovePlayer(usize),
    SetCursor(usize),
    StartTournament,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchResult {
    WinP1,
    Draw,
    WinP2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    pub p1: usize,
    pub p2: usize,
    pub result: Option<MatchResult>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Round(Vec<Match>);

impl Round {
    pub fn matches(&self) -> &[Match] {
        &self.0
    }
}

/// Scores in a chess tournament are multiples of 0.5. The `usize` inside
/// represents the score as a multiple of 0.5-s. Examples:
///
/// Real score | representation
/// --------------------------
///   0.0      |    Score(0)
///   0.5      |    Score(1)
///   1.0      |    Score(2)
///   8.5      |    Score(17)
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Score(usize);

const SCORE_LOSS: Score = Score(0);
const SCORE_DRAW: Score = Score(1);
const SCORE_WIN: Score = Score(2);

impl Display for Score {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let a = self.0 / 2;
        let b = if self.0 % 2 == 1 { 5 } else { 0 };
        write!(f, "{a}.{b}")
    }
}

impl Add for Score {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    Lobby {
        players: Vec<String>,
        cursor: usize,
    },
    InProgress {
        players: Vec<(String, Score)>,
        cursor: usize,
        rounds: Vec<Round>,
    },
}

impl Default for State {
    fn default() -> Self {
        Self::Lobby {
            players: vec![],
            cursor: 0,
        }
    }
}

pub fn do_action(state: State, action: &Action) -> State {
    match (state, action) {
        (State::Lobby { players, .. }, Action::SetCursor(idx)) => State::Lobby {
            players,
            cursor: *idx,
        },
        (State::Lobby { players, .. }, Action::AddPlayer(name)) => {
            let mut players = players;
            players.push(name.clone());
            let cursor = players.len() - 1;
            State::Lobby { players, cursor }
        }
        (State::Lobby { players, .. }, Action::RemovePlayer(idx)) => {
            let mut new_players = Vec::with_capacity(players.len().saturating_sub(1));
            new_players.extend_from_slice(&players[0..*idx]);
            new_players.extend_from_slice(&players[idx + 1..]);
            let players = new_players;
            let cursor = if players.is_empty() {
                0
            } else {
                (*idx).min(players.len() - 1)
            };
            State::Lobby { players, cursor }
        }
        (State::Lobby { players, .. }, Action::StartTournament) => State::InProgress {
            players: players.into_iter().map(|name| (name, Score(0))).collect(),
            cursor: 0,
            rounds: vec![Round(vec![
                Match {
                    p1: 0,
                    p2: 1,
                    result: None,
                },
                Match {
                    p1: 2,
                    p2: 3,
                    result: Some(MatchResult::WinP1),
                },
            ])],
        },
        (
            State::InProgress {
                players, rounds, ..
            },
            Action::SetCursor(idx),
        ) => State::InProgress {
            players,
            cursor: *idx,
            rounds,
        },
        (state, _) => state,
    }
}

pub fn replay(actions: &[Action]) -> State {
    actions.iter().fold(State::default(), do_action)
}

pub fn intent_to_action<F: FnOnce() -> Option<String>>(
    intent: Intent,
    state: &State,
    prompt_player_name: F,
) -> Option<Action> {
    match state {
        State::Lobby { players, cursor } => match intent {
            Intent::Up if *cursor > 0 => Some(Action::SetCursor(cursor - 1)),
            Intent::Down if cursor + 1 < players.len() => Some(Action::SetCursor(cursor + 1)),
            Intent::AddPlayer => prompt_player_name().and_then(|name| {
                match players.iter().position(|n| &name == n) {
                    None => Some(Action::AddPlayer(name)),
                    Some(idx) if idx != *cursor => Some(Action::SetCursor(idx)),
                    _ => None,
                }
            }),
            Intent::RemovePlayer if !players.is_empty() => Some(Action::RemovePlayer(*cursor)),
            Intent::StartTournament if !players.is_empty() => Some(Action::StartTournament),
            _ => None,
        },
        State::InProgress { cursor, rounds, .. } => match intent {
            Intent::Up if *cursor > 0 => Some(Action::SetCursor(cursor - 1)),
            Intent::Down if cursor + 1 < rounds.iter().map(|r| r.0.len()).sum::<usize>() => {
                Some(Action::SetCursor(cursor + 1))
            }
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::assert_matches;

    #[test]
    fn test_new_state_is_empty() {
        let state = State::default();
        assert_matches!(state, State::Lobby { players, cursor } if players.is_empty() && cursor == 0);
    }

    #[test]
    fn test_do_action_add_player() {
        let state = State::default();
        let state = do_action(state, &Action::AddPlayer("Alice".to_string()));
        assert_matches!(state, State::Lobby { ref players, cursor } if players == &["Alice"] && cursor == 0);

        let state = do_action(state, &Action::AddPlayer("Bob".to_string()));
        assert_matches!(state, State::Lobby { ref players, cursor } if players == &["Alice", "Bob"] && cursor == 1);
    }

    #[test]
    fn test_do_action_set_cursor() {
        let state = State::Lobby {
            players: vec!["Alice".to_string(), "Bob".to_string()],
            cursor: 1,
        };
        let state = do_action(state, &Action::SetCursor(0));
        assert_matches!(state, State::Lobby { cursor, .. } if cursor == 0);
    }

    #[test]
    fn test_do_action_remove_player() {
        let state = State::Lobby {
            players: vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            cursor: 2,
        };
        let state = do_action(state, &Action::RemovePlayer(1));
        assert_matches!(state, State::Lobby { ref players, cursor } if players == &["Alice", "Charlie"] && cursor == 1);

        // Removing the last remaining player
        let single_player = State::Lobby {
            players: vec!["Alice".to_string()],
            cursor: 0,
        };
        let empty = do_action(single_player, &Action::RemovePlayer(0));
        assert_matches!(empty, State::Lobby { players, cursor } if players.is_empty() && cursor == 0);
    }

    #[test]
    fn test_replay() {
        let actions = vec![
            Action::AddPlayer("Alice".to_string()),
            Action::AddPlayer("Bob".to_string()),
            Action::SetCursor(0),
        ];
        let state = replay(&actions);
        assert_matches!(state, State::Lobby { ref players, cursor } if players == &["Alice", "Bob"] && cursor == 0);
    }

    #[test]
    fn test_score_formatting() {
        let score = Score::default();
        assert_eq!(score, SCORE_LOSS);
        assert_eq!(score.to_string(), "0.0");

        assert_eq!(SCORE_LOSS.to_string(), "0.0");
        assert_eq!(SCORE_DRAW.to_string(), "0.5");
        assert_eq!(SCORE_WIN.to_string(), "1.0");

        let score = SCORE_WIN + SCORE_WIN + SCORE_WIN + SCORE_DRAW;
        assert_eq!(score.to_string(), "3.5");
    }

    #[test]
    fn test_intent_to_action() {
        let mut state = State::default();

        // Empty state
        assert_eq!(intent_to_action(Intent::Up, &state, || None), None);
        assert_eq!(intent_to_action(Intent::Down, &state, || None), None);
        assert_eq!(
            intent_to_action(Intent::RemovePlayer, &state, || None),
            None
        );

        // Add unique player
        assert_eq!(
            intent_to_action(Intent::AddPlayer, &state, || Some("Alice".to_string())),
            Some(Action::AddPlayer("Alice".to_string()))
        );

        state = State::Lobby {
            players: vec!["Alice".to_string(), "Bob".to_string()],
            cursor: 0,
        };

        // Duplicate player rejected and instead the cursor is moved to it.
        assert_eq!(
            intent_to_action(Intent::AddPlayer, &state, || Some("Alice".to_string())),
            None
        );

        // Navigation
        assert_eq!(intent_to_action(Intent::Up, &state, || None), None);
        assert_eq!(
            intent_to_action(Intent::Down, &state, || None),
            Some(Action::SetCursor(1))
        );

        state = State::Lobby {
            players: vec!["Alice".to_string(), "Bob".to_string()],
            cursor: 1,
        };
        assert_eq!(
            intent_to_action(Intent::Up, &state, || None),
            Some(Action::SetCursor(0))
        );
        assert_eq!(intent_to_action(Intent::Down, &state, || None), None);

        // Remove
        assert_eq!(
            intent_to_action(Intent::RemovePlayer, &state, || None),
            Some(Action::RemovePlayer(1))
        );
    }

    #[test]
    fn test_score_formatting() {
        let score = Score::default();
        assert_eq!(score, SCORE_LOSS);
        assert_eq!(score.to_string(), "0.0");

        assert_eq!(SCORE_LOSS.to_string(), "0.0");
        assert_eq!(SCORE_DRAW.to_string(), "0.5");
        assert_eq!(SCORE_WIN.to_string(), "1.0");

        let score = SCORE_WIN + SCORE_WIN + SCORE_WIN + SCORE_DRAW;
        assert_eq!(score.to_string(), "3.5");
    }
}
