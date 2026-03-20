//! PDF 工具模块
//!
//! 基于 MuPDF 提供 PDF 文件处理能力。

use image::codecs::jpeg::JpegEncoder;
use image::ExtendedColorType;
use mupdf::{Colorspace, Document, Matrix};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::oneshot;

pub struct PdfHelper;

impl PdfHelper {
    pub fn page_count(path: &str) -> Option<usize> {
        let document = Document::open(path).ok()?;
        document.page_count().ok()?.try_into().ok()
    }
}

#[derive(Debug)]
pub struct PdfRenderService {
    sender: mpsc::Sender<RenderCommand>,
    worker: Mutex<Option<thread::JoinHandle<()>>>,
}

#[derive(Debug, Clone, Copy)]
pub struct PdfRenderStats {
    pub open_documents: usize,
    pub idle_ttl_secs: u64,
}

impl PdfRenderService {
    pub fn new() -> std::io::Result<Self> {
        let (sender, receiver) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("auxm-pdf-renderer".to_string())
            .spawn(move || pdf_render_worker_loop(receiver))
            .map_err(|err| {
                std::io::Error::other(format!("failed to start pdf render worker: {err}"))
            })?;

        Ok(Self {
            sender,
            worker: Mutex::new(Some(worker)),
        })
    }

    pub async fn write_page_svg(
        &self,
        path: &str,
        page_index: usize,
        width: u32,
        target_path: &Path,
    ) -> std::io::Result<()> {
        let (response_tx, response_rx) = oneshot::channel();
        self.sender
            .send(RenderCommand::Render(RenderRequest {
                path: path.to_string(),
                page_index,
                width,
                target_path: target_path.to_path_buf(),
                response_tx,
            }))
            .map_err(|_| std::io::Error::other("pdf render worker is not available"))?;

        response_rx
            .await
            .map_err(|_| std::io::Error::other("pdf render worker stopped unexpectedly"))?
    }

    pub async fn write_cover_svg(
        &self,
        path: &str,
        width: u32,
        target_path: &Path,
    ) -> std::io::Result<()> {
        self.write_page_svg(path, 0, width, target_path).await
    }

    pub async fn write_cover_jpeg(
        &self,
        path: &str,
        width: u32,
        target_path: &Path,
    ) -> std::io::Result<()> {
        let (response_tx, response_rx) = oneshot::channel();
        self.sender
            .send(RenderCommand::RenderCoverJpeg(RenderRequest {
                path: path.to_string(),
                page_index: 0,
                width,
                target_path: target_path.to_path_buf(),
                response_tx,
            }))
            .map_err(|_| std::io::Error::other("pdf render worker is not available"))?;

        response_rx
            .await
            .map_err(|_| std::io::Error::other("pdf render worker stopped unexpectedly"))?
    }

    pub async fn render_page_svg_bytes(
        &self,
        path: &str,
        page_index: usize,
        width: u32,
    ) -> std::io::Result<Vec<u8>> {
        let (response_tx, response_rx) = oneshot::channel();
        self.sender
            .send(RenderCommand::RenderSvgBytes(ByteRenderRequest {
                path: path.to_string(),
                page_index,
                width,
                response_tx,
            }))
            .map_err(|_| std::io::Error::other("pdf render worker is not available"))?;

        response_rx
            .await
            .map_err(|_| std::io::Error::other("pdf render worker stopped unexpectedly"))?
    }

    pub async fn stats(&self) -> std::io::Result<PdfRenderStats> {
        let (response_tx, response_rx) = oneshot::channel();
        self.sender
            .send(RenderCommand::Stats { response_tx })
            .map_err(|_| std::io::Error::other("pdf render worker is not available"))?;

        response_rx
            .await
            .map_err(|_| std::io::Error::other("pdf render worker stopped unexpectedly"))
    }
}

impl Default for PdfRenderService {
    fn default() -> Self {
        Self {
            sender: mpsc::channel().0,
            worker: Mutex::new(None),
        }
    }
}

impl Drop for PdfRenderService {
    fn drop(&mut self) {
        let _ = self.sender.send(RenderCommand::Shutdown);
        match self.worker.lock() {
            Ok(mut guard) => {
                if let Some(worker) = guard.take() {
                    let _ = worker.join();
                }
            }
            Err(err) => {
                tracing::warn!("failed to stop pdf render worker cleanly: {}", err);
            }
        }
    }
}

enum RenderCommand {
    Render(RenderRequest),
    RenderCoverJpeg(RenderRequest),
    RenderSvgBytes(ByteRenderRequest),
    Stats {
        response_tx: oneshot::Sender<PdfRenderStats>,
    },
    Shutdown,
}

struct RenderRequest {
    path: String,
    page_index: usize,
    width: u32,
    target_path: PathBuf,
    response_tx: oneshot::Sender<std::io::Result<()>>,
}

struct ByteRenderRequest {
    path: String,
    page_index: usize,
    width: u32,
    response_tx: oneshot::Sender<std::io::Result<Vec<u8>>>,
}

#[derive(Default)]
struct DocumentCache {
    documents: HashMap<String, CachedDocument>,
}

struct CachedDocument {
    document: Document,
    last_used_at: Instant,
}

impl DocumentCache {
    const IDLE_TTL: Duration = Duration::from_secs(90);

    fn get_or_open(&mut self, path: &str) -> std::io::Result<&Document> {
        self.evict_expired();

        if let Some(entry) = self.documents.get_mut(path) {
            entry.last_used_at = Instant::now();
        } else {
            let document = Document::open(path).map_err(mupdf_to_io_error)?;
            self.documents.insert(
                path.to_string(),
                CachedDocument {
                    document,
                    last_used_at: Instant::now(),
                },
            );
        }

        self.documents
            .get(path)
            .map(|entry| &entry.document)
            .ok_or_else(|| std::io::Error::other("document cache lost newly inserted entry"))
    }

