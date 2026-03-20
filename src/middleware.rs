//! 中间件模块
//!
//! 提供应用程序的 CORS 配置。

use tower_http::cors::{Any, CorsLayer};

/// CORS 中间件配置
pub fn cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
}
