use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use owo_colors::OwoColorize;
use std::path::PathBuf;
use time::ext::NumericalDuration;
use time::{Date, Duration, Time};

mod config;
mod data;
mod log_entries;
mod projects;
mod schema;
mod tasks;
mod utils;

use config::Config;
use log_entries::Period;

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("{} {e}", "Error:".red().bold());
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Log(args) => add_log(args),
        Command::Show(args) => show(args),
        Command::Task(cmds) => task(cmds),
        Command::Project(cmds) => project(cmds),
        Command::Config(args) => config(args),
    }
}

fn add_log(options: AddLogCmd) -> Result<()> {
    let config = Config::read()?.unwrap_or_default();

    let now = time::OffsetDateTime::now_local()?;
    let today = now.date();

    let date = if options.today {
        today
    } else if options.yesterday {
        today - 1.days()
    } else if let Some(weekday) = options.weekday {
        today.prev_occurrence(weekday)
    } else if let Some(date) = options.date {
        date
    } else if let Some(day) = options.day {
        match (options.month, options.year) {
            (None, None) if day > today.day() => {
                let last_month = today - (day as i64).days();
                last_month.replace_day(day)?
            }
            (None, None) => today.replace_day(day)?,
            (None, Some(_)) => unreachable!("Invalid argument combination"),
            (Some(month), None) => {
                let year = today.year()
                    - ((month == today.month() && day > today.day())
                        || month as u8 > today.month() as u8) as i32;
                Date::from_calendar_date(year, month, day)?
            }
            (Some(month), Some(year)) => Date::from_calendar_date(year, month, day)?,
        }
    } else if now.time()
        < config
            .day_change_threshold
            .unwrap_or_else(|| Time::from_hms(12, 0, 0).unwrap())
    {
        today - 1.days()
    } else {
        today
    };

    let mut conn = data::open(config.data_path.as_ref())?;

    let project = projects::get_default_or_create_interactive(&mut conn)?;

    let issue = tasks::get_or_create_interactive(
        &mut conn,
        project,
        options.issue,
        options.name.as_deref(),
    )?;

    let entry = log_entries::LogEntry {
        date,
        duration: options.time,
        task: issue,
    };

    log_entries::add_log(&mut conn, entry)?;

    Ok(())
}

fn show(args: ShowCmd) -> Result<()> {
    let config = Config::read()?.unwrap_or_default();
    let mut conn = data::open(config.data_path.as_ref())?;

    let now = time::OffsetDateTime::now_local()?;
    let today = now.date();

    let period = if args.all {
        None
    } else {
        let from = args.from.unwrap_or_else(|| {
            (today - Duration::days(today.day() as i64))
                .replace_day(1)
                .unwrap()
        });
        let to = args
            .to
            .unwrap_or_else(|| today - Duration::days(today.day() as i64));
        Some(Period { from, to })
    };

    let project = projects::get_default_or_create_interactive(&mut conn)?;

    match args.by {
        LogFormat::Day => log_entries::show_by_day(&mut conn, project, period),
        LogFormat::Issue => log_entries::show_by_issue(&mut conn, project, period, true),
    }
}

fn task(cmds: TaskCmd) -> Result<()> {
    let config = Config::read()?.unwrap_or_default();
    let mut conn = data::open(config.data_path.as_ref())?;

    let project = projects::get_default_or_create_interactive(&mut conn)?;

    match cmds {
        TaskCmd::Update {
            id,
            issue,
            no_issue,
            name,
        } => {
            let issue = issue.map(Some).or_else(|| no_issue.then_some(None));
            tasks::update(&mut conn, tasks::TaskId(id), name.as_deref(), issue)
        }
        TaskCmd::List => tasks::list(&mut conn, project),

        TaskCmd::Search { query } => tasks::search(&mut conn, project, query),
    }
}

fn project(cmds: ProjectCmd) -> Result<()> {
    let config = Config::read()?.unwrap_or_default();
    let mut conn = data::open(config.data_path.as_ref())?;
    match cmds {
        ProjectCmd::Create => {
            projects::create_interactive(&mut conn)?;
            Ok(())
        }
        ProjectCmd::List => projects::list_all(&mut conn),
        ProjectCmd::Default => projects::set_default_interactive(&mut conn),
    }
}

