//! `SeaORM` Entity for page_spreads table
//!
//! 跨页拼接标记 — 用户手动标记的跨页图(spread)关系。

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
        "page_spreads"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize, Deserialize)]
pub struct Model {
    pub book_id: String,
    pub filename: String,
    pub next_file: String,
    pub direction: String,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    BookId,
    Filename,
    NextFile,
    Direction,
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
            Self::NextFile => ColumnType::Text.def(),
            Self::Direction => ColumnType::Text.def(),
            Self::CreatedAt => ColumnType::Integer.def(),
        }
    }
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl sea_orm::entity::prelude::RelationTrait for Relation {
    fn def(&self) -> sea_orm::entity::prelude::RelationDef {
        panic!("No relations defined for page_spreads")
    }
}

impl ActiveModelBehavior for ActiveModel {}
