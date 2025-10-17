use ic_cdk::{init, post_upgrade, pre_upgrade, query, update};
use ic_sql_migrate::{include_migrations, Migration};
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager},
    DefaultMemoryImpl,
};
use std::{cell::RefCell, path::Path};
use turso::Connection;

static MIGRATIONS: &[Migration] = include_migrations!();

thread_local! {
    static CONNECTION: RefCell<Option<Connection>> = const { RefCell::new(None) };

    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
}

const MOUNTED_MEMORY_ID: u8 = 20;
const DB_FILE_NAME: &str = "/DB/main.db";

pub async fn init_db() -> Connection {
    let db = turso::Builder::new_local(DB_FILE_NAME)
        .build()
        .await
        .unwrap();

    let connection = db.connect().unwrap();

    CONNECTION.with_borrow_mut(|c| {
        *c = Some(connection.clone());
    });

    connection
}

pub async fn get_connection() -> Connection {
    if let Some(conn) = CONNECTION.with_borrow(|c| c.clone()) {
        conn
    } else {
        init_db().await
    }
}

fn close_database() {
    CONNECTION.with_borrow_mut(|db| {
        *db = None;
    });
}

fn mount_memory_files() {
    MEMORY_MANAGER.with_borrow(|m| {
        ic_wasi_polyfill::init_with_memory_manager(&[0u8; 32], &[], m, 200..210);

        // unmount old mount, in case it was created
        ic_wasi_polyfill::unmount_memory_file(DB_FILE_NAME);

        // mount virtual memory as file for faster DB operations
        let memory = m.get(MemoryId::new(MOUNTED_MEMORY_ID));
        ic_wasi_polyfill::mount_memory_file(
            DB_FILE_NAME,
            Box::new(memory),
            ic_wasi_polyfill::MountedFileSizePolicy::MemoryPages,
        );

        // remove lock if it exists
        let _ = std::fs::remove_dir_all(format!("{DB_FILE_NAME}.lock"));

        // create folder before opening the database
        let path = Path::new(&DB_FILE_NAME).parent();
        if let Some(path) = path {
            // create containing folder for the database
            let _ = std::fs::create_dir_all(path);
        }
    });
}

async fn run_migrations() {
    let mut conn = get_connection().await;
    ic_sql_migrate::turso::migrate(&mut conn, MIGRATIONS)
        .await
        .unwrap();
}

#[init]
async fn init() {
    mount_memory_files();
    run_migrations().await;
}

#[pre_upgrade]
fn pre_upgrade() {
    close_database();
}

#[post_upgrade]
async fn post_upgrade() {
    mount_memory_files();
    run_migrations().await;
}

#[query]
async fn run() -> String {
    ic_cdk::println!("Starting migration verification...");

    let conn = get_connection().await;
    // Count migrations - using Turso's async API
    let migration_count = match conn.query("SELECT COUNT(*) FROM _migrations", ()).await {
        Ok(mut rows) => {
            if let Ok(Some(row)) = rows.next().await {
                row.get_value(0)
                    .ok()
                    .and_then(|v| v.as_integer().copied())
                    .unwrap_or(0)
            } else {
                0
            }
        }
        Err(_) => 0,
    };

    ic_cdk::println!("Migrations executed: {}", migration_count);

    let total_migrations = MIGRATIONS.len() as i64;
    if migration_count == total_migrations {
        ic_cdk::println!("All {} migrations have run successfully.", total_migrations);

        // Count persons in the database - using Turso's async API
        let person_count = match conn.query("SELECT COUNT(*) FROM person", ()).await {
            Ok(mut rows) => {
                if let Ok(Some(row)) = rows.next().await {
                    row.get_value(0)
                        .ok()
                        .and_then(|v| v.as_integer().copied())
                        .unwrap_or(0)
                } else {
                    0
                }
            }
            Err(_) => 0,
        };

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
async fn perf1() -> String {
    use ic_cdk::api::{performance_counter, time};

    // Record starting instruction count and time
    let start_instructions = performance_counter(0);
    let start_time = time();

    ic_cdk::println!("Starting performance test: inserting 1000 records with ~1KB data each");

    // Generate random seed from current time
    let seed = start_time as u32;

    let mut conn = get_connection().await;

    // Start a transaction for better performance
    let tx = conn.transaction().await.unwrap();

    // Insert 1000 records
    for i in 0..1000 {
        // Generate ~1KB of random data
        let data = generate_random_data(seed + i, 1024);
        let random_value = ((seed + i) * 2654435761) % 1000000; // Simple hash for random value

        tx.execute(
            "INSERT INTO perf_test (data, random_value) VALUES (?1, ?2)",
            (data, random_value as i64),
        )
        .await
        .unwrap();
    }

    // Commit the transaction
    tx.commit().await.unwrap();

    // Count total records in the table
    let mut rows = conn
        .query("SELECT COUNT(*) FROM perf_test", ())
        .await
        .unwrap();

    let count = if let Ok(Some(row)) = rows.next().await {
        row.get_value(0)
            .ok()
            .and_then(|v| v.as_integer().copied())
            .unwrap_or(0)
    } else {
        0
    };

    // Record ending instruction count and time
    let end_instructions = performance_counter(0);
    let instructions_used = end_instructions - start_instructions;

    ic_cdk::println!("Performance test completed");
    ic_cdk::println!("Instructions used: {}", instructions_used);
    ic_cdk::println!("Total records in perf_test table: {}", count);

    format!(
        "Performance test completed: Inserted 1000 records. Instructions used: {instructions_used}. Total records: {count}"
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
