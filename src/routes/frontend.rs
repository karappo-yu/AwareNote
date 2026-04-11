//! 前端静态资源路由模块
//!
//! 提供前端 SPA 静态文件分发和 favicon 服务。
//! 前端构建产物放在 `src/frontend/` 目录下（index.html + assets/ 等）。

use axum::routing::get_service;
use axum::Router;
use std::path::PathBuf;
use tower_http::services::{ServeDir, ServeFile};

/// 获取前端静态文件目录
///
/// 优先查找可执行文件同级的 Resources/frontend（macOS .app bundle 场景），
/// 回退到编译时的 CARGO_MANIFEST_DIR/src/frontend（开发场景）。
fn get_frontend_dir() -> PathBuf {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(contents_idx) = exe_path.ancestors().nth(2) {
            let resources_dir = contents_idx.join("Resources");
            if resources_dir.join("frontend").exists() {
                return resources_dir.join("frontend");
            }
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/frontend")
}

pub fn routes() -> Router {
    let frontend_dir = get_frontend_dir();
    let index_path = frontend_dir.join("index.html");
    let favicon_path = frontend_dir.join("favicon.ico");

    // SPA 静态文件服务：
    // - /assets/*, /icons.svg 等静态资源直接返回文件
    // - 其他未匹配路径 fallback 到 index.html（SPA 路由由前端处理）
    // - /favicon.ico 单独路由
    let serve_dir = ServeDir::new(&frontend_dir)
        .fallback(ServeFile::new(&index_path));

    Router::new()
        .route_service("/favicon.ico", get_service(ServeFile::new(favicon_path)))
        .nest_service("/", get_service(serve_dir))
}
