use crate::ui::app::App;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
};

// Add this function to render the cell configuration popup
pub fn render_cell_config(f: &mut ratatui::Frame, app: &App, size: Rect) {
    // Calculate popup dimensions
    let popup_width = 50;
    let popup_height = 10;

    let popup_x = (size.width - popup_width) / 2;
    let popup_y = (size.height - popup_height) / 2;

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Create a clear background for the popup
    f.render_widget(Clear, popup_area);

    // Get the widget at the selected cell
    let widget_info = if let Some(widget) = app.get_widget_at_selected_cell() {
        format!("Topic: {}\nLabel: {}", widget.topic, widget.label)
    } else {
        "No widget at selected cell".to_string()
    };

    // Render the popup
    let config_block = Block::default()
        .title("Widget Configuration")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let config_text = vec![
        Line::from(widget_info),
        Line::from(""),
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
        Line::from(vec![
            "[".dim(),
            "Esc".red().bold(),
            "] ".dim(),
            "Exit".reset(),
        ]),
    ];

    let config_paragraph = Paragraph::new(config_text)
        .block(config_block)
        .alignment(Alignment::Left);

    f.render_widget(config_paragraph, popup_area);
}

pub fn render_label_edit(f: &mut ratatui::Frame, app: &App, size: Rect) {
    // Calculate popup dimensions
    let popup_width = 50;
    let popup_height = 6;

    let popup_x = (size.width - popup_width) / 2;
    let popup_y = (size.height - popup_height) / 2;

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Create a clear background for the popup
    f.render_widget(Clear, popup_area);

    // Render the popup
    let edit_block = Block::default()
        .title("Edit Widget Label")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let input_text = format!(
        "{}{}",
        app.label_edit,
        if app.fuzzy_search.cursor_visible {
            "█"
        } else {
            " "
        }
    );

    let edit_text = vec![
        Line::from("Enter new label:"),
        Line::from(""),
        Line::from(input_text),
        Line::from(""),
        Line::from(vec![
            "[".dim(),
            "Enter".green().bold(),
            "] ".dim(),
            "Save".reset(),
            " [".dim(),
            "Esc".red().bold(),
            "] ".dim(),
            "Exit".reset(),
        ]),
    ];

    let edit_paragraph = Paragraph::new(edit_text)
        .block(edit_block)
        .alignment(Alignment::Left);

    f.render_widget(edit_paragraph, popup_area);
}
