use axum::{extract::State, response::Json};
use serde::{Deserialize, Serialize};

use crate::{runtime, service::assets::CacheClearTarget, AppError, AppState, Config};

#[derive(Serialize)]
pub struct ConfigStats {
    pub total_books: usize,
    pub cache_size_mb: f64,
    pub server_status: String,
    pub version: String,
}

#[derive(Serialize)]
pub struct ConfigSettings {
    pub app_name: String,
    pub host: String,
    pub port: u16,
    pub log_level: String,
    pub database_url: String,
    pub root_path: String,
    pub scan_paths: Vec<String>,
    pub image_exts: Vec<String>,
    pub min_image_count: usize,
    pub cover_width: u32,
    pub image_page_preview_width: u32,
    pub oversized_image_avg_pixels: u64,
    pub pdf_svg_width: u32,
    pub max_render_jobs: usize,
    pub http_concurrency_limit: usize,
    pub database_max_connections: u32,
    pub database_min_connections: u32,
    pub file_io_concurrency: usize,
}

#[derive(Serialize)]
pub struct ConfigResponse {
    pub stats: ConfigStats,
    pub settings: ConfigSettings,
}

#[derive(Deserialize)]
pub struct ConfigUpdateRequest {
    pub app_name: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub log_level: Option<String>,
    pub database_url: Option<String>,
    pub scan_paths: Option<Vec<String>>,
    pub image_exts: Option<Vec<String>>,
    pub min_image_count: Option<usize>,
    pub cover_width: Option<u32>,
    pub image_page_preview_width: Option<u32>,
    pub oversized_image_avg_pixels: Option<u64>,
    pub pdf_svg_width: Option<u32>,
    pub max_render_jobs: Option<usize>,
    pub http_concurrency_limit: Option<usize>,
    pub database_max_connections: Option<u32>,
    pub database_min_connections: Option<u32>,
    pub file_io_concurrency: Option<usize>,
}

#[derive(Serialize)]
pub struct RootPathResponse {
    pub root_path: String,
}

#[derive(Serialize)]
pub struct ClearCacheResponse {
    pub success: bool,
    pub space_freed_mb: f64,
    pub target: String,
}

pub async fn get_config(State(state): State<AppState>) -> Result<Json<ConfigResponse>, AppError> {
    let config = Config::load(runtime::config_path());
    let snapshot = state.db_service.get_snapshot().await?;
    let asset_stats = state.asset_cache.stats().await?;
    let cache_size_mb = asset_stats.cover_cache_size_mb
        + asset_stats.image_page_cache_size_mb
        + asset_stats.pdf_svg_cache_size_mb;

    Ok(Json(ConfigResponse {
        stats: ConfigStats {
            total_books: snapshot.book_files.len(),
            cache_size_mb,
            server_status: "healthy".to_string(),
            version: config.version.clone(),
        },
        settings: ConfigSettings {
            app_name: config.app_name.clone(),
            host: config.host.clone(),
            port: config.port,
            log_level: config.log_level.clone(),
            database_url: config.database_url.clone(),
            root_path: config
                .scanner
                .scan_paths
                .first()
                .cloned()
                .unwrap_or_default(),
            scan_paths: config.scanner.scan_paths.clone(),
            image_exts: config.scanner.image_extensions.clone(),
            min_image_count: config.scanner.min_image_count,
            cover_width: config.cache.cover_width,
            image_page_preview_width: config.cache.image_page_preview_width,
            oversized_image_avg_pixels: config.cache.oversized_image_avg_pixels,
            pdf_svg_width: config.cache.pdf_svg_width,
            max_render_jobs: config.cache.max_render_jobs,
            http_concurrency_limit: config.internal.http_concurrency_limit,
            database_max_connections: config.internal.database_max_connections,
            database_min_connections: config.internal.database_min_connections,
            file_io_concurrency: config.internal.file_io_concurrency,
        },
    }))
}

