//! 前端路由模块
//!
//! 以单文件工具页模式提供前端页面和静态资源。

use axum::response::Html;
use axum::routing::{get, get_service};
use axum::Router;
use std::path::PathBuf;
use tower_http::services::{ServeDir, ServeFile};

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
    let favicon_path = frontend_dir.join("favicon.ico");

    Router::new()
        .route("/", get(index))
        .route("/config", get(config))
        .route("/img_book_detail", get(img_book_detail))
        .route("/img_book_detail/:id", get(img_book_detail))
        .route("/pdf_book_detail", get(pdf_book_detail))
        .route("/pdf_book_detail/:id", get(pdf_book_detail))
        .route("/img_swiper", get(img_swiper))
        .route("/img_swiper/:id", get(img_swiper))
        .route("/pdf_swiper", get(pdf_swiper))
        .route("/pdf_swiper/:id", get(pdf_swiper))
        .route_service("/favicon.ico", get_service(ServeFile::new(favicon_path)))
        .nest_service("/static", ServeDir::new(frontend_dir))
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../frontend/index.html"))
}

async fn config() -> Html<&'static str> {
    Html(include_str!("../frontend/config.html"))
}

async fn img_book_detail() -> Html<&'static str> {
    Html(include_str!("../frontend/img_book_detail.html"))
}

async fn pdf_book_detail() -> Html<&'static str> {
    Html(include_str!("../frontend/pdf_book_detail.html"))
}

async fn img_swiper() -> Html<&'static str> {
    Html(include_str!("../frontend/img_swiper.html"))
}

async fn pdf_swiper() -> Html<&'static str> {
    Html(include_str!("../frontend/pdf_swiper.html"))
}
