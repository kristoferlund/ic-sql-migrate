//! A lightweight database migration library for Internet Computer (ICP) canisters.
//!
//! This library provides automatic database schema management and version control
//! for SQLite (via `ic-rusqlite`) and Turso databases in ICP canisters. Migrations
//! are embedded at compile time and executed during canister initialization and upgrades.
//!
//! # Features
//!
//! **IMPORTANT**: You must enable exactly one database feature for this library to work:
//! - **SQLite support** via `ic-rusqlite` (feature: `sqlite`)
//! - **Turso support** for distributed SQLite (feature: `turso`)
//!
//! Additional capabilities:
//! - **Automatic migration execution** on canister `init` and `post_upgrade`
//! - **Compile-time migration embedding** via `include!()` macro
//! - **Transaction-based execution** for atomicity
//!
//! The library has no default features. Attempting to use it without enabling
//! either `sqlite` or `turso` will result in compilation errors when trying to
//! access the database modules.
//!
//! # Quick Start for ICP Canisters
//!
//! ## 1. Prerequisites
//! In addition to having the Rust toolchain setup and dfx, you need to install the `wasi2ic` tool that replaces WebAssembly System Interface (WASI) specific function calls with their corresponding polyfill implementations. This allows you to run Wasm binaries compiled for wasm32-wasi on the Internet Computer.
//!
//! ```bash
//! cargo install wasi2ic
//! ```
//!
//! ### Configure dfx.json
//! You also need to configure your `dfx.json` to compile for the `wasm32-wasip1` target and use `wasi2ic` to process the binary:
//!
//! ```json
//! {
//!   "canisters": {
//!     "your_canister": {
//!       "candid": "your_canister.did",
//!       "package": "your_canister",
//!       "type": "custom",
//!       "build": [
//!         "cargo build --target wasm32-wasip1 --release",
//!         "wasi2ic target/wasm32-wasip1/release/your_canister.wasm target/wasm32-wasip1/release/your_canister-wasi2ic.wasm"
//!       ],
//!       "wasm": "target/wasm32-wasip1/release/your_canister-wasi2ic.wasm"
//!     }
//!   }
//! }
//! ```
//!
//! ### For Turso
//! No additional toolchain setup required beyond Rust and DFX.
//!
//! ## 2. Add to Cargo.toml
//! ```toml
//! [dependencies]
//! ic-sql-migrate = { version = "0.0.4", features = ["sqlite"] } # or feature "turso"
//! ic-rusqlite = { version = "0.4.2", features = ["precompiled"], default-features = false }
//! # or turso = "0.1.4" for Turso
//! ic-cdk = "0.18.7"
//!
//! [build-dependencies]
//! ic-sql-migrate = "0.0.4"
//! ```
//!
//! ## 3. Create build.rs
//! ```no_run
//! fn main() {
//!     ic_sql_migrate::list(Some("migrations")).unwrap();
//! }
//! ```
//!
//! ## 4. Use in canister
//! ```ignore
//! use ic_cdk::{init, post_upgrade, pre_upgrade};
//! use ic_rusqlite::{close_connection, with_connection, Connection};
//!
//! static MIGRATIONS: &[ic_sql_migrate::Migration] = ic_sql_migrate::include!();
//!
//! fn run_migrations() {
//!     with_connection(|mut conn| {
//!         let conn: &mut Connection = &mut conn;
//!         ic_sql_migrate::sqlite::up(conn, MIGRATIONS).unwrap();
//!     });
//! }
//!
//! #[init]
//! fn init() {
//!     run_migrations();
//! }
//!
//! #[pre_upgrade]
//! fn pre_upgrade() {
//!     close_connection();
//! }
//!
//! #[post_upgrade]
//! fn post_upgrade() {
//!     run_migrations();
//! }
//! ```

mod db;

#[cfg(feature = "turso")]
pub use crate::db::turso;

#[cfg(feature = "sqlite")]
pub use crate::db::sqlite;

#[cfg(feature = "turso")]
use ::turso as turso_crate;

use thiserror::Error;

/// Custom error type for migration operations.
///
/// This enum represents all possible errors that can occur during migration operations.
/// The actual database error variant depends on the feature flag enabled (either `sqlite` or `turso`).
#[derive(Debug, Error)]
pub enum Error {
    /// I/O operation failed during build-time migration discovery
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// A specific migration failed to execute
    ///
    /// Contains the migration ID and the error message from the database
    #[error("Migration '{id}' failed: {message}")]
    MigrationFailed { id: String, message: String },

    /// Environment variable was not found during build-time processing
    #[error("Environment variable '{0}' not set")]
    EnvVarNotFound(String),

    /// Database error from the underlying database driver
    #[error("Database error: {0}")]
    Database(Box<dyn std::error::Error + Send + Sync>),
}

