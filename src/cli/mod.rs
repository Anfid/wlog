use anyhow::Result;
use clap::{Parser, Subcommand};

mod common;
mod config;
mod logs;
mod projects;
mod tasks;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Add a new log entry
    #[clap(visible_alias("new"), alias("n"), alias("l"))]
    Log(logs::AddLogCmd),
    /// Display logged work information
    #[clap(alias("s"))]
    Show(logs::ShowCmd),
    /// Manage tasks
    #[command(subcommand)]
    #[clap(alias("issue"), alias("t"))]
    Task(tasks::TaskCmd),
    /// Manage projects
    #[command(subcommand)]
    #[clap(alias("p"))]
    Project(projects::ProjectCmd),
    /// Update configuration
    #[command(subcommand)]
    Config(config::ConfigCmd),
}

#[derive(Debug, Parser)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

impl Cli {
    pub fn dispatch(self) -> Result<()> {
        match self.command {
            Command::Log(cmd) => cmd.dispatch(),
            Command::Show(cmd) => cmd.dispatch(),
            Command::Task(cmd) => cmd.dispatch(),
            Command::Project(cmd) => cmd.dispatch(),
            Command::Config(cmd) => cmd.dispatch(),
        }
    }
}
