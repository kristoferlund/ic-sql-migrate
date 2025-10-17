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
//! - **Compile-time migration embedding** via `include_migrations!()` macro
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
//! ic-sql-migrate = { version = "0.0.5", features = ["sqlite"] } # or feature "turso"
//! ic-rusqlite = { version = "0.4.2", features = ["precompiled"], default-features = false }
//! # or turso = "0.1.4" for Turso
//! ic-cdk = "0.18.7"
//!
//! [build-dependencies]
//! ic-sql-migrate = "0.0.5"
//! ```
//!
//! ## 3. Create build.rs
//! ```no_run
//! ic_sql_migrate::Builder::new().build().unwrap();
//! ```
//!
//! ## 4. Use in canister
//! ```ignore
//! use ic_cdk::{init, post_upgrade, pre_upgrade};
//! use ic_rusqlite::{close_connection, with_connection, Connection};
//!
//! static MIGRATIONS: &[ic_sql_migrate::Migration] = ic_sql_migrate::include_migrations!();
//!
//! fn run_migrations() {
//!     with_connection(|mut conn| {
//!         let conn: &mut Connection = &mut conn;
//!         ic_sql_migrate::sqlite::migrate(conn, MIGRATIONS).unwrap();
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

/// Type alias for seed functions that take a SQLite connection.
///
/// Seed functions are called after migrations to populate initial data.
#[cfg(feature = "sqlite")]
pub type SqliteSeedFn = fn(&rusqlite::Connection) -> MigrateResult<()>;

/// Type alias for async seed functions that take a Turso connection.
///
/// Seed functions are called after migrations to populate initial data.
#[cfg(feature = "turso")]
pub type TursoSeedFn =
    fn(
        &turso_crate::Connection,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = MigrateResult<()>> + Send>>;

/// Represents a single database seed with its unique identifier and execution function.
///
/// Seeds are typically created at compile time and executed after migrations
/// to populate initial or test data using Rust code rather than SQL.
///
/// # Example
/// ```
/// use ic_sql_migrate::Seed;
///
/// fn seed_users(conn: &rusqlite::Connection) -> ic_sql_migrate::MigrateResult<()> {
///     conn.execute("INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com')", [])?;
///     Ok(())
/// }
///
/// static SEEDS: &[Seed] = &[
///     Seed::new("001_initial_users", seed_users),
/// ];
/// ```
#[cfg(feature = "sqlite")]
#[derive(Clone, Copy)]
pub struct Seed {
    pub id: &'static str,
    pub seed_fn: SqliteSeedFn,
}

#[cfg(feature = "sqlite")]
impl Seed {
    pub const fn new(id: &'static str, seed_fn: SqliteSeedFn) -> Self {
        Self { id, seed_fn }
    }
}

#[cfg(feature = "turso")]
#[derive(Clone, Copy)]
pub struct Seed {
    pub id: &'static str,
    pub seed_fn: TursoSeedFn,
}

#[cfg(feature = "turso")]
impl Seed {
    pub const fn new(id: &'static str, seed_fn: TursoSeedFn) -> Self {
        Self { id, seed_fn }
    }
}

/// Represents a single database migration with its unique identifier and SQL content.
///
/// Migrations are typically created at compile time by the `include_migrations!()` macro
/// from SQL files in your migrations directory. Each migration consists of:
/// - An identifier (usually the filename without extension)
/// - The SQL statements to execute
///
/// # Example in ICP Canister
/// ```
/// use ic_sql_migrate::Migration;
///
/// // Typically included via the include_migrations!() macro:
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

/// Includes all migration files discovered by the Builder at compile time.
///
/// This macro expands to a static slice of `Migration` structs containing
/// all SQL files found in the migrations directory. The migrations are ordered
/// alphabetically by filename, so it's recommended to prefix them with numbers
/// (e.g., `001_initial.sql`, `002_add_users.sql`).
///
/// # Prerequisites
/// You must call `ic_sql_migrate::Builder::new().build()` in your `build.rs` file to generate
/// the migration data that this macro includes.
///
/// # Example in ICP Canister
/// ```ignore
/// // In your canister lib.rs
/// use ic_cdk::{init, post_upgrade};
/// use ic_rusqlite::{with_connection, Connection};
///
/// static MIGRATIONS: &[ic_sql_migrate::Migration] = ic_sql_migrate::include_migrations!();
///
/// fn run_migrations() {
///     with_connection(|mut conn| {
///         let conn: &mut Connection = &mut conn;
///         ic_sql_migrate::sqlite::migrate(conn, MIGRATIONS).unwrap();
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
macro_rules! include_migrations {
    () => {
        include!(concat!(env!("OUT_DIR"), "/migrations_gen.rs"))
    };
}

/// Builder for configuring migration and seed discovery at compile time.
///
/// This builder allows you to customize the directories where migrations and seeds
/// are located. By default, it looks for migrations in `migrations/` and seeds in `src/seeds/`.
///
/// # Example in build.rs
/// ```no_run
/// // Use defaults (migrations/ and src/seeds/)
/// // If either directory doesn't exist, it will be skipped automatically
/// ic_sql_migrate::Builder::new().build().unwrap();
///
/// // Custom directories
/// ic_sql_migrate::Builder::new()
///     .with_migrations_dir("db/migrations")
///     .with_seeds_dir("src/db/seeds")
///     .build()
///     .unwrap();
/// ```
pub struct Builder {
    migrations_dir: String,
    seeds_dir: String,
}

impl Builder {
    /// Creates a new builder with default settings.
    ///
    /// Defaults:
    /// - Migrations directory: `migrations/`
    /// - Seeds directory: `src/seeds/`
    pub fn new() -> Self {
        Self {
            migrations_dir: "migrations".to_string(),
            seeds_dir: "src/seeds".to_string(),
        }
    }

    /// Sets the directory where migration SQL files are located.
    ///
    /// # Arguments
    /// * `dir` - Path relative to `Cargo.toml`
    pub fn with_migrations_dir(mut self, dir: impl Into<String>) -> Self {
        self.migrations_dir = dir.into();
        self
    }

    /// Sets the directory where seed Rust files are located.
    ///
    /// # Arguments
    /// * `dir` - Path relative to `Cargo.toml`
    pub fn with_seeds_dir(mut self, dir: impl Into<String>) -> Self {
        self.seeds_dir = dir.into();
        self
    }

    /// Executes the builder, discovering and generating code for migrations and seeds.
    ///
    /// This method automatically handles missing directories by generating empty arrays.
    /// You don't need to specify whether directories exist or not.
    ///
    /// # Errors
    /// Returns an I/O error if file system operations fail or required environment
    /// variables are not set.
    pub fn build(self) -> std::io::Result<()> {
        use std::env;
        use std::fs;
        use std::path::Path;

        let manifest_dir = env::var("CARGO_MANIFEST_DIR").map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "CARGO_MANIFEST_DIR not set")
        })?;

        let out_dir = env::var("OUT_DIR")
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::NotFound, "OUT_DIR not set"))?;

        // Process migrations
        let migrations_dir = Path::new(&manifest_dir).join(&self.migrations_dir);
        println!("cargo:rerun-if-changed={}", migrations_dir.display());

        let migrations_dest = Path::new(&out_dir).join("migrations_gen.rs");

        if !migrations_dir.exists() {
            fs::write(migrations_dest, "&[]")?;
        } else {
            let migration_files = collect_migration_files(&migrations_dir)?;
            let generated_code = generate_migrations_code(&migration_files);
            fs::write(migrations_dest, generated_code)?;
        }

        // Process seeds - generate mod.rs in the seeds directory
        let seeds_dir = Path::new(&manifest_dir).join(&self.seeds_dir);
        println!("cargo:rerun-if-changed={}", seeds_dir.display());

        if seeds_dir.exists() {
            let seed_files = collect_seed_files(&seeds_dir)?;
            if !seed_files.is_empty() {
                let generated_code = generate_seeds_code(&seed_files);
                let mod_file = seeds_dir.join("mod.rs");
                fs::write(mod_file, generated_code)?;
            }
        }

        Ok(())
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
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

