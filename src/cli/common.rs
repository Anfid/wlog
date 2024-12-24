use crate::config::Config;
use crate::log_entries::Period;
use anyhow::Result;
use clap::Args;
use time::ext::NumericalDuration;
use time::{Date, Duration, Time};

#[derive(Debug, Clone, Args)]
pub struct DateArgGroup {
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
}

impl DateArgGroup {
    pub fn to_date(self, config: &Config, now: time::OffsetDateTime) -> Result<Date> {
        let today = now.date();

        let date = if self.today {
            today
        } else if self.yesterday {
            today - 1.days()
        } else if let Some(weekday) = self.weekday {
            today.prev_occurrence(weekday)
        } else if let Some(date) = self.date {
            date
        } else if let Some(day) = self.day {
            match (self.month, self.year) {
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

        Ok(date)
    }
}

#[derive(Debug, Args)]
pub struct PeriodArgGroup {
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

impl PeriodArgGroup {
    pub fn to_period(self, now: time::OffsetDateTime) -> Option<Period> {
        let today = now.date();

        if self.all {
            None
        } else {
            let from = self.from.unwrap_or_else(|| {
                (today - Duration::days(today.day() as i64))
                    .replace_day(1)
                    .unwrap()
            });
            let to = self
                .to
                .unwrap_or_else(|| today - Duration::days(today.day() as i64));
            Some(Period { from, to })
        }
    }
}

pub fn time_value_parser(v: &str) -> Result<Time, time::error::Parse> {
    Time::parse(v, &time::format_description::well_known::Iso8601::TIME)
}

pub fn date_value_parser(v: &str) -> Result<Date, time::error::Parse> {
    Date::parse(v, &time::format_description::well_known::Iso8601::DATE)
}

pub fn duration_value_parser(v: &str) -> Result<Duration> {
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
