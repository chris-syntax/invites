//! The `invitations` table. Times are unix seconds (`i64`); `expires_at` is null
//! for a non-expiring invitation and `max_uses` is null for an unlimited one;
//! `revoked` is stored as a boolean.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "invitations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub token: String,
    pub label: String,
    pub created_by: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub max_uses: Option<i64>,
    pub accounts_created: i64,
    pub revoked: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
