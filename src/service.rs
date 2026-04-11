//! 服务模块
//!
//! 包含业务逻辑层。
pub mod assets;
pub mod database;
pub mod memory;
pub mod restart;

pub const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS libraries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    root_path TEXT NOT NULL UNIQUE,
    last_scanned_at INTEGER
);

CREATE TABLE IF NOT EXISTS categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    library_id INTEGER NOT NULL,
    parent_id INTEGER,
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    mtime INTEGER NOT NULL,
    FOREIGN KEY (library_id) REFERENCES libraries(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_id) REFERENCES categories(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS book_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    category_id INTEGER NOT NULL,
    path TEXT NOT NULL UNIQUE,
    title TEXT,
    kind TEXT NOT NULL,
    size INTEGER NOT NULL,
    mtime INTEGER NOT NULL,
    page_count INTEGER NOT NULL,
    pages_json TEXT,
    content_signature TEXT,
    is_oversized INTEGER NOT NULL DEFAULT 0,
    avg_page_pixels INTEGER NOT NULL DEFAULT 0,
    is_favorite INTEGER DEFAULT 0,
    cover_path TEXT,
    created_at TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now', '+8 hours')),
    FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS page_spreads (
    book_id TEXT NOT NULL,
    filename TEXT NOT NULL,
    next_file TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (book_id, filename)
);

CREATE TABLE IF NOT EXISTS user_data (
    book_id TEXT NOT NULL DEFAULT '',
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (book_id, key)
);

CREATE TABLE IF NOT EXISTS page_favorites (
    book_id TEXT NOT NULL,
    filename TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    PRIMARY KEY (book_id, filename)
);
"#;
