use crate::projects::ProjectId;
use crate::schema::{schedule_logs, schedule_settings};
use anyhow::Result;
use diesel::prelude::*;
use diesel::upsert::excluded;
use time::{Date, Weekday};

#[derive(Debug, Clone, Copy)]
pub struct WeekBasedSchedule(pub u8);

impl WeekBasedSchedule {
    pub fn new(weekdays: &[Weekday], flexible: bool) -> Self {
        let bitmap = weekdays
            .iter()
            .map(|weekday| 1 << weekday.number_days_from_monday())
            .fold(0, |acc, weekday| acc | weekday)
            | ((flexible as u8) << 7);
        Self(bitmap)
    }

    pub fn is_flexible(&self) -> bool {
        self.0 & 0x80 > 0
    }

    pub fn to_weekdays(self) -> Vec<Weekday> {
        (0u8..7)
            .filter_map(|weekday| {
                if self.0 & (1 << weekday) > 0 {
                    let weekday = match weekday {
                        0 => Weekday::Monday,
                        1 => Weekday::Tuesday,
                        2 => Weekday::Wednesday,
                        3 => Weekday::Thursday,
                        4 => Weekday::Friday,
                        5 => Weekday::Saturday,
                        6 => Weekday::Sunday,
                        _ => unreachable!(),
                    };
                    Some(weekday)
                } else {
                    None
                }
            })
            .collect()
    }

    fn from_bitmap(v: i32) -> Self {
        Self(v.to_le_bytes()[0])
    }
}

pub struct ScheduleLog(u32);

impl ScheduleLog {
    fn from_bitmap(v: i32) -> Self {
        Self(u32::from_ne_bytes(v.to_ne_bytes()))
    }

    fn to_bitmap(&self) -> i32 {
        i32::from_ne_bytes(self.0.to_ne_bytes())
    }

    fn from_weekly(schedule: WeekBasedSchedule, date: time::Date) -> Self {
        let first_weekday = date
            .replace_day(1)
            .unwrap()
            .weekday()
            .number_days_from_monday();
        let bitmap =
            (0..time::util::days_in_month(date.month(), date.year())).fold(0u32, |acc, i| {
                let weekday = (i + first_weekday) % 7;
                let is_workday = ((1 << weekday) & schedule.0) > 0;
                acc | (is_workday as u32) << i
            }) | ((schedule.is_flexible() as u32) << 31);
        Self(bitmap)
    }

    pub fn is_workday(&self, ord: u8) -> bool {
        self.0 & (1 << (ord - 1)) != 0
    }
}

pub fn set(
    conn: &mut SqliteConnection,
    project_id: ProjectId,
    schedule: WeekBasedSchedule,
) -> Result<()> {
    let schedule = Schedule {
        project_id,
        weekdays: Some(schedule.0 as i32),
        workday_minutes: Some(8 * 60),
    };
    diesel::insert_into(schedule_settings::table)
        .values(&schedule)
        .on_conflict(schedule_settings::project_id)
        .do_update()
        .set(&schedule)
        .execute(conn)?;
    Ok(())
}

pub fn log(conn: &mut SqliteConnection, project_id: ProjectId, date: Date) -> Result<()> {
    let schedule: Option<Schedule> = schedule_settings::table
        .find(project_id)
        .get_result(conn)
        .optional()?;
    let bitmap = if let Some(schedule) = schedule {
        let schedule = WeekBasedSchedule::from_bitmap(schedule.weekdays.unwrap());
        ScheduleLog::from_weekly(schedule, date).to_bitmap()
    } else {
        0
    };

    let log = ScheduleLogEntry {
        project_id,
        month: date.year() * 12 + date.month() as i32,
        bitmap,
    };

    diesel::insert_into(schedule_logs::table)
        .values(log)
        .on_conflict((schedule_logs::project_id, schedule_logs::month))
        .do_update()
        .set(schedule_logs::bitmap.eq(excluded(schedule_logs::bitmap)))
        .execute(conn)?;
    Ok(())
}

pub fn get(
    conn: &mut SqliteConnection,
    project_id: ProjectId,
) -> Result<Option<WeekBasedSchedule>> {
    schedule_settings::table
        .find(project_id)
        .select(schedule_settings::weekdays)
        .get_result::<Option<i32>>(conn)
        .map(Option::unwrap)
        .map(WeekBasedSchedule::from_bitmap)
        .optional()
        .map_err(Into::into)
}

pub fn get_log(
    conn: &mut SqliteConnection,
    project_id: ProjectId,
    date: Date,
) -> Result<Option<ScheduleLog>> {
    let month = date.year() * 12 + date.month() as i32;
    schedule_logs::table
        .find((project_id, month))
        .select(schedule_logs::bitmap)
        .get_result::<i32>(conn)
        .map(ScheduleLog::from_bitmap)
        .optional()
        .map_err(Into::into)
}

#[derive(Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::schedule_logs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ScheduleLogEntry {
    project_id: ProjectId,
    // Month number since 1BCE, year * 12 + month
    month: i32,
    bitmap: i32,
}

#[derive(Debug, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = crate::schema::schedule_settings)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Schedule {
    project_id: ProjectId,
    weekdays: Option<i32>,
    workday_minutes: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_to_log() {
        let schedule = WeekBasedSchedule(0b00011111);
        let date = time::Date::from_calendar_date(2024, time::Month::December, 1).unwrap();

        let bitmap = ScheduleLog::from_weekly(schedule, date).to_bitmap();
        let expected = 0b01100111110011111001111100111110;
        if bitmap != expected {
            panic!("expected: {expected:#034b}\n  actual: {bitmap:#034b}");
        }
    }
}
