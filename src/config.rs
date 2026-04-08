use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct Hook {
    pub event: String,
    pub command: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CustomCommand {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub attach: bool,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub tick_rate_ms: u64,
    pub log_tail_lines: String,
    pub sidebar_width: u16,
    #[serde(default)]
    pub custom_commands: Vec<CustomCommand>,
    #[serde(default = "default_cpu_alert_threshold")]
    pub cpu_alert_threshold: f64,
    #[serde(default = "default_memory_alert_threshold")]
    pub memory_alert_threshold: f64,
    #[serde(default)]
    pub hooks: Vec<Hook>,
    pub docker_host: Option<String>,
}

fn default_cpu_alert_threshold() -> f64 {
    80.0
}

fn default_memory_alert_threshold() -> f64 {
    90.0
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            tick_rate_ms: 250,
            log_tail_lines: "100".to_string(),
            sidebar_width: 40,
            custom_commands: vec![],
            cpu_alert_threshold: 80.0,
            memory_alert_threshold: 90.0,
            hooks: vec![],
            docker_host: None,
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        let path = Self::config_path();
        match fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rustydocker")
            .join("config.toml")
    }
}
