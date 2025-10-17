cargo test --manifest-path packages/ic-sql-migrate/Cargo.toml --lib --features sqlite 2>&1 | tail -10
cargo test --manifest-path packages/ic-sql-migrate/Cargo.toml --lib --features turso 2>&1 | tail -10
