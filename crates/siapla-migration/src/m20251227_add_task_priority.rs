use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Task::Table)
                    .add_column(ColumnDef::new(Task::Priority).float().not_null().default(1.0))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(Table::alter().table(Task::Table).drop_column(Task::Priority).to_owned())
            .await?;

        Ok(())
    }
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum Task {
    Table,
    Priority,
}
