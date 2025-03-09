use crate::ui::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Clear, Padding, Paragraph},
};

// Add this function to render the cell configuration popup
pub fn render_cell_config(f: &mut ratatui::Frame, app: &App, size: Rect) {
    // Calculate popup dimensions - half of screen width/height with minimums
    let popup_width = (size.width / 2).max(50);
    let popup_height = 12; // Fixed height with room for two boxes and padding

    let popup_x = (size.width - popup_width) / 2;
    let popup_y = (size.height - popup_height) / 2;

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Create a clear background for the popup
    f.render_widget(Clear, popup_area);

    // Split popup vertically into two boxes (info and controls)
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Widget info box (2 rows)
            Constraint::Length(7), // Controls box
        ])
        .margin(0)
        .split(popup_area);

    // Get the widget at the selected cell
    let (topic, label) = if let Some(widget) = app.get_widget_at_selected_cell() {
        (widget.topic.clone(), widget.label.clone())
    } else {
        ("No widget selected".to_string(), "".to_string())
    };

    // Create info box with two rows
    let info_text = vec![
        Line::from(vec!["Label: ".bold(), label.reset()]),
        Line::from(vec!["Topic: ".bold(), topic.reset()]),
    ];

    let info_box = Paragraph::new(info_text)
        .block(
            Block::default()
                .title("Widget Info")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue))
                .padding(Padding::new(1, 0, 0, 0)),
        )
        .alignment(Alignment::Left);

    // Create controls box
    let controls_text = vec![
        Line::from(vec![
            "[".dim(),
            "s".green().bold(),
            "] ".dim(),
            "Change Source".reset(),
        ]),
        Line::from(vec![
            "[".dim(),
            "l".yellow().bold(),
            "] ".dim(),
            "Edit Label".reset(),
        ]),
        Line::from(""),
        Line::from(vec![
            "[".dim(),
            "Esc".red().bold(),
            "] ".dim(),
            "Exit".reset(),
        ]),
    ];

    let controls_box = Paragraph::new(controls_text)
        .block(
            Block::default()
                .title("Controls")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue))
                .padding(Padding::new(1, 0, 0, 0)),
        )
        .alignment(Alignment::Left);

    // Render both boxes
    f.render_widget(info_box, layout[0]);
    f.render_widget(controls_box, layout[1]);
}

pub fn render_label_edit(f: &mut ratatui::Frame, app: &App, size: Rect) {
    // Calculate popup dimensions - half of screen width/height with minimums
    let popup_width = (size.width / 2).max(50);
    let popup_height = 10; // Fixed height with room for input box and controls

    let popup_x = (size.width - popup_width) / 2;
    let popup_y = (size.height - popup_height) / 2;

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Create a clear background for the popup
    f.render_widget(Clear, popup_area);

    // Split the popup into sections
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input box
            Constraint::Length(5), // Controls
        ])
        .margin(0)
        .split(popup_area);

    // Create the input text with cursor
    let input_text = format!(
        "{}{}",
        app.label_edit,
        if app.cursor_visible { "_" } else { " " }
    );

    // Create input box
    let input_box = Paragraph::new(input_text)
        .block(
            Block::default()
                .title("Edit Label")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue))
                .padding(Padding::horizontal(1)),
        )
        .alignment(Alignment::Left);

    // Create controls box
    let help_text = vec![
        Line::from(vec![
            "[".dim(),
            "Enter".green().bold(),
            "] ".dim(),
            "Save".reset(),
        ]),
        Line::from(vec![
            "[".dim(),
            "Ctrl+D".yellow().bold(),
            "] ".dim(),
            "Clear".reset(),
        ]),
        Line::from(vec![
            "[".dim(),
            "Esc".red().bold(),
            "] ".dim(),
            "Cancel".reset(),
        ]),
    ];

    let controls_box = Paragraph::new(help_text)
        .block(
            Block::default()
                .title("Controls")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue))
                .padding(Padding::new(1, 0, 0, 0)),
        )
        .alignment(Alignment::Left);

    // Render both boxes
    f.render_widget(input_box, layout[0]);
    f.render_widget(controls_box, layout[1]);
}
