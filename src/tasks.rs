use crate::projects::ProjectId;
use crate::schema::tasks;
use crate::utils::{prompt, prompt_opt, yn_prompt};
use anyhow::Result;
use diesel::prelude::*;

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct TaskId(pub i32);

pub fn get_or_create(
    conn: &mut SqliteConnection,
    project: ProjectId,
    issue: Option<i32>,
    name: Option<&str>,
) -> Result<TaskId> {
    match (issue, name) {
        (None, None) => create_interactive(conn, project, None),
        (None, Some(name)) => {
            let task: Option<i32> = tasks::table
                .select(tasks::id)
                .filter(tasks::project_id.eq(project.0))
                .filter(tasks::name.eq(name))
                .order_by(tasks::issue.is_null().desc())
                .first(conn)
                .optional()?;
            if let Some(task) = task {
                Ok(TaskId(task))
            } else {
                new_task(
                    conn,
                    NewTask {
                        project_id: project.0,
                        issue: None,
                        name,
                        description: None,
                    },
                )
            }
        }
        (Some(issue), None) => {
            // TODO: task picker mechanism, if multiple available
            let task: Option<i32> = tasks::table
                .select(tasks::id)
                .filter(tasks::project_id.eq(project.0))
                .filter(tasks::issue.eq(&issue))
                .first(conn)
                .optional()?;
            if let Some(task) = task {
                Ok(TaskId(task))
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
                        description: None,
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
    let desc: Option<String> = prompt_opt("Description")?;

    let task = NewTask {
        project_id: project.0,
        name: task_name.as_ref(),
        issue: issue_number,
        description: desc.as_deref(),
    };

    let num_confirm = task
        .issue
        .map(|n| format!("issue number {n}"))
        .unwrap_or_else(|| String::from("no issue number"));
    let desc_confirm = task
        .description
        .map(|d| format!("description \"{d}\""))
        .unwrap_or_else(|| String::from("no description"));
    if yn_prompt(&format!(
        "Create a new issue with name \"{task_name}\", {num_confirm} and {desc_confirm}?"
    ))? {
        new_task(conn, task)
    } else {
        anyhow::bail!("An issue wasn't created")
    }
}

pub fn new_task(conn: &mut SqliteConnection, new_task: NewTask) -> Result<TaskId> {
    diesel::insert_into(tasks::table)
        .values(&new_task)
        .returning(tasks::id)
        .get_result::<i32>(conn)
        .map(TaskId)
        .map_err(Into::into)
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::tasks)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct DbTask {
    pub id: i32,
    pub project_id: i32,
    pub name: String,
    pub issue: Option<i32>,
    pub description: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::tasks)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewTask<'a> {
    pub project_id: i32,
    pub name: &'a str,
    pub issue: Option<i32>,
    pub description: Option<&'a str>,
}
