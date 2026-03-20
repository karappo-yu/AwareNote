//! 书籍模块
//!
//! 提供书籍相关的 API。

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderValue, Request, Response, StatusCode},
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::process::Command;
use tower::ServiceExt;
use tower_http::services::ServeFile;

use crate::domain::book_files;
use crate::service::assets::PdfPageSvgAsset;
use crate::{AppError, AppState};

#[derive(Deserialize)]
pub struct ListQuery {
    pub page: Option<usize>,
    pub page_size: Option<usize>,
    pub sort: Option<String>,
    pub all: Option<bool>,
}

#[derive(Deserialize)]
pub struct PageQuery {
    pub realsize: Option<bool>,
}

#[derive(Serialize, Clone)]
pub struct BookResponse {
    pub id: String,
    pub path: String,
    pub title: Option<String>,
    pub kind: String,
    #[serde(rename = "type")]
    pub book_type: String,
    pub size: i64,
    pub mtime: i64,
    pub page_count: i64,
    pub is_favorite: bool,
    pub cover_path: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Serialize)]
pub struct BooksResponse {
    pub success: bool,
    pub message: String,
    pub items: Vec<BookResponse>,
    pub data: Vec<BookResponse>,
    pub page: usize,
    pub page_size: usize,
    pub total: usize,
    pub total_pages: usize,
}

#[derive(Serialize)]
pub struct BookDetailResponse {
    pub id: String,
    pub title: String,
    pub path: String,
    #[serde(rename = "type")]
    pub book_type: String,
    pub page_count: i64,
    pub is_favorite: bool,
    pub description: Option<String>,
    pub optimization_strategy: i32,
    pub avg_page_pixels: Option<i64>,
    pub is_oversized: bool,
}

#[derive(Serialize)]
pub struct FavoriteMutationResponse {
    pub success: bool,
}

#[derive(Serialize)]
pub struct LocalActionResponse {
    pub success: bool,
}

pub async fn list_books(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<BooksResponse>, AppError> {
    let sort_desc = query.sort.as_deref() != Some("created_at_asc");
    if query.all.unwrap_or(false) {
        let books = state.db_service.list_all_books(sort_desc).await?;
        let total = books.len();
        return Ok(Json(build_books_response(books, total, 1, total.max(1))));
    }

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(10).clamp(1, 100);
    let (books, total) = state
        .db_service
        .list_books(page, page_size, sort_desc)
        .await?;
    Ok(Json(build_books_response(books, total, page, page_size)))
}

pub async fn list_favorite_books(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<BooksResponse>, AppError> {
    if query.all.unwrap_or(false) {
        let books = state.db_service.list_all_favorite_books().await?;
        let total = books.len();
        return Ok(Json(build_books_response(books, total, 1, total.max(1))));
    }

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(100).clamp(1, 200);
    let (books, total) = state
        .db_service
        .list_favorite_books(page, page_size)
        .await?;
    Ok(Json(build_books_response(books, total, page, page_size)))
}

pub async fn get_book(
    State(state): State<AppState>,
    Path(book_id): Path<i64>,
) -> Result<Json<BookDetailResponse>, AppError> {
    let book = state
        .db_service
        .get_book(book_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("book {book_id}")))?;
    Ok(Json(to_book_detail_response(&book)))
}

pub async fn favorite_book(
    State(state): State<AppState>,
    Path(book_id): Path<i64>,
) -> Result<Json<FavoriteMutationResponse>, AppError> {
    let updated = state.db_service.set_book_favorite(book_id, true).await?;
    if !updated {
        return Err(AppError::NotFound(format!("book {book_id}")));
    }
    Ok(Json(FavoriteMutationResponse { success: true }))
}

pub async fn unfavorite_book(
    State(state): State<AppState>,
    Path(book_id): Path<i64>,
) -> Result<Json<FavoriteMutationResponse>, AppError> {
    let updated = state.db_service.set_book_favorite(book_id, false).await?;
    if !updated {
        return Err(AppError::NotFound(format!("book {book_id}")));
    }
    Ok(Json(FavoriteMutationResponse { success: true }))
}

pub async fn reveal_book_in_finder(
    State(state): State<AppState>,
    Path(book_id): Path<i64>,
) -> Result<Json<LocalActionResponse>, AppError> {
    let book = state
        .db_service
        .get_book(book_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("book {book_id}")))?;

    let status = Command::new("open")
        .arg("-R")
        .arg(&book.path)
        .status()
        .map_err(|err| AppError::InternalServerError(format!("failed to open finder: {err}")))?;

    if !status.success() {
        return Err(AppError::InternalServerError(
            "failed to reveal book in finder".to_string(),
        ));
    }

    Ok(Json(LocalActionResponse { success: true }))
}

