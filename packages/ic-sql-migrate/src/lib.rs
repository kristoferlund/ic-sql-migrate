//! A lightweight SQLite migration library for Internet Computer (ICP) canisters.
//!
//! This library provides automatic database schema management and version control
//! through SQL migration files that are embedded at compile time and executed at runtime.

mod db;

#[cfg(feature = "turso")]
pub use crate::db::turso;

#[cfg(feature = "sqlite")]
pub use crate::db::sqlite;

#[cfg(feature = "turso")]
use ::turso as turso_crate;

use thiserror::Error;

/// Custom error type for migration operations.
#[derive(Debug, Error)]
pub enum Error {
    /// I/O operation failed
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Migration execution failed
    #[error("Migration '{id}' failed: {message}")]
    MigrationFailed { id: String, message: String },

    /// Environment variable not found
    #[error("Environment variable '{0}' not set")]
    EnvVarNotFound(String),

    /// Database error
    #[cfg(all(feature = "sqlite", not(feature = "turso")))]
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// Database error
    #[cfg(all(feature = "turso", not(feature = "sqlite")))]
    #[error("Database error: {0}")]
    Database(#[from] turso_crate::Error),
}

// Compile-time check to ensure at least one database feature is enabled
#[cfg(not(any(feature = "sqlite", feature = "turso")))]
compile_error!("At least one database feature must be enabled: either 'sqlite' or 'turso'");

// Compile-time check to prevent both features from being enabled
#[cfg(all(feature = "sqlite", feature = "turso"))]
compile_error!(
    "Cannot enable both 'sqlite' and 'turso' features at the same time. Please choose one."
);

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
///     ic_sql_migrate::list(Some("migrations")).unwrap();
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
