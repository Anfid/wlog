use crate::log_entries::Period;
use crate::projects::ProjectId;
use crate::schema::comments;
use anyhow::Result;
use diesel::prelude::*;
use time::{Date, Duration};

#[derive(Debug)]
pub struct Comment {
    pub date: Date,
    pub text: String,
    pub duration: Option<Duration>,
}

#[derive(Debug)]
pub struct CommentExpanded {
    pub date: Date,
    pub text: String,
    pub duration: Option<Duration>,
}

pub fn add_comment(
    conn: &mut SqliteConnection,
    project: ProjectId,
    comment: Comment,
) -> Result<()> {
    let entry = DbNewComment::from_domain(project, comment);
    diesel::insert_into(comments::table)
        .values(entry)
        .execute(conn)?;
    Ok(())
}

pub fn get_by_period(
    conn: &mut SqliteConnection,
    project: ProjectId,
    period: Option<&Period>,
) -> Result<Vec<CommentExpanded>> {
    let mut query = comments::table
        .filter(comments::project_id.eq(project.0))
        .into_boxed();
    if let Some(period) = period {
        query = query
            .filter(comments::date.ge(period.from))
            .filter(comments::date.le(period.to));
    }
    query
        .select(DbComment::as_select())
        .order_by(comments::date)
        .load_iter::<DbComment, _>(conn)?
        .map(|res| res.map(Into::into).map_err(Into::into))
        .collect()
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(table_name = crate::schema::comments)]
struct DbComment {
    date: Date,
    duration_minutes: Option<i32>,
    text: String,
}

#[derive(Insertable)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(table_name = crate::schema::comments)]
struct DbNewComment {
    project_id: ProjectId,
    date: Date,
    duration_minutes: Option<i32>,
    text: String,
}

impl DbNewComment {
    fn from_domain(project: ProjectId, comment: Comment) -> Self {
        DbNewComment {
            project_id: project,
            date: comment.date,
            duration_minutes: comment.duration.map(|d| d.whole_minutes() as i32),
            text: comment.text,
        }
    }
}

impl From<DbComment> for CommentExpanded {
    fn from(db: DbComment) -> Self {
        Self {
            date: db.date,
            text: db.text,
            duration: db.duration_minutes.map(|m| Duration::minutes(m as i64)),
        }
    }
}