fn config(cmd: ConfigCmd) -> Result<()> {
    match cmd {
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

#[derive(Debug, Subcommand)]
enum Command {
    /// Add a new log entry
    #[clap(visible_alias("new"), alias("n"), alias("l"))]
    Log(AddLogCmd),
    /// Display logged work information
    #[clap(alias("s"))]
    Show(ShowCmd),
    /// Manage tasks
    #[command(subcommand)]
    #[clap(alias("issue"), alias("t"))]
    Task(TaskCmd),
    /// Manage projects
    #[command(subcommand)]
    #[clap(alias("p"))]
    Project(ProjectCmd),
    /// Update configuration
    #[command(subcommand)]
    Config(ConfigCmd),
}

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Args)]
struct AddLogCmd {
    /// Duration in hours and minutes. Default unit is hours
    #[arg(short, long, value_parser = duration_value_parser)]
    time: Duration,
    /// Log entry date, today
    #[arg(long, group = "date_group")]
    today: bool,
    /// Log entry date, yesterday
    #[arg(long, group = "date_group")]
    yesterday: bool,
    /// Log entry date, nearest past weekday
    #[arg(short, long, group = "date_group")]
    weekday: Option<time::Weekday>,
    /// Log entry date, string in ISO8601 format
    #[arg(long, value_parser = date_value_parser, group = "date_group")]
    date: Option<Date>,
    /// Log entry day
    #[arg(short, long, group = "date_group")]
    day: Option<u8>,
    /// Log entry month
    #[arg(short, long, requires = "day")]
    month: Option<time::Month>,
    /// Log entry year
    #[arg(long, requires = "month")]
    year: Option<i32>,
    /// Link issue number
    #[arg(short, long)]
    issue: Option<i32>,
    /// Task name
    #[arg(long)]
    name: Option<String>,
}

#[derive(Debug, Args)]
struct ShowCmd {
    /// Group entries by
    #[arg(long, required = false, default_value = "day")]
    by: LogFormat,
    /// List all logs
    #[arg(long)]
    all: bool,
    /// Only show entries starting from this date, string in ISO8601 format
    #[arg(long, value_parser = date_value_parser)]
    from: Option<Date>,
    /// Only show entries up to this date, string in ISO8601 format
    #[arg(long, value_parser = date_value_parser)]
    to: Option<Date>,
}

#[derive(Debug, Default, Clone, Copy, clap::ValueEnum)]
enum LogFormat {
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

#[derive(Debug, Subcommand)]
enum TaskCmd {
    Update {
        #[arg(long)]
        id: i32,
        #[arg(long = "set-name")]
        name: Option<String>,
        #[arg(long = "set-issue", group = "issue_value")]
        issue: Option<i32>,
        #[arg(long = "remove-issue", group = "issue_value")]
        no_issue: bool,
    },
    List,
    Search {
        query: String,
    },
}

#[derive(Debug, Subcommand)]
enum ProjectCmd {
    Create,
    List,
    Default,
}

#[derive(Debug, Subcommand)]
enum ConfigCmd {
    DataPath {
        new_path: Option<PathBuf>,
    },
    DayChangeThreshold {
        #[arg(value_parser = time_value_parser)]
        new_threshold: Option<time::Time>,
    },
    Reset,
}

fn time_value_parser(v: &str) -> Result<Time, time::error::Parse> {
    Time::parse(v, &time::format_description::well_known::Iso8601::TIME)
}

fn date_value_parser(v: &str) -> Result<Date, time::error::Parse> {
    Date::parse(v, &time::format_description::well_known::Iso8601::DATE)
}

fn duration_value_parser(v: &str) -> Result<Duration> {
    let mut unit = 60;
    let mut result = 0;
    let mut acc = 0;
    for c in v.chars() {
        match c {
            '0'..='9' => acc = acc * 10 + (c as u8 - b'0') as i64,
            'h' => {
                result += acc * 60;
                acc = 0;
                unit = 1;
            }
            'm' => {
                result += acc;
                acc = 0;
                unit = 0;
            }
            unexpected => anyhow::bail!("Unexpected character in duration: '{unexpected}'"),
        }
    }
    if unit == 0 && acc != 0 {
        anyhow::bail!("Unable to parse duration, unknown unit for value {acc}");
    }
    result += acc * unit;
    Ok(Duration::minutes(result))
}
