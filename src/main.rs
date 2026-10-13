use ratatui::DefaultTerminal;
use ratatui::Frame;
use ratatui::crossterm::event;
use ratatui::crossterm::event::Event;
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyEventKind;
use ratatui::layout::Alignment;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use std::io;

#[derive(Clone, Debug)]
enum Action {
    // Navigation
    Up,
    Down,

    // Lobby
    AddPlayer(String),
    RemovePlayer(usize),
}

fn main() -> io::Result<()> {
    // Initializes the terminal and enters alternate screen / raw mode
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    // Restores terminal state even if run errors
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut actions: Vec<Action> = vec![];
    let mut redo_actions: Vec<Action> = vec![];

    loop {
        terminal.draw(|frame| render(frame, &format!("Actions: {actions:?}")))?;

        if let Event::Key(k) = event::read()?
            && k.kind == KeyEventKind::Press
        {
            match k.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('h') => todo!("show help floating window"),

                KeyCode::Char('u') => actions.pop().inspect(|a| redo_actions.push(a.clone())),
                KeyCode::Char('U') => redo_actions.pop().inspect(|a| actions.push(a.clone())),

                key_code => {
                    redo_actions.clear();
                    key_to_action(key_code).inspect(|a| actions.push(a.clone()))
                }
            };
        }
    }
}

fn key_to_action(key_code: KeyCode) -> Option<Action> {
    match key_code {
        KeyCode::Char('a') => Some(Action::AddPlayer("Poxos".to_string())),
        KeyCode::Char('r') => Some(Action::RemovePlayer(0)),
        KeyCode::Char('k') => Some(Action::Up),
        KeyCode::Char('j') => Some(Action::Down),
        _ => None,
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
