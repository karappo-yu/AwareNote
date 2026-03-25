//! Scanner 扫描引擎模块
//!
//! 提供文件系统扫描、增量比对功能。

pub mod engine;
pub mod pdf;
pub mod strategy;
pub mod types;

pub use engine::Scanner;
pub use strategy::ConfigurableRecognizer;
pub use types::{CachedBookMetadata, ScanResult};
