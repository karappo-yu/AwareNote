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
use crate::domain::page_favorites;
use crate::domain::page_spreads;
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

/// GET /api/books/:id/pages — 返回图片包的页面文件名列表和尺寸
#[derive(Serialize)]
pub struct BookPagesResponse {
    pub pages: Vec<PageInfo>,
}

#[derive(Serialize)]
pub struct PageInfo {
    pub filename: String,
    pub w: Option<u32>,
    pub h: Option<u32>,
}

pub async fn image_book_pages(
    State(state): State<AppState>,
    Path(book_id): Path<i64>,
) -> Result<Json<BookPagesResponse>, AppError> {
    let book = state
        .db_service
        .get_book(book_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("book {book_id}")))?;

    let meta: Vec<crate::scanner::strategy::PageMeta> = book
        .pages_meta_json
        .as_deref()
        .and_then(|json| serde_json::from_str(json).ok())
        .unwrap_or_default();

    if book.kind == "pdf" {
        // PDF 书籍：用序号作为 filename，尺寸从 pages_meta_json 取
        let page_count = book.page_count as usize;
        let pages: Vec<PageInfo> = (0..page_count)
            .map(|i| {
                let filename = (i + 1).to_string(); // 1-based page number
                let (w, h) = meta.get(i).map(|m| (Some(m.w), Some(m.h))).unwrap_or((None, None));
                PageInfo { filename, w, h }
            })
            .collect();
        return Ok(Json(BookPagesResponse { pages }));
    }

    let paths = page_paths(&book)?;
    let pages: Vec<PageInfo> = paths
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let filename = std::path::Path::new(p)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            let (w, h) = meta.get(i).map(|m| (Some(m.w), Some(m.h))).unwrap_or((None, None));
            PageInfo { filename, w, h }
        })
        .collect();
    Ok(Json(BookPagesResponse { pages }))
}

/// GET /api/books/:id/page/:filename — 按文件名请求图片包页面
pub async fn image_book_page_by_name(
    State(state): State<AppState>,
    Path((book_id, filename)): Path<(i64, String)>,
    Query(query): Query<PageQuery>,
) -> Result<Response<Body>, AppError> {
    let book = state
        .db_service
        .get_book(book_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("book {book_id}")))?;
    if book.kind == "pdf" {
        return Err(AppError::BadRequest("not available for pdf books".to_string()));
    }
    let pages = page_paths(&book)?;
    let page_path = pages
        .iter()
        .find(|p| {
            std::path::Path::new(p)
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n == filename)
        })
        .ok_or_else(|| AppError::NotFound(format!("page {filename}")))?;
    if query.realsize.unwrap_or(false) || !book.is_oversized {
        return file_response(std::path::Path::new(page_path)).await;
    }
    // 找到该文件名在 pages 中的索引，用于缓存 key
    let page_index = pages
        .iter()
        .position(|p| {
            std::path::Path::new(p)
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n == filename)
        })
        .unwrap_or(0);
    let cached = state
        .asset_cache
        .get_or_create_image_page_preview(&book.path, page_index, page_path)
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

// ============== Spread API ==============

#[derive(Serialize)]
pub struct SpreadResponse {
    pub book_id: String,
    pub filename: String,
    pub next_file: String,
    pub direction: String,
    pub created_at: i64,
}

impl From<page_spreads::Model> for SpreadResponse {
    fn from(m: page_spreads::Model) -> Self {
        SpreadResponse {
            book_id: m.book_id,
            filename: m.filename,
            next_file: m.next_file,
            direction: m.direction,
            created_at: m.created_at,
        }
    }
}

#[derive(Serialize)]
pub struct SpreadListResponse {
    pub spreads: Vec<SpreadResponse>,
}

/// GET /api/books/:id/spreads
pub async fn list_spreads(
    State(state): State<AppState>,
    Path(book_id): Path<i64>,
) -> Result<Json<SpreadListResponse>, AppError> {
    let book_id_str = book_id.to_string();
    let spreads = state.db_service.get_spreads(&book_id_str).await?;
    Ok(Json(SpreadListResponse {
        spreads: spreads.into_iter().map(SpreadResponse::from).collect(),
    }))
}

