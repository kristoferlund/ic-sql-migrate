pub use anyhow::Result;

pub trait Migration: Sync {
    fn id(&self) -> &'static str;
    fn up(&self, conn: &mut rusqlite::Connection) -> Result<()>;
    fn down(&self, conn: &mut rusqlite::Connection) -> Result<()>;
}

fn ensure_table(conn: &mut rusqlite::Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS _migrations (id TEXT PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP)",
        [],
    )?;
    Ok(())
}

fn applied(conn: &rusqlite::Connection) -> Result<std::collections::HashSet<String>> {
    let mut s = conn.prepare("SELECT id FROM _migrations")?;
    let rows = s.query_map([], |r| r.get::<_, String>(0))?;
    Ok(rows.filter_map(Result::ok).collect())
}

pub fn run_up(conn: &mut rusqlite::Connection, migs: &[&dyn Migration]) -> Result<()> {
    ensure_table(conn)?;
    let seen = applied(conn)?;
    for m in migs {
        if seen.contains(m.id()) {
            continue;
        }
        m.up(conn)?;
        conn.execute("INSERT INTO _migrations(id) VALUES (?)", [m.id()])?;
    }
    Ok(())
}

#[macro_export]
macro_rules! include_migrations {
    () => {
        include!(concat!(env!("OUT_DIR"), "/migrations_gen.rs"));
    };
}
