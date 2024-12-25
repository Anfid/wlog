use crate::projects::{Project, ProjectId};
use crate::schema::tasks;
use crate::utils::{fmt_issue_linked, prompt, prompt_opt, yn_prompt};
use anyhow::Result;
use diesel::deserialize::{FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::prelude::*;
use diesel::serialize::ToSql;
use diesel::sqlite::Sqlite;
use owo_colors::OwoColorize;

#[derive(Debug, Eq, PartialEq, Hash, AsExpression, FromSqlRow)]
#[diesel(sql_type = diesel::sql_types::Integer)]
pub struct TaskId(pub i32);

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = crate::schema::tasks)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Task {
    pub id: TaskId,
    pub name: String,
    pub issue: Option<i32>,
}

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
                        project_id: project,
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
                        project_id: project,
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
        project_id: project,
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
        anyhow::bail!("A task wasn't created")
    }
}

pub fn list(conn: &mut SqliteConnection, project: Project) -> Result<()> {
    let tasks = tasks::table
        .filter(tasks::project_id.eq(project.id.0))
        .select(Task::as_select())
        .limit(50)
        .get_results(conn)?;

    print_task_list(&project.url, &tasks);

    if tasks.len() == 50 {
        println!("Task list was truncated");
    }
    Ok(())
}

pub fn search(conn: &mut SqliteConnection, project: &Project, query: String) -> Result<()> {
    let mut query = query
        .replace("\\", "\\\\")
        .replace("%", "\\%")
        .replace("_", "\\_");
    query.insert(0, '%');
    query.push('%');

    let tasks = tasks::table
        .filter(tasks::project_id.eq(project.id.0))
        .select(Task::as_select())
        .filter(tasks::name.like(query))
        .get_results(conn)?;

    print_task_list(&project.url, &tasks);

    Ok(())
}

pub fn update(
    conn: &mut SqliteConnection,
    project: &Project,
    id: TaskId,
    name: Option<&str>,
    issue: Option<Option<i32>>,
) -> Result<()> {
    let task = diesel::update(tasks::table.find(id.0))
        .set(TaskUpdate { name, issue })
        .returning(Task::as_select())
        .get_result(conn)?;

    eprintln!("{} Task has been updated", "Success:".green().bold());
    print_task_list(&project.url, &[task]);

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

fn print_task_list(project_url: &str, tasks: &[Task]) {
    let mut table = comfy_table::Table::new();
    table.load_preset(crate::utils::TABLE_STYLE);
    table.set_header(["ID", "Issue", "Name"]);
    table.add_rows(tasks.iter().map(|task| {
        [
            task.id.0.to_string(),
            task.issue
                .map(|i| fmt_issue_linked(i, project_url))
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

#[derive(Insertable)]
#[diesel(table_name = crate::schema::tasks)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewTask<'a> {
    pub project_id: ProjectId,
    pub name: &'a str,
    pub issue: Option<i32>,
}

impl FromSql<diesel::sql_types::Integer, Sqlite> for TaskId {
    fn from_sql(
        bytes: <Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> diesel::deserialize::Result<Self> {
        <i32 as FromSql<diesel::sql_types::Integer, Sqlite>>::from_sql(bytes).map(TaskId)
    }
}

impl ToSql<diesel::sql_types::Integer, Sqlite> for TaskId {
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, Sqlite>,
    ) -> diesel::serialize::Result {
        <i32 as ToSql<diesel::sql_types::Integer, Sqlite>>::to_sql(&self.0, out)
    }
}
