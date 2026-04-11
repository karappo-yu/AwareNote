//! `SeaORM` Entity for user_data table
//!
//! 用户数据表 — 存储全局设置和 per-book 设置。
//! book_id='' 为全局设置，非空为某本书的设置。

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
        "user_data"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize, Deserialize)]
pub struct Model {
    pub book_id: String,
    pub key: String,
    pub value: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    BookId,
    Key,
    Value,
}

#[derive(Copy, Clone, Debug, EnumIter, DerivePrimaryKey)]
pub enum PrimaryKey {
    BookId,
    Key,
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
            Self::Key => ColumnType::Text.def(),
            Self::Value => ColumnType::Text.def(),
        }
    }
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl sea_orm::entity::prelude::RelationTrait for Relation {
    fn def(&self) -> sea_orm::entity::prelude::RelationDef {
        panic!("No relations defined for user_data")
    }
}

impl ActiveModelBehavior for ActiveModel {}
