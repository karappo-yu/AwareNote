//! 系统模块
//!
//! 提供系统相关的端点，包括健康检查和扫描功能。

use crate::service::database::{BookChangeDetail, CategoryChangeDetail, LibraryChangeDetail};
use crate::{AppError, AppState};
use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive, Sse},
        Json,
    },
};
use serde::Serialize;
use std::convert::Infallible;
use sysinfo::{Pid, ProcessesToUpdate, System};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
    pub storage_mode: String,
    pub libraries: usize,
    pub categories: usize,
    pub book_files: usize,
}

#[derive(Serialize)]
pub struct MemoryDebugResponse {
    pub pid: u32,
    pub memory_rss_mb: f64,
    pub memory_virtual_mb: f64,
    pub memory_used_percent: Option<f64>,
    pub total_system_memory_mb: f64,
    pub process_threads: Option<usize>,
    pub storage_mode: String,
    pub cover_cache_files: usize,
    pub cover_cache_size_mb: f64,
    pub image_page_cache_files: usize,
    pub image_page_cache_size_mb: f64,
    pub pdf_svg_cache_files: usize,
    pub pdf_svg_cache_size_mb: f64,
    pub pdf_open_documents: usize,
    pub pdf_document_idle_ttl_secs: u64,
    pub cache_root_dir: String,
}

#[derive(Serialize)]
pub struct MemoryReleaseResponse {
    pub success: bool,
    pub released_bytes: u64,
    pub released_mb: f64,
}

pub async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    let snapshot = state.db_service.get_snapshot().await.ok();
    Json(HealthResponse {
        status: "ok".to_string(),
        service: "auxm".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        storage_mode: state.db_service.storage_mode().to_string(),
        libraries: snapshot.as_ref().map(|s| s.libraries.len()).unwrap_or(0),
        categories: snapshot.as_ref().map(|s| s.categories.len()).unwrap_or(0),
        book_files: snapshot.as_ref().map(|s| s.book_files.len()).unwrap_or(0),
    })
}

pub async fn debug_memory(
    State(state): State<AppState>,
) -> Result<Json<MemoryDebugResponse>, AppError> {
    let asset_stats = state.asset_cache.stats().await?;
    let mut system = System::new_all();
    let pid = Pid::from_u32(std::process::id());
    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
    system.refresh_memory();

    let process = system
        .process(pid)
        .ok_or_else(|| AppError::InternalServerError("failed to inspect process".to_string()))?;

    let rss = process.memory();
    let virtual_mem = process.virtual_memory();
    let total_mem = system.total_memory();
    let memory_used_percent = if total_mem == 0 {
        None
    } else {
        Some((rss as f64 / total_mem as f64) * 100.0)
    };

    Ok(Json(MemoryDebugResponse {
        pid: std::process::id(),
        memory_rss_mb: bytes_to_mb(rss),
        memory_virtual_mb: bytes_to_mb(virtual_mem),
        memory_used_percent,
        total_system_memory_mb: bytes_to_mb(total_mem),
        process_threads: thread_count(process),
        storage_mode: state.db_service.storage_mode().to_string(),
        cover_cache_files: asset_stats.cover_cache_files,
        cover_cache_size_mb: asset_stats.cover_cache_size_mb,
        image_page_cache_files: asset_stats.image_page_cache_files,
        image_page_cache_size_mb: asset_stats.image_page_cache_size_mb,
        pdf_svg_cache_files: asset_stats.pdf_svg_cache_files,
        pdf_svg_cache_size_mb: asset_stats.pdf_svg_cache_size_mb,
        pdf_open_documents: asset_stats.pdf_render.open_documents,
        pdf_document_idle_ttl_secs: asset_stats.pdf_render.idle_ttl_secs,
        cache_root_dir: asset_stats.root_dir.display().to_string(),
    }))
}

pub async fn release_memory() -> Result<Json<MemoryReleaseResponse>, AppError> {
    let released_bytes = crate::service::memory::release_unused_memory().await?;
    Ok(Json(MemoryReleaseResponse {
        success: true,
        released_bytes,
        released_mb: bytes_to_mb(released_bytes),
    }))
}

#[derive(Serialize)]
pub struct ScanResponse {
    pub success: bool,
    pub message: String,
    pub inserted_libraries: usize,
    pub inserted_categories: usize,
    pub inserted_book_files: usize,
    pub deleted_libraries: usize,
    pub deleted_categories: usize,
    pub deleted_book_files: usize,
    pub updated_book_files: usize,
    pub inserted_library_details: Vec<LibraryChangeDetail>,
    pub deleted_library_details: Vec<LibraryChangeDetail>,
    pub inserted_category_details: Vec<CategoryChangeDetail>,
    pub deleted_category_details: Vec<CategoryChangeDetail>,
    pub inserted_book_file_details: Vec<BookChangeDetail>,
    pub deleted_book_file_details: Vec<BookChangeDetail>,
    pub updated_book_file_details: Vec<BookChangeDetail>,
}

#[derive(Serialize)]
struct ScanStreamMessage {
    kind: String,
    message: String,
    data: Option<serde_json::Value>,
}

