use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScheduleTime {
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Config {
    pub capture_area: Option<Rect>,
    pub schedule_times: Vec<ScheduleTime>,
    pub save_directory: PathBuf,
    pub base_filename: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            capture_area: None,
            schedule_times: Vec::new(),
            save_directory: dirs::document_dir().unwrap_or_default().join("Captures"),
            base_filename: None,
        }
    }
}

impl Config {
    fn path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_default();
        path.push("CaptureScheduler");
        fs::create_dir_all(&path).ok();
        path.push("config.json");
        path
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let json_str = serde_json::to_string_pretty(self)?;
        fs::write(Self::path(), json_str)
    }

    pub fn load() -> Result<Self, std::io::Error> {
        let path = Self::path();
        if path.exists() {
            let json_str = fs::read_to_string(path)?;
            serde_json::from_str(&json_str).map_err(Into::into)
        } else {
            Ok(Self::default())
        }
    }
}
