use ic_rusqlite::Connection;
use migrations::{Migration, Result};

pub struct M;

impl Migration for M {
    fn id(&self) -> &'static str {
        "m000_initial"
    }

    fn up(&self, conn: &mut Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS person (
            id   INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            age INTEGER
        )",
            [],
        )?;
        Ok(())
    }

    fn down(&self, _: &mut Connection) -> Result<()> {
        todo!()
    }
}
