use ic_cdk::{
    api::{performance_counter, time},
    init, post_upgrade, pre_upgrade, query, update,
};
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

#[query]
fn run() -> String {
    ic_cdk::println!("Starting migration verification...");

    let migration_count = with_connection(|mut conn| {
        let conn: &mut Connection = &mut conn;
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
            .unwrap_or(0);
        count
    });

    ic_cdk::println!("Migrations executed: {}", migration_count);

    let total_migrations = MIGRATIONS.len() as i64;
    if migration_count == total_migrations {
        ic_cdk::println!("All {} migrations have run successfully.", total_migrations);

        let person_count = with_connection(|mut conn| {
            let conn: &mut Connection = &mut conn;
            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM person", [], |row| row.get(0))
                .unwrap_or(0);
            count
        });

        ic_cdk::println!("Found {} records in person table.", person_count);
        format!(
            "Success: All {total_migrations} migrations executed. {person_count} persons in database."
        )
    } else {
        ic_cdk::println!(
            "Migration verification failed: {} out of {} migrations executed.",
            migration_count,
            total_migrations
        );
        format!("Error: Only {migration_count} out of {total_migrations} migrations executed.")
    }
}

#[update]
fn perf1() -> String {
    // Record starting instruction count and time
    let start_instructions = performance_counter(0);
    let start_time = time();

    ic_cdk::println!("Starting performance test: inserting 1000 records with ~1KB data each");

    // Generate random seed from current time
    let seed = start_time as u32;

    let result = with_connection(|mut conn| {
        let conn: &mut Connection = &mut conn;

        // Start a transaction for better performance
        let tx = conn.transaction().unwrap();

        // Prepare the insert statement
        let mut stmt = tx
            .prepare("INSERT INTO perf_test (data, random_value) VALUES (?1, ?2)")
            .unwrap();

        // Insert 1000 records
        for i in 0..1000 {
            // Generate ~1KB of random data
            let data = generate_random_data(seed + i, 1024);
            let random_value = ((seed + i) * 2654435761) % 1000000; // Simple hash for random value

            stmt.execute([
                &data as &dyn ic_rusqlite::ToSql,
                &random_value as &dyn ic_rusqlite::ToSql,
            ])
            .unwrap();
        }

        drop(stmt);
        tx.commit().unwrap();

        // Count total records in the table
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM perf_test", [], |row| row.get(0))
            .unwrap();

        count
    });

    // Record ending instruction count and time
    let end_instructions = performance_counter(0);
    let instructions_used = end_instructions - start_instructions;

    ic_cdk::println!("Performance test completed");
    ic_cdk::println!("Instructions used: {}", instructions_used);
    ic_cdk::println!("Total records in perf_test table: {}", result);

    format!(
        "Performance test completed: Inserted 1000 records. Instructions used: {instructions_used}. Total records: {result}"
    )
}

/// Generate random-looking data of specified size
fn generate_random_data(seed: u32, size: usize) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut result = String::with_capacity(size);
    let mut current = seed;

    for _ in 0..size {
        // Simple linear congruential generator
        current = ((current as u64 * 1664525 + 1013904223) % (1 << 32)) as u32;
        let char_index = (current % CHARS.len() as u32) as usize;
        result.push(CHARS[char_index] as char);
    }

    result
}
