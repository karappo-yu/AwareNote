use crate::config::CacheConfig;
use crate::domain::book_files;
use crate::scanner::pdf::{PdfRenderService, PdfRenderStats};
use fast_image_resize as fr;
use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, RgbImage};
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct AssetCacheService {
    root_dir: PathBuf,
    config: Arc<RwLock<CacheConfig>>,
    render_limiter: Arc<RwLock<Arc<Semaphore>>>,
    file_io_limiter: Arc<Semaphore>,
    pdf_renderer: Arc<PdfRenderService>,
}

pub enum PdfPageSvgAsset {
    CachedFile(PathBuf),
    GeneratedBytes(Vec<u8>),
}

pub struct AssetCacheStats {
    pub root_dir: PathBuf,
    pub cover_cache_files: usize,
    pub cover_cache_size_mb: f64,
    pub image_page_cache_files: usize,
    pub image_page_cache_size_mb: f64,
    pub pdf_svg_cache_files: usize,
    pub pdf_svg_cache_size_mb: f64,
    pub pdf_render: PdfRenderStats,
}

#[derive(Debug, Clone, Copy)]
pub enum CacheClearTarget {
    All,
    Covers,
    ImagePages,
    PdfSvg,
}

impl AssetCacheService {
    pub fn new(config: CacheConfig, file_io_concurrency: usize) -> std::io::Result<Self> {
        let root_dir = crate::runtime::app_cache_root()?;
        std::fs::create_dir_all(root_dir.join("covers"))?;
        std::fs::create_dir_all(root_dir.join("image_pages"))?;
        std::fs::create_dir_all(root_dir.join("pdf_svg"))?;
        Ok(Self {
            root_dir,
            render_limiter: Arc::new(RwLock::new(Arc::new(Semaphore::new(
                config.max_render_jobs.max(1),
            )))),
            file_io_limiter: Arc::new(Semaphore::new(file_io_concurrency.max(1))),
            config: Arc::new(RwLock::new(config)),
            pdf_renderer: Arc::new(PdfRenderService::new()?),
        })
    }

    pub fn update_config(&self, config: CacheConfig) -> std::io::Result<()> {
        {
            let mut guard = self.config.write().map_err(|err| {
                std::io::Error::other(format!("asset cache config lock poisoned: {err}"))
            })?;
            *guard = config.clone();
        }
        {
            let mut limiter = self.render_limiter.write().map_err(|err| {
                std::io::Error::other(format!("asset cache limiter lock poisoned: {err}"))
            })?;
            *limiter = Arc::new(Semaphore::new(config.max_render_jobs.max(1)));
        }
        Ok(())
    }

    pub fn config(&self) -> CacheConfig {
        match self.config.read() {
            Ok(guard) => guard.clone(),
            Err(err) => {
                tracing::warn!(
                    "asset cache config lock poisoned, using inner value: {}",
                    err
                );
                err.into_inner().clone()
            }
        }
    }

    pub async fn get_or_create_image_cover(
        &self,
        book_path: &str,
        source_image_path: &str,
    ) -> std::io::Result<PathBuf> {
        let cache_path = self.cover_cache_path(book_path);
        if cache_path.exists() {
            return Ok(cache_path);
        }
        let limiter = self
            .render_limiter
            .read()
            .map(|guard| guard.clone())
            .map_err(|err| {
                std::io::Error::other(format!("asset cache limiter lock poisoned: {err}"))
            })?;
        let _permit = limiter
            .acquire()
            .await
            .map_err(|err| std::io::Error::other(format!("render limiter closed: {err}")))?;
        if cache_path.exists() {
            return Ok(cache_path);
        }
        let width = self.config().cover_width;
        let cache_path_clone = cache_path.clone();
        let source = source_image_path.to_string();
        run_image_cover_job(source, cache_path_clone, width).await?;
        Ok(cache_path)
    }

