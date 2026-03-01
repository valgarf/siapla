use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{big_unsigned, pk_auto, string, timestamp};
use sea_query::{Expr, OnConflict, Query};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Revision::Table)
                    .if_not_exists()
                    .col(big_unsigned(Revision::Id).auto_increment().primary_key().take())
                    .col(timestamp(Revision::Timestamp))
                    .col(string(Revision::PlanState))
                    .to_owned(),
            )
            .await?;

        manager
            .rename_table(Table::rename().table(Task::Table, TaskIteration::Table).to_owned())
            .await?;

        manager
            .rename_table(
                Table::rename().table(Resource::Table, ResourceIteration::Table).to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(TaskHeader::Table)
                    .if_not_exists()
                    .col(pk_auto(TaskHeader::Id))
                    .col(ColumnDef::new(TaskHeader::RevCreated).big_unsigned())
                    .col(ColumnDef::new(TaskHeader::RevDeleted).big_unsigned().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_TaskHeader_RevCreated")
                            .from(TaskHeader::Table, TaskHeader::RevCreated)
                            .to(Revision::Table, Revision::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_TaskHeader_RevDeleted")
                            .from(TaskHeader::Table, TaskHeader::RevDeleted)
                            .to(Revision::Table, Revision::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ResourceHeader::Table)
                    .if_not_exists()
                    .col(pk_auto(ResourceHeader::Id))
                    .col(ColumnDef::new(ResourceHeader::RevCreated).big_unsigned())
                    .col(ColumnDef::new(ResourceHeader::RevDeleted).big_unsigned().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_ResourceHeader_RevCreated")
                            .from(ResourceHeader::Table, ResourceHeader::RevCreated)
                            .to(Revision::Table, Revision::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_ResourceHeader_RevDeleted")
                            .from(ResourceHeader::Table, ResourceHeader::RevDeleted)
                            .to(Revision::Table, Revision::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(TaskIteration::Table)
                    .add_column(ColumnDef::new(TaskIteration::HeaderId).integer().null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(TaskIteration::Table)
                    .add_column(
                        ColumnDef::new(TaskIteration::RevCreated)
                            .big_unsigned()
                            .not_null()
                            .default(1),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(TaskIteration::Table)
                    .add_column(ColumnDef::new(TaskIteration::RevDeleted).big_unsigned().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(ResourceIteration::Table)
                    .add_column(ColumnDef::new(ResourceIteration::HeaderId).integer().null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(ResourceIteration::Table)
                    .add_column(
                        ColumnDef::new(ResourceIteration::RevCreated)
                            .big_unsigned()
                            .not_null()
                            .default(1),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(ResourceIteration::Table)
                    .add_column(
                        ColumnDef::new(ResourceIteration::RevDeleted).big_unsigned().null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Dependency::Table)
                    .add_column(
                        ColumnDef::new(Dependency::RevCreated)
                            .big_unsigned()
                            .not_null()
                            .default(1),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Dependency::Table)
                    .add_column(ColumnDef::new(Dependency::RevDeleted).big_unsigned().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Vacation::Table)
                    .add_column(
                        ColumnDef::new(Vacation::RevCreated)
                            .big_unsigned()
                            .not_null()
                            .default(1),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Vacation::Table)
                    .add_column(ColumnDef::new(Vacation::RevDeleted).big_unsigned().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Availability::Table)
                    .add_column(
                        ColumnDef::new(Availability::RevCreated)
                            .big_unsigned()
                            .not_null()
                            .default(1),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Availability::Table)
                    .add_column(ColumnDef::new(Availability::RevDeleted).big_unsigned().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(ResourceConstraint::Table)
                    .add_column(
                        ColumnDef::new(ResourceConstraint::RevCreated)
                            .big_unsigned()
                            .not_null()
                            .default(1),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(ResourceConstraint::Table)
                    .add_column(
                        ColumnDef::new(ResourceConstraint::RevDeleted).big_unsigned().null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Allocation::Table)
                    .add_column(
                        ColumnDef::new(Allocation::Revision)
                            .big_unsigned()
                            .not_null()
                            .default(1),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(AllocatedResource::Table)
                    .add_column(
                        ColumnDef::new(AllocatedResource::Revision)
                            .big_unsigned()
                            .not_null()
                            .default(1),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Issue::Table)
                    .add_column(
                        ColumnDef::new(Issue::Revision).big_unsigned().not_null().default(1),
                    )
                    .to_owned(),
            )
            .await?;

        let backend = manager.get_database_backend();
        let db = manager.get_connection();

        let seed_revision = Query::insert()
            .into_table(Revision::Table)
            .columns([Revision::Id, Revision::Timestamp, Revision::PlanState])
            .values_panic([1.into(), Expr::current_timestamp().into(), "NOT_CALCULATED".into()])
            .on_conflict(OnConflict::column(Revision::Id).do_nothing().to_owned())
            .to_owned();

        db.execute(backend.build(&seed_revision)).await?;

        let copy_task_headers = Query::insert()
            .into_table(TaskHeader::Table)
            .columns([TaskHeader::Id, TaskHeader::RevCreated, TaskHeader::RevDeleted])
            .select_from(
                Query::select()
                    .column(TaskIteration::Id)
                    .expr(Expr::value(1))
                    .expr(Expr::value(Value::BigUnsigned(None)))
                    .from(TaskIteration::Table)
                    .to_owned(),
            )
            .map_err(|e| DbErr::Custom(e.to_string()))?
            .to_owned();

        db.execute(backend.build(&copy_task_headers)).await?;

        let copy_resource_headers = Query::insert()
            .into_table(ResourceHeader::Table)
            .columns([ResourceHeader::Id, ResourceHeader::RevCreated, ResourceHeader::RevDeleted])
            .select_from(
                Query::select()
                    .column(ResourceIteration::Id)
                    .expr(Expr::value(1))
                        .expr(Expr::value(Value::BigUnsigned(None)))
                    .from(ResourceIteration::Table)
                    .to_owned(),
            )
            .map_err(|e| DbErr::Custom(e.to_string()))?
            .to_owned();

        db.execute(backend.build(&copy_resource_headers)).await?;

        let fill_task_header_ids = Query::update()
            .table(TaskIteration::Table)
            .value(TaskIteration::HeaderId, Expr::col(TaskIteration::Id))
            .and_where(Expr::col(TaskIteration::HeaderId).is_null())
            .to_owned();

        db.execute(backend.build(&fill_task_header_ids)).await?;

        let fill_resource_header_ids = Query::update()
            .table(ResourceIteration::Table)
            .value(ResourceIteration::HeaderId, Expr::col(ResourceIteration::Id))
            .and_where(Expr::col(ResourceIteration::HeaderId).is_null())
            .to_owned();

        db.execute(backend.build(&fill_resource_header_ids)).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let db = manager.get_connection();

        let delete_dependency = Query::delete()
            .from_table(Dependency::Table)
            .and_where(Expr::col(Dependency::RevDeleted).is_not_null())
            .to_owned();
        db.execute(backend.build(&delete_dependency)).await?;

        let delete_vacation = Query::delete()
            .from_table(Vacation::Table)
            .and_where(Expr::col(Vacation::RevDeleted).is_not_null())
            .to_owned();
        db.execute(backend.build(&delete_vacation)).await?;

        let delete_availability = Query::delete()
            .from_table(Availability::Table)
            .and_where(Expr::col(Availability::RevDeleted).is_not_null())
            .to_owned();
        db.execute(backend.build(&delete_availability)).await?;

        let delete_resource_constraint = Query::delete()
            .from_table(ResourceConstraint::Table)
            .and_where(Expr::col(ResourceConstraint::RevDeleted).is_not_null())
            .to_owned();
        db.execute(backend.build(&delete_resource_constraint)).await?;

        let delete_task_iteration = Query::delete()
            .from_table(TaskIteration::Table)
            .and_where(Expr::col(TaskIteration::RevDeleted).is_not_null())
            .to_owned();
        db.execute(backend.build(&delete_task_iteration)).await?;

        let delete_resource_iteration = Query::delete()
            .from_table(ResourceIteration::Table)
            .and_where(Expr::col(ResourceIteration::RevDeleted).is_not_null())
            .to_owned();
        db.execute(backend.build(&delete_resource_iteration)).await?;

        let delete_task_header = Query::delete()
            .from_table(TaskHeader::Table)
            .and_where(Expr::col(TaskHeader::RevDeleted).is_not_null())
            .to_owned();
        db.execute(backend.build(&delete_task_header)).await?;

        let delete_resource_header = Query::delete()
            .from_table(ResourceHeader::Table)
            .and_where(Expr::col(ResourceHeader::RevDeleted).is_not_null())
            .to_owned();
        db.execute(backend.build(&delete_resource_header)).await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Revision {
    Table,
    Id,
    Timestamp,
    PlanState,
}

#[derive(DeriveIden)]
enum Task {
    Table,
}

#[derive(DeriveIden)]
enum Resource {
    Table,
}

#[derive(DeriveIden)]
enum TaskHeader {
    Table,
    Id,
    RevCreated,
    RevDeleted,
}

#[derive(DeriveIden)]
enum ResourceHeader {
    Table,
    Id,
    RevCreated,
    RevDeleted,
}

#[derive(DeriveIden)]
enum TaskIteration {
    Table,
    Id,
    HeaderId,
    RevCreated,
    RevDeleted,
}

#[derive(DeriveIden)]
enum ResourceIteration {
    Table,
    Id,
    HeaderId,
    RevCreated,
    RevDeleted,
}

#[derive(DeriveIden)]
enum Dependency {
    Table,
    RevCreated,
    RevDeleted,
}

#[derive(DeriveIden)]
enum Vacation {
    Table,
    RevCreated,
    RevDeleted,
}

#[derive(DeriveIden)]
enum Availability {
    Table,
    RevCreated,
    RevDeleted,
}

#[derive(DeriveIden)]
enum ResourceConstraint {
    Table,
    RevCreated,
    RevDeleted,
}

#[derive(DeriveIden)]
enum Allocation {
    Table,
    Revision,
}

#[derive(DeriveIden)]
enum AllocatedResource {
    Table,
    Revision,
}

#[derive(DeriveIden)]
enum Issue {
    Table,
    Revision,
}