#[derive(Deserialize)]
pub struct CreateSpreadRequest {
    pub filename: String,
    pub next_file: String,
    pub direction: Option<String>,
}

#[derive(Serialize)]
pub struct CreateSpreadResponse {
    pub success: bool,
    pub spread: SpreadResponse,
}

/// POST /api/books/:id/spreads
pub async fn create_spread(
    State(state): State<AppState>,
    Path(book_id): Path<i64>,
    Json(body): Json<CreateSpreadRequest>,
) -> Result<Json<CreateSpreadResponse>, AppError> {
    let book_id_str = book_id.to_string();
    // 校验书籍存在
    let _book = state
        .db_service
        .get_book(book_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("book {book_id}")))?;
    let spread = state
        .db_service
        .create_spread(&book_id_str, &body.filename, &body.next_file, body.direction.as_deref().unwrap_or("ltr"))
        .await?;
    Ok(Json(CreateSpreadResponse {
        success: true,
        spread: SpreadResponse::from(spread),
    }))
}

#[derive(Serialize)]
pub struct DeleteSpreadResponse {
    pub success: bool,
}

/// DELETE /api/books/:id/spreads/:filename
pub async fn delete_spread(
    State(state): State<AppState>,
    Path((book_id, filename)): Path<(i64, String)>,
) -> Result<Json<DeleteSpreadResponse>, AppError> {
    let book_id_str = book_id.to_string();
    let deleted = state.db_service.delete_spread(&book_id_str, &filename).await?;
    if !deleted {
        return Err(AppError::NotFound(format!(
            "spread for book {book_id} filename {filename}"
        )));
    }
    Ok(Json(DeleteSpreadResponse { success: true }))
}

// ============== Page Favorites API ==============

#[derive(Serialize)]
pub struct PageFavoriteResponse {
    pub book_id: String,
    pub filename: String,
    pub created_at: i64,
}

impl From<page_favorites::Model> for PageFavoriteResponse {
    fn from(m: page_favorites::Model) -> Self {
        PageFavoriteResponse {
            book_id: m.book_id,
            filename: m.filename,
            created_at: m.created_at,
        }
    }
}

#[derive(Serialize)]
pub struct PageFavoriteListResponse {
    pub favorites: Vec<PageFavoriteResponse>,
}

/// GET /api/books/:id/page-favorites
pub async fn list_page_favorites(
    State(state): State<AppState>,
    Path(book_id): Path<i64>,
) -> Result<Json<PageFavoriteListResponse>, AppError> {
    let book_id_str = book_id.to_string();
    let favorites = state.db_service.get_page_favorites(&book_id_str).await?;
    Ok(Json(PageFavoriteListResponse {
        favorites: favorites.into_iter().map(PageFavoriteResponse::from).collect(),
    }))
}

#[derive(Deserialize)]
pub struct CreatePageFavoriteRequest {
    pub filename: String,
}

#[derive(Serialize)]
pub struct CreatePageFavoriteResponse {
    pub success: bool,
    pub favorite: PageFavoriteResponse,
}

/// POST /api/books/:id/page-favorites
pub async fn create_page_favorite(
    State(state): State<AppState>,
    Path(book_id): Path<i64>,
    Json(body): Json<CreatePageFavoriteRequest>,
) -> Result<Json<CreatePageFavoriteResponse>, AppError> {
    let book_id_str = book_id.to_string();
    let _book = state
        .db_service
        .get_book(book_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("book {book_id}")))?;
    let fav = state
        .db_service
        .create_page_favorite(&book_id_str, &body.filename)
        .await?;
    Ok(Json(CreatePageFavoriteResponse {
        success: true,
        favorite: PageFavoriteResponse::from(fav),
    }))
}

#[derive(Serialize)]
pub struct DeletePageFavoriteResponse {
    pub success: bool,
}

