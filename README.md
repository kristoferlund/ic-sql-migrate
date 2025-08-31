# ICP SQLite Migration Library

A lightweight SQLite migration library for Internet Computer (ICP) canisters, providing automatic database schema management and version control.

Crates.io: [ic-sql-migrate](https://crates.io/crates/ic-sql-migrate)

## Features

- **Automatic Migration Execution**: Runs migrations automatically on canister startup and upgrade
- **Version Tracking**: Maintains a `_migrations` table to track executed migrations
- **Simple API**: Minimal setup with `load()`, `include!()` and `up()` functions
- **SQLite Integration**: Built on `ic_rusqlite` for seamless SQLite database operations

## Quick Start

### 1. Add migrations crate to Cargo.toml

The dependency needs to be added to `[build-dependencies]` as well, since it is used from the `build.rs` build script.

```toml
[dependencies]
ic-sql-migrate = "0.0.1" 

[build-dependencies]
ic-sql-migrate = "0.0.1" 
```

### 2. Create migration files

Create SQL files in a `migrations/` directory:

```sql
-- migrations/000_initial.sql
CREATE TABLE IF NOT EXISTS person (
    id   INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 50),
    age INTEGER
);
```

### 3. Configure build.rs

List all migration files in a specified folder (defaults to `migrations`) to make them available to later be included and run. 

```rust
// build.rs
fn main() {
    ic_sql_migrate::list(Some("migrations")).unwrap();
}
```

### 4. Run migrations on init and upgrade 

```rust
use ic_cdk::{init, post_upgrade, pre_upgrade};
use ic_rusqlite::{close_connection, with_connection, Connection};

static MIGRATIONS: &[ic_sql_migrate::Migration] = ic_sql_migrate::include!();

fn run_migrations() {
    with_connection(|mut conn| {
        let conn: &mut Connection = &mut conn;
        ic_sql_migrate::up(conn, MIGRATIONS).unwrap();
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


## Example

See the [SQLite Example](./examples/sqlite/README.md) for a complete implementation demonstrating:

- Migration file structure and naming conventions
- Automatic migration execution on canister lifecycle events
- Verification of migration status
- Integration with ICP canister development workflow

## API Reference

### `ic_sql_migrate::list(migrations_dir_name: Option<&str>) -> std::io::Result<()>`

To make all SQL migration files automatically available to the `include!()` macro, this function should be called in the `build.rs` of the integrating canister.  

### `ic_sql_migrate::include!()`

Macro to include all the migration files that the `list()` function listed. 

### `ic_sql_migrate::up(conn: &mut Connection, ic_sql_migrate: &[Migration]) -> Result<()>`

Executes all pending migrations in order.

## Migration File Format

Migration files should be named with sequential numbers and descriptive names:

```
migrations/
  000_initial.sql
  001_add_email_column.sql
  002_seed_data.sql
```

Files are executed in numerical order and tracked in the `_migrations` table.

## License

This project is licensed under the MIT License. See the LICENSE file for more details.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request if you have any suggestions or improvements.