    fn evict_expired(&mut self) {
        let now = Instant::now();
        self.documents
            .retain(|_, entry| now.duration_since(entry.last_used_at) < Self::IDLE_TTL);
    }

    fn stats(&self) -> PdfRenderStats {
        PdfRenderStats {
            open_documents: self.documents.len(),
            idle_ttl_secs: Self::IDLE_TTL.as_secs(),
        }
    }
}

fn pdf_render_worker_loop(receiver: mpsc::Receiver<RenderCommand>) {
    let mut cache = DocumentCache::default();

    loop {
        match receiver.recv_timeout(Duration::from_secs(15)) {
            Ok(command) => match command {
                RenderCommand::Render(request) => {
                    let result = render_pdf_page_svg(
                        &mut cache,
                        &request.path,
                        request.page_index,
                        request.width,
                        &request.target_path,
                    );
                    let _ = request.response_tx.send(result);
                }
                RenderCommand::RenderCoverJpeg(request) => {
                    let result = render_pdf_cover_jpeg_direct(
                        &request.path,
                        request.width,
                        &request.target_path,
                    );
                    let _ = request.response_tx.send(result);
                }
                RenderCommand::RenderSvgBytes(request) => {
                    let result = render_pdf_page_svg_bytes(
                        &mut cache,
                        &request.path,
                        request.page_index,
                        request.width,
                    );
                    let _ = request.response_tx.send(result);
                }
                RenderCommand::Stats { response_tx } => {
                    let _ = response_tx.send(cache.stats());
                }
                RenderCommand::Shutdown => break,
            },
            Err(mpsc::RecvTimeoutError::Timeout) => cache.evict_expired(),
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn render_pdf_page_svg(
    cache: &mut DocumentCache,
    path: &str,
    page_index: usize,
    width: u32,
    target_path: &Path,
) -> std::io::Result<()> {
    if target_path.exists() {
        return Ok(());
    }

    let document = cache.get_or_open(path)?;
    let page_count: usize = document
        .page_count()
        .map_err(mupdf_to_io_error)?
        .try_into()
        .map_err(|_| std::io::Error::other("invalid page count"))?;

    if page_index >= page_count {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("page {} out of range for {}", page_index + 1, path),
        ));
    }

    let page = document
        .load_page(page_index as i32)
        .map_err(mupdf_to_io_error)?;
    let bounds = page.bounds().map_err(mupdf_to_io_error)?;
    let raw_width = (bounds.x1 - bounds.x0).abs().max(1.0);
    let scale = if width == 0 {
        1.0
    } else {
        width as f32 / raw_width
    };
    let matrix = Matrix::new_scale(scale, scale);
    let display_list = page.to_display_list(true).map_err(mupdf_to_io_error)?;
    let svg = display_list.to_svg(&matrix).map_err(mupdf_to_io_error)?;

    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(target_path, svg)
}

fn render_pdf_page_svg_bytes(
    cache: &mut DocumentCache,
    path: &str,
    page_index: usize,
    width: u32,
) -> std::io::Result<Vec<u8>> {
    let document = cache.get_or_open(path)?;
    let page_count: usize = document
        .page_count()
        .map_err(mupdf_to_io_error)?
        .try_into()
        .map_err(|_| std::io::Error::other("invalid page count"))?;

    if page_index >= page_count {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("page {} out of range for {}", page_index + 1, path),
        ));
    }

    let page = document
        .load_page(page_index as i32)
        .map_err(mupdf_to_io_error)?;
    let bounds = page.bounds().map_err(mupdf_to_io_error)?;
    let raw_width = (bounds.x1 - bounds.x0).abs().max(1.0);
    let scale = if width == 0 {
        1.0
    } else {
        width as f32 / raw_width
    };
    let matrix = Matrix::new_scale(scale, scale);
    let display_list = page.to_display_list(true).map_err(mupdf_to_io_error)?;
    let svg = display_list.to_svg(&matrix).map_err(mupdf_to_io_error)?;
    Ok(svg.into_bytes())
}

fn render_pdf_cover_jpeg_direct(path: &str, width: u32, target_path: &Path) -> std::io::Result<()> {
    if target_path.exists() {
        return Ok(());
    }

    let document = Document::open(path).map_err(mupdf_to_io_error)?;
    let page = document.load_page(0).map_err(mupdf_to_io_error)?;
    let bounds = page.bounds().map_err(mupdf_to_io_error)?;
    let raw_width = (bounds.x1 - bounds.x0).abs().max(1.0);
    let scale = if width == 0 {
        1.0
    } else {
        width as f32 / raw_width
    };
    let matrix = Matrix::new_scale(scale, scale);
    let colorspace = Colorspace::device_rgb();
    let pixmap = page
        .to_pixmap(&matrix, &colorspace, false, false)
        .map_err(mupdf_to_io_error)?;

    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(target_path)?;
    let writer = BufWriter::new(file);
    let mut encoder = JpegEncoder::new_with_quality(writer, 85);
    if pixmap.n() != 3 {
        return Err(std::io::Error::other(format!(
            "unsupported pixmap channel count: {}",
            pixmap.n()
        )));
    }
    encoder
        .encode(
            pixmap.samples(),
            pixmap.width(),
            pixmap.height(),
            ExtendedColorType::Rgb8,
        )
        .map_err(|err| std::io::Error::other(err.to_string()))
}

fn mupdf_to_io_error(err: mupdf::Error) -> std::io::Error {
    std::io::Error::other(err.to_string())
}
