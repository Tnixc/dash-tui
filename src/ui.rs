use crate::{fuzzy::FuzzySearch, nt::NtUpdate};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};
use std::{
    collections::HashMap,
    io,
    sync::mpsc::Receiver,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Window {
    Main,
    FuzzySearch,
}

pub struct App {
    pub values: HashMap<String, String>,
    pub connection_status: ConnectionStatus,
    pub available_topics: Vec<String>,
    pub mode: Window,
    pub fuzzy_search: FuzzySearch,
}
impl App {
    pub fn new() -> App {
        App {
            values: HashMap::new(),
            connection_status: ConnectionStatus::Disconnected,
            available_topics: Vec::new(),
            mode: Window::Main,
            fuzzy_search: FuzzySearch::new(),
        }
    }
}

pub fn run_ui(receiver: Receiver<NtUpdate>) -> Result<(), io::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();

    // Main loop
    let tick_rate = Duration::from_millis(20);
    let mut last_tick = Instant::now();

    loop {
        // Draw UI
        terminal.draw(|f| ui(f, &mut app))?;

        // Check for events (with timeout)
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        ////////////////////////////////////////
        // Key bindings
        ////////////////////////////////////////
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match app.mode {
                    Window::Main => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('a') => app.enter_fuzzy_search(),
                        _ => {}
                    },
                    Window::FuzzySearch => {
                        match key.code {
                            KeyCode::Esc => app.exit_fuzzy_search(),
                            KeyCode::Enter => {
                                if let Some(selected) = app.handle_search_selection() {
                                    // TODO: Add selected topic to subscription list
                                    todo!()
                                }
                            }
                            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                app.fuzzy_search.move_selection(-1);
                            }
                            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                app.fuzzy_search.move_selection(1);
                            }
                            KeyCode::Up => {
                                app.fuzzy_search.move_selection(-1);
                            }
                            KeyCode::Down => {
                                app.fuzzy_search.move_selection(1);
                            }
                            KeyCode::Backspace => {
                                app.fuzzy_search.input.pop();
                                app.fuzzy_search.update_matches(&app.available_topics);
                            }
                            KeyCode::Char(c) => {
                                app.fuzzy_search.input.push(c);
                                app.fuzzy_search.update_matches(&app.available_topics);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Check for updates from NT
        while let Ok(update) = receiver.try_recv() {
            match update {
                NtUpdate::KV(key, value) => {
                    app.values.insert(key, value);
                    // If we're receiving values, we must be connected
                    app.connection_status = ConnectionStatus::Connected;
                }
                NtUpdate::ConnectionStatus(status) => {
                    app.connection_status = status;
                }
                NtUpdate::AvailableTopics(topics) => {
                    app.available_topics = topics;
                    if app.mode == Window::FuzzySearch {
                        app.fuzzy_search.update_matches(&app.available_topics);
                    }
                }
            }
        }

        // Tick handling
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn ui(f: &mut ratatui::Frame, app: &mut App) {
    let size = f.area();

    // Create the layout with sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Min(1),    // NT values (takes up all available space)
                Constraint::Length(3), // Status bar
                Constraint::Length(1), // Help text
            ]
            .as_ref(),
        )
        .split(size);

    ////////////////////////////////////////
    // NT values
    ////////////////////////////////////////
    let nt_block = Block::default().title("NT Values").borders(Borders::ALL);
    let mut nt_lines = Vec::new();
    for (key, value) in &app.values {
        nt_lines.push(Line::from(vec![
            Span::raw(format!("{}: ", key)),
            Span::styled(value, Style::default().fg(Color::Yellow)),
        ]));
    }

    if nt_lines.is_empty() {
        nt_lines.push(Line::from(vec![Span::styled(
            "None",
            Style::default().fg(Color::Red),
        )]));
    }

    let nt_text = Paragraph::new(nt_lines)
        .block(nt_block)
        .alignment(Alignment::Left);

    f.render_widget(nt_text, chunks[0]);

    ////////////////////////////////////////
    // Render status bar
    ////////////////////////////////////////
    let (status_text, border_color) = match app.connection_status {
        ConnectionStatus::Connected => ("CONNECTED", Color::Green),
        ConnectionStatus::Disconnected => ("DISCONNECTED", Color::Red),
    };

    let status_block = Block::default()
        .title("Network Tables Status")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let status_text = Paragraph::new(Line::from(vec![Span::styled(
        format!("NT Status: {}", status_text),
        Style::default().fg(border_color),
    )]))
    .block(status_block)
    .alignment(Alignment::Center);

    f.render_widget(status_text, chunks[1]);

    ////////////////////////////////////////
    // Help text
    ////////////////////////////////////////
    let help_text = match app.mode {
        Window::Main => Line::from(vec![
            Span::raw("Press "),
            Span::styled("q", Style::default().fg(Color::Red)),
            Span::raw(" to quit"),
            Span::styled(" | ", Style::default().fg(Color::Black)),
            Span::styled("a", Style::default().fg(Color::Green)),
            Span::raw(" to search topics"),
        ]),
        Window::FuzzySearch => Line::from(vec![
            Span::raw("Press "),
            Span::styled("Esc", Style::default().fg(Color::Red)),
            Span::raw(" to cancel | "),
            Span::styled("Enter", Style::default().fg(Color::Green)),
            Span::raw(" to select | "),
            Span::styled("Ctrl+↑/↓", Style::default().fg(Color::Yellow)),
            Span::raw(" to navigate"),
        ]),
    };

    let help_paragraph = Paragraph::new(help_text).alignment(Alignment::Center);
    f.render_widget(help_paragraph, chunks[2]);

    ////////////////////////////////////////
    // Fuzzy Search (if active)
    ////////////////////////////////////////
    if app.mode == Window::FuzzySearch {
        // Calculate popup dimensions
        let popup_width = size.width.min(100).max(70);
        let popup_height = size.height.min(20).max(10);

        let popup_x = (size.width - popup_width) / 2;
        let popup_y = (size.height - popup_height) / 2;

        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        // Create a clear background for the popup
        f.render_widget(Clear, popup_area);

        // Split popup into search input and results list
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search input
                Constraint::Min(1),    // Results list
            ])
            .split(popup_area);

        // Render search input
        let input_block = Block::default()
            .title("Search NT Topics")
            .borders(Borders::ALL);

        let input_text = Paragraph::new(app.fuzzy_search.input.as_str())
            .style(Style::default())
            .block(input_block);

        f.render_widget(input_text, popup_layout[0]);

        // Render results list
        let results_block = Block::default()
            .title(format!(
                "Results ({} found)",
                app.fuzzy_search.matches.len()
            ))
            .borders(Borders::ALL);

        let list_items: Vec<ListItem> = app
            .fuzzy_search
            .matches
            .iter()
            .map(|topic| ListItem::new(topic.as_str()))
            .collect();

        let results_list = List::new(list_items).block(results_block).highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

        f.render_stateful_widget(
            results_list,
            popup_layout[1],
            &mut app.fuzzy_search.list_state,
        );
    }
}
