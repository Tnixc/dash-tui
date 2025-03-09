use clipboard::{ClipboardContext, ClipboardProvider};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::{
    config::{Config, GridPosition, Widget},
    ui::fuzzy::FuzzySearch,
    ui::{ConnectionStatus, Window},
};

pub struct App {
    pub values: HashMap<String, String>,
    pub connection_status: ConnectionStatus,
    pub available_topics: HashSet<String>,
    pub mode: Window,
    pub fuzzy_search: FuzzySearch,
    pub config: Config,
    pub paused: bool,
    pub selected_cell: Option<(usize, usize)>,
    pub label_edit: String,
    pub max_rows: usize,
    pub last_activity: Instant,
    pub cursor_visible: bool,
    pub highlight_visible: bool,
    pub copy_message: Option<String>,
    pub copy_message_timestamp: Option<Instant>,
}
impl App {
    pub fn new() -> App {
        App {
            values: HashMap::new(),
            connection_status: ConnectionStatus::Disconnected,
            available_topics: HashSet::new(),
            mode: Window::Main,
            fuzzy_search: FuzzySearch::new(),
            config: Config::load().unwrap_or_else(|_| Config {
                widgets: Vec::new(),
            }),
            paused: false,
            selected_cell: None,
            label_edit: String::new(),
            max_rows: 8,
            last_activity: Instant::now(),
            highlight_visible: false,
            cursor_visible: false,
            copy_message: None,
            copy_message_timestamp: None,
        }
    }

    pub fn add_widget(&mut self, widget: Widget) -> Result<(), Box<dyn std::error::Error>> {
        self.config.add_widget(widget)?;
        Ok(())
    }

    pub fn find_next_grid_position(&self) -> GridPosition {
        // Find first empty cell in the grid (5 columns, dynamic rows)
        for row in 0..self.max_rows {
            for col in 0..5 {
                if !self.is_position_occupied(row, col) {
                    return GridPosition {
                        row,
                        col,
                        row_span: 1,
                        col_span: 1,
                    };
                }
            }
        }

        // If no empty cells, default to top-left
        GridPosition {
            row: 0,
            col: 0,
            row_span: 1,
            col_span: 1,
        }
    }

    fn is_position_occupied(&self, row: usize, col: usize) -> bool {
        self.config.widgets.iter().any(|w| {
            row >= w.position.row
                && row < w.position.row + w.position.row_span
                && col >= w.position.col
                && col < w.position.col + w.position.col_span
        })
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn move_selection(&mut self, row_delta: isize, col_delta: isize) {
        let (row, col) = self.selected_cell.unwrap_or((0, 0));

        // Calculate new position with bounds checking
        let new_row = (row as isize + row_delta)
            .max(0)
            .min((self.max_rows - 1) as isize) as usize;
        let new_col = (col as isize + col_delta).max(0).min(4) as usize;

        self.selected_cell = Some((new_row, new_col));
        self.update_activity();
    }

    pub fn enter_cell_config(&mut self) {
        if self.selected_cell.is_some() {
            self.mode = Window::CellConfig;
        }
    }

    pub fn exit_cell_config(&mut self) {
        self.mode = Window::Main;
    }

    pub fn get_widget_at_selected_cell(&self) -> Option<&Widget> {
        if let Some((row, col)) = self.selected_cell {
            self.config
                .widgets
                .iter()
                .find(|w| w.position.row == row && w.position.col == col)
        } else {
            None
        }
    }

    pub fn get_widget_at_selected_cell_mut(&mut self) -> Option<&mut Widget> {
        if let Some((row, col)) = self.selected_cell {
            self.config
                .widgets
                .iter_mut()
                .find(|w| w.position.row == row && w.position.col == col)
        } else {
            None
        }
    }

    pub fn enter_label_edit(&mut self) {
        if let Some(widget) = self.get_widget_at_selected_cell() {
            self.label_edit = widget.label.clone();
            self.mode = Window::LabelEdit;
        }
    }

    pub fn exit_label_edit(&mut self) {
        self.mode = Window::CellConfig;
    }

    pub fn save_label(&mut self) {
        let new_label = self.label_edit.clone();

        if let Some(widget) = self.get_widget_at_selected_cell_mut() {
            widget.label = new_label;
            self.config.save().unwrap_or_else(|e| {
                log::error!("Failed to save config: {}", e);
            });
        }
        self.exit_label_edit();
    }

    pub fn update_activity(&mut self) {
        self.last_activity = Instant::now();
        self.highlight_visible = true;
    }

    pub fn check_highlight_timeout(&mut self) {
        const HIGHLIGHT_TIMEOUT: u64 = 5; // 5 seconds
        if self.last_activity.elapsed().as_secs() > HIGHLIGHT_TIMEOUT {
            self.highlight_visible = false;
        }
    }

    pub fn set_copy_message(&mut self, value: String) {
        self.copy_message = Some(value);
        self.copy_message_timestamp = Some(Instant::now());
    }

    pub fn check_copy_message_timeout(&mut self) {
        if let Some(timestamp) = self.copy_message_timestamp {
            if timestamp.elapsed() > Duration::from_secs(1) {
                self.copy_message = None;
                self.copy_message_timestamp = None;
            }
        }
    }

    pub fn copy_selected_value(&mut self) {
        if let Some((row, col)) = self.selected_cell {
            if let Some(widget) = self
                .config
                .widgets
                .iter()
                .find(|w| w.position.row == row && w.position.col == col)
            {
                if let Some(value) = self.values.get(&widget.topic) {
                    if let Ok(mut ctx) = ClipboardContext::new() {
                        if ctx.set_contents(value.to_owned()).is_ok() {
                            self.set_copy_message(format!("Copied: {}", value));
                        }
                    }
                }
            }
        }
    }
}
