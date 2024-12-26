use crate::{data, projects, Config};
use anyhow::Result;
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum ProjectCmd {
    /// Create a new project
    Create,
    /// List all existing projects
    List,
    /// Pick a default project
    Default,
}

impl ProjectCmd {
    pub fn dispatch(self) -> Result<()> {
        let config = Config::read()?.unwrap_or_default();
        let mut conn = data::open(config.data_path.as_ref())?;
        match self {
            ProjectCmd::Create => {
                projects::create_interactive(&mut conn)?;
                Ok(())
            }
            ProjectCmd::List => projects::list_all(&mut conn),
            ProjectCmd::Default => projects::set_default_interactive(&mut conn),
        }
    }
}
