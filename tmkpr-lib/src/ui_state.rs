use serde::{Deserialize, Serialize};

use crate::error::{TmkprError, TmkprResult};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct UiState {
    pub entry_sort: String,
    pub entry_filter_project: String,
    pub entry_filter_date: String,
}

impl UiState {
    pub fn load() -> TmkprResult<Self> {
        let path = ui_state_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(&path)?;
        toml::from_str(&contents)
            .map_err(|e| TmkprError::Config(format!("parse error in {}: {}", path.display(), e)))
    }

    pub fn save(&self) -> TmkprResult<()> {
        let path = ui_state_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents =
            toml::to_string_pretty(self).map_err(|e| TmkprError::Config(e.to_string()))?;
        std::fs::write(&path, contents)?;
        Ok(())
    }
}

pub fn ui_state_path() -> TmkprResult<std::path::PathBuf> {
    dirs::config_dir()
        .map(|d| d.join("tmkpr").join("ui-state.toml"))
        .ok_or_else(|| TmkprError::Config("could not determine config directory".to_string()))
}
