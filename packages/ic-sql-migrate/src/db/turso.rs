//! Turso database migration support for ICP canisters.
//!
//! This module provides migration functionality for Turso databases in Internet Computer canisters
//! using the `turso` crate. It manages database schema versioning through a `_migrations` table
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
//! use turso::Connection;
//! use std::cell::RefCell;
//!
//! static MIGRATIONS: &[ic_sql_migrate::Migration] = ic_sql_migrate::include!();
//!
//! thread_local! {
//!     static CONNECTION: RefCell<Option<Connection>> = const { RefCell::new(None) };
//! }
//!
//! async fn get_connection() -> Connection {
//!     // Initialize or get existing connection
//!     // See examples for complete implementation
//! }
//!
//! async fn run_migrations() {
//!     let mut conn = get_connection().await;
//!     ic_sql_migrate::turso::migrate(&mut conn, MIGRATIONS).await.unwrap();
//! }
//!
//! #[init]
//! async fn init() {
//!     // Initialize memory/storage
//!     run_migrations().await;
//! }
//!
//! #[pre_upgrade]
//! fn pre_upgrade() {
//!     // Close database connection
//! }
//!
//! #[post_upgrade]
//! async fn post_upgrade() {
//!     // Re-initialize memory/storage
//!     run_migrations().await;
//! }
//! ```

use std::collections::HashSet;
use turso::Connection;

use crate::{Error, MigrateResult, Migration, Seed};

/// Ensures the migrations tracking table exists in the database.
///
/// Creates a `_migrations` table if it doesn't exist, which tracks:
/// - `id`: The unique identifier of each applied migration
/// - `applied_at`: Timestamp when the migration was applied
async fn ensure_migrations_table(conn: &Connection) -> MigrateResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS _migrations (
            id TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        (),
    )
    .await?;
    Ok(())
}