pub async fn book_cover(
    State(state): State<AppState>,
    Path(book_id): Path<i64>,
) -> Result<Response<Body>, AppError> {
    let book = state
        .db_service
        .get_book(book_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("book {book_id}")))?;

    if book.kind == "pdf" {
        let cached = state
            .asset_cache
            .get_or_create_pdf_cover(&book.path)
            .await?;
        return file_response(&cached).await;
    }

    let source_cover_path = book
        .cover_path
        .clone()
        .or_else(|| first_image_path(&book))
        .ok_or_else(|| AppError::NotFound("cover not found".to_string()))?;
    let cached = state
        .asset_cache
        .get_or_create_image_cover(&book.path, &source_cover_path)
        .await?;
    file_response(&cached).await
}

pub async fn image_book_page(
    State(state): State<AppState>,
    Path((book_id, page)): Path<(i64, usize)>,
    Query(query): Query<PageQuery>,
) -> Result<Response<Body>, AppError> {
    let book = state
        .db_service
        .get_book(book_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("book {book_id}")))?;
    let pages = page_paths(&book)?;
    let page_path = pages
        .get(page.saturating_sub(1))
        .ok_or_else(|| AppError::NotFound(format!("page {page}")))?;
    if query.realsize.unwrap_or(false) || !book.is_oversized {
        return file_response(std::path::Path::new(page_path)).await;
    }
    let cached = state
        .asset_cache
        .get_or_create_image_page_preview(&book.path, page.saturating_sub(1), page_path)
        .await?;
    file_response(&cached).await
}

pub async fn pdf_book_page_svg(
    State(state): State<AppState>,
    Path((book_id, page)): Path<(i64, usize)>,
) -> Result<Response<Body>, AppError> {
    let book = state
        .db_service
        .get_book(book_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("book {book_id}")))?;
    let asset = state
        .asset_cache
        .get_pdf_page_svg(&book.path, page.saturating_sub(1))
        .await?;
    match asset {
        PdfPageSvgAsset::CachedFile(path) => file_response(&path).await,
        PdfPageSvgAsset::GeneratedBytes(bytes) => binary_response("image/svg+xml", bytes),
    }
}

fn build_books_response(
    books: Vec<book_files::Model>,
    total: usize,
    page: usize,
    page_size: usize,
) -> BooksResponse {
    let items: Vec<BookResponse> = books
        .into_iter()
        .map(|book| to_book_response(&book))
        .collect();
    let total_pages = if total == 0 {
        0
    } else {
        total.div_ceil(page_size)
    };
    BooksResponse {
        success: true,
        message: "查询成功".to_string(),
        data: items.clone(),
        items,
        page,
        page_size,
        total,
        total_pages,
    }
}

fn to_book_response(book: &book_files::Model) -> BookResponse {
    BookResponse {
        id: book.id.to_string(),
        path: book.path.clone(),
        title: book.title.clone(),
        kind: book.kind.clone(),
        book_type: frontend_book_type(book),
        size: book.size,
        mtime: book.mtime,
        page_count: book.page_count,
        is_favorite: book.is_favorite,
        cover_path: book.cover_path.clone(),
        created_at: book.created_at.clone(),
    }
}

fn to_book_detail_response(book: &book_files::Model) -> BookDetailResponse {
    BookDetailResponse {
        id: book.id.to_string(),
        title: book.title.clone().unwrap_or_else(|| file_stem(&book.path)),
        path: book.path.clone(),
        book_type: frontend_book_type(book),
        page_count: book.page_count,
        is_favorite: book.is_favorite,
        description: None,
        optimization_strategy: if book.kind == "pdf" || book.is_oversized {
            2
        } else {
            1
        },
        avg_page_pixels: (book.avg_page_pixels > 0).then_some(book.avg_page_pixels),
        is_oversized: book.is_oversized,
    }
}

fn frontend_book_type(book: &book_files::Model) -> String {
    if book.kind == "pdf" {
        "pdf_book".to_string()
    } else {
        "image_book".to_string()
    }
}

fn page_paths(book: &book_files::Model) -> Result<Vec<String>, AppError> {
    serde_json::from_str::<Vec<String>>(book.pages_json.as_deref().unwrap_or("[]"))
        .map_err(|err| AppError::InternalServerError(format!("invalid page json: {err}")))
}

fn first_image_path(book: &book_files::Model) -> Option<String> {
    page_paths(book)
        .ok()
        .and_then(|pages| pages.into_iter().next())
}

fn file_stem(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
}

async fn file_response(path: &std::path::Path) -> Result<Response<Body>, AppError> {
    let request = Request::builder().uri("/").body(Body::empty())?;
    let response = ServeFile::new(path)
        .oneshot(request)
        .await
        .map_err(|err| AppError::InternalServerError(format!("failed to serve file: {err}")))?;
    let (parts, body) = response.into_parts();
    Ok(Response::from_parts(parts, Body::new(body)))
}

fn binary_response(content_type: &str, bytes: Vec<u8>) -> Result<Response<Body>, AppError> {
    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(content_type)
            .map_err(|err| AppError::InternalServerError(format!("invalid content type: {err}")))?,
    );
    Ok(response)
}
