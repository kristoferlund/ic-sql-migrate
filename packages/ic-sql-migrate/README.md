# ic-sql-migrate

A lightweight SQL migration library for Internet Computer (ICP) canisters with support for multiple database backends.

[![Crates.io](https://img.shields.io/crates/v/ic-sql-migrate.svg)](https://crates.io/crates/ic-sql-migrate)
[![Documentation](https://docs.rs/ic-sql-migrate/badge.svg)](https://docs.rs/ic-sql-migrate)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- 🚀 **Multi-Database Support**: Works with both SQLite (via rusqlite) and Turso databases
  - SQLite: For Internet Computer canisters (with ic-rusqlite)
  - Turso: For standalone async Rust applications
- 📦 **Compile-Time Embedding**: Migration files are embedded into your binary at compile time
- 🔄 **Automatic Migration**: Tracks and applies migrations automatically
- ⚡ **Async/Sync Support**: Unified async API that works with both sync (SQLite) and async (Turso) databases
- 🔒 **Transactional**: All migrations run in transactions for safety
- 🎯 **Zero Runtime Files**: No need to manage migration files at runtime

## Installation

Add `ic-sql-migrate` to your `Cargo.toml` with the appropriate feature flag:

### For SQLite support:
```toml
[dependencies]
ic-sql-migrate = { version = "0.0.1", features = ["sqlite"] }
rusqlite = "0.37.0"
tokio = { version = "1", features = ["full"] }
```

### For Turso support:
```toml
[dependencies]
ic-sql-migrate = { version = "0.0.1", features = ["turso"] }
turso = "0.1.4"
tokio = { version = "1", features = ["full"] }
```

### Quick Start

**Note:** The `from_sqlite()` and `from_turso()` functions are necessary to wrap the native database connections into our unified `Connection` type. This allows the migration system to work with different database backends while providing a consistent API.

### 1. Create your migration files

Create a `migrations` directory in your project root and add SQL migration files:

```
migrations/
├── 001_create_users.sql
├── 002_create_posts.sql
└── 003_add_indexes.sql
```

Example migration file (`migrations/001_create_users.sql`):
```sql
CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### 2. Set up build script

Create or update your `build.rs` file:

```rust
fn main() {
    // This will embed all SQL files from the migrations directory
    ic_sql_migrate::list(None).unwrap();
}
```

### 3. Include and run migrations

#### Using SQLite:

```rust
use ic_sql_migrate::{from_sqlite, up};

// Include all migrations at compile time
static MIGRATIONS: &[ic_sql_migrate::Migration] = ic_sql_migrate::include!();

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open SQLite connection
    let sqlite_conn = rusqlite::Connection::open("my_database.db")?;
    
    // Wrap for migration support
    let conn = from_sqlite(sqlite_conn);
    
    // Run all pending migrations
    up(&conn, MIGRATIONS).await?;
    
    println!("All migrations completed successfully!");
    Ok(())
}
```

#### Using Turso:

```rust
use ic_sql_migrate::{from_turso, up};
use turso::Builder;

// Include all migrations at compile time
static MIGRATIONS: &[ic_sql_migrate::Migration] = ic_sql_migrate::include!();

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create Turso database connection
    let db = Builder::new_local("my_database.db").build().await?;
    let turso_conn = db.connect()?;
    
    // Wrap for migration support
    let conn = from_turso(turso_conn);
    
    // Run all pending migrations
    up(&conn, MIGRATIONS).await?;
    
    println!("All migrations completed successfully!");
    Ok(())
}
```

## How It Works

1. **Build Time**: The `list()` function in your `build.rs` scans the migrations directory and generates code to embed all SQL files into your binary.

2. **Runtime**: The `up()` function:
   - Creates a `_migrations` table to track applied migrations
   - Compares embedded migrations with applied ones
   - Executes pending migrations in order
   - Records each migration as applied
   - All operations happen in a transaction for safety

## Advanced Usage

### Custom Migration Directory

You can specify a custom directory name for your migrations:

```rust
// In build.rs
fn main() {
    ic_sql_migrate::list(Some("database/migrations")).unwrap();
}
```

### Manual Migration Creation

Instead of using files, you can create migrations programmatically:

```rust
use ic_sql_migrate::Migration;

let migrations = &[
    Migration::new(
        "001_create_users",
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);"
    ),
    Migration::new(
        "002_add_email",
        "ALTER TABLE users ADD COLUMN email TEXT;"
    ),
];

// Run migrations
up(&conn, migrations).await?;
```

### Working with the Connection Type

Both database types are wrapped in the unified `Connection` enum, allowing you to write database-agnostic code:

```rust
use ic_sql_migrate::Connection;

async fn setup_database(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    // This works with both SQLite and Turso connections
    conn.execute(
        "INSERT INTO users (username, email) VALUES (?, ?)",
        &["alice", "alice@example.com"]
    ).await?;
    
    Ok(())
}
```

## Migration Best Practices

1. **Naming Convention**: Use a consistent naming pattern like `XXX_description.sql` where XXX is a sequential number (e.g., `001_create_users.sql`, `002_add_posts.sql`)

2. **Idempotency**: While migrations are tracked and won't run twice, write your migrations to be idempotent when possible

3. **Backwards Compatibility**: Always write migrations that maintain backwards compatibility with existing data

4. **Testing**: Test your migrations in a development environment before deploying to production

5. **Small, Focused Migrations**: Keep each migration focused on a single change for easier debugging and rollback

## Examples

Check out the `examples` directory for complete working examples:

- `examples/sqlite/` - Internet Computer canister using SQLite with ic-rusqlite
- `examples/turso/` - Standalone Rust application using Turso

The SQLite example shows IC canister deployment:
```bash
cd examples/sqlite
dfx deploy
dfx canister call backend run
```

The Turso example runs as a regular Rust application:
```bash
cd examples/turso
cargo run
```

## Feature Flags

- `sqlite` - Enables SQLite support via rusqlite (use this for IC canisters)
- `turso` - Enables Turso database support (use this for standalone async Rust apps)

**Important:** You must enable exactly one database feature. The library does not have a default database backend.

### When to Use Which Feature

| Use Case | Feature Flag | Database | Example |
|----------|-------------|----------|---------|
| Internet Computer Canisters | `sqlite` | SQLite via ic-rusqlite | See `examples/sqlite/` |
| Standalone Rust Applications | `turso` | Turso (async SQLite) | See `examples/turso/` |
| Web Servers (async) | `turso` | Turso | - |
| CLI Tools (sync) | `sqlite` | SQLite via rusqlite | - |

## API Reference

### Core Functions

#### `up(conn: &Connection, migrations: &[Migration]) -> MigrateResult<()>`
Executes all pending migrations in order.

#### `from_sqlite(conn: rusqlite::Connection) -> Connection`
Wraps a rusqlite connection for use with the migration system (requires `sqlite` feature).

#### `from_turso(conn: turso::Connection) -> Connection`
Wraps a Turso connection for use with the migration system (requires `turso` feature).

### Build Script Functions

#### `list(migrations_dir_name: Option<&str>) -> std::io::Result<()>`
Scans for migration files and generates code to embed them. Call this in your `build.rs`.

### Macros

#### `ic_sql_migrate::include!()`
Includes all migrations discovered by the `list()` function at compile time.

## Migration Table Schema

The library automatically creates a `_migrations` table to track applied migrations:

```sql
CREATE TABLE _migrations (
    id TEXT PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
)
```

## Error Handling

The library uses a custom `Error` type that encompasses various error scenarios:

- Database connection errors
- SQL execution errors
- Migration-specific errors
- I/O errors (during build time)

All errors implement the standard `Error` trait and can be converted to `Box<dyn std::error::Error>`.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Author

Kristofer Lund

## Acknowledgments

This library is designed specifically for Internet Computer (ICP) canisters but works perfectly well in any Rust application that needs SQL migrations.