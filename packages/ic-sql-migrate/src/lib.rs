//! A lightweight SQLite migration library for Internet Computer (ICP) canisters.
//!
//! This library provides automatic database schema management and version control
//! through SQL migration files that are embedded at compile time and executed at runtime.

use rusqlite::Connection;
use std::collections::HashSet;
use thiserror::Error;

/// Custom error type for migration operations.
#[derive(Debug, Error)]
pub enum Error {
    /// Database operation failed
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// I/O operation failed
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Migration execution failed
    #[error("Migration '{id}' failed: {message}")]
    MigrationFailed { id: String, message: String },

    /// Environment variable not found
    #[error("Environment variable '{0}' not set")]
    EnvVarNotFound(String),
}

pub type MigrateResult<T> = std::result::Result<T, Error>;

/// Represents a single database migration with its unique identifier and SQL content.
#[derive(Debug, Clone)]
pub struct Migration {
    /// Unique identifier for the migration, derived from the filename
    pub id: &'static str,
    /// SQL statements to execute for this migration
    pub sql: &'static str,
}

impl Migration {
    /// Creates a new migration with the given ID and SQL content.
    ///
    /// # Arguments
    /// * `id` - Unique identifier for the migration
    /// * `sql` - SQL statements to execute
    pub const fn new(id: &'static str, sql: &'static str) -> Self {
        Self { id, sql }
    }
}

/// Ensures the migrations tracking table exists in the database.
///
/// Creates a `_migrations` table if it doesn't exist, which tracks:
/// - `id`: The unique identifier of each applied migration
/// - `applied_at`: Timestamp when the migration was applied
fn ensure_migrations_table(conn: &mut Connection) -> MigrateResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS _migrations (
            id TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    Ok(())
}

/// Retrieves the set of already applied migration IDs from the database.
fn get_applied_migrations(conn: &Connection) -> MigrateResult<HashSet<String>> {
    let mut statement = conn.prepare("SELECT id FROM _migrations")?;

    let migration_ids = statement.query_map([], |row| row.get::<_, String>(0))?;

    let mut applied_set = HashSet::new();
    for id in migration_ids.into_iter().flatten() {
        applied_set.insert(id);
    }

    Ok(applied_set)
}

/// Executes all pending migrations in order.
///
/// This function:
/// 1. Ensures the migrations tracking table exists
/// 2. Identifies which migrations have already been applied
/// 3. Executes pending migrations in the order they appear in the slice
/// 4. Records each migration as applied
///
/// All migrations are executed within a single transaction for atomicity.
///
/// # Arguments
/// * `conn` - Mutable reference to the SQLite connection
/// * `migrations` - Slice of migrations to apply in order
///
/// # Errors
/// Returns an error if:
/// - Database operations fail
/// - Migration SQL is invalid
/// - Transaction cannot be committed
pub fn up(conn: &mut Connection, migrations: &[Migration]) -> MigrateResult<()> {
    ensure_migrations_table(conn)?;
    let applied_migrations = get_applied_migrations(conn)?;

    // Check if there are any migrations to apply
    let pending_migrations: Vec<&Migration> = migrations
        .iter()
        .filter(|m| !applied_migrations.contains(m.id))
        .collect();

    if pending_migrations.is_empty() {
        return Ok(());
    }

    // Start transaction for all migrations
    let tx = conn.transaction()?;

    for migration in pending_migrations {
        // Execute the migration SQL
        tx.execute_batch(migration.sql)
            .map_err(|e| Error::MigrationFailed {
                id: migration.id.to_string(),
                message: e.to_string(),
            })?;

        // Record migration as applied
        tx.execute("INSERT INTO _migrations(id) VALUES (?)", [migration.id])?;
    }

    // Commit all migrations atomically
    tx.commit()?;

    Ok(())
}

/// Includes all migration files discovered by the `list` function at compile time.
///
/// This macro expands to a static slice of `Migration` structs containing
/// all SQL files found in the migrations directory.
///
/// # Example
/// ```ignore
/// static MIGRATIONS: &[migrations::Migration] = ic_sql_migrate::include!();
/// ```
#[macro_export]
macro_rules! include {
    () => {
        include!(concat!(env!("OUT_DIR"), "/migrations_gen.rs"))
    };
}

