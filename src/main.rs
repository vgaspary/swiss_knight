mod render;
mod state;
mod undo;

use ratatui::DefaultTerminal;
use ratatui::crossterm::event;
use ratatui::crossterm::event::Event;
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyEventKind;
use state::Intent;
use state::State;
use state::do_action;
use state::intent_to_action;
use state::undo_action;
use std::io;
use undo::UndoStack;

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut actions = UndoStack::new();
    let mut state = State::new();

    loop {
        terminal.draw(|frame| render::render(frame, &state))?;

        if let Event::Key(k) = event::read()?
            && k.kind == KeyEventKind::Press
        {
            match k.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('?') => render::help_dialog(terminal, &state)?,

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
        KeyCode::Char('a') => render::prompt_player_name(terminal)
            .ok()
            .flatten()
            .map(Intent::AddPlayer),
        KeyCode::Char('d') => Some(Intent::RemovePlayer),
        KeyCode::Char('k') => Some(Intent::Up),
        KeyCode::Char('j') => Some(Intent::Down),
        _ => None,
    }
}
