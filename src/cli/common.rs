use crate::config::Config;
use crate::log_entries::Period;
use anyhow::Result;
use clap::Args;
use time::ext::NumericalDuration;
use time::{Date, Duration, Time, Weekday};

#[derive(Debug, Clone, Default, Args)]
pub struct DateArgGroup {
    /// Log entry date, today
    #[arg(long, group = "date_group")]
    today: bool,
    /// Log entry date, yesterday
    #[arg(long, group = "date_group")]
    yesterday: bool,
    /// Log entry date, nearest past weekday
    #[arg(short, long, value_parser = weekday_value_parser, group = "date_group")]
    weekday: Option<Weekday>,
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
    pub fn to_date(&self, config: &Config, now: time::OffsetDateTime) -> Result<Date> {
        let today = now.date();

        let date = if self.today {
            today
        } else if self.yesterday {
            today.previous_day().unwrap()
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
        } else if now.time() < config.day_change_threshold() {
            today.previous_day().unwrap()
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
    /// Only show entries from this day
    #[arg(long)]
    today: bool,
    /// Only show entries for the last 7 days
    #[arg(short, long)]
    week: bool,
}

impl PeriodArgGroup {
    pub fn to_period(&self, config: &Config, now: time::OffsetDateTime) -> Option<Period> {
        let today = if now.time() < config.day_change_threshold() {
            now.date().previous_day().unwrap()
        } else {
            now.date()
        };

        if self.all {
            None
        } else if self.today {
            Some(Period {
                from: today,
                to: today,
            })
        } else if self.week {
            Some(Period {
                from: today - 7.days(),
                to: today,
            })
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
    let mut result = None;
    let mut number = None;
    for c in v.chars() {
        match c {
            '0'..='9' => number = Some(number.unwrap_or(0) * 10 + (c as u8 - b'0') as i64),
            'h' => {
                let res = result.unwrap_or(0);
                let acc = number.ok_or_else(|| anyhow::anyhow!("Number expected before unit"))?;
                result = Some(res + acc * 60);
                number = None;
                unit = 1;
            }
            'm' => {
                let res = result.unwrap_or(0);
                let acc = number.ok_or_else(|| anyhow::anyhow!("Number expected before unit"))?;
                result = Some(acc + res);
                number = None;
                unit = 0;
            }
            unexpected => anyhow::bail!("Unexpected character in duration: '{unexpected}'"),
        }
    }
    if unit == 0 && number.is_some() {
        anyhow::bail!(
            "Unable to parse duration, unknown unit for value {}",
            number.unwrap()
        );
    }
    let minutes = match (result, number) {
        (Some(r), Some(n)) => r + n * unit,
        (Some(r), None) => r,
        (None, Some(n)) => n * unit,
        (None, None) => anyhow::bail!("Number expected"),
    };

    Ok(Duration::minutes(minutes))
}

pub fn weekday_value_parser(v: &str) -> Result<Weekday> {
    let weekday = match v.to_lowercase().as_str() {
        "mon" | "monday" => Weekday::Monday,
        "tue" | "tuesday" => Weekday::Tuesday,
        "wed" | "wednesday" => Weekday::Wednesday,
        "thu" | "thursday" => Weekday::Thursday,
        "fri" | "friday" => Weekday::Friday,
        "sat" | "saturday" => Weekday::Saturday,
        "sun" | "sunday" => Weekday::Sunday,
        _ => anyhow::bail!("Invalid weekday: \"{v}\""),
    };
    Ok(weekday)
}

#[cfg(test)]
mod tests {
    use time::{Month, OffsetDateTime};

    use super::*;

    #[test]
    fn date_arg_group() {
        let config = Config::default();
        let now = OffsetDateTime::new_utc(
            Date::from_calendar_date(2025, Month::January, 26).unwrap(),
            Time::from_hms(10, 36, 21).unwrap(),
        );
        let group = DateArgGroup {
            today: true,
            ..Default::default()
        };
        assert_eq!(group.to_date(&config, now).unwrap(), now.date());

        let group = DateArgGroup {
            yesterday: true,
            ..Default::default()
        };
        assert_eq!(group.to_date(&config, now).unwrap(), now.date() - 1.days());

        let group = DateArgGroup {
            weekday: Some(Weekday::Monday),
            ..Default::default()
        };
        assert_eq!(
            group.to_date(&config, now).unwrap(),
            now.date().prev_occurrence(Weekday::Monday)
        );

        let group = DateArgGroup {
            day: Some(100),
            ..Default::default()
        };
        assert_eq!(group.to_date(&config, now).ok(), None);

        let group = DateArgGroup {
            day: Some(3),
            ..Default::default()
        };
        assert_eq!(
            group.to_date(&config, now).unwrap(),
            now.date().replace_day(3).unwrap()
        );
    }

    #[test]
    fn duration_parser() {
        let data = [
            ("1", Some(60)),
            ("10h", Some(10 * 60)),
            ("8h30", Some(8 * 60 + 30)),
            ("6h21m", Some(6 * 60 + 21)),
            ("90m", Some(90)),
            ("0", Some(0)),
            ("0h", Some(0)),
            ("0m", Some(0)),
            ("0h0m", Some(0)),
            ("10a", None),
            ("hm", None),
            ("", None),
        ];
        for (input, minutes) in data {
            let parsed = duration_value_parser(input).ok();
            assert_eq!(parsed, minutes.map(Duration::minutes));
        }
    }

    #[test]
    fn weekday_parser() {
        let data = [
            ("monday", Some(Weekday::Monday)),
            ("tuesday", Some(Weekday::Tuesday)),
            ("wednesday", Some(Weekday::Wednesday)),
            ("thursday", Some(Weekday::Thursday)),
            ("friday", Some(Weekday::Friday)),
            ("saturday", Some(Weekday::Saturday)),
            ("sunday", Some(Weekday::Sunday)),
            ("tursday", None),
            ("", None),
            ("mon", Some(Weekday::Monday)),
            ("tue", Some(Weekday::Tuesday)),
            ("wed", Some(Weekday::Wednesday)),
            ("thu", Some(Weekday::Thursday)),
            ("fri", Some(Weekday::Friday)),
            ("sat", Some(Weekday::Saturday)),
            ("sun", Some(Weekday::Sunday)),
        ];
        for (input, output) in data {
            let parsed = weekday_value_parser(input).ok();
            assert_eq!(parsed, output);
        }
    }
}
