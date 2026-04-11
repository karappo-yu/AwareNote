//! `SeaORM` Entity for page_favorites table
//!
//! 页面收藏 — 用户在查看器中收藏的单页标记。

use sea_orm::entity::prelude::{
    ActiveModelBehavior, ColumnDef, ColumnTrait, ColumnType, ColumnTypeTrait, DeriveActiveModel,
    DeriveColumn, DeriveEntity, DeriveModel, DerivePrimaryKey, EntityName, EnumIter,
    PrimaryKeyTrait,
};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &str {
        "page_favorites"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize, Deserialize)]
pub struct Model {
    pub book_id: String,
    pub filename: String,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    BookId,
    Filename,
    CreatedAt,
}

#[derive(Copy, Clone, Debug, EnumIter, DerivePrimaryKey)]
pub enum PrimaryKey {
    BookId,
    Filename,
}

impl PrimaryKeyTrait for PrimaryKey {
    type ValueType = (String, String);
    fn auto_increment() -> bool {
        false
    }
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::BookId => ColumnType::Text.def(),
            Self::Filename => ColumnType::Text.def(),
            Self::CreatedAt => ColumnType::Integer.def(),
        }
    }
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl sea_orm::entity::prelude::RelationTrait for Relation {
    fn def(&self) -> sea_orm::entity::prelude::RelationDef {
        panic!("No relations defined for page_favorites")
    }
}

impl ActiveModelBehavior for ActiveModel {}
