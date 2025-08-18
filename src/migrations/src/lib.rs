pub use anyhow::Result;
use ic_rusqlite::Connection;
use std::collections::HashSet;

pub struct SqlMigration {
    pub id: &'static str,
    pub sql: &'static str,
}

impl SqlMigration {
    pub const fn new(id: &'static str, sql: &'static str) -> Self {
        Self { id, sql }
    }
}

fn ensure_table(conn: &mut Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS _migrations (id TEXT PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP)",
        [],
    )?;
    Ok(())
}

fn applied(conn: &Connection) -> Result<HashSet<String>> {
    let mut s = conn.prepare("SELECT id FROM _migrations")?;
    let rows = s.query_map([], |r| r.get::<_, String>(0))?;
    Ok(rows.filter_map(Result::ok).collect())
}

pub fn run_up(conn: &mut Connection, migrations: &[SqlMigration]) -> Result<()> {
    ensure_table(conn)?;
    let seen = applied(conn)?;

    // Start transaction for all migrations
    let tx = conn.transaction()?;

    for migration in migrations {
        if seen.contains(migration.id) {
            continue;
        }

        // Execute the SQL
        tx.execute_batch(migration.sql)?;

        // Record migration as applied
        tx.execute("INSERT INTO _migrations(id) VALUES (?)", [migration.id])?;
    }

    // Commit all migrations
    tx.commit()?;
    Ok(())
}

#[macro_export]
macro_rules! include_migrations_from_dir {
    ($dir:literal, [$($migration_name:literal),* $(,)?]) => {
        &[
            $(
                migrations::SqlMigration::new(
                    $migration_name,
                    include_str!(concat!($dir, "/", $migration_name, ".sql"))
                ),
            )*
        ]
    };
}

// Keep the old macro for backward compatibility
#[macro_export]
macro_rules! include_sql_migrations {
    ($($id:literal => $path:literal),* $(,)?) => {
        &[
            $(
                migrations::SqlMigration::new($id, include_str!($path)),
            )*
        ]
    };
}

#[macro_export]
macro_rules! include_migrations {
    () => {
        include!(concat!(env!("OUT_DIR"), "/migrations_gen.rs"))
    };
}
