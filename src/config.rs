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
    pub fn config_path() -> Result<PathBuf, String> {
        let mut path = dirs::config_dir()
            .ok_or_else(|| "Cannot determine config directory".to_string())?;
        path.push("rust-dashboard");
        fs::create_dir_all(&path)
            .map_err(|e| format!("Cannot create config directory: {}", e))?;
        path.push("config.toml");
        Ok(path)
    }

    pub fn load() -> Self {
        let path = match Self::config_path() {
            Ok(p) => p,
            Err(e) => {
                log::warn!("Config path error, using defaults: {}", e);
                return Self::default();
            }
        };
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
        let path = Self::config_path().map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        let contents = toml::to_string_pretty(self)?;
        fs::write(&path, contents)?;
        Ok(())
    }
}