/// Discovers and lists all SQL migration files for inclusion at compile time.
///
/// This function should be called in `build.rs` to generate code that embeds
/// all migration files into the binary. It scans the specified directory for
/// `.sql` files and generates Rust code to include them.
///
/// # Arguments
/// * `migrations_dir_name` - Optional custom directory name (defaults to "migrations")
///
/// # Example
/// ```no_run
/// // In build.rs
/// fn main() {
///     migrations::list(Some("migrations")).unwrap();
/// }
/// ```
///
/// # Errors
/// Returns an I/O error if:
/// - The output directory cannot be written to
/// - File system operations fail
pub fn list(migrations_dir_name: Option<&str>) -> std::io::Result<()> {
    use std::env;
    use std::fs;
    use std::path::Path;

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "CARGO_MANIFEST_DIR not set")
    })?;

    let dir_name = migrations_dir_name.unwrap_or("migrations");
    let migrations_dir = Path::new(&manifest_dir).join(dir_name);

    // Ensure cargo rebuilds when migrations change
    println!("cargo:rerun-if-changed={}", migrations_dir.display());

    // Generate the output file path
    let out_dir = env::var("OUT_DIR")
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::NotFound, "OUT_DIR not set"))?;
    let dest_path = Path::new(&out_dir).join("migrations_gen.rs");

    // If migrations directory doesn't exist, create empty migrations array
    if !migrations_dir.exists() {
        fs::write(dest_path, "&[]")?;
        return Ok(());
    }

    // Collect all SQL files
    let migration_files = collect_migration_files(&migrations_dir)?;

    // Generate and write the Rust code
    let generated_code = generate_migrations_code(&migration_files);
    fs::write(dest_path, generated_code)?;

    Ok(())
}

/// Collects all SQL migration files from the specified directory.
///
/// Returns a sorted list of (migration_id, file_path) tuples.
fn collect_migration_files(
    migrations_dir: &std::path::Path,
) -> std::io::Result<Vec<(String, String)>> {
    use std::fs;

    let mut migration_files = Vec::new();

    let entries = fs::read_dir(migrations_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Only process .sql files
        if path.extension().and_then(|s| s.to_str()) != Some("sql") {
            continue;
        }

        if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
            let absolute_path = path.to_string_lossy().to_string();
            migration_files.push((file_stem.to_string(), absolute_path));

            // Ensure cargo rebuilds when this specific file changes
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }

    // Sort migration files by name to ensure consistent ordering
    migration_files.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(migration_files)
}

/// Generates Rust code for including migration files.
///
/// Creates a static array initialization with all migration files.
fn generate_migrations_code(migration_files: &[(String, String)]) -> String {
    let mut code = String::from("&[\n");

    for (migration_id, file_path) in migration_files {
        code.push_str(&format!(
            "    ic_sql_migrate::Migration::new(\"{migration_id}\", include_str!(\"{file_path}\")),\n"
        ));
    }

    code.push_str("]\n");
    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_migration_creation() {
        let migration = Migration::new("001_test", "CREATE TABLE test (id INTEGER);");
        assert_eq!(migration.id, "001_test");
        assert_eq!(migration.sql, "CREATE TABLE test (id INTEGER);");
    }

    #[test]
    fn test_ensure_migrations_table() {
        let mut conn = Connection::open_in_memory().unwrap();
        ensure_migrations_table(&mut conn).unwrap();

        // Verify table exists
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='_migrations'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_up_migrations() {
        let mut conn = Connection::open_in_memory().unwrap();

        let migrations = &[
            Migration::new(
                "001_create_users",
                "CREATE TABLE users (id INTEGER PRIMARY KEY);",
            ),
            Migration::new("002_add_email", "ALTER TABLE users ADD COLUMN email TEXT;"),
        ];

        // Run migrations
        up(&mut conn, migrations).unwrap();

        // Verify migrations were applied
        let applied = get_applied_migrations(&conn).unwrap();
        assert!(applied.contains("001_create_users"));
        assert!(applied.contains("002_add_email"));

        // Verify table structure
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('users') WHERE name='email'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_up_migrations_idempotency() {
        let mut conn = Connection::open_in_memory().unwrap();

        let migrations = &[Migration::new(
            "001_test",
            "CREATE TABLE test (id INTEGER);",
        )];

        // Run migrations twice
        up(&mut conn, migrations).unwrap();
        up(&mut conn, migrations).unwrap();

        // Should only be applied once
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM _migrations WHERE id='001_test'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_migration_failure_rollback() {
        let mut conn = Connection::open_in_memory().unwrap();

        let migrations = &[
            Migration::new("001_valid", "CREATE TABLE test (id INTEGER);"),
            Migration::new("002_invalid", "INVALID SQL STATEMENT;"),
        ];

        // Run migrations - should fail on second one
        let result = up(&mut conn, migrations);
        assert!(result.is_err());

        // Verify first migration was not committed due to transaction rollback
        let applied = get_applied_migrations(&conn).unwrap();
        assert!(applied.is_empty());

        // Verify table was not created
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='test'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }
}
