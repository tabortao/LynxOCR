use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    /// Model storage path
    pub model_path: String,
    /// Active OCR model: "ppocr-v4" | "ppocr-v5" | "ppocr-v6"
    #[serde(default = "default_active_ocr_model")]
    pub active_ocr_model: String,
    /// Sidebar collapsed state
    #[serde(default)]
    pub sidebar_collapsed: bool,
    /// OCR screenshot shortcut (e.g. "Ctrl+Shift+O")
    #[serde(default = "default_ocr_screenshot_shortcut")]
    pub ocr_screenshot_shortcut: String,
}

fn default_ocr_screenshot_shortcut() -> String {
    "Ctrl+Shift+O".to_string()
}

fn default_active_ocr_model() -> String {
    "ppocr-v6".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        let app_data = app_data_dir();
        let model_path = app_data.join("models").to_string_lossy().to_string();
        Self {
            model_path,
            active_ocr_model: default_active_ocr_model(),
            sidebar_collapsed: false,
            ocr_screenshot_shortcut: default_ocr_screenshot_shortcut(),
        }
    }
}

impl AppConfig {
    /// Returns the path to the config file.
    fn config_file_path() -> PathBuf {
        app_data_dir().join("config.json")
    }

    /// Load config from disk. Returns default if file doesn't exist or is invalid.
    pub fn load() -> Self {
        let path = Self::config_file_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(config) => {
                        log::info!("[AppConfig] loaded from {}", path.display());
                        return config;
                    }
                    Err(e) => {
                        log::warn!("[AppConfig] failed to parse config: {e}, using defaults");
                    }
                },
                Err(e) => {
                    log::warn!("[AppConfig] failed to read config: {e}, using defaults");
                }
            }
        }
        Self::default()
    }

    /// Save config to disk.
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_file_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config dir: {e}"))?;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {e}"))?;
        std::fs::write(&path, content).map_err(|e| format!("Failed to write config: {e}"))?;
        log::info!("[AppConfig] saved to {}", path.display());
        Ok(())
    }
}

fn app_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("LynxOCR")
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        home.join(".local").join("share").join("LynxOCR")
    }
}