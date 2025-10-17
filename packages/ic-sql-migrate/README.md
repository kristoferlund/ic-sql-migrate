# ic-sql-migrate

A lightweight database migration library for Internet Computer (ICP) canisters with support for SQLite (via `ic-rusqlite`) and Turso databases.

[![Crates.io](https://img.shields.io/crates/v/ic-sql-migrate.svg)](https://crates.io/crates/ic-sql-migrate)
[![Documentation](https://docs.rs/ic-sql-migrate/badge.svg)](https://docs.rs/ic-sql-migrate)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Table of Contents

- [Installation](#installation)
- [Deployment Configuration](#deployment-configuration)
- [Usage](#usage)
  - [1. Create Migration Files](#1-create-migration-files)
  - [2. Set Up build.rs](#2-set-up-buildrs)
  - [3. Use in Your Canister](#3-use-in-your-canister)
- [Data Seeding](#data-seeding)
- [API Reference](#api-reference)
- [How It Works](#how-it-works)
- [Migration Best Practices](#migration-best-practices)
- [Troubleshooting](#troubleshooting)
- [Examples](#examples)
- [Differences Between Database Backends](#differences-between-database-backends)
- [Contributing](#contributing)
- [License](#license)

## Installation

### Prerequisites

**IMPORTANT**: You must enable exactly one database feature (`sqlite` or `turso`) for this library to work. There is no default feature.

In addition to having the Rust toolchain setup and dfx, you need to install the `wasi2ic` tool (for SQLite only) that replaces WebAssembly System Interface (WASI) specific function calls with their corresponding polyfill implementations:

```bash
cargo install wasi2ic
```

### Add to Cargo.toml

For SQLite support (most common for ICP):

```toml
[dependencies]
ic-sql-migrate = { version = "0.0.4", features = ["sqlite"] }
ic-rusqlite = { version = "0.4.2", features = ["precompiled"], default-features = false }
ic-cdk = "0.18.7"

[build-dependencies]
ic-sql-migrate = "0.0.4"
```

For Turso support:

```toml
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

## Usage

### 1. Create Migration Files

Create a `migrations/` directory with SQL files. Each migration should be:
- **Numbered sequentially** (e.g., `000_initial.sql`, `001_add_users.sql`)
- **Idempotent when possible** (use `IF NOT EXISTS` clauses)
- **Forward-only** (this library doesn't support rollbacks)

Example migration file:

```sql
-- migrations/000_initial.sql
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    email TEXT
);
```

### 2. Set Up build.rs

Use the Builder to configure discovery of migrations and seeds at compile time:

```rust
fn main() {
    ic_sql_migrate::Builder::new()
        .with_migrations_dir("migrations")
        .with_seeds_dir("src/seeds")
        .build()
        .unwrap();
}
```

The Builder automatically handles missing directories by generating empty arrays.

### 3. Use in Your Canister

#### SQLite Example

```rust
use ic_cdk::{init, post_upgrade, pre_upgrade};
use ic_rusqlite::{close_connection, with_connection, Connection};

mod seeds;

static MIGRATIONS: &[ic_sql_migrate::Migration] = ic_sql_migrate::include_migrations!();

fn run_migrations_and_seeds() {
    with_connection(|mut conn| {
        let conn: &mut Connection = &mut conn;
        ic_sql_migrate::sqlite::migrate(conn, MIGRATIONS).unwrap();
        ic_sql_migrate::sqlite::seed(conn, seeds::SEEDS).unwrap();
    });
}

#[init]
fn init() {
    run_migrations_and_seeds();
}

#[pre_upgrade]
fn pre_upgrade() {
    close_connection();
}

#[post_upgrade]
fn post_upgrade() {
    run_migrations_and_seeds();
}
```

#### Turso Example

```rust
use ic_cdk::{init, post_upgrade, pre_upgrade};
use turso::Connection;

mod seeds;

static MIGRATIONS: &[ic_sql_migrate::Migration] = ic_sql_migrate::include_migrations!();

thread_local! {
    static CONNECTION: RefCell<Option<Connection>> = const { RefCell::new(None) };
}

async fn get_connection() -> Connection {
    if let Some(conn) = CONNECTION.with_borrow(|c| c.clone()) {
        conn
    } else {
        // Initialize connection
        init_db().await
    }
}

async fn run_migrations_and_seeds() {
    let mut conn = get_connection().await;
    ic_sql_migrate::turso::migrate(&mut conn, MIGRATIONS).await.unwrap();
    ic_sql_migrate::turso::seed(&mut conn, seeds::SEEDS).await.unwrap();
}

#[init]
async fn init() {
    run_migrations_and_seeds().await;
}

#[post_upgrade]
async fn post_upgrade() {
    run_migrations_and_seeds().await;
}
```

## Data Seeding

In addition to schema migrations, this library supports data seeding using Rust functions. Seeds are useful for populating initial data, test data, or reference data.

### Creating Seed Files

Create seed files in the `src/seeds/` directory (or a custom directory specified in `build.rs`). Each seed file is a regular Rust module (`.rs` file) that exports a `seed` function.

Seed files are executed in alphabetical order by filename, so use a sortable prefix:
- `src/seeds/seed_001_initial_users.rs`
- `src/seeds/seed_002_categories.rs`

### SQLite Seed Example

**File: `src/seeds/seed_001_initial_users.rs`**

```rust
use ic_sql_migrate::MigrateResult;
use ic_rusqlite::Connection;

pub fn seed(conn: &Connection) -> MigrateResult<()> {
    conn.execute(
        "INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com')",
        [],
    )?;
    conn.execute(
        "INSERT INTO users (name, email) VALUES ('Bob', 'bob@example.com')",
        [],
    )?;
    Ok(())
}
```

### Turso Seed Example

**File: `src/seeds/seed_001_initial_users.rs`**

```rust
use ic_sql_migrate::MigrateResult;
use turso::Connection;
use std::pin::Pin;
use std::future::Future;

pub fn seed(conn: &Connection) -> Pin<Box<dyn Future<Output = MigrateResult<()>> + Send>> {
    let conn = conn.clone();
    Box::pin(async move {
        conn.execute(
            "INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com')",
            (),
        ).await?;
        Ok(())
    })
}
```

### Using Seeds in Your Canister

**Step 1: Add the seeds module to your `src/lib.rs`:**

```rust
mod seeds;  // This is auto-generated by the build script
```

**Step 2: Use seeds in your lifecycle functions (see examples above)**

### Seed Best Practices

1. **Naming Convention**: Use sequential numbering with descriptive names (e.g., `seed_001_initial_users.rs`)
2. **One Seed Per File**: Each seed file should contain a single `pub fn seed()` function
3. **Part of Source Tree**: Seeds are in `src/seeds/`, giving you full IDE support and access to your app code
4. **Import from Your App**: You can import types, functions, and modules from your application using `crate::`
5. **Forward-Only**: Seeds do not support rollbacks - once applied, they remain
6. **Idempotent Functions**: Write seed functions that can safely run multiple times if needed
7. **Alphabetical Order**: Seeds are executed alphabetically by filename
8. **Run After Migrations**: Seeds always execute after migrations to ensure schema is ready

## API Reference

### Core Functions

#### Migrations

**For SQLite:**
```rust
pub fn migrate(conn: &mut rusqlite::Connection, migrations: &[Migration]) -> MigrateResult<()>
```
Executes all pending migrations synchronously.

**For Turso:**
```rust
pub async fn migrate(conn: &mut turso::Connection, migrations: &[Migration]) -> MigrateResult<()>
```
Executes all pending migrations asynchronously.

#### Seeds

**For SQLite:**
```rust
pub fn seed(conn: &mut rusqlite::Connection, seeds: &[Seed]) -> MigrateResult<()>
```
Executes all pending seeds synchronously.

**For Turso:**
```rust
pub async fn seed(conn: &mut turso::Connection, seeds: &[Seed]) -> MigrateResult<()>
```
Executes all pending seeds asynchronously.

### Build Script

#### `Builder::new()`

Creates a new builder with default settings.

```rust
// Use defaults (migrations/ and src/seeds/)
ic_sql_migrate::Builder::new().build().unwrap();

// Custom directories
ic_sql_migrate::Builder::new()
    .with_migrations_dir("db/migrations")
    .with_seeds_dir("src/db/seeds")
    .build()
    .unwrap();
```

**Note**: Missing directories are handled automatically - they generate empty arrays.

### Macros

#### `ic_sql_migrate::include_migrations!()`

Includes all migrations discovered by the Builder at compile time.

```rust
static MIGRATIONS: &[ic_sql_migrate::Migration] = ic_sql_migrate::include_migrations!();
```

#### `ic_sql_migrate::seeds!()`

Helper macro to manually create a static array of seeds (for advanced use cases).

```rust
static SEEDS: &[ic_sql_migrate::Seed] = ic_sql_migrate::seeds![
    Seed::new("001_users", my_seed_fn),
];
```

**Note:** In most cases, seeds are auto-discovered from `src/seeds/` and accessed via the generated `mod seeds` module.

### Types

#### `Migration`

```rust
pub struct Migration {
    pub id: &'static str,    // Unique identifier (filename without extension)
    pub sql: &'static str,   // SQL statements to execute
}
```

#### `Seed`

```rust
pub struct Seed {
    pub id: &'static str,          // Unique identifier
    pub seed_fn: SeedFn,           // Function to execute
}
```

#### `Error`

Custom error type that wraps database-specific errors and migration/seed failures.

### Database Schema

The library automatically creates these tracking tables:

**Migrations Table:**
```sql
CREATE TABLE _migrations (
    id TEXT PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
)
```

**Seeds Table:**
```sql
CREATE TABLE _seeds (
    id TEXT PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
)
```

## How It Works

1. **Build Time**: `Builder` in `build.rs` scans your migrations and seeds directories
   - Migrations: SQL files embedded as static strings into your canister binary
   - Seeds: Rust modules discovered and auto-generated into `src/seeds/mod.rs` with a `SEEDS` constant

2. **WASI to IC Conversion**: The `wasi2ic` tool converts WASI-specific function calls to IC-compatible polyfills (SQLite only)

3. **Canister Init/Upgrade**:
   - On `init`: Calls `migrate()` to set up the database schema, then calls `seed()` to populate initial data
   - On `post_upgrade`: Calls `migrate()` and `seed()` to apply any new migrations and seeds

4. **Migration Tracking**: 
   - A `_migrations` table is automatically created to track which migrations have been applied
   - Pending migrations are executed in alphabetical order within a transaction
   - Each successful migration is recorded to prevent duplicate execution

5. **Seed Tracking**: 
   - A `_seeds` table is automatically created to track which seeds have been applied
   - Pending seeds are executed in alphabetical order within transactions
   - Each successful seed is recorded to prevent duplicate execution

6. **Transaction Safety**: All pending migrations and seeds run in transactions. If any operation fails, changes are rolled back, ensuring data consistency.

## Migration Best Practices

1. **Naming Convention**: Use sequential numbering like `001_description.sql`, `002_description.sql` to ensure correct execution order

2. **Forward-Only**: This library only supports forward migrations (no rollbacks). Plan your schema changes carefully.

3. **Idempotent SQL**: While migrations are tracked, write idempotent SQL when possible using `IF NOT EXISTS` clauses

4. **Small Changes**: Keep each migration focused on a single logical change

5. **Test Locally**: Always test migrations using `dfx deploy --local` before mainnet deployment

6. **Document Changes**: Include comments in your migration files explaining what each migration does

## Troubleshooting

### "Both features enabled" error

You can only use one database backend at a time. Ensure exactly one of `sqlite` or `turso` is enabled in your `Cargo.toml`.

### Migrations not found

Ensure your migrations directory exists and contains `.sql` files, and that `build.rs` is properly configured to point to it.

### "wasi2ic: command not found" 

Install the `wasi2ic` tool:
```bash
cargo install wasi2ic
```

### Migration failures

Check the canister logs with `dfx canister logs <canister_name>` for detailed error messages. Common issues:
- Invalid SQL syntax in migration files
- Trying to create tables that already exist (use `IF NOT EXISTS`)
- Foreign key constraint violations

### Seeds not executing

Verify:
- Seed files are in the `src/seeds/` directory (or configured directory)
- Each seed file exports a `pub fn seed()` function
- The module is declared in your canister code: `mod seeds;`

## Examples

Complete working examples are available in the repository:

- [`examples/sqlite`](../../examples/sqlite) - Advanced example with the Chinook database and complex queries
- [`examples/turso`](../../examples/turso) - Turso integration example with basic migrations

### Running the SQLite Example

```bash
cd examples/sqlite
dfx start --clean
dfx deploy
dfx canister call sqlite-example verify_migrations
```

### Running the Turso Example

```bash
cd examples/turso
dfx start --clean
dfx deploy
dfx canister call turso run
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Author

Kristofer Lund
