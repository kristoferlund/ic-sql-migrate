# ic-sql-migrate

A lightweight database migration library for Internet Computer (ICP) canisters with support for SQLite and Turso databases.

[![Crates.io](https://img.shields.io/crates/v/ic-sql-migrate.svg)](https://crates.io/crates/ic-sql-migrate)
[![Documentation](https://docs.rs/ic-sql-migrate/badge.svg)](https://docs.rs/ic-sql-migrate)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- 🚀 **Multi-Database Support**: Works with SQLite (via `ic-rusqlite`) and Turso databases
- 📦 **Compile-Time Embedding**: Migration files are embedded into your binary at compile time
- 🔄 **Automatic Migration**: Tracks and applies migrations automatically on canister init and upgrade
- 🔒 **Transactional**: All migrations run in transactions for safety
- 🏗️ **ICP Native**: Designed specifically for Internet Computer canisters

## Prerequisites

**IMPORTANT**: You must enable exactly one database feature (`sqlite` or `turso`) for this library to work. There is no default feature.

### For SQLite Support

Using SQLite in ICP canisters requires the WASI SDK toolchain. Follow the setup instructions at [ic-rusqlite](https://crates.io/crates/ic-rusqlite) or run this automated setup script:

```bash
curl -fsSL https://raw.githubusercontent.com/wasm-forge/ic-rusqlite/main/prepare.sh | sh
```

This will install:
- `wasi2ic` tool for WASI to IC compilation
- `wasm32-wasip1` Rust target
- WASI-SDK with WASI-oriented clang
- Required environment variables (`WASI_SDK_PATH` and `PATH`)

### For Turso Support

Turso requires no additional toolchain setup beyond the standard Rust and DFX installation.

## Installation

Add to both `[dependencies]` and `[build-dependencies]` in your `Cargo.toml`:

### For SQLite support (most common for ICP):
```toml
[dependencies]
# Note: You MUST specify either "sqlite" or "turso" feature - there is no default
ic-sql-migrate = { version = "0.0.2", features = ["sqlite"] }
ic-rusqlite = "0.37.0"
ic-cdk = "0.16"

[build-dependencies]
ic-sql-migrate = "0.0.1"
```

### For Turso support:
```toml
[dependencies]
# Note: You MUST specify either "sqlite" or "turso" feature - there is no default
ic-sql-migrate = { version = "0.0.2", features = ["turso"] }
turso = "0.1.4"
ic-cdk = "0.16"

[build-dependencies]
ic-sql-migrate = "0.0.2"
```

**Important:** 
- You **MUST** choose exactly one database feature (`sqlite` or `turso`)
- The features are mutually exclusive (cannot use both)
- There is no default feature - the library will not work without selecting one

## Quick Start

### 1. Create migration files

Create a `migrations` directory in your project root and add SQL files:

```
migrations/
├── 001_initial.sql
├── 002_add_users.sql
└── 003_add_indexes.sql
```

Example migration file (`migrations/001_initial.sql`):
```sql
CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### 2. Set up build.rs

Create a `build.rs` file in your project root:

```rust
fn main() {
    // This will embed all SQL files from the migrations directory
    ic_sql_migrate::list(Some("migrations")).unwrap();
}
```

### 3. Use in your canister

#### SQLite Example (with ic-rusqlite):

```rust
use ic_cdk::{init, post_upgrade, pre_upgrade};
use ic_rusqlite::{close_connection, with_connection, Connection};

// Include all migrations at compile time
static MIGRATIONS: &[ic_sql_migrate::Migration] = ic_sql_migrate::include!();

fn run_migrations() {
    with_connection(|mut conn| {
        let conn: &mut Connection = &mut conn;
        ic_sql_migrate::sqlite::up(conn, MIGRATIONS).unwrap();
    });
}

#[init]
fn init() {
    run_migrations();
}

#[pre_upgrade]
fn pre_upgrade() {
    close_connection();
}

#[post_upgrade]
fn post_upgrade() {
    run_migrations();
}
```

#### Turso Example:

```rust
use ic_cdk::{init, post_upgrade, pre_upgrade};
use turso::Connection;
use std::cell::RefCell;

static MIGRATIONS: &[ic_sql_migrate::Migration] = ic_sql_migrate::include!();

thread_local! {
    static CONNECTION: RefCell<Option<Connection>> = const { RefCell::new(None) };
}

async fn get_connection() -> Connection {
    // Initialize or return existing connection
    // See examples/turso for complete implementation
}

async fn run_migrations() {
    let mut conn = get_connection().await;
    ic_sql_migrate::turso::up(&mut conn, MIGRATIONS).await.unwrap();
}

#[init]
async fn init() {
    // Initialize memory/storage (see examples)
    run_migrations().await;
}

#[pre_upgrade]
fn pre_upgrade() {
    CONNECTION.with_borrow_mut(|c| *c = None);
}

#[post_upgrade]
async fn post_upgrade() {
    // Re-initialize memory/storage
    run_migrations().await;
}
```

## How It Works

1. **Build Time**: The `list()` function in `build.rs` scans your migrations directory and generates code to embed all SQL files into your canister binary.

2. **Canister Init/Upgrade**:
   - On `init`: Runs all migrations to set up the database schema
   - On `post_upgrade`: Runs any new migrations added since the last deployment

3. **Migration Tracking**: A `_migrations` table is automatically created to track which migrations have been applied, preventing duplicate execution.

4. **Transaction Safety**: All pending migrations run in a single transaction. If any migration fails, all changes are rolled back.

## Migration Best Practices

1. **Naming Convention**: Use sequential numbering like `001_description.sql`, `002_description.sql` to ensure correct execution order

2. **Forward-Only**: This library only supports forward migrations (no rollbacks). Plan your schema changes carefully.

3. **Idempotent SQL**: While migrations are tracked, write idempotent SQL when possible using `IF NOT EXISTS` clauses

4. **Small Changes**: Keep each migration focused on a single logical change

5. **Test Locally**: Always test migrations using `dfx deploy --local` before mainnet deployment

## Examples

Complete working examples are available in the repository:

- [`examples/sqlite`](https://github.com/kristoferlund/ic-sql-migrate/tree/main/examples/sqlite) - ICP canister with SQLite
- [`examples/turso`](https://github.com/kristoferlund/ic-sql-migrate/tree/main/examples/turso) - ICP canister with Turso

### Running the SQLite Example

```bash
cd examples/sqlite
dfx start --clean
dfx deploy
dfx canister call sqlite run '()'
```

## API Reference

### Core Functions

#### For SQLite
```rust
pub fn up(conn: &mut rusqlite::Connection, migrations: &[Migration]) -> MigrateResult<()>
```
Executes all pending migrations synchronously.

#### For Turso
```rust
pub async fn up(conn: &mut turso::Connection, migrations: &[Migration]) -> MigrateResult<()>
```
Executes all pending migrations asynchronously.

### Build Script Function

```rust
pub fn list(migrations_dir_name: Option<&str>) -> std::io::Result<()>
```
Discovers and embeds migration files at compile time. Call this in `build.rs`.

### Macros

#### `ic_sql_migrate::include!()`
Includes all migrations discovered by `list()` at compile time.

### Types

#### `Migration`
```rust
pub struct Migration {
    pub id: &'static str,    // Unique identifier (filename without extension)
    pub sql: &'static str,   // SQL statements to execute
}
```

#### `Error`
Custom error type that wraps database-specific errors and migration failures.

## Migration Table Schema

The library automatically creates this table:

```sql
CREATE TABLE _migrations (
    id TEXT PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
)
```

## Troubleshooting

### Library doesn't compile / "module not found" errors
You **must** enable either the `sqlite` or `turso` feature in your `Cargo.toml`. The library has no default features and will not work without explicitly selecting a database backend.

### "Both features enabled" error
You can only use one database backend at a time. Remove one of the features.

### Migrations not found
Ensure your migrations directory exists and contains `.sql` files, and that `build.rs` is properly configured.

### Migration failures
Check the canister logs with `dfx canister logs <canister_name>` for detailed error messages.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Author

Kristofer Lund

## Acknowledgments

Built specifically for the Internet Computer ecosystem to provide reliable database migrations for canisters using SQL databases.
