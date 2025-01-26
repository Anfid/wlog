use clap::Parser;
use owo_colors::OwoColorize;

mod cli;
mod config;
mod data;
mod log_entries;
mod projects;
mod schedule;
mod schema;
mod tasks;
mod utils;

use cli::Cli;
use config::Config;

fn main() {
    let result = Cli::parse().dispatch();
    if let Err(e) = result {
        eprintln!("{} {e}", "Error:".red().bold());
        std::process::exit(1);
    }
}
