//! Scanner 类型定义模块
//!
//! 定义扫描结果的数据结构，与数据库实体解耦。

pub use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum ItemClassification {
    Book(ScannedBookFile),
    Category(ScannedCategory),
    Ignore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub book_files: Vec<ScannedBookFile>,
    pub categories: Vec<ScannedCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedBookFile {
    pub path: String,
    pub title: Option<String>,
    pub kind: String,
    pub size: i64,
    pub mtime: i64,
    pub page_count: i64,
    pub pages_json: Option<String>,
    pub is_oversized: bool,
    pub avg_page_pixels: i64,
    pub cover_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedCategory {
    pub name: String,
    pub path: String,
    pub mtime: i64,
}
