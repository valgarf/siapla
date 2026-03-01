pub use sea_orm_migration::prelude::*;

mod m20250322_create_tables;
mod m20251014_add_allocation_booking;
mod m20251227_add_task_priority;
mod m20260301_add_revisioning;
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250322_create_tables::Migration),
            Box::new(m20251014_add_allocation_booking::Migration),
            Box::new(m20251227_add_task_priority::Migration),
            Box::new(m20260301_add_revisioning::Migration),
        ]
    }
}