    pub async fn get_or_create_pdf_cover(&self, book_path: &str) -> std::io::Result<PathBuf> {
        let cache_path = self.cover_cache_path(book_path);
        if cache_path.exists() {
            return Ok(cache_path);
        }
        let limiter = self
            .render_limiter
            .read()
            .map(|guard| guard.clone())
            .map_err(|err| {
                std::io::Error::other(format!("asset cache limiter lock poisoned: {err}"))
            })?;
        let _permit = limiter
            .acquire()
            .await
            .map_err(|err| std::io::Error::other(format!("render limiter closed: {err}")))?;
        if cache_path.exists() {
            return Ok(cache_path);
        }
        let width = self.config().cover_width;
        self.pdf_renderer
            .write_cover_jpeg(book_path, width, &cache_path)
            .await?;
        Ok(cache_path)
    }

    pub async fn get_pdf_page_svg(
        &self,
        book_path: &str,
        page_index: usize,
    ) -> std::io::Result<PdfPageSvgAsset> {
        let cache_path = self.pdf_svg_cache_path(book_path, page_index + 1);
        if cache_path.exists() {
            return Ok(PdfPageSvgAsset::CachedFile(cache_path));
        }
        let limiter = self
            .render_limiter
            .read()
            .map(|guard| guard.clone())
            .map_err(|err| {
                std::io::Error::other(format!("asset cache limiter lock poisoned: {err}"))
            })?;
        let _permit = limiter
            .acquire()
            .await
            .map_err(|err| std::io::Error::other(format!("render limiter closed: {err}")))?;
        if cache_path.exists() {
            return Ok(PdfPageSvgAsset::CachedFile(cache_path));
        }
        let width = self.config().pdf_svg_width;
        self.pdf_renderer
            .write_page_svg(book_path, page_index, width, &cache_path)
            .await?;
        Ok(PdfPageSvgAsset::CachedFile(cache_path))
    }

    pub async fn get_or_create_image_page_preview(
        &self,
        book_path: &str,
        page_index: usize,
        page_path: &str,
    ) -> std::io::Result<PathBuf> {
        let width = self.config().image_page_preview_width.max(256);
        let cache_path = self.image_page_cache_path(book_path, page_index, width, "jpg");
        if cache_path.exists() {
            return Ok(cache_path);
        }
        let limiter = self
            .render_limiter
            .read()
            .map(|guard| guard.clone())
            .map_err(|err| {
                std::io::Error::other(format!("asset cache limiter lock poisoned: {err}"))
            })?;
        let _permit = limiter
            .acquire()
            .await
            .map_err(|err| std::io::Error::other(format!("render limiter closed: {err}")))?;
        if cache_path.exists() {
            return Ok(cache_path);
        }
        let source = page_path.to_string();
        let cache_path_clone = cache_path.clone();
        tokio::task::spawn_blocking(move || {
            create_image_page_preview(&source, &cache_path_clone, width)
        })
        .await
        .map_err(|err| std::io::Error::other(err.to_string()))??;
        Ok(cache_path)
    }

    pub async fn precompute_book_covers(
        &self,
        books: &[book_files::Model],
    ) -> std::io::Result<usize> {
        self.precompute_book_covers_with_progress(books, |_| {})
            .await
    }

    pub async fn precompute_book_covers_with_progress<F>(
        &self,
        books: &[book_files::Model],
        mut on_progress: F,
    ) -> std::io::Result<usize>
    where
        F: FnMut(String),
    {
        let jobs: Vec<(&book_files::Model, Option<String>)> = books
            .iter()
            .filter_map(|book| {
                if book.kind == "pdf" {
                    Some((book, None))
                } else {
                    book.cover_path
                        .clone()
                        .or_else(|| first_image_path(book))
                        .map(|source_image_path| (book, Some(source_image_path)))
                }
            })
            .collect();

        let total = jobs.len();
        if total == 0 {
            return Ok(0);
        }

        let mut generated = 0usize;

        for (index, (book, source_image_path)) in jobs.iter().enumerate() {
            let title = book.title.clone().unwrap_or_else(|| book.path.clone());
            let cache_path = self.cover_cache_path(&book.path);
            if cache_path.exists() {
                continue;
            }

            let result = match source_image_path.as_deref() {
                Some(source_image_path) => {
                    self.get_or_create_image_cover(&book.path, source_image_path)
                        .await
                }
                None => self.get_or_create_pdf_cover(&book.path).await,
            };

            match result {
                Ok(_) => {
                    generated += 1;
                    on_progress(format!("已生成封面 {}/{}: {}", index + 1, total, title));
                }
                Err(err) => {
                    on_progress(format!("封面生成失败: {} ({})", book.path, err));
                    tracing::warn!("failed to precompute cover for {}: {}", book.path, err);
                }
            }
        }

        Ok(generated)
    }

