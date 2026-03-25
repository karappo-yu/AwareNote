//! Scanner 识别策略模块
//!
//! 定义文件系统节点的识别逻辑，决定如何判断文件或目录是否为书籍。

use super::types::{CachedBookMetadata, ScannedBookFile, ScannedCategory};
use crate::config::{CacheConfig, ScannerConfig};
use crate::scanner::pdf::PdfHelper;
use std::path::Path;
use std::time::SystemTime;

const IMAGE_FOLDER_KIND: &str = "image_folder";
const PDF_KIND: &str = "pdf";

#[derive(Debug, Clone)]
pub struct ConfigurableRecognizer {
    pub image_extensions: Vec<String>,
    pub min_image_count: usize,
    pub oversized_image_avg_pixels: u64,
}

#[derive(Debug, Clone)]
struct ImageEntry {
    path: String,
    mtime: i64,
    size: i64,
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
        existing_book: Option<&CachedBookMetadata>,
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
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };
                if file_type.is_dir() {
                    has_subdirs = true;
                    continue;
                }
                if !file_type.is_file() {
                    continue;
                }

                let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) else {
                    continue;
                };
                let ext = ext.to_lowercase();
                if self.image_extensions.contains(&ext) {
                    let Ok(metadata) = entry.metadata() else {
                        continue;
                    };
                    direct_images.push(ImageEntry {
                        path: entry_path.to_string_lossy().to_string(),
                        mtime: system_time_to_secs(metadata.modified().ok()),
                        size: metadata.len() as i64,
                    });
                } else if ext == "pdf" {
                    has_pdf = true;
                }
            }
        }

        direct_images.sort_by(|a, b| a.path.cmp(&b.path));
        let has_image_book = direct_images.len() >= self.min_image_count;
        let is_mixed_container = has_pdf || has_subdirs;

        DirectoryInspection {
            category: (is_mixed_container || !has_image_book)
                .then(|| fill_category_model(path, metadata)),
            book: has_image_book
                .then(|| fill_image_book_model(path, metadata, self, direct_images, existing_book)),
            recurse: is_mixed_container || !has_image_book,
        }
    }

    pub fn analyze_file(
        &self,
        path: &Path,
        metadata: &std::fs::Metadata,
        existing_book: Option<&CachedBookMetadata>,
    ) -> Option<ScannedBookFile> {
        if self.is_hidden(path) {
            return None;
        }

        match path.extension().and_then(|e| e.to_str()) {
            Some("pdf") => Some(fill_pdf_book_model(path, metadata, existing_book)),
            _ => None,
        }
    }
}

fn count_pdf_pages(path: &Path) -> Option<usize> {
    PdfHelper::page_count(&path.to_string_lossy())
}

fn fill_pdf_book_model(
    path: &Path,
    metadata: &std::fs::Metadata,
    existing_book: Option<&CachedBookMetadata>,
) -> ScannedBookFile {
    let mtime = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let size = metadata.len() as i64;

    let (page_count, pages_json, content_signature, cover_path, is_oversized, avg_page_pixels) =
        if let Some(cached) = existing_book.filter(|cached| {
            cached.kind == PDF_KIND && cached.mtime == mtime && cached.size == size
        }) {
            (
                cached.page_count,
                cached.pages_json.clone(),
                cached.content_signature.clone(),
                cached.cover_path.clone(),
                cached.is_oversized,
                cached.avg_page_pixels,
            )
        } else {
            let count = count_pdf_pages(path).unwrap_or(0) as i64;
            (count, None, None, None, false, 0)
        };

    ScannedBookFile {
        path: path.to_string_lossy().to_string(),
        title: path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string()),
        kind: PDF_KIND.to_string(),
        size,
        mtime,
        page_count,
        pages_json,
        content_signature,
        is_oversized,
        avg_page_pixels,
        cover_path,
    }
}

fn fill_image_book_model(
    path: &Path,
    metadata: &std::fs::Metadata,
    recognizer: &ConfigurableRecognizer,
    images: Vec<ImageEntry>,
    existing_book: Option<&CachedBookMetadata>,
) -> ScannedBookFile {
    let mtime = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let size = metadata.len() as i64;
    let (page_count, pages_json, cover_path, content_signature, is_oversized, avg_page_pixels) =
        fill_image_book_payload_from_images(recognizer, images, existing_book, mtime, size);

    ScannedBookFile {
        path: path.to_string_lossy().to_string(),
        title: path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string()),
        kind: IMAGE_FOLDER_KIND.to_string(),
        size,
        mtime,
        page_count,
        pages_json,
        content_signature,
        is_oversized,
        avg_page_pixels,
        cover_path,
    }
}

