use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub refresh_interval_seconds: u32,
    pub theme: String,
    pub window_width: Option<f32>,
    pub window_height: Option<f32>,
    pub window_x: Option<f32>,
    pub window_y: Option<f32>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            refresh_interval_seconds: 2,
            theme: "Dark".to_string(),
            window_width: None,
            window_height: None,
            window_x: None,
            window_y: None,
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("rust-dashboard");
        fs::create_dir_all(&path).ok();
        path.push("config.toml");
        path
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str(&contents) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path();
        let contents = toml::to_string_pretty(self)?;
        fs::write(&path, contents)?;
        Ok(())
    }
}
