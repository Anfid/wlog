use super::common::{duration_value_parser, DateArgGroup, PeriodArgGroup};
use crate::{data, log_entries, projects, tasks, Config};
use anyhow::Result;
use clap::{Args, ValueEnum};
use time::{Duration, OffsetDateTime};

#[derive(Debug, Args)]
pub struct AddLogCmd {
    /// Duration in hours and minutes. Default unit is hours
    #[arg(short, long, value_parser = duration_value_parser)]
    time: Duration,
    /// Date
    #[clap(flatten)]
    date: DateArgGroup,
    /// Link issue number
    #[arg(short, long)]
    issue: Option<i32>,
    /// Task name
    #[arg(long)]
    name: Option<String>,
}

#[derive(Debug, Args)]
pub struct ShowCmd {
    /// Group entries by
    #[arg(long, required = false, default_value = "day")]
    by: LogFormat,
    /// Period
    #[clap(flatten)]
    period: PeriodArgGroup,
}

#[derive(Debug, Default, Clone, Copy, ValueEnum)]
pub enum LogFormat {
    #[default]
    #[clap(alias("date"))]
    Day,
    #[clap(alias("task"))]
    Issue,
}

impl std::str::FromStr for LogFormat {
    type Err = &'static str;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "issue" => Ok(LogFormat::Issue),
            "day" => Ok(LogFormat::Day),
            _ => Err("Unknown log format"),
        }
    }
}

impl AddLogCmd {
    pub fn dispatch(self) -> Result<()> {
        let config = Config::read()?.unwrap_or_default();

        let mut conn = data::open(config.data_path.as_ref())?;

        let now = OffsetDateTime::now_local()?;
        let date = self.date.to_date(&config, now)?;
        let project = projects::get_default_or_create_interactive(&mut conn)?;

        let issue = tasks::get_or_create_interactive(
            &mut conn,
            project.id,
            self.issue,
            self.name.as_deref(),
        )?;

        let entry = log_entries::LogEntry {
            date,
            duration: self.time,
            task: issue,
        };

        log_entries::add_log(&mut conn, entry)?;

        Ok(())
    }
}

impl ShowCmd {
    pub fn dispatch(self) -> Result<()> {
        let config = Config::read()?.unwrap_or_default();
        let mut conn = data::open(config.data_path.as_ref())?;

        let now = OffsetDateTime::now_local()?;
        let period = self.period.to_period(&config, now);

        let project = projects::get_default_or_create_interactive(&mut conn)?;

        match self.by {
            LogFormat::Day => log_entries::show_by_day(&mut conn, &project, period),
            LogFormat::Issue => log_entries::show_by_task(&mut conn, &project, period, true),
        }
    }
}
