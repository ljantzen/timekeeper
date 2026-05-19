use std::path::PathBuf;

use chrono::Weekday;
use serde::{Deserialize, Serialize};

use crate::error::{TmkprError, TmkprResult};
use crate::models::LOCAL_USER_ID;
use crate::nlp::TimeFormat;

pub const APP_NAME: &str = "tmkpr";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub database: DatabaseConfig,
    pub user: UserConfig,
    pub display: DisplayConfig,
    pub pomodoro: PomodoroConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub date_format: String,
    pub color: bool,
    pub week_start: WeekdayDef,
    #[serde(default)]
    pub time_format: TimeFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PomodoroConfig {
    pub work_duration_minutes: u64,
    pub break_duration_minutes: u64,
    pub sessions_before_long_break: u64,
    pub long_break_duration_minutes: u64,
    pub notify_desktop: bool,
    pub message_timeout_secs: u64,
    pub auto_start_break: bool,
    #[serde(default)]
    pub max_cycles: u64,
    #[serde(default)]
    pub sound_work_to_break: Option<String>,
    #[serde(default)]
    pub sound_break_to_work: Option<String>,
}

/// Serde-compatible wrapper for chrono::Weekday.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WeekdayDef {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl From<WeekdayDef> for Weekday {
    fn from(w: WeekdayDef) -> Self {
        match w {
            WeekdayDef::Mon => Weekday::Mon,
            WeekdayDef::Tue => Weekday::Tue,
            WeekdayDef::Wed => Weekday::Wed,
            WeekdayDef::Thu => Weekday::Thu,
            WeekdayDef::Fri => Weekday::Fri,
            WeekdayDef::Sat => Weekday::Sat,
            WeekdayDef::Sun => Weekday::Sun,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database: DatabaseConfig {
                path: default_db_path().unwrap_or_else(|_| PathBuf::from("tmkpr.db")),
            },
            user: UserConfig {
                user_id: LOCAL_USER_ID.to_string(),
            },
            display: DisplayConfig {
                date_format: "%Y-%m-%d %H:%M".to_string(),
                color: true,
                week_start: WeekdayDef::Mon,
                time_format: TimeFormat::H24,
            },
            pomodoro: PomodoroConfig::default(),
        }
    }
}

impl Default for PomodoroConfig {
    fn default() -> Self {
        Self {
            work_duration_minutes: 25,
            break_duration_minutes: 5,
            sessions_before_long_break: 4,
            long_break_duration_minutes: 15,
            notify_desktop: false,
            message_timeout_secs: 5,
            auto_start_break: false,
            max_cycles: 0,
            sound_work_to_break: None,
            sound_break_to_work: None,
        }
    }
}

impl Config {
    pub fn load() -> TmkprResult<Self> {
        let path = config_path()?;
        if !path.exists() {
            let cfg = Config::default();
            cfg.save()?;
            return Ok(cfg);
        }
        let contents = std::fs::read_to_string(&path)?;
        toml::from_str(&contents)
            .map_err(|e| TmkprError::Config(format!("parse error in {}: {}", path.display(), e)))
    }

    pub fn save(&self) -> TmkprResult<()> {
        let path = config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents =
            toml::to_string_pretty(self).map_err(|e| TmkprError::Config(e.to_string()))?;
        std::fs::write(&path, contents)?;
        Ok(())
    }
}

pub fn config_path() -> TmkprResult<PathBuf> {
    dirs::config_dir()
        .map(|d| d.join(APP_NAME).join("config.toml"))
        .ok_or_else(|| TmkprError::Config("could not determine config directory".to_string()))
}

pub fn default_db_path() -> TmkprResult<PathBuf> {
    dirs::data_local_dir()
        .map(|d| d.join(APP_NAME).join("tmkpr.db"))
        .ok_or_else(|| TmkprError::Config("could not determine data directory".to_string()))
}
