use ic_cdk::{init, post_upgrade, pre_upgrade, query};
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
