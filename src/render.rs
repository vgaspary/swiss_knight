use crate::state::Score;
use crate::state::State;
use ratatui::DefaultTerminal;
use ratatui::Frame;
use ratatui::crossterm::event;
use ratatui::crossterm::event::Event;
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyEventKind;
use ratatui::layout::Alignment;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
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

    match state {
        State::Lobby { players, cursor } => {
            if players.is_empty() {
                let mut help_text = vec![
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
                ];
                help_text.extend(keybinding_lines());

                let help = Paragraph::new(help_text)
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Tournament Lobby"),
                    );
                frame.render_widget(help, area);
            } else {
                let items: Vec<ListItem> = players
                    .iter()
                    .enumerate()
                    .map(|(i, name)| ListItem::new(format!("{}. {}", i + 1, name)))
                    .collect();

                let list = List::new(items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(format!("Tournament Lobby ({} players)", players.len())),
                    )
                    .highlight_style(
                        Style::default()
                            .add_modifier(Modifier::REVERSED)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("> ");

                let mut list_state = ListState::default();
                list_state.select(Some((*cursor).min(players.len() - 1)));

                frame.render_stateful_widget(list, area, &mut list_state);
            }
        }
        State::InProgress {
            players,
            cursor,
            prev_rounds,
            current_round,
        } => {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(area);

            let scores = crate::swiss::score(prev_rounds, Some(current_round));
            let mut rankings: Vec<(usize, Score)> = (0..players.len())
                .map(|i| (i, scores.get(&i).cloned().unwrap_or_default()))
                .collect();
            // Sort by score descending, then initial pairing index ascending for stability.
            rankings.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

            let mut rounds = prev_rounds.clone();
            rounds.push(current_round.clone());
            render_scoreboard(frame, players, &rankings, chunks[0]);
            render_rounds(frame, players, &rounds, Some(*cursor), None, chunks[1]);
        }
        State::Concluded {
            players,
            rankings,
            rounds,
            scroll,
        } => {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(area);

            render_scoreboard(frame, players, rankings, chunks[0]);
            render_rounds(frame, players, rounds, None, Some(*scroll), chunks[1]);
        }
    }
}

