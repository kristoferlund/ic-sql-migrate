# SQLite Chinook Database Example for ICP Canisters

This example demonstrates running SQLite databases on the Internet Computer using `ic-sql-migrate` with the complete Chinook music store database. It explores how ICP canisters handle complex databases with thousands of records and sophisticated queries.

## Overview

This canister imports the entire Chinook database (a sample database representing a digital media store) and provides five test endpoints that demonstrate both complex read queries and intensive write operations. The example allows you to assess SQLite's capabilities when running on ICP.

## Prerequisites

- [DFX](https://internetcomputer.org/docs/building-apps/getting-started/install) installed
- Rust toolchain installed
- WASI SDK and toolchain configured for `ic-rusqlite`

### Setting up WASI SDK for SQLite

SQLite support requires the WASI SDK to compile `ic-rusqlite`. Follow the setup instructions at [ic-rusqlite](https://crates.io/crates/ic-rusqlite) or run this automated setup script:

```bash
curl -fsSL https://raw.githubusercontent.com/wasm-forge/ic-rusqlite/main/prepare.sh | sh
```

## The Chinook Database

The Chinook database is a sample database that represents a digital media store, including:

### Tables (11 total)
- **Customer** - Store customers with addresses and support representatives
- **Employee** - Store employees including sales support agents
- **Invoice** - Customer purchases
- **InvoiceLine** - Individual items within invoices
- **Track** - Music tracks available for purchase
- **Album** - Albums containing tracks
- **Artist** - Musical artists
- **Genre** - Music genres (Rock, Jazz, etc.)
- **MediaType** - Format of tracks (MP3, AAC, etc.)
- **Playlist** - Curated track collections
- **PlaylistTrack** - Many-to-many relationship for playlists

### Data Volume
- 59 customers across 24 countries
- 275 artists
- 347 albums
- 3,503 tracks
- 412 invoices with 2,240 line items
- Pre-populated playlists demonstrating relationships

## Project Structure

```
sqlite/
├── migrations/
│   └── 000_init.sql    # Complete Chinook database schema and data
├── src/
│   └── lib.rs          # Canister implementation with test endpoints
├── build.rs            # Embeds migrations at compile time
├── Cargo.toml          # Dependencies and build configuration
└── sqlite.did          # Candid interface definition
```

## Test Endpoints

The canister provides five test endpoints (`test1` through `test5`) that demonstrate various database capabilities:

### Read Operations (Query Methods)

#### `test1` - Top Customers Analysis
- Identifies top 15 customers by total purchase amount
- Performs complex JOINs across Customer, Invoice, and Employee tables
- Returns customer details, support rep, purchase history, and spending patterns
- Calculates total spent, average invoice, and purchase date range

#### `test2` - Genre and Artist Analytics
- Analyzes top 10 genres by revenue with track counts and artist diversity
- Identifies top 10 best-selling artists with album and track statistics
- Uses aggregation functions and GROUP BY clauses
- Demonstrates multi-table JOINs and revenue calculations

#### `test3` - Sales Trends Analysis
- Analyzes sales by year and country with customer metrics
- Evaluates employee performance through their customer sales
- Shows temporal data handling and geographic distribution
- Calculates averages, totals, and unique customer counts

### Write Operations (Update Methods)

#### `test4` - Massive Bulk Invoice Generation
- Creates 250 new invoices with 5-25 line items each (thousands of records)
- Implements bulk discount logic and country-specific tax calculations
- Creates customer statistics in temporary tables
- Generates audit log entries for all operations
- Demonstrates transaction handling and bulk INSERT performance

#### `test5` - Complex Playlist Manipulation
- Creates 15+ genre-specific playlists with up to 500 tracks each
- Generates year-based retrospective playlists (2009-2014)
- Simulates collaborative playlists for 30 active customers
- Creates and populates analytics tables:
  - `TrackAnalytics` - Play counts, ratings, popularity scores
  - `PlaylistRecommendations` - Relationships between playlists
  - `PlaylistMetadata` - Duration, track counts, play statistics
- Demonstrates complex CREATE TABLE, bulk INSERTs, and data analysis

## Performance Characteristics

Each endpoint reports instruction counts, allowing you to evaluate SQLite's performance on ICP:

- **Read queries** typically use 10-50 million instructions
- **Write operations** use 100-500 million instructions for thousands of records
- Database handles complex JOINs, subqueries, and aggregations efficiently

## Quick Start

### 1. Start the local Internet Computer replica:
```bash
dfx start --clean --background
```

### 2. Deploy the canister:
```bash
dfx deploy sqlite-example
```

### 3. Verify the database loaded correctly:
```bash
dfx canister call sqlite-example run
```

Expected output:
```
Success: All 1 migrations executed. Chinook database loaded with 59 customers, 3503 tracks, 347 albums, 412 invoices.
```

### 4. Run the test endpoints:

```bash
# Read operations - analyze existing data
dfx canister call sqlite-example test1  # Top customers with purchase history
dfx canister call sqlite-example test2  # Genre and artist revenue analysis
dfx canister call sqlite-example test3  # Sales trends by geography and time

# Write operations - generate new data
dfx canister call sqlite-example test4  # Generate 250+ invoices with line items
dfx canister call sqlite-example test5  # Create playlists and analytics data
```

## Sample Output

### test1 - Top Customers
```
=== Top 15 Customers Analysis (Instructions: 25432187) ===

1. ID: 6 | Helena Holý (Prague, Czech Republic) | Rep: Steve Johnson | Total: $49.62 | Invoices: 7 | Avg: $7.09 | Period: 2009-01-02 to 2013-12-05
2. ID: 26 | Richard Cunningham (Fort Worth, USA) | Rep: Steve Johnson | Total: $47.62 | Invoices: 7 | Avg: $6.80 | Period: 2009-01-08 to 2012-12-28
...
```

### test4 - Bulk Invoice Generation
```
Test 4 completed: Created 250 invoices with 3750 line items ($45678.90 total revenue), 250 audit records. Instructions used: 387654321
```

## Key Implementation Details

### Migration System
The single migration file (`000_init.sql`) contains the entire Chinook database structure and data:
- 11 CREATE TABLE statements with proper foreign keys
- Thousands of INSERT statements for initial data
- Indexes for optimal query performance

### Connection Management
Uses `ic_rusqlite::with_connection()` for safe database access:
```rust
with_connection(|mut conn| {
    let conn: &mut Connection = &mut conn;
    // Perform database operations
});
```

### Performance Monitoring
Each endpoint uses `ic_cdk::api::performance_counter` to track instruction usage:
```rust
let start_instructions = performance_counter(0);
// ... perform operations ...
let end_instructions = performance_counter(0);
let instructions_used = end_instructions - start_instructions;
```

## Extending the Example

To add new test endpoints:

1. Create a new function with `#[query]` (read) or `#[update]` (write) attribute
2. Use `with_connection()` to access the database
3. Track performance with `performance_counter()`
4. Add the endpoint to `sqlite.did`

Example:
```rust
#[query]
fn test6() -> String {
    let start = performance_counter(0);

    let result = with_connection(|mut conn| {
        // Your database operations here
    });

    let instructions = performance_counter(0) - start;
    format!("Result: {:?}, Instructions: {}", result, instructions)
}
```

## Troubleshooting

### "Migration failed" error
- Check SQL syntax in the migration file
- View logs: `dfx canister logs sqlite`
- Ensure the database isn't corrupted

### Performance issues
- Monitor instruction counts in responses
- Consider adding indexes for frequently queried columns
- Batch write operations in transactions

### Memory constraints
- The Chinook database is relatively small (~1MB)
- Monitor canister memory usage with `dfx canister status`
- Consider pagination for very large result sets

## What This Example Shows

This example allows you to explore:
- **SQLite's capabilities on ICP** with complex queries and operations
- **Performance characteristics** when working with thousands of records
- **SQL feature support** including JOINs, subqueries, and transactions
- **Database patterns** familiar from traditional applications

## License

MIT - See LICENSE file in the repository root
