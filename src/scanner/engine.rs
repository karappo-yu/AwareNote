//! Scanner 扫描引擎模块
//!
//! 实现文件系统的递归扫描功能。

use super::strategy::ConfigurableRecognizer;
use super::types::{ScanResult, ScannedBookFile, ScannedCategory};
use std::fs;
use std::path::Path;
use std::time::SystemTime;

pub struct Scanner<'a> {
    recognizer: &'a ConfigurableRecognizer,
}

impl<'a> Scanner<'a> {
    pub fn new(recognizer: &'a ConfigurableRecognizer) -> Self {
        Self { recognizer }
    }

    pub fn scan(&self, root: &Path) -> ScanResult {
        let mut book_files = Vec::new();
        let mut categories = Vec::new();

        let meta = match fs::metadata(root) {
            Ok(m) => m,
            Err(_) => {
                return ScanResult {
                    book_files,
                    categories,
                };
            }
        };

        if meta.is_dir() {
            let inspection = self.recognizer.inspect_directory(root, &meta);
            categories.push(
                inspection
                    .category
                    .unwrap_or_else(|| build_root_category(root, &meta)),
            );
            if let Some(book) = inspection.book {
                book_files.push(book);
            }
            if inspection.recurse {
                fs::read_dir(root)
                    .ok()
                    .map(|entries| entries.flatten())
                    .into_iter()
                    .flatten()
                    .for_each(|entry| {
                        self.scan_dir(&entry.path(), &mut book_files, &mut categories)
                    });
            }
        } else {
            self.scan_dir(root, &mut book_files, &mut categories);
        }

        ScanResult {
            book_files,
            categories,
        }
    }

    fn scan_dir(
        &self,
        path: &Path,
        book_files: &mut Vec<ScannedBookFile>,
        categories: &mut Vec<ScannedCategory>,
    ) {
        let meta = match fs::metadata(path) {
            Ok(m) => m,
            Err(_) => return,
        };

        if meta.is_dir() {
            if self.recognizer.is_hidden(path) {
                return;
            }
            let inspection = self.recognizer.inspect_directory(path, &meta);
            if let Some(category) = inspection.category {
                categories.push(category);
            }
            if let Some(book) = inspection.book {
                book_files.push(book);
            }
            if inspection.recurse {
                fs::read_dir(path)
                    .ok()
                    .map(|entries| entries.flatten())
                    .into_iter()
                    .flatten()
                    .for_each(|entry| self.scan_dir(&entry.path(), book_files, categories));
            }
            return;
        }

        if let Some(book) = self.recognizer.analyze_file(path, &meta) {
            book_files.push(book);
        }
    }
}

fn build_root_category(path: &Path, metadata: &std::fs::Metadata) -> ScannedCategory {
    let name = path
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
        name,
        path: path.to_string_lossy().to_string(),
        mtime,
    }
}
