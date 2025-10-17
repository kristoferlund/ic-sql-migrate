use ic_sql_migrate::MigrateResult;
use ic_rusqlite::Connection;

pub fn seed(conn: &Connection) -> MigrateResult<()> {
    conn.execute(
        "INSERT INTO Album (AlbumId, Title, ArtistId) VALUES (9998, 'Another Test Album', 1)",
        [],
    )?;
    ic_cdk::println!("Seed 002: Inserted another test album");
    Ok(())
}
