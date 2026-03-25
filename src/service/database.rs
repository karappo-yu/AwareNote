//! 数据库服务模块
//!
//! 提供数据库操作的服务，包括本地文件和数据库的同步。

//! データベースサービスモジュール
//!
//! ローカルファイルとデータベースの同期を含む、データベース操作サービスを提供します。

use crate::config::{CacheConfig, ScannerConfig};
use crate::domain::{book_files, categories, libraries};
use crate::scanner::{CachedBookMetadata, ConfigurableRecognizer, ScanResult, Scanner};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectOptions, ConnectionTrait, DatabaseConnection, DbErr,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Statement, TransactionTrait,
};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// データベースデータセット
///
/// データベースから照会されたすべてのデータを含みます。
pub struct DatabaseData {
    pub libraries: Vec<libraries::Model>,
    pub categories: Vec<categories::Model>,
    pub book_files: Vec<book_files::Model>,
}

#[derive(Debug, Clone)]
pub struct DatabaseSnapshot {
    pub libraries: Vec<libraries::Model>,
    pub categories: Vec<categories::Model>,
    pub book_files: Vec<book_files::Model>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CategoryNode {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub book_count: usize,
    pub sub_categories: Vec<CategoryNode>,
}

/// 増分比較結果
///
/// 比較後の追加、削除、変更データを含みます。
pub struct DiffData {
    pub new_library_paths: Vec<String>,
    pub deleted_library_ids: Vec<i64>,
    pub new_category_paths: Vec<String>,
    pub deleted_category_ids: Vec<i64>,
    pub new_book_file_paths: Vec<String>,
    pub deleted_book_file_ids: Vec<i64>,
    pub updated_book_files: Vec<book_files::Model>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LibraryChangeDetail {
    pub path: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CategoryChangeDetail {
    pub path: String,
    pub name: String,
    pub library_path: Option<String>,
    pub parent_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BookChangeDetail {
    pub path: String,
    pub title: String,
    pub category_path: Option<String>,
}

/// 同期レポート
///
/// 同期操作の結果統計を含みます。
#[derive(Debug, Clone, Serialize)]
pub struct SyncReport {
    pub inserted_libraries: usize,
    pub deleted_libraries: usize,
    pub inserted_categories: usize,
    pub deleted_categories: usize,
    pub inserted_book_files: usize,
    pub deleted_book_files: usize,
    pub updated_book_files: usize,
    pub inserted_library_details: Vec<LibraryChangeDetail>,
    pub deleted_library_details: Vec<LibraryChangeDetail>,
    pub inserted_category_details: Vec<CategoryChangeDetail>,
    pub deleted_category_details: Vec<CategoryChangeDetail>,
    pub inserted_book_file_details: Vec<BookChangeDetail>,
    pub deleted_book_file_details: Vec<BookChangeDetail>,
    pub updated_book_file_details: Vec<BookChangeDetail>,
}

/// データベースサービス
///
/// データベース接続管理と、ローカルファイルとデータベース間の同期を担当します。
#[derive(Clone)]
pub struct DatabaseService {
    db: DatabaseConnection,
    persistent_path: String,
    scanner_config: Arc<RwLock<ScannerConfig>>,
    cache_config: Arc<RwLock<CacheConfig>>,
}

impl DatabaseService {
    pub async fn new(config: &crate::config::Config) -> Result<Self, DbErr> {
        let persistent_path = normalize_sqlite_path(&config.database_url);
        ensure_database_file(&persistent_path)?;

        let file_url = format!("sqlite:{}", persistent_path);
        let mut options = ConnectOptions::new(file_url);
        options
            .max_connections(config.internal.database_max_connections.max(1))
            .min_connections(
                config
                    .internal
                    .database_min_connections
                    .min(config.internal.database_max_connections.max(1)),
            )
            .connect_timeout(Duration::from_secs(5))
            .sqlx_logging(false);
        let conn = sea_orm::Database::connect(options).await?;

        Self::init_schema(&conn).await?;
        Self::ensure_schema_columns(&conn).await?;

        Ok(Self {
            db: conn,
            persistent_path,
            scanner_config: Arc::new(RwLock::new(config.scanner.clone())),
            cache_config: Arc::new(RwLock::new(config.cache.clone())),
        })
    }

    /// データベースファイルのパスを取得する
    pub fn get_database_path(&self) -> &str {
        &self.persistent_path
    }

    pub fn storage_mode(&self) -> &'static str {
        "sqlite-file"
    }

    pub fn get_scanner_config(&self) -> ScannerConfig {
        match self.scanner_config.read() {
            Ok(guard) => guard.clone(),
            Err(err) => {
                tracing::warn!("scanner config lock poisoned, using inner value: {}", err);
                err.into_inner().clone()
            }
        }
    }

    pub fn update_scanner_config(&self, scanner_config: ScannerConfig) {
        match self.scanner_config.write() {
            Ok(mut guard) => *guard = scanner_config,
            Err(err) => {
                tracing::warn!(
                    "scanner config lock poisoned, updating inner value: {}",
                    err
                );
                *err.into_inner() = scanner_config;
            }
        }
    }

    pub fn get_cache_config(&self) -> CacheConfig {
        match self.cache_config.read() {
            Ok(guard) => guard.clone(),
            Err(err) => {
                tracing::warn!("cache config lock poisoned, using inner value: {}", err);
                err.into_inner().clone()
            }
        }
    }

    pub fn update_cache_config(&self, cache_config: CacheConfig) {
        match self.cache_config.write() {
            Ok(mut guard) => *guard = cache_config,
            Err(err) => {
                tracing::warn!("cache config lock poisoned, updating inner value: {}", err);
                *err.into_inner() = cache_config;
            }
        }
    }

    async fn init_schema(db: &DatabaseConnection) -> Result<(), DbErr> {
        use crate::service::SCHEMA_SQL;

        let statements: Vec<&str> = SCHEMA_SQL
            .split(';')
            .filter(|s| !s.trim().is_empty())
            .collect();
        for stmt in statements {
            if !stmt.trim().is_empty() {
                db.execute_unprepared(stmt).await?;
            }
        }
        Ok(())
    }

    async fn ensure_schema_columns(db: &DatabaseConnection) -> Result<(), DbErr> {
        let rows = db
            .query_all(Statement::from_string(
                db.get_database_backend(),
                "PRAGMA table_info(book_files)",
            ))
            .await?;
        let columns: std::collections::HashSet<String> = rows
            .iter()
            .filter_map(|row| row.try_get::<String>("", "name").ok())
            .collect();

        let alters = [
            (
                "is_oversized",
                "ALTER TABLE book_files ADD COLUMN is_oversized INTEGER NOT NULL DEFAULT 0",
            ),
            (
                "content_signature",
                "ALTER TABLE book_files ADD COLUMN content_signature TEXT",
            ),
            (
                "avg_page_pixels",
                "ALTER TABLE book_files ADD COLUMN avg_page_pixels INTEGER NOT NULL DEFAULT 0",
            ),
        ];

        for (column, sql) in alters {
            if !columns.contains(column) {
                db.execute_unprepared(sql).await?;
            }
        }

        Ok(())
    }

    /// データベースからすべてのデータを取得する
    pub async fn get_all(&self) -> Result<DatabaseData, DbErr> {
        let libraries = libraries::Entity::find().all(&self.db).await?;
        let categories = categories::Entity::find().all(&self.db).await?;
        let book_files = book_files::Entity::find().all(&self.db).await?;

        Ok(DatabaseData {
            libraries,
            categories,
            book_files,
        })
    }

    /// データベースのスナップショットを取得する
    pub async fn get_snapshot(&self) -> Result<DatabaseSnapshot, DbErr> {
        let data = self.get_all().await?;
        Ok(DatabaseSnapshot {
            libraries: data.libraries,
            categories: data.categories,
            book_files: data.book_files,
        })
    }

    pub async fn get_book(&self, book_id: i64) -> Result<Option<book_files::Model>, DbErr> {
        book_files::Entity::find_by_id(book_id).one(&self.db).await
    }

    pub async fn list_favorite_books(
        &self,
        page: usize,
        page_size: usize,
    ) -> Result<(Vec<book_files::Model>, usize), DbErr> {
        let query = book_files::Entity::find()
            .filter(book_files::Column::IsFavorite.eq(true))
            .order_by_desc(book_files::Column::CreatedAt);
        let paginator = query.paginate(&self.db, page_size as u64);
        let total = paginator.num_items().await? as usize;
        let books = paginator.fetch_page(page.saturating_sub(1) as u64).await?;
        Ok((books, total))
    }

    pub async fn list_all_favorite_books(&self) -> Result<Vec<book_files::Model>, DbErr> {
        book_files::Entity::find()
            .filter(book_files::Column::IsFavorite.eq(true))
            .order_by_desc(book_files::Column::CreatedAt)
            .all(&self.db)
            .await
    }

    pub async fn set_book_favorite(&self, book_id: i64, is_favorite: bool) -> Result<bool, DbErr> {
        let Some(book) = self.get_book(book_id).await? else {
            return Ok(false);
        };

        let active_model = book_files::ActiveModel {
            id: sea_orm::Set(book.id),
            category_id: sea_orm::Set(book.category_id),
            path: sea_orm::Set(book.path),
            title: sea_orm::Set(book.title),
            kind: sea_orm::Set(book.kind),
            size: sea_orm::Set(book.size),
            mtime: sea_orm::Set(book.mtime),
            page_count: sea_orm::Set(book.page_count),
            pages_json: sea_orm::Set(book.pages_json),
            content_signature: sea_orm::Set(book.content_signature),
            is_oversized: sea_orm::Set(book.is_oversized),
            avg_page_pixels: sea_orm::Set(book.avg_page_pixels),
            is_favorite: sea_orm::Set(is_favorite),
            cover_path: sea_orm::Set(book.cover_path),
            created_at: sea_orm::Set(book.created_at),
        };
        active_model.update(&self.db).await?;
        Ok(true)
    }

    pub async fn list_categories_tree(&self) -> Result<Vec<CategoryNode>, DbErr> {
        let snapshot = self.get_snapshot().await?;
        Ok(build_category_tree(
            &snapshot.categories,
            &snapshot.book_files,
        ))
    }

    pub async fn list_books_by_category(
        &self,
        category_id: i64,
    ) -> Result<Vec<book_files::Model>, DbErr> {
        let snapshot = self.get_snapshot().await?;
        let category_ids = collect_descendant_category_ids(category_id, &snapshot.categories);
        let books = snapshot
            .book_files
            .into_iter()
            .filter(|book| category_ids.contains(&book.category_id))
            .collect();
        Ok(books)
    }

    /// スキャンを実行してキャッシュを更新する
    pub async fn scan_and_refresh(&self) -> Result<SyncReport, DbErr> {
        let db_data = self.get_all().await?;
        let scan_data = self.scan_all_with_existing(&db_data.book_files).await;
        self.sync(&db_data, &scan_data).await
    }

    /// 从内存数据库分页查询图书
    pub async fn list_books(
        &self,
        page: usize,
        page_size: usize,
        sort_desc: bool,
    ) -> Result<(Vec<book_files::Model>, usize), DbErr> {
        let query = if sort_desc {
            book_files::Entity::find().order_by_desc(book_files::Column::CreatedAt)
        } else {
            book_files::Entity::find().order_by_asc(book_files::Column::CreatedAt)
        };
        let paginator = query.paginate(&self.db, page_size as u64);
        let total = paginator.num_items().await? as usize;
        let books = paginator.fetch_page(page.saturating_sub(1) as u64).await?;

        Ok((books, total))
    }

    pub async fn list_all_books(&self, sort_desc: bool) -> Result<Vec<book_files::Model>, DbErr> {
        let query = if sort_desc {
            book_files::Entity::find().order_by_desc(book_files::Column::CreatedAt)
        } else {
            book_files::Entity::find().order_by_asc(book_files::Column::CreatedAt)
        };
        query.all(&self.db).await
    }

    /// 設定されたすべてのパスをスキャンする
    pub async fn scan_all(&self) -> ScanResult {
        self.scan_all_with_existing(&[]).await
    }

    pub async fn scan_all_with_existing(&self, existing_books: &[book_files::Model]) -> ScanResult {
        let scanner_config = self.get_scanner_config();
        let cache_config = self.get_cache_config();
        let scan_paths = scanner_config.scan_paths.clone();
        let existing_books: Vec<CachedBookMetadata> = existing_books
            .iter()
            .map(|book| CachedBookMetadata {
                path: book.path.clone(),
                title: book.title.clone(),
                kind: book.kind.clone(),
                size: book.size,
                mtime: book.mtime,
                page_count: book.page_count,
                pages_json: book.pages_json.clone(),
                content_signature: book.content_signature.clone(),
                is_oversized: book.is_oversized,
                avg_page_pixels: book.avg_page_pixels,
                cover_path: book.cover_path.clone(),
            })
            .collect();

        let result = tokio::task::spawn_blocking(move || {
            let mut all_result = ScanResult {
                categories: vec![],
                book_files: vec![],
            };

            for scan_path in &scan_paths {
                let recognizer =
                    ConfigurableRecognizer::from((scanner_config.clone(), cache_config.clone()));
                let scanner = Scanner::with_existing_books(&recognizer, existing_books.clone());
                let scan_result = scanner.scan(Path::new(scan_path));
                all_result.categories.extend(scan_result.categories);
                all_result.book_files.extend(scan_result.book_files);
            }

            all_result
        })
        .await
        .unwrap_or_else(|_| ScanResult {
            categories: vec![],
            book_files: vec![],
        });

        result
    }

    /// 増分比較
    pub async fn diff(
        &self,
        db_data: &DatabaseData,
        scan_data: &ScanResult,
    ) -> Result<DiffData, DbErr> {
        let scanner_config = self.get_scanner_config();
        let db_library_paths: std::collections::HashSet<_> = db_data
            .libraries
            .iter()
            .map(|l| l.root_path.clone())
            .collect();
        let scan_library_paths: std::collections::HashSet<_> =
            scanner_config.scan_paths.iter().cloned().collect();

        let new_library_paths: Vec<_> = scanner_config
            .scan_paths
            .iter()
            .filter(|p| !db_library_paths.contains(*p))
            .cloned()
            .collect();

        let deleted_library_ids: Vec<_> = db_data
            .libraries
            .iter()
            .filter(|l| !scan_library_paths.contains(&l.root_path))
            .map(|l| l.id)
            .collect();

        let db_category_paths: std::collections::HashSet<_> =
            db_data.categories.iter().map(|c| c.path.clone()).collect();
        let scan_category_paths: std::collections::HashSet<_> = scan_data
            .categories
            .iter()
            .map(|c| c.path.clone())
            .collect();

        let mut new_category_paths: Vec<_> = scan_data
            .categories
            .iter()
            .filter(|c| !db_category_paths.contains(&c.path))
            .map(|c| c.path.clone())
            .collect();
        // 親ディレクトリが先に処理されるようにパスの長さで昇順ソート（重要！）
        new_category_paths.sort_by_key(|p| p.len());

        let mut deleted_categories: Vec<_> = db_data
            .categories
            .iter()
            .filter(|c| !scan_category_paths.contains(&c.path))
            .collect();
        // 子ディレクトリが先に削除されるようにパスの長さで降順ソート（重要！）
        deleted_categories.sort_by_key(|c| std::cmp::Reverse(c.path.len()));
        let deleted_category_ids: Vec<i64> = deleted_categories.into_iter().map(|c| c.id).collect();

        let db_book_paths: std::collections::HashMap<_, _> = db_data
            .book_files
            .iter()
            .map(|b| (b.path.clone(), b))
            .collect();
        let scan_book_paths: std::collections::HashSet<_> = scan_data
            .book_files
            .iter()
            .map(|b| b.path.clone())
            .collect();

        let new_book_file_paths: Vec<_> = scan_data
            .book_files
            .iter()
            .filter(|b| !db_book_paths.contains_key(&b.path))
            .map(|b| b.path.clone())
            .collect();

        let deleted_book_file_ids: Vec<i64> = db_data
            .book_files
            .iter()
            .filter(|b| !scan_book_paths.contains(&b.path))
            .map(|b| b.id)
            .collect();

        // 既存のメタデータ（cover_path等）をここで引き継ぐことで、後続のDBクエリを削減
        let updated_book_files: Vec<_> = scan_data
            .book_files
            .iter()
            .filter_map(|b| {
                if let Some(db_book) = db_book_paths.get(&b.path) {
                    if book_requires_update(db_book, b) {
                        return Some(book_files::Model {
                            id: db_book.id,
                            category_id: db_book.category_id,
                            path: b.path.clone(),
                            title: b.title.clone().or(db_book.title.clone()),
                            kind: b.kind.clone(),
                            size: b.size,
                            mtime: b.mtime,
                            page_count: b.page_count,
                            pages_json: b.pages_json.clone(),
                            content_signature: b.content_signature.clone(),
                            is_oversized: b.is_oversized,
                            avg_page_pixels: b.avg_page_pixels,
                            is_favorite: db_book.is_favorite,
                            cover_path: db_book.cover_path.clone(),
                            created_at: db_book.created_at.clone(),
                        });
                    }
                }
                None
            })
            .collect();

        Ok(DiffData {
            new_library_paths,
            deleted_library_ids,
            new_category_paths,
            deleted_category_ids,
            new_book_file_paths,
            deleted_book_file_ids,
            updated_book_files,
        })
    }

    /// 実際のデータベース同期を実行する
    pub async fn sync(
        &self,
        db_data: &DatabaseData,
        scan_data: &ScanResult,
    ) -> Result<SyncReport, DbErr> {
        let diff: DiffData = self.diff(db_data, scan_data).await?;
        let txn = self.db.begin().await?;
        let mut report = SyncReport {
            inserted_libraries: 0,
            deleted_libraries: 0,
            inserted_categories: 0,
            deleted_categories: 0,
            inserted_book_files: 0,
            deleted_book_files: 0,
            updated_book_files: 0,
            inserted_library_details: Vec::new(),
            deleted_library_details: Vec::new(),
            inserted_category_details: Vec::new(),
            deleted_category_details: Vec::new(),
            inserted_book_file_details: Vec::new(),
            deleted_book_file_details: Vec::new(),
            updated_book_file_details: Vec::new(),
        };

        let mut library_id_map: HashMap<String, i64> = HashMap::new();
        let mut category_id_map: HashMap<String, i64> = HashMap::new();
        let db_library_by_id: HashMap<i64, &libraries::Model> = db_data
            .libraries
            .iter()
            .map(|library| (library.id, library))
            .collect();
        let db_category_by_id: HashMap<i64, &categories::Model> = db_data
            .categories
            .iter()
            .map(|category| (category.id, category))
            .collect();
        let db_book_by_id: HashMap<i64, &book_files::Model> = db_data
            .book_files
            .iter()
            .map(|book| (book.id, book))
            .collect();

        // --- 1. 挿入フェーズ (Top-Down: Library -> Category -> Book) ---

        for root_path in &diff.new_library_paths {
            let id = self.insert_library(&txn, root_path).await?;
            library_id_map.insert(root_path.clone(), id);
            report.inserted_libraries += 1;
            report.inserted_library_details.push(LibraryChangeDetail {
                path: root_path.clone(),
                name: path_name(root_path),
            });
        }

        for category_path in &diff.new_category_paths {
            // 最長一致で Library を検索 (サブディレクトリ構造に対応)
            let lib_id = library_id_map
                .iter()
                .filter(|(root_path, _)| category_path.starts_with(*root_path))
                .max_by_key(|(root_path, _)| root_path.len())
                .map(|(_, id)| *id)
                .or_else(|| {
                    db_data
                        .libraries
                        .iter()
                        .filter(|l| category_path.starts_with(&l.root_path))
                        .max_by_key(|l| l.root_path.len())
                        .map(|l| l.id)
                });

            let Some(lib_id) = lib_id else {
                continue;
            };

            let parent_path = Path::new(category_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string());
            let parent_id = parent_path
                .as_ref()
                .and_then(|pp| category_id_map.get(pp))
                .copied()
                .or_else(|| {
                    parent_path.and_then(|pp| {
                        db_data
                            .categories
                            .iter()
                            .find(|c| c.path == pp)
                            .map(|c| c.id)
                    })
                });

            let name = Path::new(category_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unnamed")
                .to_string();

            let mtime = scan_data
                .categories
                .iter()
                .find(|c| c.path == *category_path)
                .map(|c| c.mtime)
                .unwrap_or(0);

            let active_model = categories::ActiveModel {
                library_id: sea_orm::Set(lib_id),
                parent_id: sea_orm::Set(parent_id),
                name: sea_orm::Set(name.clone()),
                path: sea_orm::Set(category_path.clone()),
                mtime: sea_orm::Set(mtime),
                ..Default::default()
            };

            let inserted = active_model.insert(&txn).await?;
            category_id_map.insert(category_path.clone(), inserted.id);
            report.inserted_categories += 1;
            report.inserted_category_details.push(CategoryChangeDetail {
                path: category_path.clone(),
                name: name.clone(),
                library_path: Some(resolve_library_path(
                    category_path,
                    &library_id_map,
                    &db_data.libraries,
                )),
                parent_path: Path::new(category_path)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string()),
            });
        }

        for book_path in &diff.new_book_file_paths {
            let category_path =
                resolve_book_category_path(book_path, &category_id_map, &db_data.categories);

            let category_id = category_path.as_ref().and_then(|cp| {
                category_id_map.get(cp).copied().or_else(|| {
                    db_data
                        .categories
                        .iter()
                        .find(|c| c.path == *cp)
                        .map(|c| c.id)
                })
            });

            if let Some(book) = scan_data.book_files.iter().find(|b| b.path == *book_path) {
                let Some(category_id) = category_id else {
                    continue;
                };
                let active_model = book_files::ActiveModel {
                    category_id: sea_orm::Set(category_id),
                    path: sea_orm::Set(book.path.clone()),
                    title: sea_orm::Set(book.title.clone()),
                    kind: sea_orm::Set(book.kind.clone()),
                    size: sea_orm::Set(book.size),
                    mtime: sea_orm::Set(book.mtime),
                    page_count: sea_orm::Set(book.page_count),
                    pages_json: sea_orm::Set(book.pages_json.clone()),
                    content_signature: sea_orm::Set(book.content_signature.clone()),
                    is_oversized: sea_orm::Set(book.is_oversized),
                    avg_page_pixels: sea_orm::Set(book.avg_page_pixels),
                    is_favorite: sea_orm::Set(false),
                    cover_path: sea_orm::Set(book.cover_path.clone()),
                    ..Default::default()
                };
                active_model.insert(&txn).await?;
                report.inserted_book_files += 1;
                report.inserted_book_file_details.push(BookChangeDetail {
                    path: book.path.clone(),
                    title: scanned_book_title(book),
                    category_path: category_path.clone(),
                });
            }
        }

        // --- 2. 更新フェーズ ---

        for updated_book in &diff.updated_book_files {
            // N+1 クエリを排除：必要な信息都在 diff 结果中
            let active_model = book_files::ActiveModel {
                id: sea_orm::Set(updated_book.id),
                category_id: sea_orm::Set(updated_book.category_id),
                path: sea_orm::Set(updated_book.path.clone()),
                title: sea_orm::Set(updated_book.title.clone()),
                kind: sea_orm::Set(updated_book.kind.clone()),
                size: sea_orm::Set(updated_book.size),
                mtime: sea_orm::Set(updated_book.mtime),
                page_count: sea_orm::Set(updated_book.page_count),
                pages_json: sea_orm::Set(updated_book.pages_json.clone()),
                content_signature: sea_orm::Set(updated_book.content_signature.clone()),
                is_oversized: sea_orm::Set(updated_book.is_oversized),
                avg_page_pixels: sea_orm::Set(updated_book.avg_page_pixels),
                is_favorite: sea_orm::Set(updated_book.is_favorite),
                cover_path: sea_orm::Set(updated_book.cover_path.clone()),
                created_at: sea_orm::Set(updated_book.created_at.clone()),
            };
            active_model.update(&txn).await?;
            report.updated_book_files += 1;
            report.updated_book_file_details.push(BookChangeDetail {
                path: updated_book.path.clone(),
                title: display_book_title(updated_book),
                category_path: Some(resolve_category_path(
                    updated_book.category_id,
                    &db_category_by_id,
                ))
                .or_else(|| {
                    Path::new(&updated_book.path)
                        .parent()
                        .map(|p| p.to_string_lossy().to_string())
                }),
            });
        }

        // --- 3. 削除フェーズ (Bottom-Up: Book -> Category -> Library) ---
        // ※ 外部キー制約エラーを避けるためにこの順序が必須です

        for book_id in &diff.deleted_book_file_ids {
            if let Some(book) = db_book_by_id.get(book_id) {
                report.deleted_book_file_details.push(BookChangeDetail {
                    path: book.path.clone(),
                    title: display_book_title(book),
                    category_path: Some(resolve_category_path(
                        book.category_id,
                        &db_category_by_id,
                    ))
                    .filter(|path| !path.is_empty())
                    .or_else(|| {
                        Path::new(&book.path)
                            .parent()
                            .map(|p| p.to_string_lossy().to_string())
                    }),
                });
            }
            book_files::Entity::delete_by_id(*book_id)
                .exec(&txn)
                .await?;
            report.deleted_book_files += 1;
        }

        for category_id in &diff.deleted_category_ids {
            if let Some(category) = db_category_by_id.get(category_id) {
                report.deleted_category_details.push(CategoryChangeDetail {
                    path: category.path.clone(),
                    name: category.name.clone(),
                    library_path: db_library_by_id
                        .get(&category.library_id)
                        .map(|library| library.root_path.clone()),
                    parent_path: category.parent_id.and_then(|id| {
                        db_category_by_id.get(&id).map(|parent| parent.path.clone())
                    }),
                });
            }
            book_files::Entity::delete_many()
                .filter(book_files::Column::CategoryId.eq(*category_id))
                .exec(&txn)
                .await?;
            categories::Entity::delete_by_id(*category_id)
                .exec(&txn)
                .await?;
            report.deleted_categories += 1;
        }

        for library_id in &diff.deleted_library_ids {
            if let Some(library) = db_library_by_id.get(library_id) {
                report.deleted_library_details.push(LibraryChangeDetail {
                    path: library.root_path.clone(),
                    name: library.name.clone(),
                });
            }
            libraries::Entity::delete_by_id(*library_id)
                .exec(&txn)
                .await?;
            report.deleted_libraries += 1;
        }

        txn.commit().await?;

        Ok(report)
    }

    /// library レコードを挿入する
    async fn insert_library<C>(&self, db: &C, root_path: &str) -> Result<i64, DbErr>
    where
        C: ConnectionTrait,
    {
        let existing = libraries::Entity::find()
            .filter(libraries::Column::RootPath.eq(root_path))
            .one(db)
            .await?;

        if let Some(lib) = existing {
            return Ok(lib.id);
        }

        let name = Path::new(root_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unnamed")
            .to_string();

        let now = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(duration) => duration.as_secs() as i64,
            Err(err) => {
                tracing::warn!("system clock is before UNIX_EPOCH: {}", err);
                0
            }
        };

        let active_model = libraries::ActiveModel {
            name: sea_orm::Set(name),
            root_path: sea_orm::Set(root_path.to_string()),
            last_scanned_at: sea_orm::Set(now),
            ..Default::default()
        };

        Ok(active_model.insert(db).await?.id)
    }
}

fn normalize_sqlite_path(database_url: &str) -> String {
    database_url
        .strip_prefix("sqlite://")
        .or_else(|| database_url.strip_prefix("sqlite:"))
        .unwrap_or(database_url)
        .to_string()
}

fn ensure_database_file(path: &str) -> Result<(), DbErr> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|err| DbErr::Custom(format!("failed to create db directory: {err}")))?;
        }
    }

    if !Path::new(path).exists() {
        std::fs::write(path, "")
            .map_err(|err| DbErr::Custom(format!("failed to create db file: {err}")))?;
    }

    Ok(())
}