// IMPORTANT: Users must enable exactly one database feature: either 'sqlite' or 'turso'
// The library can be compiled without features for publishing to crates.io,
// but actual usage requires selecting a database backend. If no feature is selected,
// the database modules will not be available and the library cannot be used.

#[cfg(feature = "sqlite")]
impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Error::Database(Box::new(err))
    }
}

#[cfg(feature = "turso")]
impl From<turso_crate::Error> for Error {
    fn from(err: turso_crate::Error) -> Self {
        Error::Database(Box::new(err))
    }
}

/// Type alias for `Result<T, Error>` used throughout the library.
///
/// This provides a convenient shorthand for functions that can return migration errors.
pub type MigrateResult<T> = std::result::Result<T, Error>;

/// Represents a single database migration with its unique identifier and SQL content.
///
/// Migrations are typically created at compile time by the `include!()` macro
/// from SQL files in your migrations directory. Each migration consists of:
/// - An identifier (usually the filename without extension)
/// - The SQL statements to execute
///
/// # Example in ICP Canister
/// ```
/// use ic_sql_migrate::Migration;
///
/// // Typically included via the include!() macro:
/// static MIGRATIONS: &[Migration] = &[
///     Migration::new(
///         "001_create_users",
///         "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL);"
///     ),
///     Migration::new(
///         "002_add_email",
///         "ALTER TABLE users ADD COLUMN email TEXT;"
///     ),
/// ];
/// ```
#[derive(Debug, Clone)]
pub struct Migration {
    /// Unique identifier for the migration, typically derived from the filename.
    /// This ID is stored in the `_migrations` table to track which migrations have been applied.
    pub id: &'static str,
    /// SQL statements to execute for this migration.
    /// Can contain multiple statements separated by semicolons.
    pub sql: &'static str,
}

impl Migration {
    /// Creates a new migration with the given ID and SQL content.
    ///
    /// This is a `const fn`, allowing migrations to be created at compile time.
    ///
    /// # Arguments
    /// * `id` - Unique identifier for the migration (must not contain whitespace or special characters)
    /// * `sql` - SQL statements to execute (can be multiple statements separated by semicolons)
    ///
    /// # Example
    /// ```
    /// use ic_sql_migrate::Migration;
    ///
    /// // Static migrations for use in ICP canisters
    /// static INIT_MIGRATION: Migration = Migration::new(
    ///     "001_init",
    ///     "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY);"
    /// );
    /// ```
    pub const fn new(id: &'static str, sql: &'static str) -> Self {
        Self { id, sql }
    }
}

/// Includes all migration files discovered by the `list` function at compile time.
///
/// This macro expands to a static slice of `Migration` structs containing
/// all SQL files found in the migrations directory. The migrations are ordered
/// alphabetically by filename, so it's recommended to prefix them with numbers
/// (e.g., `001_initial.sql`, `002_add_users.sql`).
///
/// # Prerequisites
/// You must call `ic_sql_migrate::list()` in your `build.rs` file to generate
/// the migration data that this macro includes.
///
/// # Example in ICP Canister
/// ```ignore
/// // In your canister lib.rs
/// use ic_cdk::{init, post_upgrade};
/// use ic_rusqlite::{with_connection, Connection};
///
/// static MIGRATIONS: &[ic_sql_migrate::Migration] = ic_sql_migrate::include!();
///
/// fn run_migrations() {
///     with_connection(|mut conn| {
///         let conn: &mut Connection = &mut conn;
///         ic_sql_migrate::sqlite::up(conn, MIGRATIONS).unwrap();
///     });
/// }
///
/// #[init]
/// fn init() {
///     run_migrations();
/// }
///
/// #[post_upgrade]
/// fn post_upgrade() {
///     run_migrations();
/// }
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
/// The function will:
/// 1. Look for SQL files in the specified directory (relative to `Cargo.toml`)
/// 2. Sort them alphabetically by filename
/// 3. Generate code that includes their content at compile time
/// 4. Set up cargo to rebuild when migration files change
///
/// # Arguments
/// * `migrations_dir_name` - Optional custom directory name (defaults to "migrations")
///
/// # Example in build.rs
/// ```no_run
/// // In your canister's build.rs file
/// fn main() {
///     // Use default "migrations" directory
///     ic_sql_migrate::list(None).unwrap();
///
///     // Or specify a custom directory relative to Cargo.toml
///     ic_sql_migrate::list(Some("migrations")).unwrap();
/// }
/// ```
///
/// # File Naming Convention
/// Migration files should be named with a sortable prefix to ensure correct execution order:
/// - `001_initial_schema.sql`
/// - `002_add_users_table.sql`
/// - `003_add_indexes.sql`
///
/// # Errors
/// Returns an I/O error if:
/// - The output directory (`OUT_DIR`) cannot be written to
/// - File system operations fail
/// - Environment variables `CARGO_MANIFEST_DIR` or `OUT_DIR` are not set
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