pub async fn update_config(
    State(state): State<AppState>,
    Json(payload): Json<ConfigUpdateRequest>,
) -> Result<Json<ConfigResponse>, AppError> {
    let mut config = Config::load(runtime::config_path());
    let mut scanner = config.scanner.clone();

    if let Some(app_name) = payload.app_name {
        let app_name = app_name.trim();
        if !app_name.is_empty() {
            config.app_name = app_name.to_string();
        }
    }
    if let Some(host) = payload.host {
        let host = host.trim();
        if !host.is_empty() {
            config.host = host.to_string();
        }
    }
    if let Some(port) = payload.port {
        config.port = port.max(1);
    }
    if let Some(log_level) = payload.log_level {
        let log_level = log_level.trim();
        if !log_level.is_empty() {
            config.log_level = log_level.to_string();
        }
    }
    if let Some(database_url) = payload.database_url {
        let database_url = database_url.trim();
        if !database_url.is_empty() {
            config.database_url = database_url.to_string();
        }
    }
    if let Some(scan_paths) = payload.scan_paths {
        scanner.scan_paths = scan_paths
            .into_iter()
            .map(|path| path.trim().to_string())
            .filter(|path| !path.is_empty())
            .collect();
    }
    if let Some(image_exts) = payload.image_exts {
        scanner.image_extensions = image_exts;
    }
    if let Some(min_image_count) = payload.min_image_count {
        scanner.min_image_count = min_image_count.max(1);
    }
    if let Some(cover_width) = payload.cover_width {
        config.cache.cover_width = cover_width.max(64);
    }
    if let Some(image_page_preview_width) = payload.image_page_preview_width {
        config.cache.image_page_preview_width = image_page_preview_width.max(256);
    }
    if let Some(oversized_image_avg_pixels) = payload.oversized_image_avg_pixels {
        config.cache.oversized_image_avg_pixels = oversized_image_avg_pixels.max(1_000_000);
    }
    if let Some(pdf_svg_width) = payload.pdf_svg_width {
        config.cache.pdf_svg_width = pdf_svg_width.max(256);
    }
    if let Some(max_render_jobs) = payload.max_render_jobs {
        config.cache.max_render_jobs = max_render_jobs.max(1);
    }
    if let Some(http_concurrency_limit) = payload.http_concurrency_limit {
        config.internal.http_concurrency_limit = http_concurrency_limit.max(1);
    }
    if let Some(database_max_connections) = payload.database_max_connections {
        config.internal.database_max_connections = database_max_connections.max(1);
    }
    if let Some(database_min_connections) = payload.database_min_connections {
        config.internal.database_min_connections = database_min_connections.max(1);
    }
    if let Some(file_io_concurrency) = payload.file_io_concurrency {
        config.internal.file_io_concurrency = file_io_concurrency.max(1);
    }

    if config.internal.database_min_connections > config.internal.database_max_connections {
        config.internal.database_min_connections = config.internal.database_max_connections;
    }

    scanner.validate_scan_paths();
    config.scanner = scanner.clone();
    config.save_to_file(runtime::config_path())?;
    state.db_service.update_scanner_config(scanner);
    state.db_service.update_cache_config(config.cache.clone());
    state.asset_cache.update_config(config.cache.clone())?;

    get_config(State(state)).await
}

pub async fn get_root_path() -> Json<RootPathResponse> {
    let config = Config::load(runtime::config_path());
    Json(RootPathResponse {
        root_path: config
            .scanner
            .scan_paths
            .first()
            .cloned()
            .unwrap_or_default(),
    })
}

pub async fn clear_cache(
    State(state): State<AppState>,
) -> Result<Json<ClearCacheResponse>, AppError> {
    let freed = state.asset_cache.clear_all_cache().await?;
    Ok(Json(ClearCacheResponse {
        success: true,
        space_freed_mb: freed,
        target: "all".to_string(),
    }))
}

pub async fn clear_cache_target(
    State(state): State<AppState>,
    axum::extract::Path(target): axum::extract::Path<String>,
) -> Result<Json<ClearCacheResponse>, AppError> {
    let target = match target.as_str() {
        "all" => CacheClearTarget::All,
        "covers" => CacheClearTarget::Covers,
        "thumbnails" => CacheClearTarget::ImagePages,
        "svg" => CacheClearTarget::PdfSvg,
        other => {
            return Err(AppError::BadRequest(format!(
                "unsupported cache target: {other}"
            )))
        }
    };
    let freed = state.asset_cache.clear_cache(target).await?;
    let target_name = match target {
        CacheClearTarget::All => "all",
        CacheClearTarget::Covers => "covers",
        CacheClearTarget::ImagePages => "thumbnails",
        CacheClearTarget::PdfSvg => "svg",
    };
    Ok(Json(ClearCacheResponse {
        success: true,
        space_freed_mb: freed,
        target: target_name.to_string(),
    }))
}
