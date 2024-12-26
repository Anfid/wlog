use crate::{data, projects, tasks, Config};
use anyhow::Result;
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum TaskCmd {
    /// Update an existing task
    Update {
        #[arg(long)]
        id: i32,
        #[arg(long = "set-name")]
        name: Option<String>,
        #[arg(long = "set-issue", group = "issue_value")]
        issue: Option<i32>,
        #[arg(long = "remove-issue", group = "issue_value")]
        no_issue: bool,
    },
    /// List all existing tasks
    List,
    /// Search for a task that contains the provided substring
    Search { query: String },
}

impl TaskCmd {
    pub fn dispatch(self) -> Result<()> {
        let config = Config::read()?.unwrap_or_default();
        let mut conn = data::open(config.data_path.as_ref())?;

        let project = projects::get_default_or_create_interactive(&mut conn)?;

        match self {
            TaskCmd::Update {
                id,
                issue,
                no_issue,
                name,
            } => {
                let issue = issue.map(Some).or_else(|| no_issue.then_some(None));
                tasks::update(
                    &mut conn,
                    &project,
                    tasks::TaskId(id),
                    name.as_deref(),
                    issue,
                )
            }
            TaskCmd::List => tasks::list(&mut conn, project),

            TaskCmd::Search { query } => tasks::search(&mut conn, &project, query),
        }
    }
}