/// Renders the scoreboard panel in the order given by `rankings`.
fn render_scoreboard(
    frame: &mut Frame,
    players: &[String],
    rankings: &[(usize, Score)],
    area: Rect,
) {
    let items: Vec<ListItem> = rankings
        .iter()
        .map(|(idx, score)| {
            let name = players.get(*idx).map(|n| n.as_str()).unwrap_or("?");
            let width = area.width.saturating_sub(4) as usize; // borders + padding
            let score_str = score.to_string();
            let name_width = width.saturating_sub(score_str.len() + 1);
            ListItem::new(format!("{name:<name_width$} {score_str}"))
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Scoreboard"));
    frame.render_widget(list, area);
}

/// Renders the rounds/matches panel.
///
/// The list is a flat tree: round headers followed by their match rows.
/// If `cursor` is `Some`, the last round is treated as the active editable round
/// and the cursor selects among its match rows. If `cursor` is `None`, all rounds
/// are rendered as non-editable and `scroll_offset` controls the vertical scroll position.
fn render_rounds(
    frame: &mut Frame,
    players: &[String],
    rounds: &[crate::state::Round],
    cursor: Option<usize>,
    scroll_offset: Option<usize>,
    area: Rect,
) {
    // Build a flat list of displayable rows, tracking which are match rows.
    let mut items: Vec<ListItem> = vec![];
    let mut match_row_indices: Vec<usize> = vec![]; // maps match index -> flat list index

    let last_round_idx = rounds.len().saturating_sub(1);

    for (round_idx, round) in rounds.iter().enumerate() {
        let is_current_round = cursor.is_some() && round_idx == last_round_idx;
        let header_style = if is_current_round {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC)
        };
        items.push(ListItem::new(Line::from(Span::styled(
            format!(" Round {}", round_idx + 1),
            header_style,
        ))));

        let match_style = if is_current_round {
            Style::default()
        } else {
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC)
        };

        for m in &round.matches {
            let p1_name = players.get(m.p1).map(|n| n.as_str()).unwrap_or("?");
            let p2_name = players.get(m.p2).map(|n| n.as_str()).unwrap_or("?");
            let (p1_score, p2_score) = match m.result {
                None => (".".to_string(), ".".to_string()),
                Some(crate::state::MatchResult::WinP1) => ("1.0".to_string(), "0.0".to_string()),
                Some(crate::state::MatchResult::Draw) => ("0.5".to_string(), "0.5".to_string()),
                Some(crate::state::MatchResult::WinP2) => ("0.0".to_string(), "1.0".to_string()),
            };
            let line = if is_current_round {
                match_row_indices.push(items.len());
                Line::from(vec![
                    Span::styled(" │ ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("{p1_name}  {p1_score} - {p2_score}  {p2_name}"),
                        match_style,
                    ),
                ])
            } else {
                Line::from(Span::styled(
                    format!("   {p1_name}  {p1_score} - {p2_score}  {p2_name}"),
                    match_style,
                ))
            };
            items.push(ListItem::new(line));
        }

        if let Some(player) = round.bye {
            let player_name = players.get(player).map(|n| n.as_str()).unwrap_or("?");
            let label = format!("   {player_name}  1.0 (bye)");
            items.push(ListItem::new(Line::from(Span::styled(
                label,
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::ITALIC),
            ))));
        }
    }

    // Map the logical cursor (match index) to the flat list index.
    let selected_flat = cursor.and_then(|c| match_row_indices.get(c).copied());
    let total_items = items.len();
    let inner_height = area.height.saturating_sub(2) as usize; // top & bottom borders

    let max_offset = total_items.saturating_sub(inner_height);
    let offset = match scroll_offset {
        Some(scroll) => scroll.min(max_offset),
        None => max_offset,
    };
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Rounds"))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    let mut list_state = ListState::default()
        .with_offset(offset)
        .with_selected(selected_flat);
    frame.render_stateful_widget(list, area, &mut list_state);
}