fn path_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
}

fn display_book_title(book: &book_files::Model) -> String {
    book.title.clone().unwrap_or_else(|| path_name(&book.path))
}

fn scanned_book_title(book: &crate::scanner::types::ScannedBookFile) -> String {
    book.title.clone().unwrap_or_else(|| path_name(&book.path))
}

fn book_requires_update(
    db_book: &book_files::Model,
    scanned_book: &crate::scanner::types::ScannedBookFile,
) -> bool {
    db_book.mtime != scanned_book.mtime
        || db_book.size != scanned_book.size
        || db_book.page_count != scanned_book.page_count
        || db_book.pages_json != scanned_book.pages_json
        || db_book.content_signature != scanned_book.content_signature
        || db_book.is_oversized != scanned_book.is_oversized
        || db_book.avg_page_pixels != scanned_book.avg_page_pixels
}

fn resolve_library_path(
    category_path: &str,
    library_id_map: &HashMap<String, i64>,
    libraries: &[libraries::Model],
) -> String {
    library_id_map
        .keys()
        .filter(|root_path| category_path.starts_with(root_path.as_str()))
        .max_by_key(|root_path| root_path.len())
        .cloned()
        .or_else(|| {
            libraries
                .iter()
                .filter(|library| category_path.starts_with(&library.root_path))
                .max_by_key(|library| library.root_path.len())
                .map(|library| library.root_path.clone())
        })
        .unwrap_or_default()
}

