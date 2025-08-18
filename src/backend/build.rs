use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let migrations_dir = Path::new(&manifest_dir).join("migrations_sql");

    println!("cargo:rerun-if-changed={}", migrations_dir.display());

    // If migrations directory doesn't exist, create empty migrations
    if !migrations_dir.exists() {
        let out_dir = env::var("OUT_DIR").unwrap();
        let dest_path = Path::new(&out_dir).join("migrations_gen.rs");
        fs::write(dest_path, "&[]").unwrap();
        return;
    }

    // Read migration files from directory
    let mut migration_files = Vec::new();

    if let Ok(entries) = fs::read_dir(&migrations_dir) {
        for entry in entries {
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
        }
    }

    // Sort migration files by name to ensure consistent ordering
    migration_files.sort_by(|a, b| a.0.cmp(&b.0));

    // Generate Rust code
    let mut generated_code = String::new();
    generated_code.push_str("&[\n");

    for (migration_id, file_path) in migration_files {
        generated_code.push_str(&format!(
            "    migrations::SqlMigration::new(\"{}\", include_str!(\"{}\")),\n",
            migration_id, file_path
        ));
    }

    generated_code.push_str("]\n");

    // Write generated code to OUT_DIR
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("migrations_gen.rs");
    fs::write(dest_path, generated_code).unwrap();
}
