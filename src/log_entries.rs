use crate::projects::ProjectId;
use crate::schema::log_entries;
use crate::schema::tasks;
use crate::tasks::{DbTask, TaskId};
use anyhow::Result;
use diesel::prelude::*;
use diesel::upsert::excluded;
use time::{Date, Duration};

#[derive(Debug)]
pub struct LogEntry {
    pub date: Date,
    pub task: TaskId,
    pub duration: Duration,
}

#[derive(Debug)]
pub struct LogEntryExpanded {
    pub task_id: TaskId,
    pub task_name: String,
    pub issue_number: Option<i32>,
    pub date: Date,
    pub duration: Duration,
}

pub struct Period {
    pub from: Date,
    pub to: Date,
}

pub fn add_log(conn: &mut SqliteConnection, entry: LogEntry) -> Result<()> {
    new_log(conn, entry.into())
}

pub fn show_by_day(
    conn: &mut SqliteConnection,
    project: ProjectId,
    period: Option<Period>,
) -> Result<()> {
    let entries = get_by_day_expanded(conn, project, period)?;

    let mut table = comfy_table::Table::new();
    table.load_preset(crate::utils::TABLE_STYLE);
    table.set_header(["Date", "Weekday", "Task", "Issue", "Duration"]);
    table.add_rows(entries.iter().map(|entry| {
        [
            entry.date.to_string(),
            entry.date.weekday().to_string(),
            entry.task_name.clone(),
            entry
                .issue_number
                .map(|n| format!("#{n}"))
                .unwrap_or_else(|| "-".to_string()),
            entry.duration.to_string(),
        ]
    }));

    println!("{table}");

    let total_duration = entries
        .iter()
        .fold(Duration::ZERO, |total, log| total + log.duration);
    eprintln!("Total: {}h", total_duration.whole_hours(),);

    Ok(())
}

pub fn show_by_issue(
    conn: &mut SqliteConnection,
    project: ProjectId,
    period: Option<Period>,
    csv_to_clipboard: bool,
) -> Result<()> {
    let entries = get_by_issue_expanded(conn, project, period)?;

    let mut table = comfy_table::Table::new();
    table.load_preset(crate::utils::TABLE_STYLE);
    table.set_header(vec!["Task", "Issue", "Duration"]);
    table.add_rows(entries.iter().map(|entry| {
        [
            entry.task_name.clone(),
            entry
                .issue_number
                .map(|n| format!("#{n}"))
                .unwrap_or_else(|| "-".to_string()),
            entry.duration.to_string(),
        ]
    }));
    println!("{table}");

    if csv_to_clipboard {
        use std::io::Write;
        let csv = entries.iter().fold(Vec::new(), |mut csv, entry| {
            writeln!(
                &mut csv,
                "{}{};{}",
                entry
                    .issue_number
                    .map(|n| format!("[#{n}] "))
                    .unwrap_or_default(),
                entry.task_name.as_str(),
                entry.duration.whole_hours(),
            )
            .unwrap();
            csv
        });
        let csv = String::from_utf8(csv).unwrap();
        let mut clipboard = arboard::Clipboard::new().unwrap();
        clipboard.set_text(csv).unwrap();
    }

    Ok(())
}

pub fn get_by_day_expanded(
    conn: &mut SqliteConnection,
    project: ProjectId,
    period: Option<Period>,
) -> Result<Vec<LogEntryExpanded>> {
    let mut query = log_entries::table
        .inner_join(tasks::table)
        .filter(tasks::project_id.eq(project.0))
        .into_boxed();
    if let Some(period) = period {
        query = query
            .filter(log_entries::date.ge(period.from))
            .filter(log_entries::date.le(period.to));
    }
    query
        .select((DbLogEntry::as_select(), DbTask::as_select()))
        .order_by(log_entries::date)
        .load_iter::<(DbLogEntry, DbTask), _>(conn)?
        .map(|res| res.map(Into::into).map_err(Into::into))
        .collect()
}

pub fn get_by_issue_expanded(
    conn: &mut SqliteConnection,
    project: ProjectId,
    period: Option<Period>,
) -> Result<Vec<LogEntryExpanded>> {
    let mut query = log_entries::table
        .inner_join(tasks::table)
        .filter(tasks::project_id.eq(project.0))
        .into_boxed();
    if let Some(period) = period {
        query = query
            .filter(log_entries::date.ge(period.from))
            .filter(log_entries::date.le(period.to));
    }
    query
        .select((DbLogEntry::as_select(), DbTask::as_select()))
        .order_by(log_entries::date)
        .load_iter::<(DbLogEntry, DbTask), _>(conn)?
        .try_fold(Vec::<LogEntryExpanded>::new(), |mut acc, entry| {
            let (log, task) = entry?;
            if let Some(el) = acc.iter_mut().find(|el| el.task_id.0 == log.task_id) {
                el.duration += Duration::minutes(log.duration_minutes as i64);
            } else {
                acc.push(LogEntryExpanded::from((log, task)))
            }
            Ok(acc)
        })
}

fn new_log(conn: &mut SqliteConnection, entry: DbNewEntry) -> Result<()> {
    diesel::insert_into(log_entries::table)
        .values(entry)
        .on_conflict((log_entries::date, log_entries::task_id))
        .do_update()
        .set(
            log_entries::duration_minutes
                .eq(log_entries::duration_minutes + excluded(log_entries::duration_minutes)),
        )
        .execute(conn)?;
    Ok(())
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(table_name = crate::schema::log_entries)]
#[diesel(belongs_to(crate::tasks::Task))]
struct DbLogEntry {
    date: time::Date,
    task_id: i32,
    duration_minutes: i32,
}

#[derive(Insertable)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(table_name = crate::schema::log_entries)]
struct DbNewEntry {
    date: time::Date,
    task_id: i32,
    duration_minutes: i32,
}

impl From<LogEntry> for DbNewEntry {
    fn from(value: LogEntry) -> Self {
        DbNewEntry {
            date: value.date,
            task_id: value.task.0,
            duration_minutes: value.duration.whole_minutes() as i32,
        }
    }
}

impl From<DbLogEntry> for LogEntry {
    fn from(value: DbLogEntry) -> Self {
        LogEntry {
            date: value.date,
            task: TaskId(value.task_id),
            duration: Duration::minutes(value.duration_minutes as i64),
        }
    }
}

impl From<(DbLogEntry, DbTask)> for LogEntryExpanded {
    fn from((log, task): (DbLogEntry, DbTask)) -> Self {
        Self {
            task_id: TaskId(task.id),
            task_name: task.name,
            issue_number: task.issue,
            date: log.date,
            duration: Duration::minutes(log.duration_minutes as i64),
        }
    }
}
