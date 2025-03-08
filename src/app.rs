use std::collections::HashMap;

use crate::{
    fuzzy::FuzzySearch,
    ui::{ConnectionStatus, Window},
};

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
