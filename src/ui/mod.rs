pub mod app;
pub mod edit;
pub mod fuzzy;
use app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use fuzzy::render_fuzzy_search;
use log::info;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph},
};
use std::{
    collections::HashMap,
    io,
    sync::mpsc::Receiver,
    time::{Duration, Instant},
};

use crate::{
    config::{GridPosition, Widget, WidgetType},
    nt::NtUpdate,
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
    CellConfig,
    LabelEdit,
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

        // Check if highlight should be hidden due to inactivity
        app.check_highlight_timeout();

        ////////////////////////////////////////
        // Key bindings
        ////////////////////////////////////////
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                // Update activity timestamp for any key press
                app.update_activity();

                match app.mode {
                    Window::Main => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('a') => app.enter_fuzzy_search(),
                        KeyCode::Char(' ') => app.toggle_pause(),
                        KeyCode::Char('h') => app.move_selection(0, -1),
                        KeyCode::Char('j') => app.move_selection(1, 0),
                        KeyCode::Char('k') => app.move_selection(-1, 0),
                        KeyCode::Char('l') => app.move_selection(0, 1),
                        KeyCode::Enter => app.enter_cell_config(),
                        _ => {}
                    },
                    Window::CellConfig => match key.code {
                        KeyCode::Esc => app.exit_cell_config(),
                        KeyCode::Char('s') => {
                            // Change source (topic) - enter fuzzy search
                            app.enter_fuzzy_search();
                        }
                        KeyCode::Char('l') => {
                            // Edit label - enter label edit mode
                            app.enter_label_edit();
                        }
                        _ => {}
                    },
                    Window::FuzzySearch => match key.code {
                        KeyCode::Esc => app.exit_fuzzy_search(),
                        KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.fuzzy_search.move_selection(-1);
                        }
                        KeyCode::Enter => {
                            if let Some(selected_topic) = app.handle_search_selection() {
                                info!("Added widget for topic: {}", selected_topic);
                            }
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
                    Window::LabelEdit => match key.code {
                        KeyCode::Esc => app.exit_label_edit(),
                        KeyCode::Enter => app.save_label(),
                        KeyCode::Backspace => {
                            app.label_edit.pop();
                        }
                        KeyCode::Char(c) => {
                            app.label_edit.push(c);
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
                    // Only update values if not paused
                    if !app.paused {
                        app.values.insert(key, value);
                    }
                    // Always update connection status and available topics
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
            Constraint::Min(8),    // Content
            Constraint::Length(1), // Right padding
        ])
        .split(main_layout[0])[1];

    // Calculate how many rows can fit in the available space
    // Each row needs 3 units of height
    let available_height = padded_area.height;
    let max_rows = (available_height / 3) as usize;

    // Update the app's max_rows value
    app.max_rows = max_rows;

    // Create constraints for the rows
    let mut row_constraints = Vec::new();
    for _ in 0..max_rows {
        row_constraints.push(Constraint::Length(3));
    }

    // Create a grid layout with fixed row heights (3 rows each) in the main content area
    let grid_constraints = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
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

    // Check if we have enough space for all configured widgets
    let mut warning_message = String::new();
    let max_widget_row = app
        .config
        .widgets
        .iter()
        .map(|w| w.position.row)
        .max()
        .unwrap_or(0);

    if max_widget_row >= max_rows {
        warning_message = format!(
            "Warning: Not enough space for all widgets! ({} rows needed)",
            max_widget_row + 1
        );
    }

    // Render widgets based on their configured positions
    for widget in &app.config.widgets {
        // Skip widgets that are outside the visible area
        if widget.position.row >= max_rows {
            continue;
        }

        let widget_area = get_widget_area(&grid_cells, &widget.position);

        // Create the widget block with a transparent background
        let block = Block::default()
            .title(Span::styled(
                widget.label.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Black));

        // Render the widget with the block
        match widget.widget_type {
            WidgetType::Text => {
                let default = &"No value".to_string();
                let value = app.values.get(&widget.topic);

                let text = Paragraph::new(value.unwrap_or(default).clone())
                    .block(block)
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(if value.is_some() {
                        Color::LightYellow
                    } else {
                        Color::Black
                    }));
                f.render_widget(text, widget_area);
            }
            // Add other widget type rendering here
            _ => {}
        }
    }

    // Highlight the selected cell if in main mode and highlight is visible
    if app.mode == Window::Main && app.highlight_visible {
        if let Some((row, col)) = app.selected_cell {
            if row < grid_cells.len() && col < grid_cells[0].len() {
                let selected_area = grid_cells[row][col];
                let highlight = Block::default().borders(Borders::ALL).border_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                );
                f.render_widget(highlight, selected_area);
            }
        }
    }

    // Get status color based on connection state and pause state
    let status_color = if app.paused {
        Color::Yellow
    } else {
        match app.connection_status {
            ConnectionStatus::Connected => Color::Green,
            ConnectionStatus::Connecting => Color::Yellow,
            ConnectionStatus::Disconnected => Color::Red,
        }
    };

    // Update the status text to show pause state and warnings
    let mut status_text = vec![
        Line::from(vec![
            "Status: ".bold(),
            match app.connection_status {
                ConnectionStatus::Connected => {
                    if app.paused {
                        "Paused".yellow().bold()
                    } else {
                        "Connected".green().bold()
                    }
                }
                ConnectionStatus::Connecting => "Connecting...".yellow().bold(),
                ConnectionStatus::Disconnected => "Disconnected".red().bold(),
            },
        ]),
        Line::from(vec![
            "Topics: ".bold(),
            format!("{}", app.available_topics.len()).cyan().bold(),
        ]),
    ];

    // Add warning if needed
    if !warning_message.is_empty() {
        status_text.push(Line::from(warning_message).yellow());
    }

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
        " [".dim(),
        "Space".yellow().bold(),
        "] ".dim(),
        "Pause".reset(),
        " [".dim(),
        "hjkl".blue().bold(),
        "] ".dim(),
        "Navigate".reset(),
        " [".dim(),
        "Enter".cyan().bold(),
        "] ".dim(),
        "Configure".reset(),
    ]);
    let help_bar = Paragraph::new(help_text)
        .style(Style::default())
        .alignment(Alignment::Center);
    f.render_widget(help_bar, main_layout[2]);

    // Render fuzzy search popup if active
    if app.mode == Window::FuzzySearch {
        render_fuzzy_search(f, app, size);
    }

    // Render cell configuration popup if active
    if app.mode == Window::CellConfig {
        edit::render_cell_config(f, app, size);
    }

    // Render label edit popup if active
    if app.mode == Window::LabelEdit {
        edit::render_label_edit(f, app, size);
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
