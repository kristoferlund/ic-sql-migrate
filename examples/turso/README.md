# Turso Example Canister

This example demonstrates how to use `ic-sql-migrate` with Turso in an Internet Computer canister.

## Overview

This canister provides the same functionality as the SQLite example but uses Turso as the database backend. It demonstrates that `ic-sql-migrate` can work with different database backends while maintaining the same migration API and canister interface.

## Prerequisites

- [DFX](https://internetcomputer.org/docs/current/developer-docs/build/install-upgrade-remove) installed
- Rust toolchain with `wasm32-unknown-unknown` target

## Project Structure

```
turso/
├── migrations/         # SQL migration files (same as SQLite example)
│   ├── 000_initial.sql
│   ├── 001_person_seed.sql
│   ├── 002_add_index.sql
│   ├── 003_alter_table.sql
│   └── 004_more_seeding.sql
├── src/
│   └── lib.rs         # Canister implementation using Turso
├── backend.did        # Candid interface (same as SQLite)
├── turso.did          # Service definition
├── build.rs           # Build script
└── Cargo.toml
```

## How It Works

1. **Build Time**: The `build.rs` script embeds all SQL migration files into the canister code
2. **Runtime**: Uses Turso database with WASI polyfill for file system operations
3. **Migrations**: Run automatically on `init` and `post_upgrade`
4. **Database Storage**: Uses IC stable memory to persist the database file

## Key Implementation Details

### Database Setup
- Uses `ic-wasi-polyfill` to mount stable memory as a virtual file system
- Database file (`db.db3`) is stored in stable memory
- Connection is managed through thread-local storage with `Rc` for shared access

### Migration Execution
```rust
let conn = get_connection().await;
let migration_conn = ic_sql_migrate::from_turso((*conn).clone());
ic_sql_migrate::up(&migration_conn, MIGRATIONS).await.unwrap();
```

### Differences from SQLite Example

| Aspect | SQLite Example | Turso Example |
|--------|---------------|---------------|
| **Database Backend** | `ic-rusqlite` | `turso` |
| **Connection Type** | Synchronous | Asynchronous |
| **Connection Wrapper** | Direct use | `from_turso()` wrapper |
| **Memory Management** | Built into ic-rusqlite | Manual with ic-wasi-polyfill |
| **Query Execution** | Blocking | Async with `.await` |

## Running the Example

1. Start the local replica:
```bash
dfx start --clean
```

2. Deploy the canister:
```bash
dfx deploy
```

3. Verify migrations ran successfully:
```bash
dfx canister call backend run
```

Expected output:
```
Success: All 5 migrations executed. 7 persons in database.
```

## Canister Interface

The canister exposes the same interface as the SQLite example:

### `run() -> Text`
A query method that:
- Verifies all migrations have been executed
- Counts the number of records in the person table
- Returns a status message

## Migration Files

The example uses the exact same migration files as the SQLite example:

1. **000_initial.sql** - Creates the `person` table with id, name, and age columns
2. **001_person_seed.sql** - Seeds 5 initial person records
3. **002_add_index.sql** - Adds an index on the name column
4. **003_alter_table.sql** - Adds an email column to the person table
5. **004_more_seeding.sql** - Adds 2 more person records with email addresses

## Technical Notes

### WASI Polyfill Configuration
The canister uses `ic-wasi-polyfill` to provide file system capabilities:
- Memory IDs 200-210 are reserved for WASI operations
- Memory ID 20 is specifically mounted as the database file

### Async Operations
All Turso operations are async and use `.await`:
- Database initialization
- Migration execution
- Query operations

### State Management
- Database and connection are stored in thread-local RefCells
- Connection is closed in `pre_upgrade` to ensure clean state
- Database is reopened in `post_upgrade` with migrations re-run

## Comparison with SQLite Example

Both canisters:
- Expose the same `run()` query method
- Execute the same migrations
- Produce identical output
- Handle upgrades the same way

The main difference is internal - this example demonstrates that `ic-sql-migrate` successfully abstracts away the database backend differences, allowing you to choose between SQLite and Turso based on your requirements while maintaining the same migration workflow.

## Limitations

- Turso requires more boilerplate for IC canister integration compared to ic-rusqlite
- Some SQL patterns that work in SQLite may need adjustment for Turso (see migration files)
- Async operations add complexity but may provide better performance characteristics

## Testing

To verify the canister works identically to the SQLite example:
```bash
# Deploy both canisters
cd ../sqlite && dfx deploy
cd ../turso && dfx deploy

# Compare outputs - should be identical
dfx canister call sqlite_backend run
dfx canister call turso_backend run
```

Both should return the same success message confirming migrations and data.