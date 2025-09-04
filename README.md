# ic-sql-migrate

A lightweight database migration library for Internet Computer (ICP) canisters with support for SQLite and Turso databases.

[![Crates.io](https://img.shields.io/crates/v/ic-sql-migrate.svg)](https://crates.io/crates/ic-sql-migrate)
[![Documentation](https://docs.rs/ic-sql-migrate/badge.svg)](https://docs.rs/ic-sql-migrate)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- 🚀 **Multi-Database Support**: Works with SQLite (via `ic-rusqlite`) and Turso databases
- 📦 **Compile-Time Embedding**: Migration files are embedded into your canister at compile time
- 🔄 **Automatic Migration**: Tracks and applies migrations automatically on canister init and upgrade
- 🔒 **Transactional**: All migrations run in transactions for safety
- 🎯 **Zero Runtime Files**: No need to manage migration files at runtime
- 🏗️ **ICP Native**: Designed specifically for Internet Computer canisters

## Quick Start

### Prerequisites

**IMPORTANT**: You must enable exactly one database feature (`sqlite` or `turso`) for this library to work. There is no default feature.

#### For SQLite Support
SQLite support requires the WASI SDK toolchain. Follow the setup instructions at [ic-rusqlite](https://crates.io/crates/ic-rusqlite) or run:

```bash
curl -fsSL https://raw.githubusercontent.com/wasm-forge/ic-rusqlite/main/prepare.sh | sh
```

#### For Turso Support
No additional toolchain setup required beyond Rust and DFX.

### Installation

Add to both `[dependencies]` and `[build-dependencies]` in your `Cargo.toml`:

```toml
# For SQLite support
# Note: You MUST specify either "sqlite" or "turso" feature - there is no default
[dependencies]
ic-sql-migrate = { version = "0.0.2", features = ["sqlite"] }
ic-rusqlite = "0.37.0"
ic-cdk = "0.16"

[build-dependencies]
ic-sql-migrate = "0.0.2"
```

Or for Turso:

```toml
# For Turso support  
# Note: You MUST specify either "sqlite" or "turso" feature - there is no default
[dependencies]
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

### Basic Usage

#### 1. Create migration files

Create a `migrations/` directory with SQL files:

```sql
-- migrations/000_initial.sql
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    email TEXT
);
```

#### 2. Set up build.rs

```rust
fn main() {
    ic_sql_migrate::list(Some("migrations")).unwrap();
}
```

#### 3. Use in your canister

**SQLite Example:**

```rust
use ic_cdk::{init, post_upgrade, pre_upgrade};
use ic_rusqlite::{close_connection, with_connection, Connection};

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

**Turso Example:**

```rust
use ic_cdk::{init, post_upgrade, pre_upgrade};
use turso::Connection;

static MIGRATIONS: &[ic_sql_migrate::Migration] = ic_sql_migrate::include!();

async fn run_migrations() {
    let mut conn = get_connection().await;
    ic_sql_migrate::turso::up(&mut conn, MIGRATIONS).await.unwrap();
}

#[init]
async fn init() {
    // Initialize storage
    run_migrations().await;
}

#[post_upgrade]
async fn post_upgrade() {
    // Re-initialize storage
    run_migrations().await;
}
```

## Examples

Complete working examples are provided for both database backends:

### 📁 [SQLite Example](./examples/sqlite)
Demonstrates SQLite integration with `ic-rusqlite` in an ICP canister:
- Synchronous migration execution
- Automatic connection management
- 5 sample migrations creating and populating a `person` table

```bash
cd examples/sqlite
dfx start --clean
dfx deploy
dfx canister call sqlite run
```

### 📁 [Turso Example](./examples/turso)  
Shows Turso database usage in an ICP canister:
- Async migration execution
- Stable memory persistence with WASI polyfill
- Same migrations as SQLite example for comparison

```bash
cd examples/turso
dfx start --clean
dfx deploy
dfx canister call turso run
```

Both examples implement the same functionality, demonstrating that `ic-sql-migrate` provides a consistent migration experience regardless of the database backend.

## API Reference

### Core Functions

#### For SQLite
```rust
pub fn up(conn: &mut rusqlite::Connection, migrations: &[Migration]) -> MigrateResult<()>
```

#### For Turso
```rust
pub async fn up(conn: &mut turso::Connection, migrations: &[Migration]) -> MigrateResult<()>
```

### Build Script Function

```rust
pub fn list(migrations_dir_name: Option<&str>) -> std::io::Result<()>
```
Discovers and embeds migration files at compile time.

### Macros

#### `ic_sql_migrate::include!()`
Includes all migrations discovered by `list()` at compile time.

## Migration Best Practices

1. **Naming Convention**: Use sequential numbering (e.g., `001_init.sql`, `002_add_users.sql`)
2. **Forward-Only**: This library supports forward migrations only (no rollbacks)
3. **Idempotent SQL**: Use `IF NOT EXISTS` clauses when possible
4. **Small Changes**: Keep each migration focused on a single change
5. **Test Locally**: Always test with `dfx deploy --local` before mainnet

## How It Works

1. **Build Time**: `list()` in `build.rs` scans your migrations directory and generates code to embed SQL files
2. **Runtime**: `up()` function:
   - Creates a `_migrations` table to track applied migrations
   - Compares embedded migrations with applied ones
   - Executes pending migrations in order within a transaction
   - Records each successful migration

## Project Structure

```
ic-sql-migrate/
├── packages/
│   └── ic-sql-migrate/     # Main library crate
├── examples/
│   ├── sqlite/             # SQLite example canister
│   └── turso/              # Turso example canister
└── README.md               # This file
```

## Documentation

- [API Documentation](https://docs.rs/ic-sql-migrate)
- [Crates.io Package](https://crates.io/crates/ic-sql-migrate)
- [Changelog](./packages/ic-sql-migrate/CHANGELOG.md)

## Differences Between Database Backends

| Feature | SQLite | Turso |
|---------|--------|-------|
| **Async Operations** | No | Yes |
| **Additional Setup** | WASI SDK required | None |
| **Connection Type** | `ic_rusqlite::Connection` | `turso::Connection` |
| **Migration Function** | `sqlite::up()` | `turso::up()` (async) |
| **Best For** | Simple, synchronous operations | Async, distributed applications |

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. See our [Contributing Guidelines](./CONTRIBUTING.md) for more details.

## License

This project is licensed under the MIT License - see the [LICENSE](./LICENSE) file for details.

## Acknowledgments

- Built for the Internet Computer ecosystem
- Inspired by traditional database migration tools
- Special thanks to the [ic-rusqlite](https://crates.io/crates/ic-rusqlite) and [Turso](https://turso.tech) teams

## Support

For questions and support:
- Open an [issue](https://github.com/kristoferlund/ic-sql-migrate/issues)
- Check the [examples](./examples) for working implementations
- Read the [API documentation](https://docs.rs/ic-sql-migrate)