    pub async fn read_cached_file(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        let _permit = self
            .file_io_limiter
            .acquire()
            .await
            .map_err(|err| std::io::Error::other(format!("file I/O limiter closed: {err}")))?;
        tokio::task::spawn_blocking({
            let path = path.to_path_buf();
            move || std::fs::read(path)
        })
        .await
        .map_err(|err| std::io::Error::other(err.to_string()))?
    }

    pub async fn clear_all_cache(&self) -> std::io::Result<f64> {
        self.clear_cache(CacheClearTarget::All).await
    }

    pub async fn clear_cache(&self, target: CacheClearTarget) -> std::io::Result<f64> {
        let cover_root = self.root_dir.join("covers");
        let image_page_root = self.root_dir.join("image_pages");
        let pdf_svg_root = self.root_dir.join("pdf_svg");
        tokio::task::spawn_blocking(move || {
            let mut freed_mb = 0.0;
            match target {
                CacheClearTarget::All => {
                    freed_mb += clear_directory_and_measure_mb(&cover_root)?;
                    freed_mb += clear_directory_and_measure_mb(&image_page_root)?;
                    freed_mb += clear_directory_and_measure_mb(&pdf_svg_root)?;
                }
                CacheClearTarget::Covers => {
                    freed_mb += clear_directory_and_measure_mb(&cover_root)?;
                }
                CacheClearTarget::ImagePages => {
                    freed_mb += clear_directory_and_measure_mb(&image_page_root)?;
                }
                CacheClearTarget::PdfSvg => {
                    freed_mb += clear_directory_and_measure_mb(&pdf_svg_root)?;
                }
            }
            Ok(freed_mb)
        })
        .await
        .map_err(|err| std::io::Error::other(err.to_string()))?
    }

    pub async fn delete_book_cache(&self, book_path: &str) -> std::io::Result<()> {
        let cover_path = self.cover_cache_path(book_path);
        let image_page_dir = self.book_cache_dir("image_pages", book_path);
        let pdf_svg_dir = self.book_cache_dir("pdf_svg", book_path);
        tokio::task::spawn_blocking(move || {
            remove_file_if_exists(&cover_path)?;
            remove_dir_if_exists(&image_page_dir)?;
            remove_dir_if_exists(&pdf_svg_dir)?;
            Ok(())
        })
        .await
        .map_err(|err| std::io::Error::other(err.to_string()))?
    }

    pub async fn stats(&self) -> std::io::Result<AssetCacheStats> {
        let cover_root = self.root_dir.join("covers");
        let image_page_root = self.root_dir.join("image_pages");
        let pdf_svg_root = self.root_dir.join("pdf_svg");
        let cover_stats = tokio::task::spawn_blocking(move || directory_stats(&cover_root))
            .await
            .map_err(|err| std::io::Error::other(err.to_string()))??;
        let image_page_stats =
            tokio::task::spawn_blocking(move || directory_stats(&image_page_root))
                .await
                .map_err(|err| std::io::Error::other(err.to_string()))??;
        let pdf_svg_stats = tokio::task::spawn_blocking(move || directory_stats(&pdf_svg_root))
            .await
            .map_err(|err| std::io::Error::other(err.to_string()))??;
        let pdf_render = self.pdf_renderer.stats().await?;

        Ok(AssetCacheStats {
            root_dir: self.root_dir.clone(),
            cover_cache_files: cover_stats.files,
            cover_cache_size_mb: bytes_to_mb(cover_stats.bytes),
            image_page_cache_files: image_page_stats.files,
            image_page_cache_size_mb: bytes_to_mb(image_page_stats.bytes),
            pdf_svg_cache_files: pdf_svg_stats.files,
            pdf_svg_cache_size_mb: bytes_to_mb(pdf_svg_stats.bytes),
            pdf_render,
        })
    }

