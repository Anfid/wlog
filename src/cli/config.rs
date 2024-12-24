use super::common::time_value_parser;
use crate::Config;
use anyhow::Result;
use clap::Subcommand;
use std::path::PathBuf;
use time::Time;

#[derive(Debug, Subcommand)]
pub enum ConfigCmd {
    DataPath {
        new_path: Option<PathBuf>,
    },
    DayChangeThreshold {
        #[arg(value_parser = time_value_parser)]
        new_threshold: Option<time::Time>,
    },
    Reset,
}

impl ConfigCmd {
    pub fn dispatch(self) -> Result<()> {
        match self {
            ConfigCmd::DataPath { new_path } => match new_path {
                None => {
                    let data_path = Config::read()?.unwrap_or_default().data_path;
                    println!("{}", data_path.to_string_lossy());
                }
                Some(new_path) => {
                    Config::update_data_path(new_path)?;
                }
            },
            ConfigCmd::DayChangeThreshold { new_threshold } => match new_threshold {
                None => {
                    let threshold = Config::read()?
                        .unwrap_or_default()
                        .day_change_threshold
                        .unwrap_or_else(|| Time::from_hms(12, 0, 0).unwrap());
                    println!("{}", threshold);
                }
                Some(new_threshold) => {
                    Config::update_day_change_threshold(new_threshold)?;
                }
            },
            ConfigCmd::Reset => Config::reset()?,
        }
        Ok(())
    }
}
