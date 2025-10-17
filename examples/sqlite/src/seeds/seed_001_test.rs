use ic_rusqlite::Connection;
use ic_sql_migrate::MigrateResult;

pub fn seed(conn: &Connection) -> MigrateResult<()> {
    conn.execute(
        "INSERT INTO Album (AlbumId, Title, ArtistId) VALUES (9999, 'Test Album from Seeds', 1)",
        [],
    )?;
    ic_cdk::println!("Seed 001: Inserted test album");
    Ok(())
}