    fn cover_cache_path(&self, book_path: &str) -> PathBuf {
        self.root_dir
            .join("covers")
            .join(format!("{}.jpg", hash_key(book_path)))
    }

    fn pdf_svg_cache_path(&self, book_path: &str, page: usize) -> PathBuf {
        self.root_dir
            .join("pdf_svg")
            .join(cache_book_dir_name(book_path))
            .join(format!("page-{}.svg", page))
    }

    fn image_page_cache_path(
        &self,
        book_path: &str,
        page_index: usize,
        width: u32,
        ext: &str,
    ) -> PathBuf {
        self.root_dir
            .join("image_pages")
            .join(cache_book_dir_name(book_path))
            .join(format!("page-{}-w{}.{}", page_index + 1, width, ext))
    }

    fn book_cache_dir(&self, category: &str, book_path: &str) -> PathBuf {
        self.root_dir
            .join(category)
            .join(cache_book_dir_name(book_path))
    }
}

fn create_image_cover(source_path: &str, target_path: &Path, width: u32) -> std::io::Result<()> {
    let file = File::open(source_path)?;
    let reader = image::ImageReader::new(BufReader::new(file))
        .with_guessed_format()
        .map_err(|err| std::io::Error::other(err.to_string()))?;
    let image = reader
        .decode()
        .map_err(|err| std::io::Error::other(err.to_string()))?;
    let rgb = image.into_rgb8();
    let (src_width, src_height) = rgb.dimensions();
    let (dst_width, dst_height) = resized_cover_dimensions(src_width, src_height, width);

    let resized_bytes = if (src_width, src_height) == (dst_width, dst_height) {
        rgb.into_raw()
    } else {
        resize_rgb_image(rgb, dst_width, dst_height)?
    };

    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(target_path)?;
    let writer = BufWriter::new(file);
    let mut encoder = JpegEncoder::new_with_quality(writer, 85);
    encoder
        .encode(
            &resized_bytes,
            dst_width,
            dst_height,
            ExtendedColorType::Rgb8,
        )
        .map_err(|err| std::io::Error::other(err.to_string()))
}

fn create_image_page_preview(
    source_path: &str,
    target_path: &Path,
    width: u32,
) -> std::io::Result<()> {
    let file = File::open(source_path)?;
    let reader = image::ImageReader::new(BufReader::new(file))
        .with_guessed_format()
        .map_err(|err| std::io::Error::other(err.to_string()))?;
    let image = reader
        .decode()
        .map_err(|err| std::io::Error::other(err.to_string()))?;
    let rgb = image.into_rgb8();
    let (src_width, src_height) = rgb.dimensions();
    let (dst_width, dst_height) = resized_fit_width(src_width, src_height, width);
    let resized_bytes = if (src_width, src_height) == (dst_width, dst_height) {
        rgb.into_raw()
    } else {
        resize_rgb_image(rgb, dst_width, dst_height)?
    };

    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = File::create(target_path)?;
    let writer = BufWriter::new(file);
    let mut encoder = JpegEncoder::new_with_quality(writer, 85);
    encoder
        .encode(
            &resized_bytes,
            dst_width,
            dst_height,
            ExtendedColorType::Rgb8,
        )
        .map_err(|err| std::io::Error::other(err.to_string()))
}

async fn run_image_cover_job(
    source_path: String,
    target_path: PathBuf,
    width: u32,
) -> std::io::Result<()> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    std::thread::Builder::new()
        .name("auxm-image-cover".to_string())
        .spawn(move || {
            let result = create_image_cover(&source_path, &target_path, width);
            let _ = tx.send(result);
        })
        .map_err(|err| std::io::Error::other(err.to_string()))?;

    rx.await
        .map_err(|_| std::io::Error::other("image cover worker stopped unexpectedly"))?
}