/// Retrieves the set of already applied migration IDs from the database.
async fn get_applied_migrations(conn: &Connection) -> MigrateResult<HashSet<String>> {
    let mut rows = conn.query("SELECT id FROM _migrations", ()).await?;

    let mut applied_set = HashSet::new();
    while let Some(row) = rows.next().await? {
        let value = row.get_value(0)?;
        if let Some(text) = value.as_text() {
            applied_set.insert(text.to_string());
        }
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
/// * `conn` - Mutable reference to the Turso connection
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
/// ```no_run
/// use turso::Connection;
/// use ic_sql_migrate::Migration;
///
/// static MIGRATIONS: &[Migration] = &[
///     Migration::new("001_initial", "CREATE TABLE users (id INTEGER PRIMARY KEY);"),
///     Migration::new("002_add_email", "ALTER TABLE users ADD COLUMN email TEXT;"),
/// ];
///
/// async fn apply_migrations(conn: &mut Connection) {
///     ic_sql_migrate::turso::migrate(conn, MIGRATIONS).await.unwrap();
/// }
/// ```
pub async fn migrate(conn: &mut Connection, migrations: &[Migration]) -> MigrateResult<()> {
    ensure_migrations_table(conn).await?;
    let applied_migrations = get_applied_migrations(conn).await?;

    // Check if there are any migrations to apply
    let pending_migrations: Vec<&Migration> = migrations
        .iter()
        .filter(|m| !applied_migrations.contains(m.id))
        .collect();

    if pending_migrations.is_empty() {
        return Ok(());
    }

    // Start transaction for all migrations
    let tx = conn.transaction().await?;

    for migration in pending_migrations {
        if let Err(e) = tx.execute_batch(migration.sql).await {
            tx.rollback().await?;
            return Err(Error::MigrationFailed {
                id: migration.id.to_string(),
                message: e.to_string(),
            });
        }

        // Record migration as applied
        if let Err(e) = tx
            .execute("INSERT INTO _migrations(id) VALUES (?)", [migration.id])
            .await
        {
            tx.rollback().await?;
            return Err(Error::MigrationFailed {
                id: migration.id.to_string(),
                message: e.to_string(),
            });
        };
    }

    // Commit all migrations atomically
    tx.commit().await?;

    Ok(())
}

/// Ensures the seeds tracking table exists in the database.
///
/// Creates a `_seeds` table if it doesn't exist, which tracks:
/// - `id`: The unique identifier of each applied seed
/// - `applied_at`: Timestamp when the seed was applied
async fn ensure_seeds_table(conn: &Connection) -> MigrateResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS _seeds (
            id TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        (),
    )
    .await?;
    Ok(())
}

/// Retrieves the set of already applied seed IDs from the database.
async fn get_applied_seeds(conn: &Connection) -> MigrateResult<HashSet<String>> {
    let mut rows = conn.query("SELECT id FROM _seeds", ()).await?;

    let mut applied_set = HashSet::new();
    while let Some(row) = rows.next().await? {
        let value = row.get_value(0)?;
        if let Some(text) = value.as_text() {
            applied_set.insert(text.to_string());
        }
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
/// All seeds are executed within individual transactions for atomicity.
/// If any seed fails, changes for that seed are rolled back.
///
/// # Arguments
/// * `conn` - Mutable reference to the Turso connection
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
/// ```no_run
/// use turso::Connection;
/// use ic_sql_migrate::Seed;
///
/// async fn seed_users(conn: &mut Connection) -> ic_sql_migrate::MigrateResult<()> {
///     conn.execute("INSERT INTO users (name) VALUES ('Alice')", ()).await?;
///     Ok(())
/// }
///
/// async fn apply_seeds(conn: &mut Connection) {
///     // Seeds would be defined here
///     // ic_sql_migrate::turso::seed(conn, SEEDS).await.unwrap();
/// }
/// ```
pub async fn seed(conn: &mut Connection, seeds: &[Seed]) -> MigrateResult<()> {
    ensure_seeds_table(conn).await?;
    let applied_seeds = get_applied_seeds(conn).await?;

    let pending_seeds: Vec<&Seed> = seeds
        .iter()
        .filter(|s| !applied_seeds.contains(s.id))
        .collect();

    if pending_seeds.is_empty() {
        return Ok(());
    }

    for seed in pending_seeds {
        let tx = conn.transaction().await?;

        if let Err(e) = (seed.seed_fn)(&tx).await {
            tx.rollback().await?;
            return Err(Error::MigrationFailed {
                id: seed.id.to_string(),
                message: e.to_string(),
            });
        }

        if let Err(e) = tx
            .execute("INSERT INTO _seeds(id) VALUES (?)", [seed.id])
            .await
        {
            tx.rollback().await?;
            return Err(Error::MigrationFailed {
                id: seed.id.to_string(),
                message: e.to_string(),
            });
        }

        tx.commit().await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_migration_creation() {
        let migration = Migration::new("001_test", "CREATE TABLE test (id INTEGER);");
        assert_eq!(migration.id, "001_test");
        assert_eq!(migration.sql, "CREATE TABLE test (id INTEGER);");
    }

    #[tokio::test]
    async fn test_ensure_migrations_table() {
        // Create in-memory Turso database
        let db = turso::Builder::new_local(":memory:").build().await.unwrap();
        let conn = db.connect().unwrap();

        ensure_migrations_table(&conn).await.unwrap();

        // Verify table exists by querying it
        let mut rows = conn
            .query("SELECT COUNT(*) FROM _migrations", ())
            .await
            .unwrap();
        assert!(rows.next().await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_up_migrations() {
        // Create in-memory Turso database
        let db = turso::Builder::new_local(":memory:").build().await.unwrap();
        let mut conn = db.connect().unwrap();

        let migrations = &[
            Migration::new(
                "001_create_users",
                "CREATE TABLE users (id INTEGER PRIMARY KEY);",
            ),
            Migration::new("002_add_email", "ALTER TABLE users ADD COLUMN email TEXT;"),
        ];

        // Run migrations
        migrate(&mut conn, migrations).await.unwrap();

        // Verify migrations were applied
        let applied = get_applied_migrations(&conn).await.unwrap();
        assert!(applied.contains("001_create_users"));
        assert!(applied.contains("002_add_email"));

        // Verify table structure by checking if we can query the email column
        let result = conn
            .execute(
                "INSERT INTO users (id, email) VALUES (1, 'test@test.com')",
                (),
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_up_migrations_idempotency() {
        // Create in-memory Turso database
        let db = turso::Builder::new_local(":memory:").build().await.unwrap();
        let mut conn = db.connect().unwrap();

        let migrations = &[Migration::new(
            "001_test",
            "CREATE TABLE test (id INTEGER);",
        )];

        // Run migrations twice
        migrate(&mut conn, migrations).await.unwrap();
        migrate(&mut conn, migrations).await.unwrap();

        // Should only be applied once
        let mut rows = conn
            .query("SELECT COUNT(*) FROM _migrations WHERE id='001_test'", ())
            .await
            .unwrap();

        if let Some(row) = rows.next().await.unwrap() {
            let count = row.get_value(0).unwrap();
            assert_eq!(*count.as_integer().unwrap(), 1);
        } else {
            panic!("Expected a count result");
        }
    }

    #[tokio::test]
    async fn test_migration_failure_rollback() {
        let db = turso::Builder::new_local(":memory:").build().await.unwrap();
        let mut conn = db.connect().unwrap();

        let migrations = &[
            Migration::new("001_valid", "CREATE TABLE test (id INTEGER);"),
            Migration::new("002_invalid", "INVALID SQL STATEMENT;"),
        ];

        let result = migrate(&mut conn, migrations).await;
        assert!(result.is_err());

        let applied = get_applied_migrations(&conn).await.unwrap();
        assert!(applied.is_empty());

        let result = conn.query("SELECT * FROM test", ()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ensure_seeds_table() {
        let db = turso::Builder::new_local(":memory:").build().await.unwrap();
        let conn = db.connect().unwrap();

        ensure_seeds_table(&conn).await.unwrap();

        let mut rows = conn
            .query("SELECT COUNT(*) FROM _seeds", ())
            .await
            .unwrap();
        assert!(rows.next().await.unwrap().is_some());
    }

    fn seed_test_data(conn: &Connection) -> std::pin::Pin<Box<dyn std::future::Future<Output = MigrateResult<()>> + Send>> {
        let conn = conn.clone();
        Box::pin(async move {
            conn.execute("CREATE TABLE IF NOT EXISTS test_users (id INTEGER PRIMARY KEY, name TEXT)", ()).await?;
            conn.execute("INSERT INTO test_users (name) VALUES ('Alice')", ()).await?;
            conn.execute("INSERT INTO test_users (name) VALUES ('Bob')", ()).await?;
            Ok(())
        })
    }

    fn seed_more_data(conn: &Connection) -> std::pin::Pin<Box<dyn std::future::Future<Output = MigrateResult<()>> + Send>> {
        let conn = conn.clone();
        Box::pin(async move {
            conn.execute("INSERT INTO test_users (name) VALUES ('Charlie')", ()).await?;
            Ok(())
        })
    }

    #[tokio::test]
    async fn test_seed_execution() {
        let db = turso::Builder::new_local(":memory:").build().await.unwrap();
        let mut conn = db.connect().unwrap();

        let seeds = &[
            Seed::new("001_initial", seed_test_data),
            Seed::new("002_more", seed_more_data),
        ];

        seed(&mut conn, seeds).await.unwrap();

        let applied = get_applied_seeds(&conn).await.unwrap();
        assert!(applied.contains("001_initial"));
        assert!(applied.contains("002_more"));

        let mut rows = conn
            .query("SELECT COUNT(*) FROM test_users", ())
            .await
            .unwrap();

        if let Some(row) = rows.next().await.unwrap() {
            let count = row.get_value(0).unwrap();
            assert_eq!(*count.as_integer().unwrap(), 3);
        } else {
            panic!("Expected count result");
        }
    }

    #[tokio::test]
    async fn test_seed_idempotency() {
        let db = turso::Builder::new_local(":memory:").build().await.unwrap();
        let mut conn = db.connect().unwrap();

        let seeds = &[Seed::new("001_test", seed_test_data)];

        seed(&mut conn, seeds).await.unwrap();
        seed(&mut conn, seeds).await.unwrap();

        let mut rows = conn
            .query("SELECT COUNT(*) FROM _seeds WHERE id='001_test'", ())
            .await
            .unwrap();

        if let Some(row) = rows.next().await.unwrap() {
            let count = row.get_value(0).unwrap();
            assert_eq!(*count.as_integer().unwrap(), 1);
        } else {
            panic!("Expected count result");
        }

        let mut rows = conn
            .query("SELECT COUNT(*) FROM test_users", ())
            .await
            .unwrap();

        if let Some(row) = rows.next().await.unwrap() {
            let count = row.get_value(0).unwrap();
            assert_eq!(*count.as_integer().unwrap(), 2);
        } else {
            panic!("Expected count result");
        }
    }
}
