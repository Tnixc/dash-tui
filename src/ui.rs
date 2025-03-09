use crate::{
    app::App,
    config::{GridPosition, Widget, WidgetType},
    fuzzy::FuzzySearch,
    nt::NtUpdate,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph},
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
    Connecting,
    Disconnected,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Window {
    Main,
    FuzzySearch,
}

pub fn run_ui(receiver: Receiver<NtUpdate>) -> Result<(), io::Error> {
    let mut animation_counter = 0;
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();

    // Main loop
    let tick_rate = Duration::from_millis(5);
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
                    Window::FuzzySearch => match key.code {
                        KeyCode::Esc => app.exit_fuzzy_search(),
                        KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.fuzzy_search.move_selection(-1);
                        }
                        KeyCode::Enter => {
                            println!("Selected topic: {:?}", app.fuzzy_search.get_selected());
                            todo!()
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
                    },
                }
            }
        }

        // Check for updates from NT
        while let Ok(update) = receiver.try_recv() {
            match update {
                NtUpdate::KV(key, value) => {
                    let k = key.clone();
                    app.values.insert(key, value);
                    // If we're receiving values, we must be connected
                    app.connection_status = ConnectionStatus::Connected;
                    app.available_topics.insert(k);
                    if app.mode == Window::FuzzySearch {
                        app.fuzzy_search.update_matches(&app.available_topics);
                    }
                }
                NtUpdate::ConnectionStatus(status) => {
                    app.connection_status = status;
                }
            }
        }

        // Tick handling
        if last_tick.elapsed() >= tick_rate {
            if app.mode == Window::FuzzySearch && animation_counter % 50 == 0 {
                animation_counter += 1;
                app.fuzzy_search.cursor_visible = !app.fuzzy_search.cursor_visible;
            }
            animation_counter += 1;
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

    // Create main layout with status bar and help text
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // Main content
            Constraint::Length(3), // Status bar
            Constraint::Length(1), // Help text
        ])
        .split(size);

    // Add padding to the sides
    let padded_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(1), // Left padding
            Constraint::Min(10),   // Content
            Constraint::Length(1), // Right padding
        ])
        .split(main_layout[0])[1];

    // Create a 5x10 grid layout in the main content area
    let grid_constraints = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Ratio(1, 10),
            Constraint::Ratio(1, 10),
            Constraint::Ratio(1, 10),
            Constraint::Ratio(1, 10),
            Constraint::Ratio(1, 10),
            Constraint::Ratio(1, 10),
            Constraint::Ratio(1, 10),
            Constraint::Ratio(1, 10),
            Constraint::Ratio(1, 10),
            Constraint::Ratio(1, 10),
        ])
        .split(padded_area);

    let mut grid_cells = Vec::new();
    for row in grid_constraints.iter() {
        let cells = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Ratio(1, 5),
                Constraint::Ratio(1, 5),
                Constraint::Ratio(1, 5),
                Constraint::Ratio(1, 5),
                Constraint::Ratio(1, 5),
            ])
            .split(*row);
        grid_cells.push(cells.to_vec());
    }

    // Get status color based on connection state
    let status_color = match app.connection_status {
        ConnectionStatus::Connected => Color::Green,
        ConnectionStatus::Connecting => Color::Yellow,
        ConnectionStatus::Disconnected => Color::Red,
    };

    // Render widgets based on their configured positions
    for widget in &app.config.widgets {
        let widget_area = get_widget_area(&grid_cells, &widget.position);
        render_widget(f, widget, &app.values, widget_area);
    }

    let status_text = vec![
        Line::from(vec![
            "Status: ".bold(),
            match app.connection_status {
                ConnectionStatus::Connected => "Connected".green().bold(),
                ConnectionStatus::Connecting => "Connecting...".yellow().bold(),
                ConnectionStatus::Disconnected => "Disconnected".red().bold(),
            },
        ]),
        Line::from(vec![
            "Topics: ".bold(),
            format!("{}", app.available_topics.len()).cyan().bold(),
        ]),
    ];

    let status_bar = Paragraph::new(status_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(status_color))
                .padding(Padding::horizontal(2))
                .title("Status Bar")
                .title_alignment(Alignment::Center),
        )
        .alignment(Alignment::Left);
    f.render_widget(status_bar, main_layout[1]);

    // Render help text with more colors
    let help_text = Line::from(vec![
        "[".dim(),
        "q".red().bold(),
        "] ".dim(),
        "Quit".reset(),
        " [".dim(),
        "a".green().bold(),
        "] ".dim(),
        "Add Widget".reset(),
    ]);
    let help_bar = Paragraph::new(help_text)
        .style(Style::default())
        .alignment(Alignment::Center);
    f.render_widget(help_bar, main_layout[2]);

    // Render fuzzy search popup if active
    if app.mode == Window::FuzzySearch {
        render_fuzzy_search(f, app, size);
    }
}

