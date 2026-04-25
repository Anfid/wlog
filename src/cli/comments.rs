use super::common::{DateArgGroup, duration_value_parser};
use crate::{Config, comments, data, projects};
use clap::Args;
use eyre::Result;
use time::{Duration, OffsetDateTime};

#[derive(Debug, Args)]
pub struct AddCommentCmd {
    /// Comment text
    text: String,
    /// Duration in hours and minutes. Default unit is hours
    #[arg(short, long, value_parser = duration_value_parser)]
    time: Option<Duration>,
    /// Date
    #[clap(flatten)]
    date: DateArgGroup,
}

impl AddCommentCmd {
    pub fn dispatch(self) -> Result<()> {
        let config = Config::read()?.unwrap_or_default();
        let mut conn = data::open(config.data_path.as_ref())?;

        let now = OffsetDateTime::now_local()?;
        let date = self.date.to_date(&config, now)?;
        let project = projects::get_default_or_create_interactive(&mut conn)?;

        let comment = comments::Comment {
            date,
            text: self.text,
            duration: self.time,
        };

        comments::add_comment(&mut conn, project.id, comment)?;

        Ok(())
    }
}
