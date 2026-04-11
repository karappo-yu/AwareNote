//! 用户数据模块
//!
//! 提供全局设置和 per-book 设置的 API。

use axum::{
    extract::{Path, State},
    response::Json,
};
use serde::{Deserialize, Serialize};

use crate::domain::user_data;
use crate::{AppError, AppState};

// ============== Global Settings ==============

#[derive(Serialize)]
pub struct SettingsResponse {
    pub settings: Vec<UserDataItem>,
}

#[derive(Serialize, Deserialize)]
pub struct UserDataItem {
    pub key: String,
    pub value: String,
}

impl From<user_data::Model> for UserDataItem {
    fn from(m: user_data::Model) -> Self {
        UserDataItem {
            key: m.key,
            value: m.value,
        }
    }
}

/// GET /api/settings — 获取全局设置
pub async fn get_settings(
    State(state): State<AppState>,
) -> Result<Json<SettingsResponse>, AppError> {
    let items = state.db_service.get_user_data("").await?;
    Ok(Json(SettingsResponse {
        settings: items.into_iter().map(UserDataItem::from).collect(),
    }))
}

#[derive(Deserialize)]
pub struct UpdateSettingsRequest {
    pub settings: Vec<UserDataItem>,
}

#[derive(Serialize)]
pub struct UpdateSettingsResponse {
    pub success: bool,
}

/// PUT /api/settings — 更新全局设置
pub async fn update_settings(
    State(state): State<AppState>,
    Json(body): Json<UpdateSettingsRequest>,
) -> Result<Json<UpdateSettingsResponse>, AppError> {
    let items: Vec<(String, String)> = body
        .settings
        .into_iter()
        .map(|item| (item.key, item.value))
        .collect();
    state.db_service.set_user_data_batch("", &items).await?;
    Ok(Json(UpdateSettingsResponse { success: true }))
}

// ============== Per-Book Settings ==============

/// GET /api/books/:id/settings — 获取某本书的用户数据
pub async fn get_book_settings(
    State(state): State<AppState>,
    Path(book_id): Path<String>,
) -> Result<Json<SettingsResponse>, AppError> {
    let items = state.db_service.get_user_data(&book_id).await?;
    Ok(Json(SettingsResponse {
        settings: items.into_iter().map(UserDataItem::from).collect(),
    }))
}

#[derive(Deserialize)]
pub struct UpdateBookSettingsRequest {
    pub settings: Vec<UserDataItem>,
}

#[derive(Serialize)]
pub struct UpdateBookSettingsResponse {
    pub success: bool,
}

/// PUT /api/books/:id/settings — 更新某本书的用户数据
pub async fn update_book_settings(
    State(state): State<AppState>,
    Path(book_id): Path<String>,
    Json(body): Json<UpdateBookSettingsRequest>,
) -> Result<Json<UpdateBookSettingsResponse>, AppError> {
    let items: Vec<(String, String)> = body
        .settings
        .into_iter()
        .map(|item| (item.key, item.value))
        .collect();
    state
        .db_service
        .set_user_data_batch(&book_id, &items)
        .await?;
    Ok(Json(UpdateBookSettingsResponse { success: true }))
}
