//! Schema migrations, run on startup by the `Migrator`. Each migration is one
//! versioned module; add new ones here rather than editing old ones.

use sea_orm_migration::prelude::*;

mod m0001_create_invitations;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m0001_create_invitations::Migration)]
    }
}
