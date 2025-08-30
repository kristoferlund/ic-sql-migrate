fn main() {
    migrations::list(Some("migrations")).unwrap();
}
