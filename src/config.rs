use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{fs, io};
use crate::error::AppError;

// 화면 좌표를 저장하기 위한 구조체
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

// hh:mm:ss 형식의 시간을 저장하기 위한 구조체
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

fn get_config_path() -> Result<PathBuf, AppError> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| AppError::Config("Could not find config directory".to_string()))?
        .join("CaptureScheduler");

    fs::create_dir_all(&config_dir)?;

    Ok(config_dir.join("config.json"))
}

impl Config {
    pub fn load() -> Result<Self, AppError> {
        let path = get_config_path()?;
        match fs::read_to_string(&path) {
            Ok(s) => {
                let config = serde_json::from_str(&s)?;
                Ok(config)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let config = Config::default();
                config.save()?;
                Ok(config)
            }
            Err(e) => Err(AppError::Io(e)),
        }
    }

    pub fn save(&self) -> Result<(), AppError> {
        let path = get_config_path()?;
        let s = serde_json::to_string_pretty(self)?;
        fs::write(path, s)?;
        Ok(())
    }
}