/// Collects all Rust seed files from the specified directory.
///
/// Returns a sorted list of (seed_id, module_path) tuples.
/// Excludes mod.rs as it's the module declaration file.
fn collect_seed_files(seeds_dir: &std::path::Path) -> std::io::Result<Vec<(String, String)>> {
    use std::fs;

    let mut seed_files = Vec::new();

    let entries = fs::read_dir(seeds_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }

        if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
            // Skip mod.rs as it's the generated module file
            if file_stem == "mod" {
                continue;
            }

            let absolute_path = path.to_string_lossy().to_string();
            seed_files.push((file_stem.to_string(), absolute_path));

            println!("cargo:rerun-if-changed={}", path.display());
        }
    }

    seed_files.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(seed_files)
}

/// Generates a mod.rs file for the seeds module.
///
/// Creates a module file that:
/// 1. Declares all seed submodules in alphabetical order
/// 2. Exports a SEEDS constant with all seed functions in order
///
/// This function is feature-agnostic and generates generic code.
/// The actual type checking happens at compile time when the user's
/// crate is built with the appropriate feature.
fn generate_seeds_code(seed_files: &[(String, String)]) -> String {
    let mut code = String::new();

    code.push_str("// This file is auto-generated by ic-sql-migrate\n");
    code.push_str("// Do not edit manually\n\n");

    // Declare all submodules
    for (seed_id, _) in seed_files {
        code.push_str(&format!("pub mod {seed_id};\n"));
    }

    code.push('\n');
    code.push_str("use ic_sql_migrate::Seed;\n\n");

    // Create the SEEDS array
    code.push_str("pub static SEEDS: &[Seed] = &[\n");
    for (seed_id, _) in seed_files {
        code.push_str(&format!("    Seed::new(\"{seed_id}\", {seed_id}::seed),\n"));
    }
    code.push_str("];\n");

    code
}
