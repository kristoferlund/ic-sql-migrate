# ic-sql-migrate Examples

This directory contains examples demonstrating how to use `ic-sql-migrate` with different database backends.

## Available Examples

### 1. SQLite Example (IC Canister)
**Location:** `sqlite/`  
**Type:** Internet Computer Canister  
**Database:** SQLite via `ic-rusqlite`  
**Runtime:** Synchronous  

The SQLite example demonstrates how to use `ic-sql-migrate` within an Internet Computer canister. It uses `ic-rusqlite`, which provides IC-compatible SQLite bindings that work with the IC's execution model.

**Key Features:**
- ✅ Full IC canister support
- ✅ Persistent storage across upgrades
- ✅ Synchronous operations (wrapped as async by the library)
- ✅ Production-ready for IC deployment

**Use this when:** Building Internet Computer canisters that need SQL database capabilities.

### 2. Turso Example (Standalone)
**Location:** `turso/`  
**Type:** Standalone Rust Application  
**Database:** Turso (async Rust-native SQLite)  
**Runtime:** Asynchronous (tokio)  

The Turso example demonstrates how to use `ic-sql-migrate` with Turso, a modern async Rust implementation of SQLite. This example runs as a standalone Rust application.

**Key Features:**
- ✅ Native async/await support
- ✅ Pure Rust implementation
- ✅ High performance
- ❌ Not suitable for IC canisters

**Use this when:** Building standalone Rust applications, web servers, or any non-IC async Rust project.

## Comparison

| Feature | SQLite (IC Canister) | Turso (Standalone) |
|---------|---------------------|-------------------|
| **Platform** | Internet Computer | Any Rust platform |
| **Runtime** | Sync (wrapped as async) | Native async |
| **Database Backend** | rusqlite | turso |
| **IC Compatible** | ✅ Yes | ❌ No |
| **Connection Wrapper** | `from_sqlite()` | `from_turso()` |
| **Build Target** | wasm32-unknown-unknown | Native targets |
| **Use Case** | IC smart contracts | Rust applications |

## Running the Examples

### SQLite Example (IC Canister)

Prerequisites:
- DFX SDK installed
- Rust with wasm32-unknown-unknown target

```bash
cd sqlite

# Start local IC replica
dfx start --clean

# Deploy the canister
dfx deploy

# Run the verification
dfx canister call backend run
```

### Turso Example (Standalone)

Prerequisites:
- Rust toolchain

```bash
cd turso

# Build and run
cargo run

# Or just build
cargo build --release
```

## Migration Files

Both examples use the same set of migration files to demonstrate consistency across different backends:

1. **000_initial.sql** - Creates the initial `person` table
2. **001_person_seed.sql** - Seeds initial data
3. **002_add_index.sql** - Adds an index on the `name` column
4. **003_alter_table.sql** - Adds an `email` column
5. **004_more_seeding.sql** - Adds more sample data

## Key Implementation Differences

### Connection Setup

**SQLite (IC Canister):**
```rust
use ic_rusqlite::with_connection;

with_connection(|mut conn| {
    let conn: &mut Connection = &mut conn;
    ic_sql_migrate::up(conn, MIGRATIONS).unwrap();
});
```

**Turso (Standalone):**
```rust
let db = turso::Builder::new_local("database.db").build().await?;
let conn = db.connect()?;
let migration_conn = ic_sql_migrate::from_turso(conn);
ic_sql_migrate::up(&migration_conn, MIGRATIONS).await?;
```

### Why Two Different Approaches?

1. **IC Runtime Constraints**: The Internet Computer has specific runtime constraints that require synchronous database operations. The `ic-rusqlite` crate provides this compatibility.

2. **Modern Rust Applications**: Turso offers native async support which is ideal for modern Rust applications using tokio or other async runtimes.

3. **Unified Migration API**: Despite the different backends, `ic-sql-migrate` provides a consistent migration API through the `from_sqlite()` and `from_turso()` wrapper functions.

## Choosing the Right Example

- **For IC Development**: Use the SQLite example. It's the only option that works within IC canisters.
- **For Regular Rust Apps**: Use the Turso example for better async performance and modern Rust patterns.
- **For Learning**: Study both examples to understand how `ic-sql-migrate` adapts to different database backends.

## Notes

- The SQLite example requires careful handling of pre_upgrade/post_upgrade hooks to maintain database state
- The Turso example creates a local database file that persists between runs
- Both examples demonstrate the same migration patterns and SQL operations
- Migration files are embedded at compile time in both cases

## Contributing

When adding new examples:
1. Follow the existing structure with migrations in a `migrations/` directory
2. Include a build.rs file that calls `ic_sql_migrate::list()`
3. Document any platform-specific requirements
4. Add appropriate feature flags in Cargo.toml