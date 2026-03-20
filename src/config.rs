use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub app_name: String,
    pub version: String,
    pub host: String,
    pub port: u16,
    pub log_level: String,
    pub database_url: String,
    pub scanner: ScannerConfig,
    pub cache: CacheConfig,
    pub internal: InternalConfig,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Self {
        let mut config = Self::from_file(path);
        config.apply_env_overrides();
        config
    }

    pub fn from_env() -> Self {
        Self {
            app_name: env::var("APP_NAME").unwrap_or_else(|_| "Auxm API".to_string()),
            version: env::var("APP_VERSION").unwrap_or_else(|_| "1.0.0".to_string()),
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3001),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
            database_url: crate::runtime::default_database_url(),
            scanner: ScannerConfig::default(),
            cache: CacheConfig::default(),
            internal: InternalConfig::default(),
        }
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        match fs::read_to_string(path) {
            Ok(content) => match toml::from_str::<ConfigFile>(&content) {
                Ok(config) => {
                    let mut scanner = config.scanner.unwrap_or_default();
                    scanner.validate_scan_paths();
                    return Config {
                        app_name: config.server.app_name,
                        version: config.server.version,
                        host: config.server.host,
                        port: config.server.port,
                        log_level: config.server.log_level,
                        database_url: config.database.url,
                        scanner,
                        cache: config.cache.unwrap_or_default(),
                        internal: config.internal.unwrap_or_default(),
                    };
                }
                Err(e) => {
                    tracing::warn!("Failed to parse config file: {}, using defaults", e);
                }
            },
            Err(e) => {
                tracing::warn!("Failed to read config file: {}, using defaults", e);
            }
        }
        tracing::info!("Using default configuration");
        Config::default()
    }

    pub fn scanner_config(&self) -> ScannerConfig {
        self.scanner.clone()
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let config_file = ConfigFile {
            server: ServerConfig {
                app_name: self.app_name.clone(),
                version: self.version.clone(),
                host: self.host.clone(),
                port: self.port,
                log_level: self.log_level.clone(),
            },
            database: DatabaseConfig {
                url: self.database_url.clone(),
            },
            scanner: Some(self.scanner.clone()),
            cache: Some(self.cache.clone()),
            internal: Some(self.internal.clone()),
        };
        let content = toml::to_string_pretty(&config_file)
            .map_err(|err| std::io::Error::other(err.to_string()))?;
        fs::write(path, content)
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(value) = env::var("APP_NAME") {
            self.app_name = value;
        }
        if let Ok(value) = env::var("APP_VERSION") {
            self.version = value;
        }
        if let Ok(value) = env::var("HOST") {
            self.host = value;
        }
        if let Ok(value) = env::var("PORT") {
            if let Ok(port) = value.parse() {
                self.port = port;
            }
        }
        if let Ok(value) = env::var("LOG_LEVEL") {
            self.log_level = value;
        }
        if let Ok(value) = env::var("DATABASE_URL") {
            self.database_url = value;
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app_name: "Auxm API".to_string(),
            version: "1.0.0".to_string(),
            host: "0.0.0.0".to_string(),
            port: 3001,
            log_level: "info".to_string(),
            database_url: crate::runtime::default_database_url(),
            scanner: ScannerConfig::default(),
            cache: CacheConfig::default(),
            internal: InternalConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScannerConfig {
    pub scan_paths: Vec<String>,
    pub image_extensions: Vec<String>,
    pub min_image_count: usize,
}

impl ScannerConfig {
    pub fn validate_scan_paths(&mut self) {
        use std::path::Path;

        self.scan_paths.retain(|path| {
            let p = Path::new(path);
            p.exists() && p.is_dir()
        });

        let mut to_remove = Vec::new();

        for (i, path1) in self.scan_paths.iter().enumerate() {
            for path2 in self.scan_paths.iter().skip(i + 1) {
                let p1 = Path::new(path1);
                let p2 = Path::new(path2);

                if p2.starts_with(p1) {
                    to_remove.push(path2.clone());
                } else if p1.starts_with(p2) {
                    to_remove.push(path1.clone());
                }
            }
        }

        self.scan_paths.retain(|p| !to_remove.contains(p));
    }
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            scan_paths: vec![],
            image_extensions: vec![
                "jpg".to_string(),
                "jpeg".to_string(),
                "png".to_string(),
                "webp".to_string(),
                "gif".to_string(),
            ],
            min_image_count: 3,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheConfig {
    pub cover_width: u32,
    pub image_page_preview_width: u32,
    pub oversized_image_avg_pixels: u64,
    pub pdf_svg_width: u32,
    pub max_render_jobs: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cover_width: 480,
            image_page_preview_width: 1600,
            oversized_image_avg_pixels: 10_000_000,
            pdf_svg_width: 1400,
            max_render_jobs: 4,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InternalConfig {
    pub http_concurrency_limit: usize,
    pub database_max_connections: u32,
    pub database_min_connections: u32,
    pub file_io_concurrency: usize,
}

impl Default for InternalConfig {
    fn default() -> Self {
        Self {
            http_concurrency_limit: 128,
            database_max_connections: 16,
            database_min_connections: 1,
            file_io_concurrency: 32,
        }
    }
}

#[derive(Deserialize, Serialize)]
struct ConfigFile {
    server: ServerConfig,
    database: DatabaseConfig,
    scanner: Option<ScannerConfig>,
    cache: Option<CacheConfig>,
    internal: Option<InternalConfig>,
}

#[derive(Deserialize, Serialize)]
struct ServerConfig {
    app_name: String,
    version: String,
    host: String,
    port: u16,
    log_level: String,
}

#[derive(Deserialize, Serialize)]
struct DatabaseConfig {
    url: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_validate_scan_paths_empty() {
        let mut config = ScannerConfig {
            scan_paths: vec![],
            image_extensions: vec![],
            min_image_count: 3,
        };
        config.validate_scan_paths();
        assert!(config.scan_paths.is_empty());
    }

    #[test]
    fn test_validate_scan_paths_nonexistent() {
        let mut config = ScannerConfig {
            scan_paths: vec!["/nonexistent/path/abc123".to_string()],
            image_extensions: vec![],
            min_image_count: 3,
        };
        config.validate_scan_paths();
        assert!(config.scan_paths.is_empty());
    }

    #[test]
    fn test_validate_scan_paths_valid() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let path = temp_dir.path().to_string_lossy().to_string();

        let mut config = ScannerConfig {
            scan_paths: vec![path],
            image_extensions: vec![],
            min_image_count: 3,
        };
        config.validate_scan_paths();
        assert_eq!(config.scan_paths.len(), 1);
        Ok(())
    }

    #[test]
    fn test_validate_scan_paths_subpath() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let parent = temp_dir.path().to_string_lossy().to_string();
        let child = format!("{}/subdir", parent);

        fs::create_dir_all(&child)?;

        let mut config = ScannerConfig {
            scan_paths: vec![parent.clone(), child],
            image_extensions: vec![],
            min_image_count: 3,
        };
        config.validate_scan_paths();
        assert_eq!(config.scan_paths.len(), 1);
        assert!(config.scan_paths.contains(&parent));
        Ok(())
    }

    #[test]
    fn test_validate_scan_paths_multiple_valid() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir1 = tempfile::tempdir()?;
        let temp_dir2 = tempfile::tempdir()?;

        let mut config = ScannerConfig {
            scan_paths: vec![
                temp_dir1.path().to_string_lossy().to_string(),
                temp_dir2.path().to_string_lossy().to_string(),
            ],
            image_extensions: vec![],
            min_image_count: 3,
        };
        config.validate_scan_paths();
        assert_eq!(config.scan_paths.len(), 2);
        Ok(())
    }
}
