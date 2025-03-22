use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub widgets: Vec<Widget>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Widget {
    pub topic: String,
    pub label: String,
    pub widget_type: WidgetType,
    pub position: GridPosition,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GridPosition {
    pub row: usize,
    pub col: usize,
    pub row_span: usize,
    pub col_span: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WidgetType {
    Text,
    Graph,
    Gauge,
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = get_config_path()?;

        if !config_path.exists() {
            let default_config = Config {
                widgets: Vec::new(),
            };
            default_config.save()?;
            return Ok(default_config);
        }

        let contents = fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = get_config_path()?;

        // Ensure parent directories exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let toml_string = toml::to_string_pretty(self)?;
        fs::write(config_path, toml_string)?;
        Ok(())
    }

    pub fn add_widget(&mut self, widget: Widget) -> Result<(), Box<dyn std::error::Error>> {
        self.widgets.push(widget);
        self.save()?;
        Ok(())
    }
}

fn get_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut path = user_dirs::config_dir()?;
    path.push("dash89");
    path.push("config.toml");
    Ok(path)
}
