# ic-sql-migrate

A lightweight database migration library for Internet Computer (ICP) canisters with support for SQLite and Turso databases.

[![Crates.io](https://img.shields.io/crates/v/ic-sql-migrate.svg)](https://crates.io/crates/ic-sql-migrate)
[![Documentation](https://docs.rs/ic-sql-migrate/badge.svg)](https://docs.rs/ic-sql-migrate)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Table of Contents

- [Features](#features)
- [Quick Start](#quick-start)
  - [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Deployment Configuration](#deployment-configuration)
- [Basic Usage](#basic-usage)
- [Examples](#examples)
- [API Reference](#api-reference)
  - [Core Functions](#core-functions)
  - [Build Script Function](#build-script-function)
  - [Macros](#macros)
- [Migration Best Practices](#migration-best-practices)
- [How It Works](#how-it-works)
- [Project Structure](#project-structure)
- [Documentation](#documentation)
- [Differences Between Database Backends](#differences-between-database-backends)
- [Contributing](#contributing)
- [License](#license)
- [Acknowledgments](#acknowledgments)
- [Support](#support)

## Features

- 🚀 **Multi-Database Support**: Works with SQLite (via `ic-rusqlite`) and Turso databases
- 📦 **Compile-Time Embedding**: Migration files are embedded into your canister at compile time
- 🔄 **Automatic Migration**: Tracks and applies migrations automatically on canister init and upgrade
- 🔒 **Transactional**: All migrations run in transactions for safety
- 🏗️ **ICP Native**: Designed specifically for Internet Computer canisters

## Quick Start

### Prerequisites

**IMPORTANT**: You must enable exactly one database feature (`sqlite` or `turso`) for this library to work. There is no default feature.

In addition to having the Rust toolchain setup and dfx, you need to install the `wasi2ic` tool that replaces WebAssembly System Interface (WASI) specific function calls with their corresponding polyfill implementations. This allows you to run Wasm binaries compiled for wasm32-wasi on the Internet Computer.

```bash
cargo install wasi2ic
```

### Configure dfx.json
You also need to configure your `dfx.json` to compile for the `wasm32-wasip1` target and use `wasi2ic` to process the binary:

```json
{
  "canisters": {
    "your_canister": {
      "candid": "your_canister.did",
      "package": "your_canister",
      "type": "custom",
      "build": [
        "cargo build --target wasm32-wasip1 --release",
        "wasi2ic target/wasm32-wasip1/release/your_canister.wasm target/wasm32-wasip1/release/your_canister-wasi2ic.wasm"
      ],
      "wasm": "target/wasm32-wasip1/release/your_canister-wasi2ic.wasm"
    }
  }
}
```

### Installation

Add to both `[dependencies]` and `[build-dependencies]` in your `Cargo.toml`:

```toml
# For SQLite support
# Note: You MUST specify either "sqlite" or "turso" feature - there is no default
[dependencies]
ic-sql-migrate = { version = "0.0.4", features = ["sqlite"] }
ic-rusqlite = { version = "0.4.2", features = ["precompiled"], default-features = false }
ic-cdk = "0.18.7"

[build-dependencies]
ic-sql-migrate = "0.0.4"
```

Or for Turso:

```toml
# For Turso support
# Note: You MUST specify either "sqlite" or "turso" feature - there is no default
[dependencies]
ic-sql-migrate = { version = "0.0.4", features = ["turso"] }
turso = "0.1.4"
ic-cdk = "0.18.7"

[build-dependencies]
ic-sql-migrate = "0.0.4"
```

**Important:**
- You **MUST** choose exactly one database feature (`sqlite` or `turso`)
- The features are mutually exclusive (cannot use both)
- There is no default feature - the library will not work without selecting one

## Deployment Configuration

### dfx.json Setup (Required for SQLite)
For SQLite support, you need to configure your `dfx.json` to compile for the `wasm32-wasip1` target and use `wasi2ic` to process the binary:

```json
{
  "canisters": {
    "your_canister": {
      "candid": "your_canister.did",
      "package": "your_canister",
      "type": "custom",
      "build": [
        "cargo build --target wasm32-wasip1 --release",
        "wasi2ic target/wasm32-wasip1/release/your_canister.wasm target/wasm32-wasip1/release/your_canister-wasi2ic.wasm"
      ],
      "wasm": "target/wasm32-wasip1/release/your_canister-wasi2ic.wasm"
    }
  }
}
```

This configuration:
1. Compiles your canister for the `wasm32-wasip1` target (required for SQLite)
2. Uses `wasi2ic` to convert WASI function calls to IC-compatible polyfills
3. Points dfx to the processed WASM file for deployment

**Note**: Turso canisters use the standard `wasm32-unknown-unknown` target and don't require `wasi2ic` processing.

### Basic Usage

#### 1. Create migration files

Create a `migrations/` directory with SQL files. Each migration should be:
- **Numbered sequentially** (e.g., `000_initial.sql`, `001_add_users.sql`)
- **Idempotent when possible** (use `IF NOT EXISTS` clauses)
- **Forward-only** (this library doesn't support rollbacks)

```sql
-- migrations/000_initial.sql
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    email TEXT
);
```

#### 2. Set up build.rs

The `list()` function scans your migrations directory at compile time and generates code to embed the SQL files into your canister binary. This makes the migrations available as static data in your compiled canister:

```rust
fn main() {
    ic_sql_migrate::list(Some("migrations")).unwrap();
}
```

#### 3. Use in your canister

The `include!()` macro incorporates the migrations that were discovered by `list()` in step 2. It creates a static array of `Migration` objects containing your SQL files' contents, which you can then pass to the `up()` function to execute them:

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

### 📁 [SQLite Example](./examples/sqlite) - Advanced Database Operations
**Showcases high-performance SQLite on ICP with the full Chinook database:**
- **Complete Database**: Imports the entire Chinook music store database (11 tables, thousands of records)
- **Complex Queries**: Demonstrates advanced SQL operations including multi-table JOINs, aggregations, and analytics
- **Read Operations** (`test1-3`):
  - Top customers analysis with purchase history
  - Genre and artist revenue analytics
  - Sales trends and employee performance metrics
- **Write Operations** (`test4-5`):
  - Bulk invoice generation (250+ invoices with complex line items)
  - Massive playlist creation with track analytics and recommendations
- **Performance Tracking**: Each operation reports instruction counts, demonstrating SQLite's efficiency on ICP
- **Stress Testing**: Operations designed to process thousands of records and complex transactions

```bash
cd examples/sqlite
dfx start --clean
dfx deploy
dfx canister call sqlite run        # Verify migrations
dfx canister call sqlite test1      # Top customers analysis
dfx canister call sqlite test2      # Genre/artist analytics
dfx canister call sqlite test3      # Sales trends
dfx canister call sqlite test4      # Bulk invoice generation
dfx canister call sqlite test5      # Playlist manipulation
```

The SQLite example proves that complex, production-grade databases can run efficiently on the Internet Computer, with operations processing thousands of records in a single call while tracking instruction usage.

### 📁 [Turso Example](./examples/turso) - Basic Migration Demo
Shows Turso database usage in an ICP canister:
- Async migration execution
- Stable memory persistence with WASI polyfill
- Simple person table with basic operations
- Suitable for simpler use cases (Turso's SQL implementation is still evolving)

```bash
cd examples/turso
dfx start --clean
dfx deploy
dfx canister call turso run
```

**Note**: The SQLite example demonstrates significantly more advanced capabilities, making it the recommended choice for complex database operations on ICP.

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
2. **WASI to IC Conversion**: The `wasi2ic` tool converts WASI-specific function calls to IC-compatible polyfills, allowing the WASM binary to run on the Internet Computer
3. **Runtime**: `up()` function:
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
| **Additional Setup** | `wasi2ic` tool + dfx.json config | None |
| **WASM Target** | `wasm32-wasip1` | `wasm32-unknown-unknown` |
| **Build Processing** | Requires `wasi2ic` | Standard Rust build |
| **Connection Type** | `ic_rusqlite::Connection` | `turso::Connection` |
| **Migration Function** | `sqlite::up()` | `turso::up()` (async) |

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