/// DELETE /api/books/:id/page-favorites/:filename
pub async fn delete_page_favorite(
    State(state): State<AppState>,
    Path((book_id, filename)): Path<(i64, String)>,
) -> Result<Json<DeletePageFavoriteResponse>, AppError> {
    let book_id_str = book_id.to_string();
    let deleted = state
        .db_service
        .delete_page_favorite(&book_id_str, &filename)
        .await?;
    if !deleted {
        return Err(AppError::NotFound(format!(
            "page favorite for book {book_id} filename {filename}"
        )));
    }
    Ok(Json(DeletePageFavoriteResponse { success: true }))
}

#[derive(Serialize)]
pub struct AllPageFavoritesResponse {
    pub pages: Vec<FavoritePageItem>,
}

#[derive(Serialize)]
pub struct FavoritePageItem {
    pub book_id: String,
    pub book_title: String,
    pub book_type: String,
    pub filename: String,
    pub w: Option<u32>,
    pub h: Option<u32>,
    pub next_file: Option<String>, // spread 右页 filename，None 表示非 spread
    pub direction: Option<String>, // spread 的阅读方向 ltr/rtl
}

/// GET /api/page-favorites — 获取所有收藏页面（虚拟书籍）
pub async fn list_all_page_favorites(
    State(state): State<AppState>,
) -> Result<Json<AllPageFavoritesResponse>, AppError> {
    let favorites = state.db_service.get_all_page_favorites().await?;

    // 预加载所有涉及书籍的 spreads，避免逐条查询
    let mut book_spreads: std::collections::HashMap<String, Vec<page_spreads::Model>> = std::collections::HashMap::new();
    for fav in &favorites {
        if !book_spreads.contains_key(&fav.book_id) {
            match state.db_service.get_spreads(&fav.book_id).await {
                Ok(spreads) => {
                    tracing::info!("[page-favorites] Loaded {} spreads for book {}", spreads.len(), fav.book_id);
                    book_spreads.insert(fav.book_id.clone(), spreads);
                }
                Err(e) => {
                    tracing::warn!("[page-favorites] Failed to load spreads for book {}: {}", fav.book_id, e);
                }
            }
        }
    }

    let mut pages = Vec::new();
    for fav in &favorites {
        // 查找书籍信息
        let book_id: i64 = fav.book_id.parse().unwrap_or(0);
        if let Some(book) = state.db_service.get_book(book_id).await? {
            // 从 pages_meta_json 中获取页面尺寸
            let meta: Vec<crate::scanner::strategy::PageMeta> = book
                .pages_meta_json
                .as_deref()
                .and_then(|json| serde_json::from_str(json).ok())
                .unwrap_or_default();

            let (w, h) = if book.kind == "pdf" {
                // PDF: filename 是页码序号
                let page_idx: usize = fav.filename.parse::<usize>().unwrap_or(1).saturating_sub(1);
                meta.get(page_idx).map(|m| (Some(m.w), Some(m.h))).unwrap_or((None, None))
            } else {
                // image_book: filename 是文件名，需要在 pages_json 中找到索引
                let page_paths: Vec<String> = serde_json::from_str(book.pages_json.as_deref().unwrap_or("[]"))
                    .unwrap_or_default();
                let idx = page_paths.iter().position(|p| {
                    std::path::Path::new(p)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n == fav.filename)
                }).unwrap_or(0);
                meta.get(idx).map(|m| (Some(m.w), Some(m.h))).unwrap_or((None, None))
            };

            // 查找该页是否是 spread 的左页
            let spread_info = book_spreads
                .get(&fav.book_id)
                .and_then(|spreads| {
                    let found = spreads.iter()
                        .find(|s| s.filename == fav.filename);
                    tracing::info!("[page-favorites] Looking for spread: book={} filename={} found={} ({} spreads available)", fav.book_id, fav.filename, found.is_some(), spreads.len());
                    found
                });
            let next_file = spread_info.map(|s| s.next_file.clone());
            let direction = spread_info.map(|s| s.direction.clone());

            pages.push(FavoritePageItem {
                book_id: fav.book_id.clone(),
                book_title: book.title.clone().unwrap_or_else(|| file_stem(&book.path)),
                book_type: book.kind.clone(),
                filename: fav.filename.clone(),
                w,
                h,
                next_file,
                direction,
            });
        }
    }
    Ok(Json(AllPageFavoritesResponse { pages }))
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
