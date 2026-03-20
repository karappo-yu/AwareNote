#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Auxm - 图书管理系统 API 框架
//!
//! 这是一个基于 Axum 框架构建的图书管理系统后端 API，提供了书籍管理的 RESTful 接口。
//!
//! # 主要特性
//!
//! - **Web 框架**: 基于 Axum 0.7
//! - **配置管理**: 支持 TOML 配置文件和环境变量
//! - **日志系统**: 使用 Tracing 进行结构化日志记录
//! - **API 文档**: 集成 Swagger UI 和 OpenAPI 规范
//! - **中间件**: 支持请求日志
//!
//! # 项目结构
//!
//! - `config` - 配置管理模块
//! - `error` - 错误类型定义
//! - `middleware` - 中间件（请求日志）
//! - `routes` - 路由定义
//! - `handlers` - 请求处理器
//! - `domain` - 领域模型（实体、请求/响应 DTO）
//!
//! # 快速开始
//!
//! ```bash
//! # 运行服务
//! cargo run
//!
//! # 访问 API 文档
//! http://localhost:3001/swagger-ui
//! ```
//!
//! # 配置
//!
//! 配置文件位于 `app_config.toml`，支持以下配置项：
//!
//! - `server.app_name` - 应用名称
//! - `server.version` - 版本号
//! - `server.host` - 监听地址
//! - `server.port` - 监听端口
//! - `server.log_level` - 日志级别
//! - `database.url` - 数据库连接地址

pub mod config;
pub mod error;
pub mod middleware;
pub mod routes;
pub mod runtime;

pub mod domain;
pub mod handlers;
pub mod scanner;
pub mod service;

pub use crate::config::Config;
pub use crate::error::{AppError, ConfigError};
pub use crate::service::assets::AssetCacheService;
pub use crate::service::database::DatabaseService;
pub use axum;

/// 应用程序全局状态
///
/// 包含所有请求处理器需要访问的共享状态，如数据库服务、缓存等。
/// 在应用启动时创建，并传递给所有请求处理器。
#[derive(Clone)]
pub struct AppState {
    /// 数据库服务
    pub db_service: std::sync::Arc<crate::service::database::DatabaseService>,
    /// 资源缓存服务
    pub asset_cache: std::sync::Arc<crate::service::assets::AssetCacheService>,
}