fn resized_cover_dimensions(src_width: u32, src_height: u32, target_width: u32) -> (u32, u32) {
    let max_width = target_width.max(1);
    let max_height = max_width.saturating_mul(2);

    if src_width == 0 || src_height == 0 {
        return (max_width, max_height.max(1));
    }

    let width_ratio = max_width as f32 / src_width as f32;
    let height_ratio = max_height as f32 / src_height as f32;
    let scale = width_ratio.min(height_ratio);
    let dst_width = ((src_width as f32 * scale).round() as u32).max(1);
    let dst_height = ((src_height as f32 * scale).round() as u32).max(1);
    (dst_width, dst_height)
}

fn resized_fit_width(src_width: u32, src_height: u32, target_width: u32) -> (u32, u32) {
    let max_width = target_width.max(1);
    if src_width == 0 || src_height == 0 {
        return (max_width, max_width);
    }
    if src_width <= max_width {
        return (src_width, src_height);
    }

    let scale = max_width as f32 / src_width as f32;
    let dst_width = max_width;
    let dst_height = ((src_height as f32 * scale).round() as u32).max(1);
    (dst_width, dst_height)
}

fn resize_rgb_image(rgb: RgbImage, dst_width: u32, dst_height: u32) -> std::io::Result<Vec<u8>> {
    let (src_width, src_height) = rgb.dimensions();
    let src =
        fr::images::Image::from_vec_u8(src_width, src_height, rgb.into_raw(), fr::PixelType::U8x3)
            .map_err(|err| std::io::Error::other(err.to_string()))?;
    let mut dst = fr::images::Image::new(dst_width, dst_height, fr::PixelType::U8x3);

    let mut resizer = fr::Resizer::new();
    let options =
        fr::ResizeOptions::new().resize_alg(fr::ResizeAlg::Interpolation(fr::FilterType::Bilinear));
    resizer
        .resize(&src, &mut dst, &options)
        .map_err(|err| std::io::Error::other(err.to_string()))?;

    Ok(dst.into_vec())
}

fn hash_key(value: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn cache_book_dir_name(book_path: &str) -> String {
    let display_name = Path::new(book_path)
        .file_stem()
        .or_else(|| Path::new(book_path).file_name())
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("book");
    let sanitized: String = display_name
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let sanitized = sanitized.trim().trim_matches('.').trim();
    let sanitized = if sanitized.is_empty() {
        "book"
    } else {
        sanitized
    };
    format!("{}-{}", sanitized, hash_key(book_path))
}

fn clear_directory_and_measure_mb(dir: &Path) -> std::io::Result<f64> {
    if !dir.exists() {
        return Ok(0.0);
    }

    let bytes = directory_size_bytes(dir)?;
    std::fs::remove_dir_all(dir)?;
    std::fs::create_dir_all(dir)?;
    Ok(bytes_to_mb(bytes))
}

fn remove_dir_if_exists(dir: &Path) -> std::io::Result<()> {
    if dir.exists() {
        std::fs::remove_dir_all(dir)?;
    }
    Ok(())
}

fn remove_file_if_exists(path: &Path) -> std::io::Result<()> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

fn directory_size_bytes(dir: &Path) -> std::io::Result<u64> {
    let mut total = 0u64;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            total = total.saturating_add(directory_size_bytes(&path)?);
        } else {
            total = total.saturating_add(metadata.len());
        }
    }
    Ok(total)
}

struct DirectoryStats {
    files: usize,
    bytes: u64,
}

fn directory_stats(dir: &Path) -> std::io::Result<DirectoryStats> {
    if !dir.exists() {
        return Ok(DirectoryStats { files: 0, bytes: 0 });
    }

    let mut stats = DirectoryStats { files: 0, bytes: 0 };
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            let child = directory_stats(&path)?;
            stats.files += child.files;
            stats.bytes = stats.bytes.saturating_add(child.bytes);
        } else {
            stats.files += 1;
            stats.bytes = stats.bytes.saturating_add(metadata.len());
        }
    }
    Ok(stats)
}

fn bytes_to_mb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

fn first_image_path(book: &book_files::Model) -> Option<String> {
    serde_json::from_str::<Vec<String>>(book.pages_json.as_deref().unwrap_or("[]"))
        .ok()
        .and_then(|pages| pages.into_iter().next())
}
