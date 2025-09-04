# SQLite Migration Example for ICP Canisters

This example demonstrates how to use `ic-sql-migrate` with SQLite in an Internet Computer canister using `ic-rusqlite`.

## Overview

This canister showcases automatic database migration management in an ICP environment, demonstrating how migrations are executed during canister initialization and upgrades.

## Prerequisites

- [DFX](https://internetcomputer.org/docs/current/developer-docs/build/install-upgrade-remove) installed (version 0.15.0 or later)
- Rust toolchain installed
- WASI SDK and toolchain configured for `ic-rusqlite`

### Setting up WASI SDK for SQLite

SQLite support requires the WASI SDK to compile `ic-rusqlite`. Follow the setup instructions at [ic-rusqlite](https://crates.io/crates/ic-rusqlite) or run this automated setup script:

```bash
curl -fsSL https://raw.githubusercontent.com/wasm-forge/ic-rusqlite/main/prepare.sh | sh
```

This will install:
- `wasi2ic` tool
- `wasm32-wasip1` Rust target
- WASI-SDK with WASI-oriented clang
- Set up required environment variables (`WASI_SDK_PATH` and `PATH`)

## Project Structure

```
sqlite/
├── migrations/          # SQL migration files
│   ├── 000_initial.sql     # Creates person table
│   ├── 001_person_seed.sql # Inserts 5 initial records
│   ├── 002_add_index.sql   # Adds index on name column
│   ├── 003_alter_table.sql # Adds email column
│   └── 004_more_seeding.sql # Adds 2 more records with emails
├── src/
│   └── lib.rs          # Canister implementation
├── build.rs            # Embeds migrations at compile time
├── Cargo.toml          # Dependencies and build configuration
└── sqlite.did          # Candid interface definition
```

## How It Works

### 1. Build Time
The `build.rs` script uses `ic_sql_migrate::list()` to discover and embed all SQL files from the `migrations/` directory into the canister binary:

```rust
fn main() {
    ic_sql_migrate::list(Some("migrations")).unwrap();
}
```

### 2. Runtime Migration Management
The canister automatically runs migrations during lifecycle events:

- **`init()`**: Runs all migrations when the canister is first deployed
- **`post_upgrade()`**: Runs any new migrations after canister upgrades
- **`pre_upgrade()`**: Closes the database connection before upgrades

### 3. Migration Tracking
A `_migrations` table is automatically created to track which migrations have been applied, preventing duplicate execution.

## Migration Files

| File | Description | Records Added |
|------|-------------|---------------|
| `000_initial.sql` | Creates `person` table with id, name, and age columns | 0 |
| `001_person_seed.sql` | Inserts John Doe, Jane Smith, Mike Johnson, Sarah Williams, Tom Brown | 5 |
| `002_add_index.sql` | Creates index on name column for faster queries | 0 |
| `003_alter_table.sql` | Adds email column (nullable) to person table | 0 |
| `004_more_seeding.sql` | Inserts Alice Johnson and Bob Smith with emails | 2 |

**Total records after all migrations**: 7 persons

## Quick Start

### 1. Start the local Internet Computer replica:
```bash
dfx start --clean --background
```

### 2. Deploy the canister:
```bash
dfx deploy sqlite
```

### 3. Verify migrations ran successfully:
```bash
dfx canister call sqlite run
```

Expected output:
```
Success: All 5 migrations executed. 7 persons in database.
```

## Canister Interface

### `run() -> Text` (Query Method)
Verifies migration status and returns:
- Number of migrations executed
- Total number of expected migrations
- Count of records in the person table
- Success or error message

Example implementation:
```rust
#[query]
fn run() -> String {
    // Counts migrations in _migrations table
    // Compares with MIGRATIONS constant
    // Returns formatted status message
}
```

## Key Implementation Details

### Connection Management
Uses `ic_rusqlite::with_connection()` for safe database access:
```rust
with_connection(|mut conn| {
    let conn: &mut Connection = &mut conn;
    ic_sql_migrate::sqlite::up(conn, MIGRATIONS).unwrap();
});
```

### Lifecycle Hooks
```rust
#[init]
fn init() {
    run_migrations();  // Initialize database schema
}

#[pre_upgrade]
fn pre_upgrade() {
    close_connection();  // Clean shutdown before upgrade
}

#[post_upgrade]
fn post_upgrade() {
    run_migrations();  // Apply any new migrations
}
```

## Testing Migrations

### Check migration status:
```bash
dfx canister call sqlite run
```

### View canister logs:
```bash
dfx canister logs sqlite
```

### Manually query the database (in canister code):
```rust
with_connection(|mut conn| {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM person", [], |row| row.get(0))
        .unwrap();
    ic_cdk::println!("Person count: {}", count);
});
```

## Upgrading with New Migrations

To add new migrations:

1. Create a new migration file (e.g., `005_add_phone.sql`):
   ```sql
   ALTER TABLE person ADD COLUMN phone TEXT;
   ```

2. Rebuild and upgrade the canister:
   ```bash
   dfx deploy sqlite --upgrade-unchanged
   ```

3. The new migration will automatically run during `post_upgrade()`

## Troubleshooting

### "Migration failed" error
- Check the SQL syntax in your migration files
- View detailed logs: `dfx canister logs sqlite`
- Ensure migrations are numbered sequentially

### "Database locked" error
- The canister may be processing another request
- Try again after a moment
- Check if `pre_upgrade()` properly closes connections

### Migrations not found
- Verify `build.rs` is configured correctly
- Check that migration files have `.sql` extension
- Ensure files are in the `migrations/` directory

## Architecture Benefits

1. **Automatic Migration Management**: No manual migration tracking needed
2. **Upgrade Safety**: Migrations persist across canister upgrades
3. **Transaction Safety**: All migrations in a deployment run atomically
4. **Development Workflow**: Easy to add new migrations during development
5. **Version Control**: Migration files are tracked in git alongside code

## Comparison with Traditional Deployment

| Aspect | Traditional Server | ICP Canister |
|--------|-------------------|--------------|
| Migration Trigger | Manual or deployment script | Automatic on init/upgrade |
| State Persistence | External database | Integrated with canister |
| Rollback | Supported | Not supported (forward-only) |
| Connection Management | Connection pool | Single connection with lifecycle |
| Deployment | Separate DB migration step | Integrated in canister deployment |

## Next Steps

- Explore the [Turso example](../turso) for async database operations
- Read the [main documentation](../../README.md) for API details
- Check [ic-rusqlite documentation](https://docs.rs/ic-rusqlite) for database operations

## License

MIT - See LICENSE file in the repository root