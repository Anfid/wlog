use crate::projects::ProjectId;
use crate::schema::tasks;
use crate::utils::{prompt, prompt_opt, yn_prompt};
use anyhow::Result;
use diesel::prelude::*;
use owo_colors::OwoColorize;

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct TaskId(pub i32);

pub fn get_or_create_interactive(
    conn: &mut SqliteConnection,
    project: ProjectId,
    issue: Option<i32>,
    name: Option<&str>,
) -> Result<TaskId> {
    match (issue, name) {
        (None, None) => create_interactive(conn, project, None),
        (None, Some(name)) => {
            if let Some(task) = get_by_name(conn, project, name)? {
                Ok(task)
            } else {
                new_task(
                    conn,
                    NewTask {
                        project_id: project.0,
                        issue: None,
                        name,
                    },
                )
            }
        }
        (Some(issue), None) => {
            if let Some(task) = get_by_issue(conn, project, issue)? {
                Ok(task)
            } else {
                create_interactive(conn, project, Some(issue))
            }
        }
        (Some(issue), Some(name)) => {
            let task = tasks::table
                .select(tasks::id)
                .filter(tasks::project_id.eq(project.0))
                .filter(tasks::issue.eq(&issue))
                .filter(tasks::name.eq(name))
                .first(conn)
                .optional()?;
            if let Some(task) = task {
                Ok(TaskId(task))
            } else {
                new_task(
                    conn,
                    NewTask {
                        project_id: project.0,
                        name,
                        issue: Some(issue),
                    },
                )
            }
        }
    }
}

pub fn create_interactive(
    conn: &mut SqliteConnection,
    project: ProjectId,
    issue: Option<i32>,
) -> Result<TaskId> {
    let task_name: String = prompt("Task name")?;
    let issue_number = if issue.is_none() {
        prompt_opt("Issue number")?
    } else {
        issue
    };

    let task = NewTask {
        project_id: project.0,
        name: task_name.as_ref(),
        issue: issue_number,
    };

    let num_confirm = task
        .issue
        .map(|n| format!("issue number {n}"))
        .unwrap_or_else(|| String::from("no issue number"));
    if yn_prompt(&format!(
        "Create a new task with {num_confirm} and name \"{task_name}\"?"
    ))? {
        new_task(conn, task)
    } else {
        anyhow::bail!("An issue wasn't created")
    }
}

pub fn list(conn: &mut SqliteConnection, project: ProjectId) -> Result<()> {
    let tasks = tasks::table
        .filter(tasks::project_id.eq(project.0))
        .select(DbTask::as_select())
        .limit(50)
        .get_results(conn)?;

    print_task_list(&tasks);

    if tasks.len() == 50 {
        println!("Task list was truncated");
    }
    Ok(())
}

pub fn search(conn: &mut SqliteConnection, project: ProjectId, query: String) -> Result<()> {
    let mut query = query
        .replace("\\", "\\\\")
        .replace("%", "\\%")
        .replace("_", "\\_");
    query.insert(0, '%');
    query.push('%');

    let tasks = tasks::table
        .filter(tasks::project_id.eq(project.0))
        .select(DbTask::as_select())
        .filter(tasks::name.like(query))
        .get_results(conn)?;

    print_task_list(&tasks);

    Ok(())
}

pub fn update(
    conn: &mut SqliteConnection,
    id: TaskId,
    name: Option<&str>,
    issue: Option<Option<i32>>,
) -> Result<()> {
    let task = diesel::update(tasks::table.find(id.0))
        .set(TaskUpdate { name, issue })
        .returning(DbTask::as_select())
        .get_result(conn)?;

    eprintln!("{} Task has been updated", "Success:".green().bold());
    print_task_list(&[task]);

    Ok(())
}

pub fn new_task(conn: &mut SqliteConnection, new_task: NewTask) -> Result<TaskId> {
    diesel::insert_into(tasks::table)
        .values(&new_task)
        .returning(tasks::id)
        .get_result::<i32>(conn)
        .map(TaskId)
        .map_err(Into::into)
}

fn get_by_issue(
    conn: &mut SqliteConnection,
    project: ProjectId,
    issue: i32,
) -> Result<Option<TaskId>> {
    tasks::table
        .select(tasks::id)
        .filter(tasks::project_id.eq(project.0))
        .filter(tasks::issue.eq(&issue))
        .first(conn)
        .map(TaskId)
        .optional()
        .map_err(Into::into)
}

fn get_by_name(
    conn: &mut SqliteConnection,
    project: ProjectId,
    name: &str,
) -> Result<Option<TaskId>> {
    tasks::table
        .select(tasks::id)
        .filter(tasks::project_id.eq(project.0))
        .filter(tasks::name.eq(name))
        .first(conn)
        .map(TaskId)
        .optional()
        .map_err(Into::into)
}

fn print_task_list(tasks: &[DbTask]) {
    let mut table = comfy_table::Table::new();
    table.load_preset(crate::utils::TABLE_STYLE);
    table.set_header(["ID", "Issue", "Name"]);
    table.add_rows(tasks.iter().map(|task| {
        [
            task.id.to_string(),
            task.issue
                .map(|i| format!("#{i}"))
                .unwrap_or("-".to_string()),
            task.name.clone(),
        ]
    }));
    println!("{table}");
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = crate::schema::tasks)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TaskUpdate<'a> {
    pub name: Option<&'a str>,
    pub issue: Option<Option<i32>>,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = crate::schema::tasks)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct DbTask {
    pub id: i32,
    pub name: String,
    pub issue: Option<i32>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::tasks)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewTask<'a> {
    pub project_id: i32,
    pub name: &'a str,
    pub issue: Option<i32>,
}
