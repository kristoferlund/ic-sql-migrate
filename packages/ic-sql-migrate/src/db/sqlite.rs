//! SQLite database migration support for ICP canisters.
//!
//! This module provides migration functionality for SQLite databases in Internet Computer canisters
//! using the `ic-rusqlite` crate. It manages database schema versioning through a `_migrations` table
//! that tracks which migrations have been applied.
//!
//! # Features
//! - Automatic migration tracking via `_migrations` table
//! - Transactional migration execution (all-or-nothing)
//! - Idempotent migrations (safe to run multiple times)
//! - Ordered execution of pending migrations
//!
//! # Usage in ICP Canisters
//! ```ignore
//! use ic_cdk::{init, post_upgrade, pre_upgrade};
//! use ic_rusqlite::{close_connection, with_connection, Connection};
//!
//! static MIGRATIONS: &[ic_sql_migrate::Migration] = ic_sql_migrate::include!();
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

use rusqlite::Connection;
use std::collections::HashSet;

use crate::{Error, MigrateResult, Migration, Seed};

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
/// If any migration fails, all changes are rolled back.
///
/// # Arguments
/// * `conn` - Mutable reference to the SQLite connection
/// * `migrations` - Slice of migrations to apply in order
///
/// # Returns
/// * `Ok(())` - If all pending migrations were successfully applied or if there were no pending migrations
/// * `Err(Error)` - If any migration failed to execute
///
/// # Errors
/// Returns an error if:
/// - Database operations fail
/// - Migration SQL is invalid
/// - Transaction cannot be committed
///
/// # Example in ICP Canister
/// ```ignore
/// use ic_rusqlite::{with_connection, Connection};
/// use ic_sql_migrate::{Migration, sqlite};
///
/// static MIGRATIONS: &[Migration] = &[
///     Migration::new("001_initial", "CREATE TABLE users (id INTEGER PRIMARY KEY);"),
///     Migration::new("002_add_email", "ALTER TABLE users ADD COLUMN email TEXT;"),
/// ];
///
/// fn apply_migrations() {
///     with_connection(|mut conn| {
///         let conn: &mut Connection = &mut conn;
///         sqlite::migrate(conn, MIGRATIONS).unwrap();
///     });
/// }
/// ```
pub fn migrate(conn: &mut Connection, migrations: &[Migration]) -> MigrateResult<()> {
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

/// Ensures the seeds tracking table exists in the database.
///
/// Creates a `_seeds` table if it doesn't exist, which tracks:
/// - `id`: The unique identifier of each applied seed
/// - `applied_at`: Timestamp when the seed was applied
fn ensure_seeds_table(conn: &mut Connection) -> MigrateResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS _seeds (
            id TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    Ok(())
}

/// Retrieves the set of already applied seed IDs from the database.
fn get_applied_seeds(conn: &Connection) -> MigrateResult<HashSet<String>> {
    let mut statement = conn.prepare("SELECT id FROM _seeds")?;

    let seed_ids = statement.query_map([], |row| row.get::<_, String>(0))?;

    let mut applied_set = HashSet::new();
    for id in seed_ids.into_iter().flatten() {
        applied_set.insert(id);
    }

    Ok(applied_set)
}

/// Executes all pending seeds in order.
///
/// This function:
/// 1. Ensures the seeds tracking table exists
/// 2. Identifies which seeds have already been applied
/// 3. Executes pending seeds in the order they appear in the slice
/// 4. Records each seed as applied
///
/// All seeds are executed within a single transaction for atomicity.
/// If any seed fails, all changes are rolled back.
///
/// # Arguments
/// * `conn` - Mutable reference to the SQLite connection
/// * `seeds` - Slice of seeds to apply in order
///
/// # Returns
/// * `Ok(())` - If all pending seeds were successfully applied or if there were no pending seeds
/// * `Err(Error)` - If any seed failed to execute
///
/// # Errors
/// Returns an error if:
/// - Database operations fail
/// - Seed function returns an error
/// - Transaction cannot be committed
///
/// # Example
/// ```ignore
/// use ic_rusqlite::{with_connection, Connection};
/// use ic_sql_migrate::{Seed, sqlite};
///
/// fn seed_users(conn: &mut Connection) -> ic_sql_migrate::MigrateResult<()> {
///     conn.execute("INSERT INTO users (name) VALUES ('Alice')", [])?;
///     Ok(())
/// }
///
/// static SEEDS: &[Seed] = &[
///     Seed::new("001_users", seed_users),
/// ];
///
/// fn apply_seeds() {
///     with_connection(|mut conn| {
///         let conn: &mut Connection = &mut conn;
///         sqlite::seed(conn, SEEDS).unwrap();
///     });
/// }
/// ```
pub fn seed(conn: &mut Connection, seeds: &[Seed]) -> MigrateResult<()> {
    ensure_seeds_table(conn)?;
    let applied_seeds = get_applied_seeds(conn)?;

    let pending_seeds: Vec<&Seed> = seeds
        .iter()
        .filter(|s| !applied_seeds.contains(s.id))
        .collect();

    if pending_seeds.is_empty() {
        return Ok(());
    }

    for seed in pending_seeds {
        let tx = conn.transaction()?;

        (seed.seed_fn)(&tx).map_err(|e| Error::MigrationFailed {
            id: seed.id.to_string(),
            message: e.to_string(),
        })?;

        tx.execute("INSERT INTO _seeds(id) VALUES (?)", [seed.id])?;

        tx.commit()?;
    }

    Ok(())
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
        migrate(&mut conn, migrations).unwrap();

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
        migrate(&mut conn, migrations).unwrap();
        migrate(&mut conn, migrations).unwrap();

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

        let result = migrate(&mut conn, migrations);
        assert!(result.is_err());

        let applied = get_applied_migrations(&conn).unwrap();
        assert!(applied.is_empty());

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='test'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_ensure_seeds_table() {
        let mut conn = Connection::open_in_memory().unwrap();
        ensure_seeds_table(&mut conn).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='_seeds'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    fn seed_test_data(conn: &Connection) -> MigrateResult<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS test_users (id INTEGER PRIMARY KEY, name TEXT)",
            [],
        )?;
        conn.execute("INSERT INTO test_users (name) VALUES ('Alice')", [])?;
        conn.execute("INSERT INTO test_users (name) VALUES ('Bob')", [])?;
        Ok(())
    }

    fn seed_more_data(conn: &Connection) -> MigrateResult<()> {
        conn.execute("INSERT INTO test_users (name) VALUES ('Charlie')", [])?;
        Ok(())
    }

    #[test]
    fn test_seed_execution() {
        let mut conn = Connection::open_in_memory().unwrap();

        let seeds = &[
            Seed::new("001_initial", seed_test_data),
            Seed::new("002_more", seed_more_data),
        ];

        seed(&mut conn, seeds).unwrap();

        let applied = get_applied_seeds(&conn).unwrap();
        assert!(applied.contains("001_initial"));
        assert!(applied.contains("002_more"));

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM test_users", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_seed_idempotency() {
        let mut conn = Connection::open_in_memory().unwrap();

        let seeds = &[Seed::new("001_test", seed_test_data)];

        seed(&mut conn, seeds).unwrap();
        seed(&mut conn, seeds).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM _seeds WHERE id='001_test'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        let user_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM test_users", [], |row| row.get(0))
            .unwrap();
        assert_eq!(user_count, 2);
    }
}
