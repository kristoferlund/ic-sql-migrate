use ic_cdk::{init, post_upgrade, pre_upgrade, query};
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager},
    DefaultMemoryImpl,
};
use std::cell::RefCell;
use turso::Connection;

static MIGRATIONS: &[ic_sql_migrate::Migration] = ic_sql_migrate::include!();

thread_local! {
    static CONNECTION: RefCell<Option<Connection>> = const { RefCell::new(None) };

    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
}

const MOUNTED_MEMORY_ID: u8 = 20;
const DB_FILE_NAME: &str = "db.db3";

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
    MEMORY_MANAGER.with(|m| {
        let m = m.borrow();
        ic_wasi_polyfill::init_with_memory_manager(&[0u8; 32], &[], &m, 200..210);

        // mount virtual memory as file for faster DB operations
        let memory = m.get(MemoryId::new(MOUNTED_MEMORY_ID));
        ic_wasi_polyfill::mount_memory_file(DB_FILE_NAME, Box::new(memory));
    });
}

async fn run_migrations() {
    let mut conn = get_connection().await;
    ic_sql_migrate::turso::up(&mut conn, MIGRATIONS)
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