fn get_widget_area(grid_cells: &[Vec<Rect>], pos: &GridPosition) -> Rect {
    let mut area = grid_cells[pos.row][pos.col];

    // If widget spans multiple cells, combine their areas
    if pos.row_span > 1 || pos.col_span > 1 {
        let end_row = (pos.row + pos.row_span - 1).min(9);
        let end_col = (pos.col + pos.col_span - 1).min(4);
        let bottom_right = grid_cells[end_row][end_col];

        area = Rect::new(
            area.x,
            area.y,
            bottom_right.x + bottom_right.width - area.x,
            bottom_right.y + bottom_right.height - area.y,
        );
    }

    area
}

fn render_widget(
    f: &mut ratatui::Frame,
    widget: &Widget,
    values: &HashMap<String, String>,
    area: Rect,
) {
    let default = &"N/A".to_string();
    let value = values.get(&widget.topic).unwrap_or(default);

    let block = Block::default()
        .title(Span::styled(
            widget.label.clone(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Black));

    match widget.widget_type {
        WidgetType::Text => {
            // Create inner area for content with padding
            let inner_area = block.inner(area);
            let content_area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Top padding
                    Constraint::Min(1),    // Content
                    Constraint::Length(1), // Bottom padding
                ])
                .split(inner_area)[1];

            let text = Paragraph::new(value.clone())
                .block(block)
                .alignment(Alignment::Center);
            f.render_widget(text, area);
        }
        // Add other widget type rendering here
        _ => {}
    }
}

fn render_fuzzy_search(f: &mut ratatui::Frame, app: &App, size: Rect) {
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
        .title("Add Widget")
        .borders(Borders::ALL)
        .padding(Padding::horizontal(1))
        .border_style(Style::new().blue());

    // Add blinking cursor to input text
    let input_text = if app.fuzzy_search.cursor_visible {
        app.fuzzy_search.input.as_str().to_owned() + "_"
    } else {
        app.fuzzy_search.input.clone()
    };

    let input_paragraph = Paragraph::new(input_text)
        .style(Style::default())
        .block(input_block);

    f.render_widget(input_paragraph, popup_layout[0]);

    // Render results list
    let results_block = Block::default()
        .title(format!(
            "Available Topics ({} found)",
            app.fuzzy_search.matches.len()
        ))
        .borders(Borders::ALL)
        .border_style(Style::new().blue());

    let items: Vec<ListItem> = app
        .fuzzy_search
        .matches
        .iter()
        .enumerate()
        .map(|(i, topic)| {
            let style = if Some(i) == app.fuzzy_search.list_state.selected() {
                Style::default()
                    .bg(Color::Black)
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(topic.clone()).style(style)
        })
        .collect();

    let list = List::new(items).block(results_block);

    // We need to use a stateful widget for the list selection
    f.render_stateful_widget(
        list,
        popup_layout[1],
        &mut app.fuzzy_search.list_state.clone(),
    );
}