pub async fn scan(State(state): State<AppState>) -> Result<Json<ScanResponse>, AppError> {
    let db_service = &state.db_service;

    let report = db_service.scan_and_refresh().await?;
    cleanup_deleted_book_caches(&state.asset_cache, &report).await;
    let snapshot = db_service.get_snapshot().await?;
    let generated_covers = state
        .asset_cache
        .precompute_book_covers(&snapshot.book_files)
        .await
        .map_err(AppError::from)?;
    tracing::info!("precomputed {} book covers after scan", generated_covers);
    crate::service::restart::schedule_process_restart(std::time::Duration::from_millis(500));

    Ok(Json(build_scan_response(report)))
}

pub async fn scan_stream(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = mpsc::unbounded_channel::<ScanStreamMessage>();

    tokio::spawn(async move {
        let send = |kind: &str,
                    message: String,
                    data: Option<serde_json::Value>,
                    tx: &mpsc::UnboundedSender<ScanStreamMessage>| {
            let _ = tx.send(ScanStreamMessage {
                kind: kind.to_string(),
                message,
                data,
            });
        };

        send("log", "开始扫描数据库和文件系统...".to_string(), None, &tx);

        let report = match state.db_service.scan_and_refresh().await {
            Ok(report) => {
                cleanup_deleted_book_caches_with_progress(&state.asset_cache, &report, |message| {
                    let _ = tx.send(ScanStreamMessage {
                        kind: "log".to_string(),
                        message,
                        data: None,
                    });
                })
                .await;
                send(
                    "log",
                    format!(
                        "扫描完成，新增 {} 本，更新 {} 本，删除 {} 本，准备生成封面缓存...",
                        report.inserted_book_files,
                        report.updated_book_files,
                        report.deleted_book_files
                    ),
                    None,
                    &tx,
                );
                report
            }
            Err(err) => {
                send("failed", format!("扫描失败: {}", err), None, &tx);
                return;
            }
        };

        let snapshot = match state.db_service.get_snapshot().await {
            Ok(snapshot) => snapshot,
            Err(err) => {
                send("failed", format!("读取扫描结果失败: {}", err), None, &tx);
                return;
            }
        };

        let cover_result = state
            .asset_cache
            .precompute_book_covers_with_progress(&snapshot.book_files, |message| {
                let _ = tx.send(ScanStreamMessage {
                    kind: "log".to_string(),
                    message,
                    data: None,
                });
            })
            .await;

        match cover_result {
            Ok(count) => {
                send(
                    "log",
                    format!("封面缓存生成完成，共处理 {} 本书", count),
                    None,
                    &tx,
                );
                let response = build_scan_response(report);
                send(
                    "complete",
                    response.message.clone(),
                    Some(serde_json::to_value(response).unwrap_or(serde_json::Value::Null)),
                    &tx,
                );
                crate::service::restart::schedule_process_restart(
                    std::time::Duration::from_millis(500),
                );
            }
            Err(err) => {
                send("failed", format!("封面缓存生成失败: {}", err), None, &tx);
            }
        }
    });

    let stream = UnboundedReceiverStream::new(rx).map(|message| {
        let event_name = message.kind.clone();
        let data = serde_json::to_string(&message).unwrap_or_else(|_| {
            "{\"kind\":\"error\",\"message\":\"failed to serialize scan event\",\"data\":null}"
                .to_string()
        });
        Ok(Event::default().event(event_name).data(data))
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

fn build_scan_response(report: crate::service::database::SyncReport) -> ScanResponse {
    ScanResponse {
        success: true,
        message: "扫描完成".to_string(),
        inserted_libraries: report.inserted_libraries,
        inserted_categories: report.inserted_categories,
        inserted_book_files: report.inserted_book_files,
        deleted_libraries: report.deleted_libraries,
        deleted_categories: report.deleted_categories,
        deleted_book_files: report.deleted_book_files,
        updated_book_files: report.updated_book_files,
        inserted_library_details: report.inserted_library_details,
        deleted_library_details: report.deleted_library_details,
        inserted_category_details: report.inserted_category_details,
        deleted_category_details: report.deleted_category_details,
        inserted_book_file_details: report.inserted_book_file_details,
        deleted_book_file_details: report.deleted_book_file_details,
        updated_book_file_details: report.updated_book_file_details,
    }
}

async fn cleanup_deleted_book_caches(
    asset_cache: &crate::service::assets::AssetCacheService,
    report: &crate::service::database::SyncReport,
) {
    cleanup_deleted_book_caches_with_progress(asset_cache, report, |message| {
        tracing::info!("{}", message);
    })
    .await;
}

async fn cleanup_deleted_book_caches_with_progress<F>(
    asset_cache: &crate::service::assets::AssetCacheService,
    report: &crate::service::database::SyncReport,
    mut on_progress: F,
) where
    F: FnMut(String),
{
    for book in &report.deleted_book_file_details {
        match asset_cache.delete_book_cache(&book.path).await {
            Ok(()) => on_progress(format!("已清理缓存: {}", book.path)),
            Err(err) => tracing::warn!("failed to delete cache for {}: {}", book.path, err),
        }
    }
}

fn bytes_to_mb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

#[cfg(target_os = "linux")]
fn thread_count(process: &sysinfo::Process) -> Option<usize> {
    process.tasks().map(|tasks| tasks.len())
}

#[cfg(not(target_os = "linux"))]
fn thread_count(_process: &sysinfo::Process) -> Option<usize> {
    None
}
