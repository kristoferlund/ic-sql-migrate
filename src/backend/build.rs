fn main() {
    migrations::generate_migrations(Some("migrations")).unwrap();
}