fn fill_image_book_payload_from_images(
    recognizer: &ConfigurableRecognizer,
    images: Vec<ImageEntry>,
    existing_book: Option<&CachedBookMetadata>,
    _mtime: i64,
    _size: i64,
) -> (
    i64,
    Option<String>,
    Option<String>,
    Option<String>,
    bool,
    i64,
) {
    let cover = images.first().map(|entry| entry.path.clone());
    let image_paths: Vec<String> = images.iter().map(|entry| entry.path.clone()).collect();
    let count = image_paths.len();
    let json = serde_json::to_string(&image_paths).ok();
    let content_signature = Some(build_image_folder_signature(&images));
    let avg_page_pixels = existing_book
        .filter(|cached| {
            cached.kind == IMAGE_FOLDER_KIND && cached.content_signature == content_signature
        })
        .map(|cached| cached.avg_page_pixels as u64)
        .unwrap_or_else(|| analyze_image_folder(&image_paths));
    let is_oversized = avg_page_pixels >= recognizer.oversized_image_avg_pixels;
    (
        count as i64,
        json,
        cover,
        content_signature,
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

fn build_image_folder_signature(images: &[ImageEntry]) -> String {
    let mut hash = 0xcbf29ce484222325u64;

    for image in images {
        for bytes in [
            image.path.as_bytes(),
            &[0],
            &image.mtime.to_le_bytes(),
            &[0],
            &image.size.to_le_bytes(),
            &[0],
        ] {
            for byte in bytes {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }
    }

    format!("{hash:016x}")
}

fn system_time_to_secs(time: Option<SystemTime>) -> i64 {
    time.and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::{CachedBookMetadata, Scanner};
    use filetime::{set_file_mtime, FileTime};
    use image::{ImageBuffer, Rgba};
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn pdf_scan_reuses_cached_metadata_when_mtime_and_size_match(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let pdf_path = temp_dir.path().join("sample.pdf");
        fs::write(&pdf_path, b"not a real pdf, but stable size")?;

        let metadata = fs::metadata(&pdf_path)?;
        let recognizer = ConfigurableRecognizer::default();
        let scanner = Scanner::with_existing_books(
            &recognizer,
            vec![CachedBookMetadata {
                path: pdf_path.to_string_lossy().to_string(),
                title: Some("cached".to_string()),
                kind: PDF_KIND.to_string(),
                size: metadata.len() as i64,
                mtime: system_time_to_secs(metadata.modified().ok()),
                page_count: 77,
                pages_json: None,
                content_signature: None,
                is_oversized: false,
                avg_page_pixels: 0,
                cover_path: None,
            }],
        );

        let result = scanner.scan(&pdf_path);
        assert_eq!(result.book_files.len(), 1);
        assert_eq!(result.book_files[0].kind, PDF_KIND);
        assert_eq!(result.book_files[0].page_count, 77);

        Ok(())
    }

    #[test]
    fn pdf_scan_recomputes_when_file_size_changes() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let pdf_path = temp_dir.path().join("sample.pdf");
        fs::write(&pdf_path, b"old size")?;

        let old_metadata = fs::metadata(&pdf_path)?;
        let recognizer = ConfigurableRecognizer::default();
        let cached = CachedBookMetadata {
            path: pdf_path.to_string_lossy().to_string(),
            title: Some("cached".to_string()),
            kind: PDF_KIND.to_string(),
            size: old_metadata.len() as i64,
            mtime: system_time_to_secs(old_metadata.modified().ok()),
            page_count: 77,
            pages_json: None,
            content_signature: None,
            is_oversized: false,
            avg_page_pixels: 0,
            cover_path: None,
        };

        fs::write(&pdf_path, b"new size that breaks cache reuse")?;

        let scanner = Scanner::with_existing_books(&recognizer, vec![cached]);
        let result = scanner.scan(&pdf_path);
        assert_eq!(result.book_files.len(), 1);
        assert_eq!(result.book_files[0].page_count, 0);

        Ok(())
    }

    #[test]
    fn image_folder_reuses_cached_avg_pixels_when_signature_matches(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let book_dir = temp_dir.path().join("book");
        fs::create_dir(&book_dir)?;
        create_png(&book_dir.join("001.png"), 10, 10)?;
        create_png(&book_dir.join("002.png"), 20, 20)?;

        let recognizer = ConfigurableRecognizer::with_config(vec!["png".to_string()], 2, 100_000);
        let initial_scan = Scanner::new(&recognizer).scan(&book_dir);
        let initial_book = initial_scan.book_files[0].clone();

        let scanner = Scanner::with_existing_books(
            &recognizer,
            vec![CachedBookMetadata {
                path: initial_book.path.clone(),
                title: initial_book.title.clone(),
                kind: initial_book.kind.clone(),
                size: initial_book.size,
                mtime: initial_book.mtime,
                page_count: initial_book.page_count,
                pages_json: initial_book.pages_json.clone(),
                content_signature: initial_book.content_signature.clone(),
                is_oversized: true,
                avg_page_pixels: 123_456,
                cover_path: initial_book.cover_path.clone(),
            }],
        );

        let result = scanner.scan(&book_dir);
        assert_eq!(result.book_files.len(), 1);
        assert_eq!(result.book_files[0].avg_page_pixels, 123_456);
        assert!(result.book_files[0].is_oversized);

        Ok(())
    }

    #[test]
    fn image_folder_recomputes_when_child_file_changes_even_if_directory_mtime_is_restored(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let book_dir = temp_dir.path().join("book");
        fs::create_dir(&book_dir)?;
        let first = book_dir.join("001.png");
        let second = book_dir.join("002.png");
        create_png(&first, 10, 10)?;
        create_png(&second, 20, 20)?;

        let recognizer = ConfigurableRecognizer::with_config(vec!["png".to_string()], 2, 10_000);
        let initial_scan = Scanner::new(&recognizer).scan(&book_dir);
        let initial_book = initial_scan.book_files[0].clone();
        let original_dir_mtime = fs::metadata(&book_dir)?.modified()?;

        std::thread::sleep(std::time::Duration::from_secs(1));
        create_png(&first, 30, 30)?;
        set_file_mtime(&book_dir, FileTime::from_system_time(original_dir_mtime))?;

        let scanner = Scanner::with_existing_books(
            &recognizer,
            vec![CachedBookMetadata {
                path: initial_book.path.clone(),
                title: initial_book.title.clone(),
                kind: initial_book.kind.clone(),
                size: initial_book.size,
                mtime: initial_book.mtime,
                page_count: initial_book.page_count,
                pages_json: initial_book.pages_json.clone(),
                content_signature: initial_book.content_signature.clone(),
                is_oversized: initial_book.is_oversized,
                avg_page_pixels: initial_book.avg_page_pixels,
                cover_path: initial_book.cover_path.clone(),
            }],
        );

        let result = scanner.scan(&book_dir);
        assert_eq!(result.book_files.len(), 1);
        assert_ne!(
            result.book_files[0].content_signature,
            initial_book.content_signature
        );
        assert_ne!(
            result.book_files[0].avg_page_pixels,
            initial_book.avg_page_pixels
        );
        assert_eq!(result.book_files[0].avg_page_pixels, 650);

        Ok(())
    }

    #[test]
    fn directory_classification_changes_when_min_image_count_changes(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let book_dir = temp_dir.path().join("maybe-book");
        fs::create_dir(&book_dir)?;
        create_png(&book_dir.join("001.png"), 10, 10)?;
        create_png(&book_dir.join("002.png"), 20, 20)?;

        let recognizer_as_book =
            ConfigurableRecognizer::with_config(vec!["png".to_string()], 2, 100_000);
        let recognizer_as_category =
            ConfigurableRecognizer::with_config(vec!["png".to_string()], 3, 100_000);

        let book_scan = Scanner::new(&recognizer_as_book).scan(&book_dir);
        let category_scan = Scanner::new(&recognizer_as_category).scan(&book_dir);

        assert_eq!(book_scan.book_files.len(), 1);
        assert_eq!(book_scan.categories.len(), 1);
        assert!(category_scan.book_files.is_empty());
        assert_eq!(category_scan.categories.len(), 1);

        Ok(())
    }

    #[test]
    fn directory_classification_changes_when_image_extensions_change(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let book_dir = temp_dir.path().join("ext-sensitive");
        fs::create_dir(&book_dir)?;
        create_png(&book_dir.join("001.png"), 10, 10)?;
        create_png(&book_dir.join("002.png"), 20, 20)?;

        let recognizer_with_png =
            ConfigurableRecognizer::with_config(vec!["png".to_string()], 2, 100_000);
        let recognizer_without_png =
            ConfigurableRecognizer::with_config(vec!["jpg".to_string()], 2, 100_000);

        let png_scan = Scanner::new(&recognizer_with_png).scan(&book_dir);
        let non_png_scan = Scanner::new(&recognizer_without_png).scan(&book_dir);

        assert_eq!(png_scan.book_files.len(), 1);
        assert!(non_png_scan.book_files.is_empty());
        assert_eq!(non_png_scan.categories.len(), 1);

        Ok(())
    }

    fn create_png(path: &Path, width: u32, height: u32) -> Result<(), Box<dyn std::error::Error>> {
        let image: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(width, height, Rgba([255, 0, 0, 255]));
        image.save(path)?;
        Ok(())
    }
}
