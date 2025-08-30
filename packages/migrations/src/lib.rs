use rusqlite::Connection;
use std::{collections::HashSet, env, fs, path::Path};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy)]
pub struct Migration {
    pub id: &'static str,
    pub sql: &'static str,
}

impl Migration {
    pub const fn new(id: &'static str, sql: &'static str) -> Self {
        Self { id, sql }
    }
}

pub fn up(conn: &mut Connection, migrations: &[Migration]) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute(
        "CREATE TABLE IF NOT EXISTS _migrations (
           id TEXT PRIMARY KEY,
           applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
         )",
        [],
    )?;
    let seen = {
        let mut s = tx.prepare("SELECT id FROM _migrations")?;
        let rows = s.query_map([], |r| r.get::<_, String>(0))?;
        let mut set = HashSet::new();
        for row in rows {
            set.insert(row?);
        }
        set
    };

    for m in migrations {
        if seen.contains(m.id) {
            continue;
        }
        tx.execute_batch(m.sql)?;
        tx.execute("INSERT INTO _migrations(id) VALUES (?)", [m.id])?;
    }
    tx.commit()?;
    Ok(())
}

#[macro_export]
macro_rules! include {
    () => {
        include!(concat!(env!("OUT_DIR"), "/migrations_gen.rs"))
    };
}

///
pub fn list(migrations_dir_name: Option<&str>) -> std::io::Result<()> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let dir_name = migrations_dir_name.unwrap_or("migrations");
    let migrations_dir = Path::new(&manifest_dir).join(dir_name);

    println!("cargo:rerun-if-changed={}", migrations_dir.display());

    // If migrations directory doesn't exist, create empty migrations
    if !migrations_dir.exists() {
        let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
        let dest_path = Path::new(&out_dir).join("migrations_gen.rs");
        fs::write(dest_path, "&[]")?;
        return Ok(());
    }

    // Read migration files from directory
    let mut migration_files = Vec::new();

    if let Ok(entries) = fs::read_dir(&migrations_dir) {
        entries.for_each(|entry| {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("sql") {
                    if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                        let absolute_path = path.to_string_lossy().to_string();
                        migration_files.push((file_stem.to_string(), absolute_path));
                        println!("cargo:rerun-if-changed={}", path.display());
                    }
                }
            }
        });
    }

    // Sort migration files by name to ensure consistent ordering
    migration_files.sort_by(|a, b| a.0.cmp(&b.0));

    // Generate Rust code
    let mut generated = String::from("&[\n");
    for (id, path) in migration_files {
        // Raw string literal avoids escaping issues
        generated.push_str(&format!(
            "    ::migrations::Migration::new(\"{id}\", include_str!(r#\"{path}\"#)),\n"
        ));
    }
    generated.push_str("]\n");

    // Write generated code to OUT_DIR
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("migrations_gen.rs");
    fs::write(dest_path, generated)?;

    Ok(())
}
