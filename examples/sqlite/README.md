# SQLite Migration Example

This example demonstrates SQLite database migrations in an ICP canister.

## Quick Start

1. **Start DFX**:
   ```bash
   dfx start --background --clean   
   ```

2. **Deploy the canister**:
   ```bash
   dfx deploy sqlite
   ```

3. **Run migration verification**:
   ```bash
   dfx canister call sqlite run '()'
   ```

## What it does

- Automatically runs 5 SQLite migrations on canister startup
- Creates a `person` table with schema and sample data
- Exposes a single `run()` function to verify all migrations executed successfully
- Uses `ic_cdk::println!` for detailed logging during execution

## Migration Files

- `000_initial.sql`: Creates person table with id, name, age
- `001_person_seed.sql`: Inserts initial sample data
- `002_add_index.sql`: Adds index on person names
- `003_alter_table.sql`: Adds email column
- `004_more_seed.sql`: Inserts additional sample data

The `run()` function verifies all migrations completed and returns the migration status and record count.
