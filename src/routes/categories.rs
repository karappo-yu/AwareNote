use axum::{routing::get, Router};

use crate::AppState;

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/categories",
            get(crate::handlers::categories::list_categories),
        )
        .route(
            "/api/categories/:id/books",
            get(crate::handlers::categories::list_category_books),
        )
        .with_state(state)
}
