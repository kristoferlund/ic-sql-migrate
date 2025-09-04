use std::collections::HashSet;
use turso::Connection;

use crate::{Error, MigrateResult, Migration};

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
    .await
    .map_err(crate::Error::Turso)?;
    Ok(())
}

/// Retrieves the set of already applied migration IDs from the database.
async fn get_applied_migrations(conn: &Connection) -> MigrateResult<HashSet<String>> {
    let mut rows = conn
        .query("SELECT id FROM _migrations", ())
        .await
        .map_err(Error::Turso)?;

    let mut applied_set = HashSet::new();
    while let Some(row) = rows.next().await.map_err(Error::Turso)? {
        let value = row.get_value(0).map_err(Error::Turso)?;
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
///
/// # Arguments
/// * `conn` - Reference to the Turso connection wrapper
/// * `migrations` - Slice of migrations to apply in order
///
/// # Errors
/// Returns an error if:
/// - Database operations fail
/// - Migration SQL is invalid
/// - Transaction cannot be committed
pub async fn up(conn: &mut Connection, migrations: &[Migration]) -> MigrateResult<()> {
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
        up(&mut conn, migrations).await.unwrap();

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
        up(&mut conn, migrations).await.unwrap();
        up(&mut conn, migrations).await.unwrap();

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
        // Create in-memory Turso database
        let db = turso::Builder::new_local(":memory:").build().await.unwrap();
        let mut conn = db.connect().unwrap();

        let migrations = &[
            Migration::new("001_valid", "CREATE TABLE test (id INTEGER);"),
            Migration::new("002_invalid", "INVALID SQL STATEMENT;"),
        ];

        // Run migrations - should fail on second one
        let result = up(&mut conn, migrations).await;
        assert!(result.is_err());

        // Verify first migration was not committed due to transaction rollback
        let applied = get_applied_migrations(&conn).await.unwrap();
        assert!(applied.is_empty());

        // Verify table was not created
        let result = conn.query("SELECT * FROM test", ()).await;
        assert!(result.is_err()); // Table shouldn't exist
    }
}
