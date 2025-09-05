# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.3] - 2025-01-02

### Added
- Added `bundled` and `wasm32-wasi-vfs` features to `rusqlite` dependency

## [0.0.2] - 2025-09-04

### Added
- Turso database support via `turso` feature flag
- Async migration execution for Turso connections
- Compile-time feature validation (requires exactly one database feature)
- Comprehensive documentation for ICP canister integration
- Example canister demonstrating Turso usage

### Changed
- Refactored error handling to use generic `Database` error variant
- Database features (`sqlite` and `turso`) are now mutually exclusive
- Improved documentation to focus on ICP canister usage patterns
- Updated examples to show proper canister lifecycle integration

### Fixed
- Error type compatibility between different database backends

## [0.0.1] - 2024-08-31

### Added
- Initial release with SQLite support via `ic-rusqlite`
- Compile-time migration embedding via `include!()` macro
- Build script support with `list()` function
- Automatic migration tracking with `_migrations` table
- Transactional migration execution
- Example ICP canister with SQLite migrations
