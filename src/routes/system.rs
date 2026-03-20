//! 系统路由模块
//!
//! 定义系统相关的 API 路由，如健康检查和扫描。

use crate::AppState;
use axum::{
    routing::{get, post},
    Router,
};

/// 创建系统相关路由
///
/// # API 端点
///
/// - `GET /health` - 健康检查
/// - `GET /scan` - 增量扫描并同步数据库
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/health", get(crate::handlers::system::health_check))
        .route(
            "/api/debug/memory",
            get(crate::handlers::system::debug_memory),
        )
        .route(
            "/api/debug/memory/release",
            post(crate::handlers::system::release_memory),
        )
        .route("/scan", get(crate::handlers::system::scan))
        .route("/scan/stream", get(crate::handlers::system::scan_stream))
        .with_state(state)
}
