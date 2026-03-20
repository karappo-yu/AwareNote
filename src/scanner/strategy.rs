//! Scanner 识别策略模块
//!
//! 定义文件系统节点的识别逻辑，决定如何判断文件或目录是否为书籍。

use super::types::{ScannedBookFile, ScannedCategory};
use crate::config::{CacheConfig, ScannerConfig};
use crate::scanner::pdf::PdfHelper;
use std::path::Path;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub enum BookKind {
    ImageFolder,
    Pdf,
}

impl BookKind {
    fn as_str(&self) -> &str {
        match self {
            BookKind::ImageFolder => "image_folder",
            BookKind::Pdf => "pdf",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigurableRecognizer {
    pub image_extensions: Vec<String>,
    pub min_image_count: usize,
    pub oversized_image_avg_pixels: u64,
}

impl Default for ConfigurableRecognizer {
    fn default() -> Self {
        Self {
            image_extensions: vec![
                "jpg".to_string(),
                "jpeg".to_string(),
                "png".to_string(),
                "webp".to_string(),
                "gif".to_string(),
            ],
            min_image_count: 3,
            oversized_image_avg_pixels: CacheConfig::default().oversized_image_avg_pixels,
        }
    }
}

impl From<(ScannerConfig, CacheConfig)> for ConfigurableRecognizer {
    fn from((config, cache): (ScannerConfig, CacheConfig)) -> Self {
        Self {
            image_extensions: config.image_extensions,
            min_image_count: config.min_image_count,
            oversized_image_avg_pixels: cache.oversized_image_avg_pixels,
        }
    }
}

impl ConfigurableRecognizer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(
        image_extensions: Vec<String>,
        min_image_count: usize,
        oversized_image_avg_pixels: u64,
    ) -> Self {
        Self {
            image_extensions,
            min_image_count,
            oversized_image_avg_pixels,
        }
    }

    pub fn is_hidden(&self, path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with('.'))
            .unwrap_or(false)
    }

    pub fn inspect_directory(
        &self,
        path: &Path,
        metadata: &std::fs::Metadata,
    ) -> DirectoryInspection {
        let mut direct_images = Vec::new();
        let mut has_pdf = false;
        let mut has_subdirs = false;

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if self.is_hidden(&entry_path) {
                    continue;
                }
                if entry_path.is_dir() {
                    has_subdirs = true;
                    continue;
                }
                if !entry_path.is_file() {
                    continue;
                }

                let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) else {
                    continue;
                };
                let ext = ext.to_lowercase();
                if self.image_extensions.contains(&ext) {
                    direct_images.push(entry_path.to_string_lossy().to_string());
                } else if ext == "pdf" {
                    has_pdf = true;
                }
            }
        }

        direct_images.sort();
        let has_image_book = direct_images.len() >= self.min_image_count;
        let is_mixed_container = has_pdf || has_subdirs;

        DirectoryInspection {
            category: (is_mixed_container || !has_image_book)
                .then(|| fill_category_model(path, metadata)),
            book: has_image_book
                .then(|| fill_image_book_model(path, metadata, self, direct_images)),
            recurse: is_mixed_container || !has_image_book,
        }
    }

    pub fn analyze_file(
        &self,
        path: &Path,
        metadata: &std::fs::Metadata,
    ) -> Option<ScannedBookFile> {
        if self.is_hidden(path) {
            return None;
        }

        match path.extension().and_then(|e| e.to_str()) {
            Some("pdf") => Some(fill_book_model(path, metadata, &BookKind::Pdf, self)),
            _ => None,
        }
    }
}

fn count_pdf_pages(path: &Path) -> Option<usize> {
    PdfHelper::page_count(&path.to_string_lossy())
}

