pub use sea_orm_migration::prelude::*;

mod m20250322_create_tables;
mod m20251014_add_allocation_booking;
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250322_create_tables::Migration),
            Box::new(m20251014_add_allocation_booking::Migration),
        ]
    }
}
