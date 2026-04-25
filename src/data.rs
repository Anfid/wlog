use diesel::prelude::*;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use eyre::{Result, anyhow};
use std::path::Path;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub fn open(path: &Path) -> Result<SqliteConnection> {
    let mut conn = SqliteConnection::establish(
        path.as_os_str()
            .to_str()
            .ok_or_else(|| anyhow!("Invalid data path"))?,
    )?;
    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|e| anyhow!("{e}"))?;
    Ok(conn)
}
