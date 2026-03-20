use axum::{
    extract::{Path, State},
    response::Json,
};

use crate::domain::book_files;
use crate::service::database::CategoryNode;
use crate::{AppError, AppState};
use serde::Serialize;

#[derive(Serialize)]
pub struct CategoryBookResponse {
    pub id: String,
    pub path: String,
    pub title: Option<String>,
    pub kind: String,
    #[serde(rename = "type")]
    pub book_type: String,
    pub size: i64,
    pub mtime: i64,
    pub page_count: i64,
    pub is_favorite: bool,
    pub cover_path: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Serialize)]
pub struct CategoryNodeResponse {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub book_count: usize,
    pub total_book_count: usize,
    pub sub_categories: Vec<CategoryNodeResponse>,
}

pub async fn list_categories(
    State(state): State<AppState>,
) -> Result<Json<Vec<CategoryNodeResponse>>, AppError> {
    let categories = state.db_service.list_categories_tree().await?;
    Ok(Json(
        categories
            .into_iter()
            .map(to_category_node_response)
            .collect(),
    ))
}

pub async fn list_category_books(
    State(state): State<AppState>,
    Path(category_id): Path<i64>,
) -> Result<Json<Vec<CategoryBookResponse>>, AppError> {
    let books = state.db_service.list_books_by_category(category_id).await?;
    Ok(Json(
        books
            .into_iter()
            .map(|book| to_book_response(&book))
            .collect(),
    ))
}

fn to_category_node_response(node: CategoryNode) -> CategoryNodeResponse {
    let sub_categories: Vec<CategoryNodeResponse> = node
        .sub_categories
        .into_iter()
        .map(to_category_node_response)
        .collect();
    let book_count = node.book_count;
    let total_book_count = book_count
        + sub_categories
            .iter()
            .map(|sub| sub.total_book_count)
            .sum::<usize>();

    CategoryNodeResponse {
        id: node.id,
        name: node.name,
        path: node.path,
        book_count,
        total_book_count,
        sub_categories,
    }
}

fn to_book_response(book: &book_files::Model) -> CategoryBookResponse {
    CategoryBookResponse {
        id: book.id.to_string(),
        path: book.path.clone(),
        title: book.title.clone(),
        kind: book.kind.clone(),
        book_type: if book.kind == "pdf" {
            "pdf_book".to_string()
        } else {
            "image_book".to_string()
        },
        size: book.size,
        mtime: book.mtime,
        page_count: book.page_count,
        is_favorite: book.is_favorite,
        cover_path: book.cover_path.clone(),
        created_at: book.created_at.clone(),
    }
}
