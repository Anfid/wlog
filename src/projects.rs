use crate::schema::{default_project, projects};
use crate::utils::{prompt, prompt_opt, yn_prompt};
use anyhow::Result;
use diesel::prelude::*;
use owo_colors::OwoColorize;

#[derive(Debug, Copy, Clone)]
pub struct ProjectId(pub i32);

pub fn get_default_or_create_interactive(conn: &mut SqliteConnection) -> Result<ProjectId> {
    if let Some(default) = get_default(conn)? {
        Ok(default)
    } else {
        let id = create_interactive(conn)?;
        set_default(conn, id)?;
        Ok(id)
    }
}

pub fn set_default_interactive(conn: &mut SqliteConnection) -> Result<()> {
    list_all(conn)?;
    let project_id = prompt("New default project ID")?;
    set_default(conn, ProjectId(project_id))?;
    eprintln!(
        "{} Default project set to {}",
        "Success:".green().bold(),
        project_id
    );
    Ok(())
}

pub fn create_interactive(conn: &mut SqliteConnection) -> Result<ProjectId> {
    let project_name = prompt_opt("Project name")?;
    let project_url = prompt("URL")?;

    let msg = if let Some(ref n) = project_name {
        format!("Create a new project with name \"{n}\" and URL {project_url}?")
    } else {
        format!("Create a new project with URL {project_url} and no name?")
    };
    if yn_prompt(&msg)? {
        let pid = create(conn, project_url, project_name)?;
        eprintln!("{} New project created", "Success:".green().bold());
        Ok(pid)
    } else {
        anyhow::bail!("A project wasn't created")
    }
}

pub fn list_all(conn: &mut SqliteConnection) -> Result<()> {
    let default_id = default_project::table
        .select(default_project::project_id)
        .find(0)
        .get_result(conn)
        .optional()?;
    let mut table = comfy_table::Table::new();
    table.load_preset(crate::utils::TABLE_STYLE);
    table.set_header(vec![" ", "ID", "Name", "URL"]);
    for project in get_all(conn)? {
        let mark = if Some(project.id) == default_id {
            "*"
        } else {
            " "
        };
        table.add_row(vec![
            mark,
            &project.id.to_string(),
            project.name.as_deref().unwrap_or(""),
            &project.url,
        ]);
    }
    println!("{table}");
    Ok(())
}

fn create(
    conn: &mut SqliteConnection,
    project_url: String,
    project_name: Option<String>,
) -> Result<ProjectId> {
    let project = NewProject {
        url: project_url,
        name: project_name,
    };
    diesel::insert_into(projects::table)
        .values(project)
        .returning(projects::id)
        .get_result(conn)
        .map(ProjectId)
        .map_err(Into::into)
}

fn get_all(conn: &mut SqliteConnection) -> Result<Vec<Project>> {
    projects::table.load(conn).map_err(Into::into)
}

fn get_default(conn: &mut SqliteConnection) -> Result<Option<ProjectId>> {
    use crate::schema::default_project::dsl::*;
    default_project
        .select(project_id)
        .find(0)
        .get_result(conn)
        .map(ProjectId)
        .optional()
        .map_err(Into::into)
}

fn set_default(conn: &mut SqliteConnection, proj_id: ProjectId) -> Result<()> {
    if !diesel::select(diesel::dsl::exists(projects::table.find(proj_id.0))).get_result(conn)? {
        anyhow::bail!("Project {} doesn't exist", proj_id.0);
    }

    diesel::insert_into(default_project::table)
        .values((
            default_project::id.eq(0),
            default_project::project_id.eq(proj_id.0),
        ))
        .on_conflict(default_project::id)
        .do_update()
        .set(default_project::project_id.eq(proj_id.0))
        .execute(conn)?;
    Ok(())
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::schema::projects)]
struct NewProject {
    url: String,
    name: Option<String>,
}

#[derive(Debug, Queryable)]
#[diesel(table_name = crate::schema::projects)]
struct Project {
    id: i32,
    url: String,
    name: Option<String>,
}
