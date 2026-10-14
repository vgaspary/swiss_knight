mod undo;

use ratatui::DefaultTerminal;
use ratatui::Frame;
use ratatui::crossterm::event;
use ratatui::crossterm::event::Event;
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyEventKind;
use ratatui::layout::Alignment;
use ratatui::layout::Rect;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;
use std::io;
use undo::UndoStack;

#[derive(Clone, Debug)]
enum Intent {
    // Navigation
    Up,
    Down,

    // Lobby
    AddPlayer(String),
    RemovePlayer,
}

#[derive(Clone, Debug)]
enum Action {
    AddPlayer(String),
    RemovePlayer(usize),
    SetCursor(usize),
}

#[derive(Debug)]
struct State {
    players: Vec<String>,
    cursor: usize,
}

impl State {
    fn new() -> State {
        State {
            players: vec![],
            cursor: 0,
        }
    }
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut actions = UndoStack::new();
    let mut state = State {
        players: vec![],
        cursor: 0,
    };

    loop {
        terminal.draw(|frame| render(frame, &format!("Actions: {actions:?}\nState: {state:?}")))?;

        if let Event::Key(k) = event::read()?
            && k.kind == KeyEventKind::Press
        {
            match k.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('h') => todo!("show help floating window"),

                KeyCode::Char('u') => {
                    actions.undo();
                    state = undo_action(actions.history());
                }
                KeyCode::Char('r') => {
                    if let Some(action) = actions.redo() {
                        state = do_action(state, action);
                    }
                }

                key_code => {
                    if let Some(action) = key_to_intent(terminal, key_code)
                        .and_then(|intent| intent_to_action(intent, &state))
                    {
                        state = do_action(state, &action);
                        actions.push(action);
                    }
                }
            };
        }
    }
}

fn key_to_intent(terminal: &mut DefaultTerminal, key_code: KeyCode) -> Option<Intent> {
    match key_code {
        KeyCode::Char('a') => prompt_player_name(terminal)
            .ok()
            .flatten()
            .map(Intent::AddPlayer),
        KeyCode::Char('d') => Some(Intent::RemovePlayer),
        KeyCode::Char('k') => Some(Intent::Up),
        KeyCode::Char('j') => Some(Intent::Down),
        _ => None,
    }
}

fn intent_to_action(intent: Intent, state: &State) -> Option<Action> {
    match intent {
        Intent::Up if state.cursor > 0 => Some(Action::SetCursor(state.cursor - 1)),
        Intent::Down if state.cursor + 1 < state.players.len() => {
            Some(Action::SetCursor(state.cursor + 1))
        }
        Intent::AddPlayer(name) if state.players.iter().find(|&n| &name == n).is_none() => {
            Some(Action::AddPlayer(name))
        }
        Intent::RemovePlayer if !state.players.is_empty() => {
            Some(Action::RemovePlayer(state.cursor))
        }
        _ => None,
    }
}

fn do_action(State { players, .. }: State, action: &Action) -> State {
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
            let mut new_players = Vec::with_capacity(players.len() - 1);
            new_players.extend_from_slice(&players[0..*idx]);
            new_players.extend_from_slice(&players[idx + 1..]);
            let players = new_players;
            let cursor = (*idx).min(players.len() - 1);
            State { players, cursor }
        }
    }
}

fn undo_action(actions: &[Action]) -> State {
    actions.iter().fold(State::new(), |s, a| do_action(s, a))
}

fn prompt_player_name(terminal: &mut DefaultTerminal) -> io::Result<Option<String>> {
    let mut player_name = String::new();

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let popup_width = 40.min(area.width);
            let popup_height = 3.min(area.height);
            let popup_area = Rect::new(
                (area.width.saturating_sub(popup_width)) / 2,
                (area.height.saturating_sub(popup_height)) / 2,
                popup_width,
                popup_height,
            );

            frame.render_widget(Clear, popup_area);
            let block = Block::default().borders(Borders::ALL).title("Player Name");
            let input = Paragraph::new(player_name.as_str()).block(block);
            frame.render_widget(input, popup_area);

            let cursor_x = (popup_area.x + 1 + player_name.chars().count() as u16)
                .min(popup_area.right().saturating_sub(2));
            frame.set_cursor_position((cursor_x, popup_area.y + 1));
        })?;

        if let Event::Key(k) = event::read()?
            && k.kind == KeyEventKind::Press
        {
            match k.code {
                KeyCode::Enter => return Ok(Some(player_name)),
                KeyCode::Esc => return Ok(None),
                KeyCode::Backspace => {
                    player_name.pop();
                }
                KeyCode::Char(c) => {
                    player_name.push(c);
                }
                _ => {}
            }
        }
    }
}

fn render(frame: &mut Frame, message: &str) {
    let label = Paragraph::new(message).alignment(Alignment::Center).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Key Press Event"),
    );
    frame.render_widget(label, frame.area());
}
