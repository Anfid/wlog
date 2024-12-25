use crate::schema::{default_project, projects};
use crate::utils::{prompt, prompt_opt, yn_prompt};
use anyhow::Result;
use diesel::deserialize::{FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::prelude::*;
use diesel::serialize::ToSql;
use diesel::sqlite::Sqlite;
use owo_colors::OwoColorize;

#[derive(Debug, Copy, Clone, AsExpression, FromSqlRow)]
#[diesel(sql_type = diesel::sql_types::Integer)]
pub struct ProjectId(pub i32);

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = crate::schema::projects)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Project {
    pub id: ProjectId,
    pub url: String,
    pub name: Option<String>,
}

pub fn get_default_or_create_interactive(conn: &mut SqliteConnection) -> Result<Project> {
    if let Some(default) = get_default(conn)? {
        Ok(default)
    } else {
        let project = create_interactive(conn)?;
        set_default(conn, project.id)?;
        Ok(project)
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

pub fn create_interactive(conn: &mut SqliteConnection) -> Result<Project> {
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
        let mark = if Some(project.id.0) == default_id {
            "*"
        } else {
            " "
        };
        table.add_row(vec![
            mark,
            &project.id.0.to_string(),
            project.name.as_deref().unwrap_or(""),
            &project.url,
        ]);
    }
    println!("{table}");
    Ok(())
}

fn create(conn: &mut SqliteConnection, url: String, name: Option<String>) -> Result<Project> {
    let project = NewProject { url, name };
    diesel::insert_into(projects::table)
        .values(project)
        .returning(Project::as_select())
        .get_result(conn)
        .map(Into::into)
        .map_err(Into::into)
}

fn get_all(conn: &mut SqliteConnection) -> Result<Vec<Project>> {
    projects::table.load(conn).map_err(Into::into)
}

fn get_default(conn: &mut SqliteConnection) -> Result<Option<Project>> {
    default_project::table
        .find(0)
        .inner_join(projects::table)
        .select(Project::as_select())
        .get_result::<Project>(conn)
        .map(Into::into)
        .optional()
        .map_err(Into::into)
}

fn set_default(conn: &mut SqliteConnection, id: ProjectId) -> Result<()> {
    if !diesel::select(diesel::dsl::exists(projects::table.find(id.0))).get_result(conn)? {
        anyhow::bail!("Project {} doesn't exist", id.0);
    }

    diesel::insert_into(default_project::table)
        .values((
            default_project::id.eq(0),
            default_project::project_id.eq(id.0),
        ))
        .on_conflict(default_project::id)
        .do_update()
        .set(default_project::project_id.eq(id.0))
        .execute(conn)?;
    Ok(())
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::schema::projects)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewProject {
    url: String,
    name: Option<String>,
}

impl FromSql<diesel::sql_types::Integer, Sqlite> for ProjectId {
    fn from_sql(
        bytes: <Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> diesel::deserialize::Result<Self> {
        <i32 as FromSql<diesel::sql_types::Integer, Sqlite>>::from_sql(bytes).map(ProjectId)
    }
}

impl ToSql<diesel::sql_types::Integer, Sqlite> for ProjectId {
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, Sqlite>,
    ) -> diesel::serialize::Result {
        <i32 as ToSql<diesel::sql_types::Integer, Sqlite>>::to_sql(&self.0, out)
    }
}
