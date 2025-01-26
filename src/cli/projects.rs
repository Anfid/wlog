use super::common::{date_value_parser, weekday_value_parser};
use crate::schedule::{ScheduleLog, WeekBasedSchedule};
use crate::{data, projects, schedule, Config};
use anyhow::Result;
use clap::Subcommand;
use owo_colors::OwoColorize;
use time::{Date, Weekday};

#[derive(Debug, Subcommand)]
pub enum ProjectCmd {
    /// Create a new project
    Create,
    /// List all existing projects
    List,
    /// Pick a default project
    Default,
}

#[derive(Debug, Subcommand)]
pub enum ScheduleCmd {
    /// Show current schedule
    Show {
        #[clap(long, value_parser = date_value_parser)]
        for_date: Option<Date>,
    },
    /// Set current schedule
    Set {
        /// Set weekly schedule
        #[clap(long, value_parser = weekday_value_parser, value_delimiter = ',', num_args=1..=7)]
        weekdays: Vec<Weekday>,
        /// Time log entries must be added for exact dates
        #[clap(long)]
        rigid: bool,
    },
}

impl ProjectCmd {
    pub fn dispatch(self) -> Result<()> {
        let config = Config::read()?.unwrap_or_default();
        let mut conn = data::open(config.data_path.as_ref())?;

        match self {
            ProjectCmd::Create => {
                projects::create_interactive(&mut conn)?;
                Ok(())
            }
            ProjectCmd::List => projects::list_all(&mut conn),
            ProjectCmd::Default => projects::set_default_interactive(&mut conn),
        }
    }
}

impl ScheduleCmd {
    pub fn dispatch(self) -> Result<()> {
        let config = Config::read()?.unwrap_or_default();
        let mut conn = data::open(config.data_path.as_ref())?;
        let project = projects::get_default_or_create_interactive(&mut conn)?;

        match self {
            ScheduleCmd::Show { for_date } => {
                if let Some(date) = for_date {
                    if let Some(bitmap) = schedule::get_log(&mut conn, project.id, date)? {
                        print_calendar(date, bitmap);
                        Ok(())
                    } else {
                        anyhow::bail!("No results")
                    }
                } else if let Some(result) = schedule::get(&mut conn, project.id)? {
                    println!("Active schedule:");
                    println!(
                        "{}",
                        result
                            .to_weekdays()
                            .into_iter()
                            .map(|weekday| weekday.to_string())
                            .collect::<Vec<String>>()
                            .join(", ")
                    );
                    println!("Flexible: {}", result.is_flexible());
                    Ok(())
                } else {
                    anyhow::bail!("No results")
                }
            }
            ScheduleCmd::Set { weekdays, rigid } => schedule::set(
                &mut conn,
                project.id,
                WeekBasedSchedule::new(&weekdays, !rigid),
            ),
        }
    }
}

pub fn print_calendar(date: time::Date, schedule: ScheduleLog) {
    let date = date.replace_day(1).unwrap();
    let weekday_ord = date.weekday().number_days_from_monday();
    println!(" Mo Tu We Th Fr Sa Su");
    print!("{: <1$}", "", weekday_ord as usize * 3);
    for i in 1..=time::util::days_in_month(date.month(), date.year()) {
        if (weekday_ord + i) % 7 == 1 && i != 0 {
            println!();
        }
        let style = if schedule.is_workday(i) {
            owo_colors::Style::new().bold()
        } else {
            owo_colors::Style::new().red()
        };
        print!(" {: >2}", i.style(style));
    }
    println!()
}
