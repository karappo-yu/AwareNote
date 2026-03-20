#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! 图书管理系统 API 入口文件
//!
//! 应用程序的启动入口，负责初始化配置、日志系统、路由并启动 HTTP 服务器。

#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

use auxm::{routes, AppState, AssetCacheService, Config, DatabaseService};
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// 应用程序入口函数
///
/// 初始化过程：
/// 1. 加载应用配置（失败时使用默认值）
/// 2. 初始化日志系统（使用配置中的日志级别）
/// 3. 打印启动信息
/// 4. 绑定 TCP 监听器
/// 5. 创建应用状态和路由
/// 6. 启动 HTTP 服务器（支持优雅关闭）
#[tokio::main]
async fn main() {
    // 1. 先加载配置（使用默认级别初始化日志，以防配置加载失败）
    let config = Config::load(auxm::runtime::config_path());

    // 2. 初始化日志系统
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| config.log_level.clone());
    let log_filter = format!("{},sqlx=off", log_level);
    init_tracing(&log_filter);

    tracing::info!(
        "Starting {} v{} on http://{}:{}",
        config.app_name,
        config.version,
        config.host,
        config.port
    );

    // 3. 绑定 TCP 监听器
    let addr = format!("{}:{}", config.host, config.port);
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => listener,
        Err(e) => {
            tracing::error!("Failed to bind to {}: {}", addr, e);
            return;
        }
    };
    match listener.local_addr() {
        Ok(local_addr) => tracing::info!("Listening on {}", local_addr),
        Err(err) => tracing::warn!("failed to inspect listening address: {}", err),
    }

    // 4. 创建数据库服务
    let db_service = match DatabaseService::new(&config).await {
        Ok(service) => std::sync::Arc::new(service),
        Err(e) => {
            tracing::error!("Failed to initialize database: {}", e);
            return;
        }
    };

    let asset_cache =
        match AssetCacheService::new(config.cache.clone(), config.internal.file_io_concurrency) {
            Ok(service) => std::sync::Arc::new(service),
            Err(e) => {
                tracing::error!("Failed to initialize asset cache: {}", e);
                return;
            }
        };

    // 5. 创建应用状态和路由
    let app_state = AppState {
        db_service,
        asset_cache,
    };
    let app = routes::create_router(app_state, &config);

    // 6. 启动服务器并处理错误
    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        tracing::error!("Server error: {}", e);
    }

    tracing::info!("Server shutdown complete");
}
fn init_tracing(log_filter: &str) {
    let env_filter = tracing_subscriber::EnvFilter::new(log_filter);
    let fmt_layer = tracing_subscriber::fmt::layer();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}

/// 优雅关闭信号处理
///
/// 监听操作系统信号（Ctrl+C、SIGTERM），在收到关闭信号后停止接受新连接，
/// 等待现有请求处理完成后再退出。
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = signal::ctrl_c().await {
            tracing::error!("failed to listen for Ctrl+C: {}", err);
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(err) => {
                tracing::error!("failed to listen for SIGTERM: {}", err);
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, shutting down gracefully...");
        }
        _ = terminate => {
            tracing::info!("Received SIGTERM, shutting down gracefully...");
        }
    }
}
