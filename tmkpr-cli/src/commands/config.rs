use anyhow::Result;
use tmkpr_lib::config::{config_path, Config};

use crate::cli::ConfigShowArgs;

pub fn show(
    _args: ConfigShowArgs,
    config: &Config,
    db_override: Option<&std::path::Path>,
    format: &str,
) -> Result<()> {
    match format {
        "json" => {
            let mut val = serde_json::to_value(config)?;
            val["_config_file"] = serde_json::Value::String(
                config_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
            );
            if let Some(db) = db_override {
                val["_db_override"] = serde_json::Value::String(db.display().to_string());
            }
            println!("{}", serde_json::to_string_pretty(&val)?);
        }
        _ => {
            let path = config_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "(unknown)".to_string());
            println!("Config file: {path}");
            if !std::path::Path::new(&path).exists() {
                println!("(file not found — showing defaults)");
            }
            if let Some(db) = db_override {
                println!("--db override: {}", db.display());
            }
            println!();
            let toml = toml::to_string_pretty(config)
                .unwrap_or_else(|e| format!("(serialization error: {e})"));
            print!("{toml}");
        }
    }
    Ok(())
}
