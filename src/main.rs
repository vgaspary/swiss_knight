use ratatui::DefaultTerminal;
use ratatui::Frame;
use ratatui::crossterm::event;
use ratatui::crossterm::event::Event;
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyEventKind;
use ratatui::crossterm::event::KeyModifiers;
use ratatui::layout::Alignment;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use std::io;

fn main() -> io::Result<()> {
    // Initializes the terminal and enters alternate screen / raw mode
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    // Restores terminal state even if run errors
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut message = String::from("Press any key (Esc to quit)");

    loop {
        terminal.draw(|frame| render(frame, &message))?;

        if let Event::Key(key_event) = event::read()? {
            // Crossterm uses `KeyEventKind::Press` for key-down events.
            // On platforms supporting key release/repeat, this filters to down/press events only.
            if key_event.kind == KeyEventKind::Press {
                let key_name = match key_event.code {
                    KeyCode::Char(' ') => "Space".to_string(),
                    KeyCode::Char(c) => c.to_string(),
                    KeyCode::Backspace => "Backspace".to_string(),
                    KeyCode::Enter => "Enter".to_string(),
                    KeyCode::Left => "Left".to_string(),
                    KeyCode::Right => "Right".to_string(),
                    KeyCode::Up => "Up".to_string(),
                    KeyCode::Down => "Down".to_string(),
                    KeyCode::Home => "Home".to_string(),
                    KeyCode::End => "End".to_string(),
                    KeyCode::PageUp => "PageUp".to_string(),
                    KeyCode::PageDown => "PageDown".to_string(),
                    KeyCode::Tab => "Tab".to_string(),
                    KeyCode::BackTab => "BackTab".to_string(),
                    KeyCode::Delete => "Delete".to_string(),
                    KeyCode::Insert => "Insert".to_string(),
                    KeyCode::F(n) => format!("F{n}"),
                    KeyCode::Esc => "Esc".to_string(),
                    other => format!("{other:?}"),
                };

                message = format!("{key_name} was pressed");

                // Exit on Esc or Ctrl+C
                if key_event.code == KeyCode::Esc
                    || (key_event.code == KeyCode::Char('c')
                        && key_event.modifiers.contains(KeyModifiers::CONTROL))
                {
                    break;
                }
            }
        }
    }

    Ok(())
}

fn render(frame: &mut Frame, message: &str) {
    let label = Paragraph::new(message).alignment(Alignment::Center).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Key Press Event"),
    );
    frame.render_widget(label, frame.area());
}
