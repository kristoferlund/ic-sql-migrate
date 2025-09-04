-- Create table for performance testing
CREATE TABLE IF NOT EXISTS perf_test (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    data TEXT NOT NULL,
    random_value INTEGER NOT NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Create index for better query performance
CREATE INDEX IF NOT EXISTS idx_perf_test_random_value ON perf_test(random_value);
