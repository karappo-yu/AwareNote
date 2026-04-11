//! 书籍路由模块
//!
//! 定义书籍相关的 API 路由。

use crate::AppState;
use axum::{
    routing::{delete, get, post},
    Router,
};

/// 创建书籍相关路由
///
/// # API 端点
///
/// - `GET /api/books` - 获取书籍列表（支持分页和排序）
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/books", get(crate::handlers::books::list_books))
        .route(
            "/api/books/favorite/list",
            get(crate::handlers::books::list_favorite_books),
        )
        .route("/api/books/:id", get(crate::handlers::books::get_book))
        .route(
            "/api/books/:id/favorite",
            post(crate::handlers::books::favorite_book)
                .delete(crate::handlers::books::unfavorite_book),
        )
        .route(
            "/api/books/:id/reveal",
            post(crate::handlers::books::reveal_book_in_finder),
        )
        .route(
            "/api/books/covers/:id",
            get(crate::handlers::books::book_cover),
        )
        .route(
            "/api/books/:id/pages",
            get(crate::handlers::books::image_book_pages),
        )
        .route(
            "/api/books/:id/spreads",
            get(crate::handlers::books::list_spreads)
                .post(crate::handlers::books::create_spread),
        )
        .route(
            "/api/books/:id/spreads/:filename",
            delete(crate::handlers::books::delete_spread),
        )
        .route(
            "/api/books/:id/page-favorites",
            get(crate::handlers::books::list_page_favorites)
                .post(crate::handlers::books::create_page_favorite),
        )
        .route(
            "/api/books/:id/page-favorites/:filename",
            delete(crate::handlers::books::delete_page_favorite),
        )
        .route(
            "/api/page-favorites",
            get(crate::handlers::books::list_all_page_favorites),
        )
        .route(
            "/api/books/:id/page/:filename",
            get(crate::handlers::books::image_book_page_by_name),
        )
        .route(
            "/api/books/:id/:page",
            get(crate::handlers::books::image_book_page),
        )
        .route(
            "/api/books/svg/:id/:page",
            get(crate::handlers::books::pdf_book_page_svg),
        )
        .with_state(state)
}
