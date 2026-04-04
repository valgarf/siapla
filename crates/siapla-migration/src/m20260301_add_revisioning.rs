use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{big_unsigned, boolean, integer, pk_auto, string, timestamp};
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
                    .add_column(ColumnDef::new(ResourceIteration::RevDeleted).big_unsigned().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Dependency::Table)
                    .add_column(
                        ColumnDef::new(Dependency::RevCreated).big_unsigned().not_null().default(1),
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
                        ColumnDef::new(Vacation::RevCreated).big_unsigned().not_null().default(1),
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
                        ColumnDef::new(Allocation::RevCreated).big_unsigned().not_null().default(1),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Allocation::Table)
                    .add_column(ColumnDef::new(Allocation::RevDeleted).big_unsigned().null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Booking::Table)
                    .if_not_exists()
                    .col(pk_auto(Booking::Id))
                    .col(integer(Booking::TaskId))
                    .col(timestamp(Booking::Start))
                    .col(timestamp(Booking::End))
                    .col(boolean(Booking::Final).not_null().default(false))
                    .col(ColumnDef::new(Booking::RevCreated).big_unsigned().not_null())
                    .col(ColumnDef::new(Booking::RevDeleted).big_unsigned().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Booking_Task")
                            .from(Booking::Table, Booking::TaskId)
                            .to(TaskIteration::Table, TaskIteration::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Booking_RevCreated")
                            .from(Booking::Table, Booking::RevCreated)
                            .to(Revision::Table, Revision::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Booking_RevDeleted")
                            .from(Booking::Table, Booking::RevDeleted)
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
                    .table(BookingResource::Table)
                    .if_not_exists()
                    .col(pk_auto(BookingResource::Id))
                    .col(integer(BookingResource::BookingId))
                    .col(integer(BookingResource::ResourceId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_BookingResource_Booking")
                            .from(BookingResource::Table, BookingResource::BookingId)
                            .to(Booking::Table, Booking::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_BookingResource_Resource")
                            .from(BookingResource::Table, BookingResource::ResourceId)
                            .to(ResourceIteration::Table, ResourceIteration::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(AllocatedResource::Table)
                    .add_column(
                        ColumnDef::new(AllocatedResource::RevCreated)
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
                    .add_column(ColumnDef::new(AllocatedResource::RevDeleted).big_unsigned().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Issue::Table)
                    .add_column(
                        ColumnDef::new(Issue::RevCreated).big_unsigned().not_null().default(1),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Issue::Table)
                    .add_column(ColumnDef::new(Issue::RevDeleted).big_unsigned().null())
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

        let rewrite_task_parent_ids_to_headers = Query::update()
            .table(TaskIteration::Table)
            .value(
                TaskIteration::ParentId,
                Expr::cust(
                    "(SELECT parent.header_id FROM task_iteration AS parent WHERE parent.id = task_iteration.parent_id)",
                ),
            )
            .and_where(Expr::col(TaskIteration::ParentId).is_not_null())
            .to_owned();

        db.execute(backend.build(&rewrite_task_parent_ids_to_headers)).await?;

        let fill_resource_header_ids = Query::update()
            .table(ResourceIteration::Table)
            .value(ResourceIteration::HeaderId, Expr::col(ResourceIteration::Id))
            .and_where(Expr::col(ResourceIteration::HeaderId).is_null())
            .to_owned();

        db.execute(backend.build(&fill_resource_header_ids)).await?;

        let copy_bookings = Query::insert()
            .into_table(Booking::Table)
            .columns([
                Booking::Id,
                Booking::TaskId,
                Booking::Start,
                Booking::End,
                Booking::Final,
                Booking::RevCreated,
                Booking::RevDeleted,
            ])
            .select_from(
                Query::select()
                    .column(Allocation::Id)
                    .column(Allocation::TaskId)
                    .column(Allocation::Start)
                    .column(Allocation::End)
                    .column(Allocation::Final)
                    .expr(Expr::value(1))
                    .expr(Expr::value(Value::BigUnsigned(None)))
                    .from(Allocation::Table)
                    .and_where(Expr::col(Allocation::AllocationType).eq("BOOKING"))
                    .to_owned(),
            )
            .map_err(|e| DbErr::Custom(e.to_string()))?
            .to_owned();
        db.execute(backend.build(&copy_bookings)).await?;

        let copy_booking_resources = Query::insert()
            .into_table(BookingResource::Table)
            .columns([BookingResource::BookingId, BookingResource::ResourceId])
            .select_from(
                Query::select()
                    .column(AllocatedResource::AllocationId)
                    .column(AllocatedResource::ResourceId)
                    .from(AllocatedResource::Table)
                    .inner_join(
                        Allocation::Table,
                        Expr::col((AllocatedResource::Table, AllocatedResource::AllocationId))
                            .equals((Allocation::Table, Allocation::Id)),
                    )
                    .and_where(
                        Expr::col((Allocation::Table, Allocation::AllocationType)).eq("BOOKING"),
                    )
                    .to_owned(),
            )
            .map_err(|e| DbErr::Custom(e.to_string()))?
            .to_owned();
        db.execute(backend.build(&copy_booking_resources)).await?;

        let delete_allocated_resources_for_bookings = Query::delete()
            .from_table(AllocatedResource::Table)
            .and_where(
                Expr::col(AllocatedResource::AllocationId).in_subquery(
                    Query::select()
                        .column(Allocation::Id)
                        .from(Allocation::Table)
                        .and_where(Expr::col(Allocation::AllocationType).eq("BOOKING"))
                        .to_owned(),
                ),
            )
            .to_owned();
        db.execute(backend.build(&delete_allocated_resources_for_bookings)).await?;

        let delete_bookings_from_allocation = Query::delete()
            .from_table(Allocation::Table)
            .and_where(Expr::col(Allocation::AllocationType).eq("BOOKING"))
            .to_owned();
        db.execute(backend.build(&delete_bookings_from_allocation)).await?;

        // Delete remaining (PLAN) allocated_resources and allocations to avoid
        // duplication when the backend recalculates on startup.
        let delete_plan_allocated_resources =
            Query::delete().from_table(AllocatedResource::Table).to_owned();
        db.execute(backend.build(&delete_plan_allocated_resources)).await?;

        let delete_plan_allocations = Query::delete().from_table(Allocation::Table).to_owned();
        db.execute(backend.build(&delete_plan_allocations)).await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Allocation::Table)
                    .drop_column(Allocation::AllocationType)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter().table(Allocation::Table).drop_column(Allocation::Final).to_owned(),
            )
            .await?;

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

        let delete_allocation = Query::delete()
            .from_table(Allocation::Table)
            .and_where(Expr::col(Allocation::RevDeleted).is_not_null())
            .to_owned();
        db.execute(backend.build(&delete_allocation)).await?;

        let delete_allocated_resource = Query::delete()
            .from_table(AllocatedResource::Table)
            .and_where(Expr::col(AllocatedResource::RevDeleted).is_not_null())
            .to_owned();
        db.execute(backend.build(&delete_allocated_resource)).await?;

        let delete_issue = Query::delete()
            .from_table(Issue::Table)
            .and_where(Expr::col(Issue::RevDeleted).is_not_null())
            .to_owned();
        db.execute(backend.build(&delete_issue)).await?;
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
    ParentId,
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
    Id,
    TaskId,
    Start,
    End,
    AllocationType,
    Final,
    RevCreated,
    RevDeleted,
}

#[derive(DeriveIden)]
enum Booking {
    Table,
    Id,
    TaskId,
    Start,
    End,
    Final,
    RevCreated,
    RevDeleted,
}

#[derive(DeriveIden)]
enum BookingResource {
    Table,
    Id,
    BookingId,
    ResourceId,
}

#[derive(DeriveIden)]
enum AllocatedResource {
    Table,
    AllocationId,
    ResourceId,
    RevCreated,
    RevDeleted,
}

#[derive(DeriveIden)]
enum Issue {
    Table,
    RevCreated,
    RevDeleted,
}
