//! 领域模型模块
//!
//! 包含 SeaORM 实体定义和 DTO 类型。

pub mod prelude;

pub mod book_files;
pub mod categories;
pub mod libraries;

use serde::{Deserialize, Serialize};

// ============== Library DTOs ==============

/// 创建 Library 请求
#[derive(Debug, Deserialize)]
pub struct CreateLibraryRequest {
    pub name: String,
    pub root_path: String,
}

/// 更新 Library 请求
#[derive(Debug, Deserialize)]
pub struct UpdateLibraryRequest {
    pub name: Option<String>,
    pub root_path: Option<String>,
}

/// Library 响应
#[derive(Debug, Serialize)]
pub struct LibraryResponse {
    pub id: i64,
    pub name: String,
    pub root_path: String,
    pub last_scanned_at: i64,
}

// ============== Category DTOs ==============

/// 创建 Category 请求
#[derive(Debug, Deserialize)]
pub struct CreateCategoryRequest {
    pub library_id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub path: String,
    pub mtime: i64,
}

/// 更新 Category 请求
#[derive(Debug, Deserialize)]
pub struct UpdateCategoryRequest {
    pub parent_id: Option<i64>,
    pub name: Option<String>,
    pub path: Option<String>,
    pub mtime: Option<i64>,
}

/// Category 响应
#[derive(Debug, Serialize)]
pub struct CategoryResponse {
    pub id: i64,
    pub library_id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub path: String,
    pub mtime: i64,
}

// ============== BookFile DTOs ==============

/// 创建 BookFile 请求
#[derive(Debug, Deserialize)]
pub struct CreateBookFileRequest {
    pub category_id: i64,
    pub path: String,
    pub kind: String,
    pub size: i64,
    pub mtime: i64,
    pub hash: Option<String>,
    pub page_count: i32,
    pub pages_json: Option<String>,
}

/// 更新 BookFile 请求
#[derive(Debug, Deserialize)]
pub struct UpdateBookFileRequest {
    pub category_id: Option<i64>,
    pub path: Option<String>,
    pub kind: Option<String>,
    pub size: Option<i64>,
    pub mtime: Option<i64>,
    pub hash: Option<String>,
    pub page_count: Option<i32>,
    pub pages_json: Option<String>,
}

/// BookFile 响应
#[derive(Debug, Serialize)]
pub struct BookFileResponse {
    pub id: i64,
    pub category_id: i64,
    pub path: String,
    pub kind: String,
    pub size: i64,
    pub mtime: i64,
    pub hash: Option<String>,
    pub page_count: i32,
    pub pages_json: Option<String>,
}

// ============== Book DTOs ==============

/// 创建 Book 请求
#[derive(Debug, Deserialize)]
pub struct CreateBookRequest {
    pub file_id: i64,
    pub title: String,
    pub cover_path: Option<String>,
}

/// 更新 Book 请求
#[derive(Debug, Deserialize)]
pub struct UpdateBookRequest {
    pub file_id: Option<i64>,
    pub title: Option<String>,
    pub cover_path: Option<String>,
    pub last_page: Option<i32>,
    pub is_finished: Option<bool>,
    pub favorite: Option<bool>,
}

/// Book 响应
#[derive(Debug, Serialize)]
pub struct BookResponse {
    pub id: i64,
    pub file_id: i64,
    pub title: String,
    pub cover_path: Option<String>,
    pub last_page: i32,
    pub is_finished: bool,
    pub favorite: bool,
    pub last_read_at: i64,
}

/// 删除响应
#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub message: String,
}
