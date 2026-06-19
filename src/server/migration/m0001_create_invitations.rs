//! Initial schema: the `invitations` table. Mirrors the original inline
//! `CREATE TABLE`, now versioned so the schema can evolve.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Invitations::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Invitations::Token).text().not_null().primary_key())
                    .col(ColumnDef::new(Invitations::Label).text().not_null())
                    .col(ColumnDef::new(Invitations::CreatedBy).text().not_null())
                    .col(ColumnDef::new(Invitations::CreatedAt).big_integer().not_null())
                    .col(ColumnDef::new(Invitations::ExpiresAt).big_integer().not_null())
                    .col(ColumnDef::new(Invitations::MaxUses).big_integer())
                    .col(
                        ColumnDef::new(Invitations::AccountsCreated)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Invitations::Revoked)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Invitations::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Invitations {
    Table,
    Token,
    Label,
    CreatedBy,
    CreatedAt,
    ExpiresAt,
    MaxUses,
    AccountsCreated,
    Revoked,
}
