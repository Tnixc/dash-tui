use crate::nt::NtUpdate;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::{
    collections::HashMap,
    io,
    sync::mpsc::Receiver,
    time::{Duration, Instant},
};

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
        terminal.draw(|f| ui(f, &app))?;

        // Check for events (with timeout)
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }

        // Check for updates from NT
        while let Ok(update) = receiver.try_recv() {
            match update {
                NtUpdate::KV(key, value) => {
                    app.values.insert(key, value);
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

struct App {
    values: HashMap<String, String>,
}

impl App {
    fn new() -> App {
        App {
            values: HashMap::new(),
        }
    }
}

fn ui(f: &mut ratatui::Frame, app: &App) {
    let size = f.area();

    // Create the layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(10), // Increase for more NT values
                Constraint::Min(0),
            ]
            .as_ref(),
        )
        .split(size);

    // Create the NT values block
    let nt_block = Block::default().title("NT values").borders(Borders::ALL);

    // Build a vector of lines for the NT values
    let mut nt_lines = Vec::new();

    // Add any other values not explicitly ordered
    for (key, value) in &app.values {
        nt_lines.push(Line::from(vec![
            Span::raw(format!("{}: ", key)),
            Span::styled(value, Style::default().fg(Color::Yellow)),
        ]));
    }

    // If we have no values, show a placeholder
    if nt_lines.is_empty() {
        nt_lines.push(Line::from(vec![Span::styled(
            "No NT values received yet",
            Style::default().fg(Color::Red),
        )]));
    }

    let nt_text = Paragraph::new(nt_lines)
        .block(nt_block)
        .alignment(Alignment::Left);

    f.render_widget(nt_text, chunks[0]);

    // Render additional help text
    let help_block = Block::default().title("Help").borders(Borders::ALL);

    let help_text = Paragraph::new(Line::from(vec![
        Span::raw("Press "),
        Span::styled("q", Style::default().fg(Color::Red)),
        Span::raw(" to quit"),
    ]))
    .block(help_block)
    .alignment(Alignment::Center);

    f.render_widget(help_text, chunks[1]);
}
