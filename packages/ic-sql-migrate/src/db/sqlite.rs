use rusqlite::Connection;
use std::collections::HashSet;

use crate::{Error, MigrateResult, Migration};

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
