use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add allocation_type (string) and final (boolean) to Allocation table
        manager
            .alter_table(
                Table::alter()
                    .table(Allocation::Table)
                    .add_column(
                        ColumnDef::new(Allocation::AllocationType)
                            .string()
                            .not_null()
                            .default("PLAN"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Allocation::Table)
                    .add_column(
                        ColumnDef::new(Allocation::Final).boolean().not_null().default(false),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Allocation::Table)
                    .drop_column(Allocation::AllocationType)
                    .drop_column(Allocation::Final)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum Allocation {
    Table,
    Id,
    TaskId,
    Start,
    End,
    AllocationType,
    Final,
}
