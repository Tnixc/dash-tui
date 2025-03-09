use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use crate::{
    config::{Config, GridPosition, Widget},
    fuzzy::FuzzySearch,
    ui::{ConnectionStatus, Window},
};

pub struct App {
    pub values: HashMap<String, String>,
    pub connection_status: ConnectionStatus,
    pub available_topics: HashSet<String>,
    pub mode: Window,
    pub fuzzy_search: FuzzySearch,
    pub config: Config,
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
        }
    }

    pub fn add_widget(&mut self, widget: Widget) -> Result<(), Box<dyn std::error::Error>> {
        self.config.add_widget(widget)?;
        Ok(())
    }

    pub fn find_next_grid_position(&self) -> GridPosition {
        // Find first empty cell in the 5x10 grid
        for row in 0..10 {
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
}
