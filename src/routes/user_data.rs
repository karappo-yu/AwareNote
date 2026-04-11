//! 用户数据路由模块
//!
//! 定义全局设置和 per-book 设置的 API 路由。

use crate::AppState;
use axum::{routing::get, Router};

/// 创建用户数据相关路由
///
/// # API 端点
///
/// - `GET /api/settings` - 获取全局设置
/// - `PUT /api/settings` - 更新全局设置
/// - `GET /api/books/:id/settings` - 获取某本书的用户数据
/// - `PUT /api/books/:id/settings` - 更新某本书的用户数据
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/settings",
            get(crate::handlers::user_data::get_settings)
                .put(crate::handlers::user_data::update_settings),
        )
        .route(
            "/api/books/:id/settings",
            get(crate::handlers::user_data::get_book_settings)
                .put(crate::handlers::user_data::update_book_settings),
        )
        .with_state(state)
}
