mod render;
mod state;
mod swiss;
mod undo;

use ratatui::DefaultTerminal;
use ratatui::crossterm::event;
use ratatui::crossterm::event::Event;
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyEventKind;
use state::Action;
use state::Intent;
use state::do_action;
use state::intent_to_action;
use state::replay;
use std::fs::File;
use std::io;
use undo::UndoStack;

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut actions = maybe_load_actions().unwrap_or_default();
    let mut state = replay(actions.history());

    loop {
        terminal.draw(|frame| render::render(frame, &state))?;

        if let Event::Key(k) = event::read()?
            && k.kind == KeyEventKind::Press
        {
            match k.code {
                KeyCode::Char('q') => {
                    return serde_yaml::to_writer(File::create("actions.yaml")?, &actions)
                        .map_err(io::Error::other);
                }
                KeyCode::Char('?') => render::help_dialog(terminal, &state)?,

                KeyCode::Char('u') => {
                    actions.undo();
                    state = replay(actions.history());
                }
                KeyCode::Char('r') => {
                    if let Some(action) = actions.redo() {
                        state = do_action(state, action);
                    }
                }

                key_code => {
                    if let Some(action) = key_to_intent(key_code).and_then(|intent| {
                        intent_to_action(intent, &state, || render::prompt_player_name(terminal))
                    }) {
                        state = do_action(state, &action);
                        actions.push(action);
                    }
                }
            };
        }
    }
}

pub fn key_to_intent(key_code: KeyCode) -> Option<Intent> {
    match key_code {
        KeyCode::Char('a') => Some(Intent::AddPlayer),
        KeyCode::Char('d') => Some(Intent::RemovePlayer),
        KeyCode::Char('k') => Some(Intent::Up),
        KeyCode::Char('j') => Some(Intent::Down),
        KeyCode::Char('h') => Some(Intent::Left),
        KeyCode::Char('l') => Some(Intent::Right),
        KeyCode::Char('s') => Some(Intent::StartTournament),
        KeyCode::Char('n') => Some(Intent::NextRound),
        KeyCode::Char('c') => Some(Intent::ConcludeTournament),
        KeyCode::Char(' ') => Some(Intent::ResetMatch),
        _ => None,
    }
}

fn maybe_load_actions() -> Option<UndoStack<Action>> {
    serde_yaml::from_reader(File::open("actions.yaml").ok()?).ok()
}