/// Prompts the user to enter a player name in a centered floating input widget.
pub fn prompt_player_name(terminal: &mut DefaultTerminal) -> Option<String> {
    let mut player_name = String::new();

    loop {
        terminal
            .draw(|frame| {
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
            })
            .ok()?;

        if let Event::Key(k) = event::read().ok()?
            && k.kind == KeyEventKind::Press
        {
            match k.code {
                KeyCode::Enter => return Some(player_name),
                KeyCode::Esc => return None,
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

/// Displays the help floating dialog listing all keybindings until dismissed.
pub fn show_help(terminal: &mut DefaultTerminal, state: &State) -> io::Result<()> {
    loop {
        terminal.draw(|frame| {
            render(frame, state);
            render_help_dialog(frame);
        })?;

        if let Event::Key(k) = event::read()?
            && k.kind == KeyEventKind::Press
        {
            match k.code {
                KeyCode::Esc | KeyCode::Char('?' | 'q' | 'Q') | KeyCode::Enter => {
                    return Ok(());
                }
                _ => {}
            }
        }
    }
}

/// Alias for [`show_help`].
pub fn help_dialog(terminal: &mut DefaultTerminal, state: &State) -> io::Result<()> {
    show_help(terminal, state)
}

/// Renders the help dialog in a centered floating window.
pub fn render_help_dialog(frame: &mut Frame) {
    let area = frame.area();
    let popup_width = 40.min(area.width);
    let popup_height = 14.min(area.height);
    let popup_area = Rect::new(
        (area.width.saturating_sub(popup_width)) / 2,
        (area.height.saturating_sub(popup_height)) / 2,
        popup_width,
        popup_height,
    );

    let mut help_text = vec![Line::from("")];
    help_text.extend(keybinding_lines());
    help_text.push(Line::from(""));
    help_text.push(Line::from(Span::styled(
        "Press Esc or ? to close",
        Style::default().fg(Color::DarkGray),
    )));

    frame.render_widget(Clear, popup_area);
    let block = Block::default().borders(Borders::ALL).title("Help");
    let dialog = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .block(block);
    frame.render_widget(dialog, popup_area);
}

fn keybinding_lines() -> Vec<Line<'static>> {
    vec![
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
                "  ?  ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Help"),
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
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn buffer_to_string(terminal: &Terminal<TestBackend>) -> String {
        let buffer = terminal.backend().buffer();
        let mut s = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                s.push_str(buffer[(x, y)].symbol());
            }
            s.push('\n');
        }
        s
    }

    #[test]
    fn test_render_empty_lobby() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = State::default();

        terminal.draw(|frame| render(frame, &state)).unwrap();
        let content = buffer_to_string(&terminal);
        assert!(content.contains("Tournament Lobby"));
        assert!(content.contains("No players in the lobby yet."));
        assert!(content.contains("Keybindings:"));
        assert!(content.contains("Help"));
    }

    #[test]
    fn test_render_lobby_with_players() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = State::Lobby {
            players: vec!["Alice".to_string(), "Bob".to_string()],
            cursor: 1,
        };

        terminal.draw(|frame| render(frame, &state)).unwrap();
        let content = buffer_to_string(&terminal);
        assert!(content.contains("Tournament Lobby (2 players)"));
        assert!(content.contains("1. Alice"));
        assert!(content.contains("2. Bob"));
    }

    #[test]
    fn test_render_help_dialog() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|frame| render_help_dialog(frame)).unwrap();
        let content = buffer_to_string(&terminal);
        assert!(content.contains("Help"));
        assert!(content.contains("Add player"));
        assert!(content.contains("Remove player"));
        assert!(content.contains("Move cursor down"));
        assert!(content.contains("Move cursor up"));
        assert!(content.contains("Undo"));
        assert!(content.contains("Redo"));
        assert!(content.contains("Quit"));
        assert!(content.contains("Press Esc or ? to close"));
    }

    #[test]
    fn test_render_help_dialog_over_state() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = State::Lobby {
            players: vec!["Alice".to_string()],
            cursor: 0,
        };

        terminal
            .draw(|frame| {
                render(frame, &state);
                render_help_dialog(frame);
            })
            .unwrap();
        let content = buffer_to_string(&terminal);
        // The background has the tournament lobby
        assert!(content.contains("Tournament Lobby (1 players)"));
        // And the floating dialog is rendered
        assert!(content.contains("Help"));
        assert!(content.contains("Press Esc or ? to close"));
    }

    #[test]
    fn test_render_help_dialog_small_screen() {
        let backend = TestBackend::new(20, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render_help_dialog(frame)).unwrap();
    }

    #[test]
    fn test_render_in_progress_bye_and_current_round_cursor() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = State::InProgress {
            players: vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            cursor: 0,
            prev_rounds: vec![crate::state::Round {
                matches: vec![crate::state::Match {
                    p1: 0,
                    p2: 1,
                    result: Some(crate::state::MatchResult::WinP1),
                }],
                bye: Some(2),
            }],
            current_round: crate::state::Round {
                matches: vec![crate::state::Match {
                    p1: 2,
                    p2: 0,
                    result: None,
                }],
                bye: Some(1),
            },
        };

        terminal.draw(|frame| render(frame, &state)).unwrap();
        let content = buffer_to_string(&terminal);
        // Bye rows display player names rather than numeric indices
        assert!(content.contains("Charlie  1.0 (bye)"));
        assert!(content.contains("Bob  1.0 (bye)"));
        // Cursor (> ) and active gutter marker (│) select the match in the current round (Round 2)
        assert!(content.contains(">  │ Charlie  . - .  Alice"));
    }

    #[test]
    fn test_render_concluded() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = State::Concluded {
            players: vec!["Alice".to_string(), "Bob".to_string()],
            // Bob (idx 1) ranked ahead of Alice (idx 0) even with equal score (e.g., via Buchholz)
            rankings: vec![
                (1, crate::state::SCORE_WIN),
                (0, crate::state::SCORE_WIN),
            ],
            rounds: vec![crate::state::Round {
                matches: vec![crate::state::Match {
                    p1: 0,
                    p2: 1,
                    result: Some(crate::state::MatchResult::Draw),
                }],
                bye: None,
            }],
            scroll: 0,
        };

        terminal.draw(|frame| render(frame, &state)).unwrap();
        let content = buffer_to_string(&terminal);
        assert!(content.contains("Scoreboard"));
        assert!(content.contains("Rounds"));
        // Bob appears before Alice in the scoreboard according to `rankings`
        let bob_pos = content.find("Bob").unwrap();
        let alice_pos = content.find("Alice").unwrap();
        assert!(bob_pos < alice_pos);
        // No active cursor or editable gutter bar in concluded state
        assert!(!content.contains("> "));
        assert!(!content.contains(" │ "));
        assert!(content.contains("   Alice  0.5 - 0.5  Bob"));
    }

    #[test]
    fn test_render_rounds_autoscrolls_to_show_whole_current_round() {
        // Height 8 gives inner height 6 (after top/bottom borders).
        // With 3 rounds of 2 matches + 1 bye each (4 lines per round = 12 lines total),
        // the widget should autoscroll to the bottom so the entire Round 3 (including its bye)
        // is visible even when the cursor is on the first match of Round 3.
        let backend = TestBackend::new(80, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        let round = crate::state::Round {
            matches: vec![
                crate::state::Match {
                    p1: 0,
                    p2: 1,
                    result: Some(crate::state::MatchResult::WinP1),
                },
                crate::state::Match {
                    p1: 2,
                    p2: 3,
                    result: Some(crate::state::MatchResult::WinP2),
                },
            ],
            bye: Some(4),
        };
        let state = State::InProgress {
            players: vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
                "Dave".to_string(),
                "Eve".to_string(),
            ],
            cursor: 0,
            prev_rounds: vec![round.clone(), round.clone()],
            current_round: round,
        };

        terminal.draw(|frame| render(frame, &state)).unwrap();
        let content = buffer_to_string(&terminal);
        assert!(content.contains("Round 3"));
        assert!(content.contains(">  │ Alice  1.0 - 0.0  Bob"));
        assert!(content.contains(" │ Charlie  0.0 - 1.0  Dave"));
        assert!(content.contains("Eve  1.0 (bye)"));
    }

    #[test]
    fn test_render_concluded_scrolls_and_clamps() {
        let backend = TestBackend::new(80, 5); // inner height = 3 rows
        let mut terminal = Terminal::new(backend).unwrap();
        let round = crate::state::Round {
            matches: vec![crate::state::Match {
                p1: 0,
                p2: 1,
                result: Some(crate::state::MatchResult::WinP1),
            }],
            bye: None,
        };
        // 2 rounds * 2 rows = 4 rows total; inner height = 3 -> max_offset = 1
        let state = State::Concluded {
            players: vec!["Alice".to_string(), "Bob".to_string()],
            rankings: vec![
                (0, crate::state::SCORE_WIN + crate::state::SCORE_WIN),
                (1, crate::state::SCORE_LOSS),
            ],
            rounds: vec![round.clone(), round],
            scroll: 100, // Even if scroll exceeds max_offset, render clamps to max_offset = 1
        };

        terminal.draw(|frame| render(frame, &state)).unwrap();
        let content = buffer_to_string(&terminal);
        // Round 1 header scrolled off the top, Round 2 is visible, and bottom is not empty
        assert!(!content.contains("Round 1"));
        assert!(content.contains("Round 2"));
    }
}
