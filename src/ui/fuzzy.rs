use std::collections::HashSet;

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Padding, Paragraph};

use crate::config::{Widget, WidgetType};
use crate::ui::Window;
use crate::ui::app::App;

pub struct Matcher {
    matcher: SkimMatcherV2,
}

impl Matcher {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
        }
    }

    pub fn match_items<'a>(&self, query: &str, items: &'a [String]) -> Vec<(i64, &'a String)> {
        let mut matches: Vec<_> = items
            .iter()
            .filter_map(|item| {
                self.matcher
                    .fuzzy_match(item, query)
                    .map(|score| (score, item))
            })
            .collect();

        // Sort by score (highest first)
        matches.sort_by(|a, b| b.0.cmp(&a.0));
        matches
    }
}

impl App {
    pub fn enter_fuzzy_search(&mut self) {
        self.mode = Window::FuzzySearch;
        // Initialize matches with all available topics
        self.fuzzy_search.update_matches(&self.available_topics);
    }

    pub fn exit_fuzzy_search(&mut self) {
        self.mode = Window::Main;
        self.fuzzy_search.input.clear();
    }

    pub fn handle_search_selection(&mut self) -> Option<String> {
        if let Some(selected_topic) = self.fuzzy_search.get_selected().cloned() {
            // If we're in cell config mode, update the existing widget
            if self.mode == Window::CellConfig {
                if let Some(widget) = self.get_widget_at_selected_cell_mut() {
                    widget.topic = selected_topic.clone();
                    let _ = self.config.save();
                    self.exit_fuzzy_search();
                    self.exit_cell_config();
                    return Some(selected_topic);
                }
            }

            // Otherwise create a new widget
            let widget = Widget {
                topic: selected_topic.clone(),
                label: selected_topic.clone(),
                widget_type: WidgetType::Text,
                position: self.find_next_grid_position(),
            };

            let _ = self.add_widget(widget);
            self.exit_fuzzy_search();
            Some(selected_topic)
        } else {
            None
        }
    }
}

pub struct FuzzySearch {
    pub input: String,
    pub matcher: Matcher,
    pub matches: Vec<String>,
    pub selected_index: usize,
    pub list_state: ListState,
}

impl FuzzySearch {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            input: String::new(),
            matcher: Matcher::new(),
            matches: Vec::new(),
            selected_index: 0,
            list_state: list_state,
        }
    }

    pub fn update_matches(&mut self, available_topics: &HashSet<String>) {
        let mut vec = available_topics.iter().cloned().collect::<Vec<_>>();
        if self.input.is_empty() {
            // If empty query, show all topics sorted alphabetically
            vec.sort();
            self.matches = vec;
        } else {
            // Otherwise do fuzzy search with score-based sorting
            let matches = self.matcher.match_items(&self.input, &vec);
            self.matches = matches.into_iter().map(|(_, item)| item.clone()).collect();
        }

        // Reset selection or adjust if out of bounds
        if self.matches.is_empty() {
            self.selected_index = 0;
            self.list_state.select(None);
        } else {
            if self.selected_index >= self.matches.len() {
                self.selected_index = self.matches.len() - 1;
            }
            self.list_state.select(Some(self.selected_index));
        }
    }

    pub fn get_selected(&self) -> Option<&String> {
        self.matches.get(self.selected_index)
    }

    pub fn move_selection(&mut self, offset: isize) {
        if self.matches.is_empty() {
            return;
        }

        let len = self.matches.len();
        let current = self.selected_index;

        let new_index = if offset.is_negative() {
            if current == 0 {
                len - 1 // Wrap to end
            } else {
                current - offset.unsigned_abs() as usize
            }
        } else {
            (current + offset as usize) % len
        };

        self.selected_index = new_index;
        self.list_state.select(Some(new_index));
    }
}

pub fn render_fuzzy_search(f: &mut ratatui::Frame, app: &mut App, size: Rect) {
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
        .border_style(Style::new().fg(Color::Blue));

    // Add blinking cursor to input text
    let input_text = if app.cursor_visible {
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
        .border_style(Style::new().fg(Color::Blue));

    let items: Vec<ListItem> = app
        .fuzzy_search
        .matches
        .iter()
        .enumerate()
        .map(|(i, topic)| {
            let style = if i == app.fuzzy_search.selected_index {
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

    // Now we can properly access list_state as mutable
    f.render_stateful_widget(list, popup_layout[1], &mut app.fuzzy_search.list_state);
}
