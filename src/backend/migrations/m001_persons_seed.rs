use ic_rusqlite::Connection;
use migrations::{Migration, Result};

pub struct M;

impl Migration for M {
    fn id(&self) -> &'static str {
        "m001_persons_seed"
    }

    fn up(&self, conn: &mut Connection) -> Result<()> {
        // Only seed if table is empty
        let existing: i64 = conn.query_row("SELECT COUNT(1) FROM person", [], |r| r.get(0))?;
        if existing == 0 {
            let tx = conn.transaction()?;
            {
                let mut stmt = tx.prepare("INSERT INTO person (name, age) VALUES (?1, ?2)")?;
                for (name, age) in [
                    ("Alice", 30),
                    ("Bob", 25),
                    ("Charlie", 35),
                    ("Diana", 28),
                    ("Eve", 40),
                ] {
                    stmt.execute((&name, &age))?;
                }
            }
            tx.commit()?;
        }
        Ok(())
    }

    fn down(&self, conn: &mut Connection) -> Result<()> {
        // Remove only the seeded rows (idempotent)
        conn.execute(
            "DELETE FROM person WHERE name IN ('Alice','Bob','Charlie','Diana','Eve')",
            [],
        )?;
        Ok(())
    }
}
