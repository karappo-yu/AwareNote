use std::path::PathBuf;

pub const CONFIG_PATH_ENV: &str = "AUXM_CONFIG_PATH";
pub const CACHE_DIR_ENV: &str = "AUXM_CACHE_DIR";

pub fn config_path() -> PathBuf {
    std::env::var_os(CONFIG_PATH_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("app_config.toml"))
}

pub fn default_database_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:auxm.db".to_string())
}

pub fn app_cache_root() -> std::io::Result<PathBuf> {
    if let Some(path) = std::env::var_os(CACHE_DIR_ENV) {
        return Ok(PathBuf::from(path));
    }

    let base = if cfg!(target_os = "macos") {
        home_dir()?.join("Library").join("Caches")
    } else if cfg!(target_os = "windows") {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| std::io::Error::other("LOCALAPPDATA is not set"))?
    } else if let Some(path) = std::env::var_os("XDG_CACHE_HOME") {
        PathBuf::from(path)
    } else {
        home_dir()?.join(".cache")
    };

    Ok(base.join("auxm"))
}

fn home_dir() -> std::io::Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| std::io::Error::other("HOME is not set"))
}
