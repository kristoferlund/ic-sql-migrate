# ic-sql-migrate

A lightweight database migration library for Internet Computer (ICP) canisters with support for SQLite and Turso databases.

[![Crates.io](https://img.shields.io/crates/v/ic-sql-migrate.svg)](https://crates.io/crates/ic-sql-migrate)
[![Documentation](https://docs.rs/ic-sql-migrate/badge.svg)](https://docs.rs/ic-sql-migrate)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview

This library provides automatic database schema management and version control for ICP canisters. Migrations are compiled into your canister binary and executed automatically during initialization and upgrades, with full support for tracking applied migrations and data seeding.

## Key Features

- 🚀 **Multi-Database Support**: SQLite (via `ic-rusqlite`) and Turso databases
- 📦 **Compile-Time Embedding**: Migrations embedded into your canister at compile time
- 🌱 **Data Seeding**: Populate initial data using Rust functions with full IDE support
- 🔄 **Automatic Tracking**: Migrations and seeds tracked to prevent duplicate execution
- 🔒 **Transactional**: All operations run in transactions for data safety
- 🏗️ **ICP Native**: Designed specifically for Internet Computer canisters

## Quick Navigation

### Getting Started
- **📖 [Detailed Documentation](./packages/ic-sql-migrate/README.md)** - Complete guide with installation, configuration, and API reference
- **💾 [SQLite Example](./examples/sqlite/README.md)** - Full-featured example with the Chinook database, complex queries, and performance tracking
- **🌍 [Turso Example](./examples/turso/README.md)** - Async example showing Turso integration on ICP

### Documentation Links
- **[API Documentation](https://docs.rs/ic-sql-migrate)** - Rust API reference on docs.rs
- **[Crates.io](https://crates.io/crates/ic-sql-migrate)** - Package information and version history
- **[Changelog](./packages/ic-sql-migrate/CHANGELOG.md)** - Version history and changes

## 30-Second Start

### 1. Add to Cargo.toml
```toml
[dependencies]
ic-sql-migrate = { version = "0.0.4", features = ["sqlite"] }
ic-rusqlite = { version = "0.4.2", features = ["precompiled"], default-features = false }
ic-cdk = "0.18.7"

[build-dependencies]
ic-sql-migrate = "0.0.4"
```

### 2. Create build.rs
```rust
fn main() {
    ic_sql_migrate::Builder::new().build().unwrap();
}
```

### 3. Create migrations/000_initial.sql
```sql
CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    email TEXT
);
```

### 4. Use in your canister
```rust
use ic_cdk::{init, post_upgrade};
use ic_rusqlite::with_connection;

static MIGRATIONS: &[ic_sql_migrate::Migration] = ic_sql_migrate::include_migrations!();

#[init]
fn init() {
    with_connection(|mut conn| {
        ic_sql_migrate::sqlite::migrate(&mut conn, MIGRATIONS).unwrap();
    });
}

#[post_upgrade]
fn post_upgrade() {
    init();
}
```

## How It Works

1. **Build Time**: `Builder` scans your `migrations/` and `src/seeds/` directories, embedding SQL files and generating seed modules
2. **WASI Conversion**: For SQLite, the `wasi2ic` tool converts WASI calls to IC-compatible polyfills
3. **Runtime**: On canister init/upgrade, migrations execute in order, tracked in a `_migrations` table to prevent re-execution
4. **Seeding**: Optional data seeding via Rust functions runs after migrations with the same tracking mechanism

## Database Backend Comparison

| Feature | SQLite | Turso |
|---------|--------|-------|
| **Async** | No | Yes |
| **Complexity** | Full SQL support | Limited SQL subset |
| **Best For** | Complex databases | Simple schemas |

See the [full comparison](./packages/ic-sql-migrate/README.md#differences-between-database-backends) in the package documentation.

## Examples

Two complete working examples demonstrate real-world usage:

### 📁 [SQLite Example](./examples/sqlite/README.md)
Advanced example with the full Chinook music database featuring:
- 11 tables with thousands of records
- Complex queries with multi-table JOINs and analytics
- Bulk write operations for stress testing
- Performance tracking with instruction counts

### 📁 [Turso Example](./examples/turso/README.md)
Async example showing Turso integration:
- Simple person table with migrations
- Async operation patterns
- Stable memory persistence

## Support & Resources

- **[Full Documentation](./packages/ic-sql-migrate/README.md)** - Complete guide with troubleshooting and advanced topics
- **[Issues](https://github.com/kristoferlund/ic-sql-migrate/issues)** - Report bugs or request features
- **[Examples](./examples)** - Working code samples for both backends

## License

MIT - See [LICENSE](./LICENSE) file for details.