fn resolve_book_category_path(
    book_path: &str,
    category_id_map: &HashMap<String, i64>,
    categories: &[categories::Model],
) -> Option<String> {
    if category_id_map.contains_key(book_path) || categories.iter().any(|c| c.path == book_path) {
        return Some(book_path.to_string());
    }

    Path::new(book_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
}

fn resolve_category_path(
    category_id: i64,
    categories: &HashMap<i64, &categories::Model>,
) -> String {
    categories
        .get(&category_id)
        .map(|category| category.path.clone())
        .unwrap_or_default()
}

fn build_category_tree(
    categories: &[categories::Model],
    books: &[book_files::Model],
) -> Vec<CategoryNode> {
    let mut books_by_category: HashMap<i64, usize> = HashMap::new();
    for book in books {
        *books_by_category.entry(book.category_id).or_insert(0) += 1;
    }

    fn build_node(
        category: &categories::Model,
        categories: &[categories::Model],
        books_by_category: &HashMap<i64, usize>,
    ) -> CategoryNode {
        let children = categories
            .iter()
            .filter(|candidate| candidate.parent_id == Some(category.id))
            .map(|child| build_node(child, categories, books_by_category))
            .collect();

        CategoryNode {
            id: category.id,
            name: category.name.clone(),
            path: category.path.clone(),
            book_count: books_by_category.get(&category.id).copied().unwrap_or(0),
            sub_categories: children,
        }
    }

    categories
        .iter()
        .filter(|category| category.parent_id.is_none())
        .map(|category| build_node(category, categories, &books_by_category))
        .collect()
}

fn collect_descendant_category_ids(category_id: i64, categories: &[categories::Model]) -> Vec<i64> {
    let mut result = vec![category_id];
    let mut index = 0;

    while index < result.len() {
        let current_id = result[index];
        for category in categories
            .iter()
            .filter(|category| category.parent_id == Some(current_id))
        {
            if !result.contains(&category.id) {
                result.push(category.id);
            }
        }
        index += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::types::ScannedBookFile;

    #[test]
    fn book_requires_update_detects_content_signature_change() {
        let db_book = book_files::Model {
            id: 1,
            category_id: 1,
            path: "/library/book".to_string(),
            title: Some("book".to_string()),
            kind: "image_folder".to_string(),
            size: 100,
            mtime: 10,
            page_count: 2,
            pages_json: Some("[\"a\",\"b\"]".to_string()),
            content_signature: Some("old-signature".to_string()),
            is_oversized: false,
            avg_page_pixels: 123,
            is_favorite: false,
            cover_path: Some("/library/book/a.png".to_string()),
            created_at: None,
        };

        let scanned_book = ScannedBookFile {
            path: db_book.path.clone(),
            title: db_book.title.clone(),
            kind: db_book.kind.clone(),
            size: db_book.size,
            mtime: db_book.mtime,
            page_count: db_book.page_count,
            pages_json: db_book.pages_json.clone(),
            content_signature: Some("new-signature".to_string()),
            is_oversized: db_book.is_oversized,
            avg_page_pixels: db_book.avg_page_pixels,
            cover_path: db_book.cover_path.clone(),
        };

        assert!(book_requires_update(&db_book, &scanned_book));
    }

    #[test]
    fn book_requires_update_ignores_identical_scanned_book() {
        let db_book = book_files::Model {
            id: 1,
            category_id: 1,
            path: "/library/book".to_string(),
            title: Some("book".to_string()),
            kind: "pdf".to_string(),
            size: 100,
            mtime: 10,
            page_count: 8,
            pages_json: None,
            content_signature: None,
            is_oversized: false,
            avg_page_pixels: 0,
            is_favorite: false,
            cover_path: None,
            created_at: None,
        };

        let scanned_book = ScannedBookFile {
            path: db_book.path.clone(),
            title: db_book.title.clone(),
            kind: db_book.kind.clone(),
            size: db_book.size,
            mtime: db_book.mtime,
            page_count: db_book.page_count,
            pages_json: db_book.pages_json.clone(),
            content_signature: db_book.content_signature.clone(),
            is_oversized: db_book.is_oversized,
            avg_page_pixels: db_book.avg_page_pixels,
            cover_path: db_book.cover_path.clone(),
        };

        assert!(!book_requires_update(&db_book, &scanned_book));
    }
}
