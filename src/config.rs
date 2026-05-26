use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

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
        let mut path =
            dirs::config_dir().ok_or_else(|| "Cannot determine config directory".to_string())?;
        path.push("rust-dashboard");
        fs::create_dir_all(&path).map_err(|e| format!("Cannot create config directory: {}", e))?;
        path.push("config.toml");
        Ok(path)
    }

    /// Load from the platform config path. Falls back to `Default` on
    /// any path-resolution / read / parse error.
    pub fn load() -> Self {
        match Self::config_path() {
            Ok(path) => Self::load_from(&path),
            Err(e) => {
                log::warn!("Config path error, using defaults: {}", e);
                Self::default()
            }
        }
    }

    /// Load from an explicit path. Falls back to `Default` if the file
    /// is missing, unreadable, or unparseable. Exposed so tests can
    /// isolate I/O from the platform config directory.
    pub fn load_from(path: &Path) -> Self {
        if path.exists() {
            if let Ok(contents) = fs::read_to_string(path) {
                if let Ok(config) = toml::from_str(&contents) {
                    return config;
                }
            }
        }
        Self::default()
    }

    /// Save to the platform config path.
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path().map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        self.save_to(&path)
    }

    /// Save to an explicit path. Exposed so tests can write to a
    /// `tempfile::tempdir` instead of the platform config directory.
    pub fn save_to(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let contents = toml::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }
}
