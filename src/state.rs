#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    // Navigation
    Up,
    Down,

    // Lobby
    AddPlayer(String),
    RemovePlayer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    AddPlayer(String),
    RemovePlayer(usize),
    SetCursor(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub players: Vec<String>,
    pub cursor: usize,
}

impl State {
    pub fn new() -> State {
        State {
            players: vec![],
            cursor: 0,
        }
    }
}

pub fn do_action(State { players, .. }: State, action: &Action) -> State {
    match action {
        Action::SetCursor(idx) => State {
            players,
            cursor: *idx,
        },
        Action::AddPlayer(name) => {
            let mut players = players;
            players.push(name.clone());
            let cursor = players.len() - 1;
            State { players, cursor }
        }
        Action::RemovePlayer(idx) => {
            let mut new_players = Vec::with_capacity(players.len().saturating_sub(1));
            new_players.extend_from_slice(&players[0..*idx]);
            new_players.extend_from_slice(&players[idx + 1..]);
            let players = new_players;
            let cursor = if players.is_empty() {
                0
            } else {
                (*idx).min(players.len() - 1)
            };
            State { players, cursor }
        }
    }
}

pub fn undo_action(actions: &[Action]) -> State {
    actions.iter().fold(State::new(), |s, a| do_action(s, a))
}

pub fn intent_to_action(intent: Intent, state: &State) -> Option<Action> {
    match intent {
        Intent::Up if state.cursor > 0 => Some(Action::SetCursor(state.cursor - 1)),
        Intent::Down if state.cursor + 1 < state.players.len() => {
            Some(Action::SetCursor(state.cursor + 1))
        }
        Intent::AddPlayer(name) => match state.players.iter().position(|n| &name == n) {
            None => Some(Action::AddPlayer(name)),
            Some(idx) if idx != state.cursor => Some(Action::SetCursor(idx)),
            _ => None,
        }
        Intent::RemovePlayer if !state.players.is_empty() => {
            Some(Action::RemovePlayer(state.cursor))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state_is_empty() {
        let state = State::new();
        assert!(state.players.is_empty());
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn test_do_action_add_player() {
        let state = State::new();
        let state = do_action(state, &Action::AddPlayer("Alice".to_string()));
        assert_eq!(state.players, vec!["Alice".to_string()]);
        assert_eq!(state.cursor, 0);

        let state = do_action(state, &Action::AddPlayer("Bob".to_string()));
        assert_eq!(state.players, vec!["Alice".to_string(), "Bob".to_string()]);
        assert_eq!(state.cursor, 1);
    }

    #[test]
    fn test_do_action_set_cursor() {
        let state = State {
            players: vec!["Alice".to_string(), "Bob".to_string()],
            cursor: 1,
        };
        let state = do_action(state, &Action::SetCursor(0));
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn test_do_action_remove_player() {
        let state = State {
            players: vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            cursor: 2,
        };
        let state = do_action(state, &Action::RemovePlayer(1));
        assert_eq!(
            state.players,
            vec!["Alice".to_string(), "Charlie".to_string()]
        );
        assert_eq!(state.cursor, 1);

        // Removing the last remaining player
        let single_player = State {
            players: vec!["Alice".to_string()],
            cursor: 0,
        };
        let empty = do_action(single_player, &Action::RemovePlayer(0));
        assert!(empty.players.is_empty());
        assert_eq!(empty.cursor, 0);
    }

    #[test]
    fn test_undo_action_replays_history() {
        let actions = vec![
            Action::AddPlayer("Alice".to_string()),
            Action::AddPlayer("Bob".to_string()),
            Action::SetCursor(0),
        ];
        let state = undo_action(&actions);
        assert_eq!(state.players, vec!["Alice".to_string(), "Bob".to_string()]);
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn test_intent_to_action() {
        let mut state = State::new();

        // Empty state
        assert_eq!(intent_to_action(Intent::Up, &state), None);
        assert_eq!(intent_to_action(Intent::Down, &state), None);
        assert_eq!(intent_to_action(Intent::RemovePlayer, &state), None);

        // Add unique player
        assert_eq!(
            intent_to_action(Intent::AddPlayer("Alice".to_string()), &state),
            Some(Action::AddPlayer("Alice".to_string()))
        );

        state.players.push("Alice".to_string());
        state.players.push("Bob".to_string());
        state.cursor = 0;

        // Duplicate player rejected and instead the cursor is moved to it.
        assert_eq!(
            intent_to_action(Intent::AddPlayer("Alice".to_string()), &state),
            None
        );

        // Navigation
        assert_eq!(intent_to_action(Intent::Up, &state), None);
        assert_eq!(
            intent_to_action(Intent::Down, &state),
            Some(Action::SetCursor(1))
        );

        state.cursor = 1;
        assert_eq!(
            intent_to_action(Intent::Up, &state),
            Some(Action::SetCursor(0))
        );
        assert_eq!(intent_to_action(Intent::Down, &state), None);

        // Remove
        assert_eq!(
            intent_to_action(Intent::RemovePlayer, &state),
            Some(Action::RemovePlayer(1))
        );
    }
}
