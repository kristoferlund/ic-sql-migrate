use std::{
    env, fs,
    path::{Path, PathBuf},
};

pub struct Options {
    pub dir: Option<PathBuf>, // defaults to "<CARGO_MANIFEST_DIR>/migrations"
}

pub fn generate(opts: Option<Options>) -> std::io::Result<()> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dir = opts
        .and_then(|o| o.dir)
        .unwrap_or_else(|| Path::new(&manifest_dir).join("migrations"));

    println!("cargo:rerun-if-changed={}", dir.display());

    let mut files: Vec<_> = fs::read_dir(&dir)?
        .filter_map(|e| {
            let p = e.ok()?.path();
            let name = p.file_name()?.to_string_lossy().into_owned();
            if p.is_file() && name.ends_with(".rs") && name.starts_with('m') {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    files.sort(); // lexicographic: m000_..., m001_...

    let mut out = String::new();

    for f in &files {
        println!("cargo:rerun-if-changed={}", dir.join(f).display());
        let stem = Path::new(f).file_stem().unwrap().to_string_lossy();
        out.push_str(&format!(
            "#[path = \"{}/{}\"] mod {};\n",
            dir.display(),
            f,
            stem
        ));
    }

    out.push_str("\nuse migrations::Migration;\n");

    // Static array of trait objects
    out.push_str("static MIGRATIONS: [&'static dyn Migration; ");
    out.push_str(&files.len().to_string());
    out.push_str("] = [\n");
    for f in &files {
        let stem = Path::new(f).file_stem().unwrap().to_string_lossy();
        out.push_str(&format!("    &{stem}::M as &dyn Migration,\n"));
    }
    out.push_str("];\n\n");

    // Export a slice view
    out.push_str("pub fn list_migrations() -> &'static [&'static dyn Migration] {\n");
    out.push_str("    &MIGRATIONS\n");
    out.push_str("}\n");

    let out_dir = env::var("OUT_DIR").unwrap();
    fs::write(Path::new(&out_dir).join("migrations_gen.rs"), out)
}
