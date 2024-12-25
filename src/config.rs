use crate::utils::yn_prompt;
use anyhow::Result;
use directories::ProjectDirs;
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use std::{io::Write, path::PathBuf};
use time::Time;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub data_path: PathBuf,
    pub day_change_threshold: Option<Time>,
}

impl Default for Config {
    fn default() -> Self {
        let data_path = directories().unwrap().data_dir().join("wlog.db");
        Self {
            data_path,
            day_change_threshold: None,
        }
    }
}

impl Config {
    pub fn read() -> Result<Option<Self>> {
        let config_path = directories()?.config_dir().join("config.toml");
        let config_str = match std::fs::read_to_string(config_path) {
            Ok(str) => str,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        toml::from_str(&config_str).map(Some).map_err(Into::into)
    }

    pub fn update_data_path(data_path: PathBuf) -> Result<Self> {
        let dirs = directories()?;
        let config_folder = dirs.config_dir();
        std::fs::create_dir_all(config_folder)?;
        let config_path = config_folder.join("config.toml");

        let mut config = match std::fs::read_to_string(&config_path) {
            Ok(str) => toml::from_str(&str)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Config::default(),
            Err(e) => return Err(e.into()),
        };
        config.data_path = data_path;

        let mut f = std::fs::File::create(&config_path)?;
        let config_str = toml::to_string_pretty(&config)?;
        f.write_all(config_str.as_bytes())?;

        eprintln!(
            "{} Data path updated to {}",
            "Success:".green().bold(),
            config.data_path.to_string_lossy(),
        );

        Ok(config)
    }

    pub fn update_day_change_threshold(threshold: Time) -> Result<Self> {
        let dirs = directories()?;
        let config_folder = dirs.config_dir();
        std::fs::create_dir_all(config_folder)?;
        let config_path = config_folder.join("config.toml");

        let mut config = match std::fs::read_to_string(&config_path) {
            Ok(str) => toml::from_str(&str)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Config::default(),
            Err(e) => return Err(e.into()),
        };
        config.day_change_threshold = Some(threshold);

        let mut f = std::fs::File::create(&config_path)?;
        let config_str = toml::to_string_pretty(&config)?;
        f.write_all(config_str.as_bytes())?;

        eprintln!(
            "{} Day change threshold updated to {threshold}",
            "Success:".green().bold()
        );

        Ok(config)
    }

    pub fn reset() -> Result<()> {
        if !yn_prompt("Do you want to reset to default configuration?")? {
            anyhow::bail!("Config reset aborted");
        }
        let dirs = directories()?;
        let config_folder = dirs.config_dir();
        std::fs::create_dir_all(config_folder)?;
        let config_path = config_folder.join("config.toml");

        let config = Config::default();

        let mut f = std::fs::File::create(&config_path)?;
        let config_str = toml::to_string_pretty(&config)?;
        f.write_all(config_str.as_bytes())?;

        eprintln!(
            "{} Default configuration restored",
            "Success:".green().bold()
        );

        Ok(())
    }

    pub fn day_change_threshold(&self) -> Time {
        self.day_change_threshold
            .unwrap_or_else(|| Time::from_hms(12, 0, 0).unwrap())
    }
}

fn directories() -> Result<ProjectDirs> {
    directories::ProjectDirs::from("net", "Anfid", "wlog")
        .ok_or_else(|| anyhow::anyhow!("Unable to find app data directory for the current system"))
}
