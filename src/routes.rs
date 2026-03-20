//! 路由模块
//!
//! 定义应用程序的所有路由，包括系统管理和 Swagger UI。

pub mod books;
pub mod categories;
pub mod config;
pub mod frontend;
pub mod system;

/// 创建并配置应用程序路由
///
/// 将所有子路由（系统、Swagger UI）和中间件合并成一个完整的 Router。
///
/// # 参数
///
/// * `state` - 应用程序状态，包含配置等信息
///
/// # 返回
///
/// 返回配置好的 Axum Router 实例
pub fn create_router(state: crate::AppState, app_config: &crate::config::Config) -> axum::Router {
    axum::Router::new()
        .merge(frontend::routes())
        .merge(system::routes(state.clone()))
        .merge(books::routes(state.clone()))
        .merge(categories::routes(state.clone()))
        .merge(config::routes(state))
        .layer(crate::middleware::cors())
        .layer(tower::limit::ConcurrencyLimitLayer::new(
            app_config.internal.http_concurrency_limit.max(1),
        ))
        .layer(
            tower_http::trace::TraceLayer::new_for_http()
                .make_span_with(|request: &axum::extract::Request| {
                    tracing::info_span!(
                        "request",
                        method = %request.method(),
                        uri = %request.uri(),
                    )
                })
                .on_response(
                    |response: &axum::response::Response,
                     latency: std::time::Duration,
                     _span: &tracing::Span| {
                        tracing::info!(
                            status = %response.status(),
                            latency = ?latency,
                        );
                    },
                ),
        )
}
