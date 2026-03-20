//! 错误类型定义模块
//!
//! 定义应用程序中使用的错误类型，实现统一的错误处理和响应格式。

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use sea_orm::DbErr;
use serde::Serialize;
use thiserror::Error;

/// 应用程序错误类型
///
/// 用于表示应用程序中可能出现的各种错误情况。
/// 每个错误变体都包含一个描述性消息，便于调试和问题排查。
#[derive(Debug, Error)]
pub enum AppError {
    /// 资源未找到错误
    #[error("资源未找到: {0}")]
    NotFound(String),

    /// 请求参数错误
    #[error("请求参数错误: {0}")]
    BadRequest(String),

    /// 未经授权
    #[error("未经授权: {0}")]
    Unauthorized(String),

    /// 服务器内部错误
    #[error("内部服务器错误: {0}")]
    InternalServerError(String),

    /// 资源冲突
    #[error("资源冲突: {0}")]
    Conflict(String),

    /// IO 错误
    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),

    /// 数据库错误
    #[error("数据库错误: {0}")]
    DatabaseError(#[from] DbErr),

    /// HTTP 错误
    #[error("HTTP 错误: {0}")]
    HttpError(#[from] axum::http::Error),
}

/// 配置错误类型
#[derive(Debug, Error)]
pub enum ConfigError {
    /// 无效的扫描路径
    #[error("无效的扫描路径: {0}")]
    InvalidScanPath(String),
}

/// 错误响应结构
#[derive(Serialize)]
pub struct ErrorResponse {
    /// 错误码
    pub code: i32,
    /// 错误消息
    pub message: String,
}

impl AppError {
    /// 获取错误对应的 HTTP 状态码
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::IoError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::HttpError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// 获取业务错误码
    pub fn error_code(&self) -> i32 {
        match self {
            AppError::NotFound(_) => 404,
            AppError::BadRequest(_) => 400,
            AppError::Unauthorized(_) => 401,
            AppError::InternalServerError(_) => 500,
            AppError::Conflict(_) => 409,
            AppError::IoError(_) => 500,
            AppError::DatabaseError(_) => 500,
            AppError::HttpError(_) => 500,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!("request failed: {}", self);
        let status = self.status_code();
        let body = Json(ErrorResponse {
            code: self.error_code(),
            message: self.to_string(),
        });

        (status, body).into_response()
    }
}
