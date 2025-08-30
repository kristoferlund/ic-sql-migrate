pub use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashSet;

pub struct Migration {
    pub id: &'static str,
    pub sql: &'static str,
}

impl Migration {
    pub const fn new(id: &'static str, sql: &'static str) -> Self {
        Self { id, sql }
    }
}

fn ensure_table(conn: &mut Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS _migrations (id TEXT PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP)",
        [],
    )?;
    Ok(())
}

fn applied(conn: &Connection) -> Result<HashSet<String>> {
    let mut s = conn.prepare("SELECT id FROM _migrations")?;
    let rows = s.query_map([], |r| r.get::<_, String>(0))?;
    Ok(rows.filter_map(Result::ok).collect())
}

pub fn up(conn: &mut Connection, migrations: &[Migration]) -> Result<()> {
    ensure_table(conn)?;
    let seen = applied(conn)?;

    // Start transaction for all migrations
    let tx = conn.transaction()?;

    for migration in migrations {
        if seen.contains(migration.id) {
            continue;
        }

        // Execute the SQL
        tx.execute_batch(migration.sql)?;

        // Record migration as applied
        tx.execute("INSERT INTO _migrations(id) VALUES (?)", [migration.id])?;
    }

    // Commit all migrations
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
    use std::env;
    use std::fs;
    use std::path::Path;

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dir_name = migrations_dir_name.unwrap_or("migrations");
    let migrations_dir = Path::new(&manifest_dir).join(dir_name);

    println!("cargo:rerun-if-changed={}", migrations_dir.display());

    // If migrations directory doesn't exist, create empty migrations
    if !migrations_dir.exists() {
        let out_dir = env::var("OUT_DIR").unwrap();
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
    let mut generated_code = String::new();
    generated_code.push_str("&[\n");

    for (migration_id, file_path) in migration_files {
        generated_code.push_str(&format!(
            "    migrations::Migration::new(\"{migration_id}\", include_str!(\"{file_path}\")),\n"
        ));
    }

    generated_code.push_str("]\n");

    // Write generated code to OUT_DIR
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("migrations_gen.rs");
    println!("{}", generated_code);
    fs::write(dest_path, generated_code)?;

    Ok(())
}
