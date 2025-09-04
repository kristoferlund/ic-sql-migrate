# Turso Migration Example for ICP Canisters

This example demonstrates how to use `ic-sql-migrate` with Turso in an Internet Computer canister.

## Overview

This canister provides the same functionality as the SQLite example but uses Turso as the database backend. It demonstrates that `ic-sql-migrate` can work with different database backends while maintaining the same migration workflow and canister interface.

## Prerequisites

- [DFX](https://internetcomputer.org/docs/current/developer-docs/build/install-upgrade-remove) installed (version 0.15.0 or later)
- Rust toolchain with `wasm32-wasi` target:
  ```bash
  rustup target add wasm32-wasi
  ```

## Project Structure

```
turso/
├── migrations/          # SQL migration files (identical to SQLite example)
│   ├── 000_initial.sql     # Creates person table
│   ├── 001_person_seed.sql # Inserts 5 initial records
│   ├── 002_add_index.sql   # Adds index on name column
│   ├── 003_alter_table.sql # Adds email column
│   └── 004_more_seeding.sql # Adds 2 more records with emails
├── src/
│   └── lib.rs          # Canister implementation using Turso
├── build.rs            # Embeds migrations at compile time
├── Cargo.toml          # Dependencies and build configuration
└── turso.did           # Candid interface definition
```

## How It Works

### 1. Build Time
The `build.rs` script uses `ic_sql_migrate::list()` to discover and embed all SQL files from the `migrations/` directory into the canister binary:

```rust
fn main() {
    ic_sql_migrate::list(Some("migrations")).unwrap();
}
```

### 2. Memory Management
Uses IC stable structures and WASI polyfill to provide file system capabilities:
- Mounts stable memory as a virtual file system
- Database file (`db.db3`) persists across upgrades
- Memory IDs 200-210 reserved for WASI operations

### 3. Runtime Migration Management
The canister automatically runs migrations during lifecycle events:

- **`init()`**: Initializes memory, creates database, runs all migrations
- **`post_upgrade()`**: Re-mounts memory, reconnects to database, runs new migrations
- **`pre_upgrade()`**: Closes database connection for clean upgrade

### 4. Migration Tracking
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
dfx deploy turso
```

### 3. Verify migrations ran successfully:
```bash
dfx canister call turso run
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

## Key Implementation Details

### Database Initialization
```rust
pub async fn init_db() -> Connection {
    let db = turso::Builder::new_local(DB_FILE_NAME)
        .build()
        .await
        .unwrap();
    let connection = db.connect().unwrap();
    
    // Store in thread-local for reuse
    CONNECTION.with_borrow_mut(|c| {
        *c = Some(connection.clone());
    });
    
    connection
}
```

### Connection Management
Uses thread-local storage with `RefCell<Option<Connection>>`:
```rust
thread_local! {
    static CONNECTION: RefCell<Option<Connection>> = const { RefCell::new(None) };
}

pub async fn get_connection() -> Connection {
    if let Some(conn) = CONNECTION.with_borrow(|c| c.clone()) {
        conn
    } else {
        init_db().await
    }
}
```

### Migration Execution
All Turso operations are async:
```rust
async fn run_migrations() {
    let mut conn = get_connection().await;
    ic_sql_migrate::turso::up(&mut conn, MIGRATIONS)
        .await
        .unwrap();
}
```

### Lifecycle Hooks
```rust
#[init]
async fn init() {
    mount_memory_files();  // Set up virtual file system
    run_migrations().await; // Initialize database schema
}

#[pre_upgrade]
fn pre_upgrade() {
    close_database();  // Clean shutdown before upgrade
}

#[post_upgrade]
async fn post_upgrade() {
    mount_memory_files();   // Restore virtual file system
    run_migrations().await;  // Apply any new migrations
}
```

## Differences from SQLite Example

| Aspect | SQLite Example | Turso Example |
|--------|---------------|---------------|
| **Database Backend** | `ic-rusqlite` | `turso` |
| **Function Signatures** | Synchronous | Async with `.await` |
| **Connection Storage** | Managed by ic-rusqlite | Thread-local with RefCell |
| **Memory Management** | Built into ic-rusqlite | Manual with ic-wasi-polyfill |
| **File System** | Handled by ic-rusqlite | Virtual FS via stable memory |
| **Database File** | Managed internally | Explicitly named `db.db3` |

## Testing Migrations

### Check migration status:
```bash
dfx canister call turso run
```

### View canister logs:
```bash
dfx canister logs turso
```

### Query operations in canister code:
```rust
let conn = get_connection().await;
let mut rows = conn.query("SELECT COUNT(*) FROM person", ()).await?;
if let Some(row) = rows.next().await? {
    let count = row.get_value(0)?
        .as_integer()
        .copied()
        .unwrap_or(0);
    ic_cdk::println!("Person count: {}", count);
}
```

## Upgrading with New Migrations

To add new migrations:

1. Create a new migration file (e.g., `005_add_phone.sql`):
   ```sql
   ALTER TABLE person ADD COLUMN phone TEXT;
   ```

2. Rebuild and upgrade the canister:
   ```bash
   dfx deploy turso --upgrade-unchanged
   ```

3. The new migration will automatically run during `post_upgrade()`

## Troubleshooting

### "Migration failed" error
- Check the SQL syntax in your migration files
- View detailed logs: `dfx canister logs turso`
- Ensure migrations are numbered sequentially

### Connection issues
- Verify the database file is properly mounted
- Check that memory management is initialized
- Ensure async functions properly await operations

### Memory-related issues
- Verify WASI polyfill is correctly initialized
- Check that memory IDs don't conflict
- Ensure stable memory is properly mounted

## Technical Notes

### WASI Polyfill Configuration
The canister uses `ic-wasi-polyfill` to provide file system capabilities required by Turso:
```rust
fn mount_memory_files() {
    MEMORY_MANAGER.with(|m| {
        let m = m.borrow();
        ic_wasi_polyfill::init_with_memory_manager(&[0u8; 32], &[], &m, 200..210);
        
        // Mount stable memory as database file
        let memory = m.get(MemoryId::new(MOUNTED_MEMORY_ID));
        ic_wasi_polyfill::mount_memory_file(DB_FILE_NAME, Box::new(memory));
    });
}
```

### Async Considerations
- All database operations must use `.await`
- Lifecycle hooks that perform database operations must be async
- Error handling should account for async operation failures

### State Persistence
- Database persists in stable memory across upgrades
- Connection state is recreated after upgrades
- Migration tracking survives canister upgrades

## Comparison Testing

To verify both examples work identically:

```bash
# Deploy both canisters
dfx deploy sqlite
dfx deploy turso

# Compare outputs - should be identical
dfx canister call sqlite run
dfx canister call turso run
```

Both should return:
```
Success: All 5 migrations executed. 7 persons in database.
```

## Architecture Benefits

1. **Async Operations**: Non-blocking database operations
2. **Explicit Memory Control**: Fine-grained control over storage
3. **Same Migration Workflow**: Despite different backends, migration process is identical
4. **Stable Memory Integration**: Database persists naturally with canister state
5. **WASI Compatibility**: Enables more complex file-based libraries

## Next Steps

- Compare with the [SQLite example](../sqlite) for synchronous operations
- Read the [main documentation](../../README.md) for API details
- Explore [Turso documentation](https://docs.turso.tech) for advanced features
- Learn about [IC stable structures](https://docs.rs/ic-stable-structures) for memory management

## License

MIT - See LICENSE file in the repository root