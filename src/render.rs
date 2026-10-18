use crate::state::State;
use ratatui::DefaultTerminal;
use ratatui::Frame;
use ratatui::crossterm::event;
use ratatui::crossterm::event::Event;
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyEventKind;
use ratatui::layout::Alignment;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::widgets::Paragraph;
use std::io;

/// Renders the tournament lobby state.
///
/// If there are no players in the lobby, a help widget listing all available
/// keybindings is displayed. Otherwise, a selectable list of players is rendered
/// with the active item at the cursor position highlighted.
pub fn render(frame: &mut Frame, state: &State) {
    let area = frame.area();

    if state.players.is_empty() {
        let help_text = vec![
            Line::from(Span::styled(
                "No players in the lobby yet.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Keybindings:",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "  a  ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("Add player"),
            ]),
            Line::from(vec![
                Span::styled(
                    "  d  ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("Remove player"),
            ]),
            Line::from(vec![
                Span::styled(
                    "  j  ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("Move cursor down"),
            ]),
            Line::from(vec![
                Span::styled(
                    "  k  ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("Move cursor up"),
            ]),
            Line::from(vec![
                Span::styled(
                    "  u  ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("Undo"),
            ]),
            Line::from(vec![
                Span::styled(
                    "  r  ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("Redo"),
            ]),
            Line::from(vec![
                Span::styled(
                    "  q  ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("Quit"),
            ]),
        ];

        let help = Paragraph::new(help_text)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Tournament Lobby"),
            );
        frame.render_widget(help, area);
    } else {
        let items: Vec<ListItem> = state
            .players
            .iter()
            .enumerate()
            .map(|(i, name)| ListItem::new(format!("{}. {}", i + 1, name)))
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(format!(
                "Tournament Lobby ({} players)",
                state.players.len()
            )))
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut list_state = ListState::default();
        list_state.select(Some(state.cursor.min(state.players.len() - 1)));

        frame.render_stateful_widget(list, area, &mut list_state);
    }
}

/// Prompts the user to enter a player name in a centered floating input widget.
pub fn prompt_player_name(terminal: &mut DefaultTerminal) -> io::Result<Option<String>> {
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