fn fill_book_model(
    path: &Path,
    metadata: &std::fs::Metadata,
    kind: &BookKind,
    recognizer: &ConfigurableRecognizer,
) -> ScannedBookFile {
    let mtime = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let size = metadata.len() as i64;

    let (page_count, pages_json, cover_path, is_oversized, avg_page_pixels) = match kind {
        BookKind::ImageFolder => fill_image_book_payload(path, recognizer),
        BookKind::Pdf => {
            let count = count_pdf_pages(path).unwrap_or(0) as i64;
            (count, None, None, false, 0)
        }
    };

    ScannedBookFile {
        path: path.to_string_lossy().to_string(),
        title: path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string()),
        kind: kind.as_str().to_string(),
        size,
        mtime,
        page_count,
        pages_json,
        is_oversized,
        avg_page_pixels,
        cover_path,
    }
}

fn fill_image_book_model(
    path: &Path,
    metadata: &std::fs::Metadata,
    recognizer: &ConfigurableRecognizer,
    images: Vec<String>,
) -> ScannedBookFile {
    let mtime = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let size = metadata.len() as i64;
    let (page_count, pages_json, cover_path, is_oversized, avg_page_pixels) =
        fill_image_book_payload_from_images(recognizer, images);

    ScannedBookFile {
        path: path.to_string_lossy().to_string(),
        title: path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string()),
        kind: BookKind::ImageFolder.as_str().to_string(),
        size,
        mtime,
        page_count,
        pages_json,
        is_oversized,
        avg_page_pixels,
        cover_path,
    }
}

fn fill_image_book_payload(
    path: &Path,
    recognizer: &ConfigurableRecognizer,
) -> (i64, Option<String>, Option<String>, bool, i64) {
    let mut images: Vec<String> = std::fs::read_dir(path)
        .ok()
        .map(|r| r.flatten())
        .into_iter()
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            if !p.is_file() {
                return None;
            }
            let ext = p.extension()?.to_str()?.to_lowercase();
            ["jpg", "jpeg", "png", "webp", "gif"]
                .contains(&ext.as_str())
                .then_some(p.to_string_lossy().to_string())
        })
        .collect();
    images.sort();
    fill_image_book_payload_from_images(recognizer, images)
}

fn fill_image_book_payload_from_images(
    recognizer: &ConfigurableRecognizer,
    images: Vec<String>,
) -> (i64, Option<String>, Option<String>, bool, i64) {
    let cover = images.first().cloned();
    let count = images.len();
    let json = serde_json::to_string(&images).ok();
    let avg_page_pixels = analyze_image_folder(&images);
    let is_oversized = avg_page_pixels >= recognizer.oversized_image_avg_pixels;
    (
        count as i64,
        json,
        cover,
        is_oversized,
        avg_page_pixels as i64,
    )
}

fn analyze_image_folder(images: &[String]) -> u64 {
    if images.is_empty() {
        return 0;
    }

    const SAMPLE_COUNT: usize = 5;
    let sample_count = images.len().min(SAMPLE_COUNT);
    if sample_count == 0 {
        return 0;
    }

    let mut total_pixels = 0u64;
    let mut measured = 0u64;

    for image_path in sample_image_paths(images, sample_count) {
        if let Ok((width, height)) = image::image_dimensions(image_path) {
            total_pixels =
                total_pixels.saturating_add((width as u64).saturating_mul(height as u64));
            measured += 1;
        }
    }

    if measured == 0 {
        0
    } else {
        total_pixels / measured
    }
}

fn sample_image_paths(images: &[String], sample_count: usize) -> Vec<&str> {
    if sample_count <= 1 {
        return images.first().map(|s| vec![s.as_str()]).unwrap_or_default();
    }

    if sample_count >= images.len() {
        return images.iter().map(String::as_str).collect();
    }

    let last = images.len() - 1;
    let mut indices = std::collections::BTreeSet::new();
    for i in 0..sample_count {
        let idx = i * last / (sample_count - 1);
        indices.insert(idx);
    }

    indices
        .into_iter()
        .map(|idx| images[idx].as_str())
        .collect()
}

fn fill_category_model(path: &Path, metadata: &std::fs::Metadata) -> ScannedCategory {
    let file_name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let mtime = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    ScannedCategory {
        name: file_name,
        path: path.to_string_lossy().to_string(),
        mtime,
    }
}

pub struct DirectoryInspection {
    pub category: Option<ScannedCategory>,
    pub book: Option<ScannedBookFile>,
    pub recurse: bool,
}
