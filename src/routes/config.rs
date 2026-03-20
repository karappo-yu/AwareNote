use axum::{
    routing::{delete, get},
    Router,
};

use crate::AppState;

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/config",
            get(crate::handlers::config::get_config).put(crate::handlers::config::update_config),
        )
        .route(
            "/api/root_path",
            get(crate::handlers::config::get_root_path),
        )
        .route(
            "/api/cache/clear",
            delete(crate::handlers::config::clear_cache),
        )
        .route(
            "/api/cache/clear/:target",
            delete(crate::handlers::config::clear_cache_target),
        )
        .with_state(state)
}
