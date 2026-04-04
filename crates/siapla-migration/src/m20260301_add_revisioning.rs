use sea_orm::DatabaseBackend;
use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::*;
use sea_query::{Expr, OnConflict, Query};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let db = manager.get_connection();
        let is_sqlite = matches!(backend, DatabaseBackend::Sqlite);

        if is_sqlite {
            db.execute_unprepared("PRAGMA foreign_keys=OFF").await?;
            db.execute_unprepared("PRAGMA legacy_alter_table=ON").await?;
        }

        // === Phase 1: Create revision table and seed it ===
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

        let seed_revision = Query::insert()
            .into_table(Revision::Table)
            .columns([Revision::Id, Revision::Timestamp, Revision::PlanState])
            .values_panic([1.into(), Expr::current_timestamp().into(), "NOT_CALCULATED".into()])
            .on_conflict(OnConflict::column(Revision::Id).do_nothing().to_owned())
            .to_owned();
        db.execute(backend.build(&seed_revision)).await?;

        // === Phase 2: Create header tables ===
        manager
            .create_table(
                Table::create()
                    .table(TaskHeader::Table)
                    .if_not_exists()
                    .col(pk_auto(TaskHeader::Id))
                    .col(ColumnDef::new(TaskHeader::RevCreated).big_unsigned().not_null())
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
                    .col(ColumnDef::new(ResourceHeader::RevCreated).big_unsigned().not_null())
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

        // === Phase 3: Recreate task -> task_iteration with proper FKs ===
        if is_sqlite {
            db.execute_unprepared("ALTER TABLE \"task\" RENAME TO \"_task_old\"").await?;
        } else {
            manager
                .rename_table(
                    Table::rename().table(Task::Table, Alias::new("_task_old")).to_owned(),
                )
                .await?;
        }

        manager
            .create_table(
                Table::create()
                    .table(TaskIterationFull::Table)
                    .col(pk_auto(TaskIterationFull::Id))
                    .col(ColumnDef::new(TaskIterationFull::ParentId).integer().null())
                    .col(string(TaskIterationFull::Title))
                    .col(string(TaskIterationFull::Description))
                    .col(string(TaskIterationFull::Designation))
                    .col(ColumnDef::new(TaskIterationFull::EarliestStart).timestamp().null())
                    .col(ColumnDef::new(TaskIterationFull::ScheduleTarget).timestamp().null())
                    .col(ColumnDef::new(TaskIterationFull::Effort).float().null())
                    .col(ColumnDef::new(TaskIterationFull::Priority).float().not_null().default(1))
                    .col(integer(TaskIterationFull::HeaderId))
                    .col(ColumnDef::new(TaskIterationFull::RevCreated).big_integer().not_null())
                    .col(ColumnDef::new(TaskIterationFull::RevDeleted).big_integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_TaskIteration_ParentId")
                            .from(TaskIterationFull::Table, TaskIterationFull::ParentId)
                            .to(TaskHeader::Table, TaskHeader::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_TaskIteration_HeaderId")
                            .from(TaskIterationFull::Table, TaskIterationFull::HeaderId)
                            .to(TaskHeader::Table, TaskHeader::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_TaskIteration_RevCreated")
                            .from(TaskIterationFull::Table, TaskIterationFull::RevCreated)
                            .to(Revision::Table, Revision::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_TaskIteration_RevDeleted")
                            .from(TaskIterationFull::Table, TaskIterationFull::RevDeleted)
                            .to(Revision::Table, Revision::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        let insert_task_headers = Query::insert()
            .into_table(TaskHeader::Table)
            .columns([TaskHeader::Id, TaskHeader::RevCreated, TaskHeader::RevDeleted])
            .select_from(
                Query::select()
                    .column(Alias::new("id"))
                    .expr(Expr::value(1))
                    .expr(Expr::value(Value::BigUnsigned(None)))
                    .from(Alias::new("_task_old"))
                    .to_owned(),
            )
            .map_err(|e| DbErr::Custom(e.to_string()))?
            .to_owned();
        db.execute(backend.build(&insert_task_headers)).await?;

        let insert_task_iterations = Query::insert()
            .into_table(TaskIterationFull::Table)
            .columns([
                TaskIterationFull::Id,
                TaskIterationFull::ParentId,
                TaskIterationFull::Title,
                TaskIterationFull::Description,
                TaskIterationFull::Designation,
                TaskIterationFull::EarliestStart,
                TaskIterationFull::ScheduleTarget,
                TaskIterationFull::Effort,
                TaskIterationFull::Priority,
                TaskIterationFull::HeaderId,
                TaskIterationFull::RevCreated,
                TaskIterationFull::RevDeleted,
            ])
            .select_from(
                Query::select()
                    .column(Alias::new("id"))
                    .column(Alias::new("parent_id"))
                    .column(Alias::new("title"))
                    .column(Alias::new("description"))
                    .column(Alias::new("designation"))
                    .column(Alias::new("earliest_start"))
                    .column(Alias::new("schedule_target"))
                    .column(Alias::new("effort"))
                    .column(Alias::new("priority"))
                    .column(Alias::new("id"))
                    .expr(Expr::value(1))
                    .expr(Expr::value(Value::BigUnsigned(None)))
                    .from(Alias::new("_task_old"))
                    .to_owned(),
            )
            .map_err(|e| DbErr::Custom(e.to_string()))?
            .to_owned();
        db.execute(backend.build(&insert_task_iterations)).await?;

        let rewrite_task_parent_ids_to_headers = Query::update()
            .table(TaskIterationFull::Table)
            .value(
                TaskIterationFull::ParentId,
                Expr::cust(
                    "(SELECT parent.\"header_id\" FROM \"task_iteration\" AS parent WHERE parent.\"id\" = \"task_iteration\".\"parent_id\")",
                ),
            )
            .and_where(Expr::col(TaskIterationFull::ParentId).is_not_null())
            .to_owned();
        db.execute(backend.build(&rewrite_task_parent_ids_to_headers)).await?;

        manager.drop_table(Table::drop().table(Alias::new("_task_old")).to_owned()).await?;

        // === Phase 4: Recreate resource -> resource_iteration with proper FKs ===
        if is_sqlite {
            db.execute_unprepared("ALTER TABLE \"resource\" RENAME TO \"_resource_old\"").await?;
        } else {
            manager
                .rename_table(
                    Table::rename().table(Resource::Table, Alias::new("_resource_old")).to_owned(),
                )
                .await?;
        }

        manager
            .create_table(
                Table::create()
                    .table(ResourceIterationFull::Table)
                    .col(pk_auto(ResourceIterationFull::Id))
                    .col(string(ResourceIterationFull::Name))
                    .col(string(ResourceIterationFull::Timezone))
                    .col(timestamp(ResourceIterationFull::Added))
                    .col(ColumnDef::new(ResourceIterationFull::Removed).timestamp().null())
                    .col(ColumnDef::new(ResourceIterationFull::HolidayId).integer().null())
                    .col(integer(ResourceIterationFull::HeaderId))
                    .col(ColumnDef::new(ResourceIterationFull::RevCreated).big_integer().not_null())
                    .col(ColumnDef::new(ResourceIterationFull::RevDeleted).big_integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_ResourceIteration_HolidayId")
                            .from(ResourceIterationFull::Table, ResourceIterationFull::HolidayId)
                            .to(Holiday::Table, Holiday::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_ResourceIteration_HeaderId")
                            .from(ResourceIterationFull::Table, ResourceIterationFull::HeaderId)
                            .to(ResourceHeader::Table, ResourceHeader::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_ResourceIteration_RevCreated")
                            .from(ResourceIterationFull::Table, ResourceIterationFull::RevCreated)
                            .to(Revision::Table, Revision::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_ResourceIteration_RevDeleted")
                            .from(ResourceIterationFull::Table, ResourceIterationFull::RevDeleted)
                            .to(Revision::Table, Revision::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        let insert_resource_headers = Query::insert()
            .into_table(ResourceHeader::Table)
            .columns([ResourceHeader::Id, ResourceHeader::RevCreated, ResourceHeader::RevDeleted])
            .select_from(
                Query::select()
                    .column(Alias::new("id"))
                    .expr(Expr::value(1))
                    .expr(Expr::value(Value::BigUnsigned(None)))
                    .from(Alias::new("_resource_old"))
                    .to_owned(),
            )
            .map_err(|e| DbErr::Custom(e.to_string()))?
            .to_owned();
        db.execute(backend.build(&insert_resource_headers)).await?;

        let insert_resource_iterations = Query::insert()
            .into_table(ResourceIterationFull::Table)
            .columns([
                ResourceIterationFull::Id,
                ResourceIterationFull::Name,
                ResourceIterationFull::Timezone,
                ResourceIterationFull::Added,
                ResourceIterationFull::Removed,
                ResourceIterationFull::HolidayId,
                ResourceIterationFull::HeaderId,
                ResourceIterationFull::RevCreated,
                ResourceIterationFull::RevDeleted,
            ])
            .select_from(
                Query::select()
                    .column(Alias::new("id"))
                    .column(Alias::new("name"))
                    .column(Alias::new("timezone"))
                    .column(Alias::new("added"))
                    .column(Alias::new("removed"))
                    .column(Alias::new("holiday_id"))
                    .column(Alias::new("id"))
                    .expr(Expr::value(1))
                    .expr(Expr::value(Value::BigUnsigned(None)))
                    .from(Alias::new("_resource_old"))
                    .to_owned(),
            )
            .map_err(|e| DbErr::Custom(e.to_string()))?
            .to_owned();
        db.execute(backend.build(&insert_resource_iterations)).await?;

        manager.drop_table(Table::drop().table(Alias::new("_resource_old")).to_owned()).await?;

        // === Phase 5: Create booking tables ===
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
                            .to(TaskHeader::Table, TaskHeader::Id)
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
                            .to(ResourceHeader::Table, ResourceHeader::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // === Phase 6: Copy bookings from allocation table ===
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

        // === Phase 7: Recreate tables with corrected FK targets ===
        // All rev_created/rev_deleted get FK to revision with ON DELETE RESTRICT.
        // task-referencing FKs now point to task_header.
        // resource-referencing FKs now point to resource_header.

        // -- dependency (predecessor_id/successor_id -> task_header) --
        manager
            .rename_table(
                Table::rename()
                    .table(Alias::new("dependency"), Alias::new("_dependency_old"))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(DependencyFull::Table)
                    .col(pk_auto(DependencyFull::Id))
                    .col(integer(DependencyFull::PredecessorId))
                    .col(integer(DependencyFull::SuccessorId))
                    .col(ColumnDef::new(DependencyFull::RevCreated).big_integer().not_null())
                    .col(ColumnDef::new(DependencyFull::RevDeleted).big_integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Dependency_PredecessorId")
                            .from(DependencyFull::Table, DependencyFull::PredecessorId)
                            .to(TaskHeader::Table, TaskHeader::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Dependency_SuccessorId")
                            .from(DependencyFull::Table, DependencyFull::SuccessorId)
                            .to(TaskHeader::Table, TaskHeader::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Dependency_RevCreated")
                            .from(DependencyFull::Table, DependencyFull::RevCreated)
                            .to(Revision::Table, Revision::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Dependency_RevDeleted")
                            .from(DependencyFull::Table, DependencyFull::RevDeleted)
                            .to(Revision::Table, Revision::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        let insert_dependencies = Query::insert()
            .into_table(DependencyFull::Table)
            .columns([
                DependencyFull::Id,
                DependencyFull::PredecessorId,
                DependencyFull::SuccessorId,
                DependencyFull::RevCreated,
                DependencyFull::RevDeleted,
            ])
            .select_from(
                Query::select()
                    .column(Alias::new("id"))
                    .column(Alias::new("predecessor_id"))
                    .column(Alias::new("successor_id"))
                    .expr(Expr::value(1))
                    .expr(Expr::value(Value::BigUnsigned(None)))
                    .from(Alias::new("_dependency_old"))
                    .to_owned(),
            )
            .map_err(|e| DbErr::Custom(e.to_string()))?
            .to_owned();
        db.execute(backend.build(&insert_dependencies)).await?;

        manager.drop_table(Table::drop().table(Alias::new("_dependency_old")).to_owned()).await?;

        manager
            .create_index(
                Index::create()
                    .name("IDX_Dependency_PredecessorId")
                    .table(DependencyFull::Table)
                    .col(DependencyFull::PredecessorId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("IDX_Dependency_SuccessorId")
                    .table(DependencyFull::Table)
                    .col(DependencyFull::SuccessorId)
                    .to_owned(),
            )
            .await?;

        // -- vacation (resource_id -> resource_header) --
        manager
            .rename_table(
                Table::rename()
                    .table(Alias::new("vacation"), Alias::new("_vacation_old"))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(VacationFull::Table)
                    .col(pk_auto(VacationFull::Id))
                    .col(integer(VacationFull::ResourceId))
                    .col(timestamp(VacationFull::From))
                    .col(timestamp(VacationFull::Until))
                    .col(ColumnDef::new(VacationFull::RevCreated).big_integer().not_null())
                    .col(ColumnDef::new(VacationFull::RevDeleted).big_integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Vacation_ResourceId")
                            .from(VacationFull::Table, VacationFull::ResourceId)
                            .to(ResourceHeader::Table, ResourceHeader::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Vacation_RevCreated")
                            .from(VacationFull::Table, VacationFull::RevCreated)
                            .to(Revision::Table, Revision::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Vacation_RevDeleted")
                            .from(VacationFull::Table, VacationFull::RevDeleted)
                            .to(Revision::Table, Revision::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        let insert_vacations = Query::insert()
            .into_table(VacationFull::Table)
            .columns([
                VacationFull::Id,
                VacationFull::ResourceId,
                VacationFull::From,
                VacationFull::Until,
                VacationFull::RevCreated,
                VacationFull::RevDeleted,
            ])
            .select_from(
                Query::select()
                    .column(Alias::new("id"))
                    .column(Alias::new("resource_id"))
                    .column(Alias::new("from"))
                    .column(Alias::new("until"))
                    .expr(Expr::value(1))
                    .expr(Expr::value(Value::BigUnsigned(None)))
                    .from(Alias::new("_vacation_old"))
                    .to_owned(),
            )
            .map_err(|e| DbErr::Custom(e.to_string()))?
            .to_owned();
        db.execute(backend.build(&insert_vacations)).await?;

        manager.drop_table(Table::drop().table(Alias::new("_vacation_old")).to_owned()).await?;

        // -- availability (resource_id -> resource_header) --
        manager
            .rename_table(
                Table::rename()
                    .table(Alias::new("availability"), Alias::new("_availability_old"))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AvailabilityFull::Table)
                    .col(pk_auto(AvailabilityFull::Id))
                    .col(integer(AvailabilityFull::ResourceId))
                    .col(string_len(AvailabilityFull::Weekday, 2).not_null())
                    .col(decimal(AvailabilityFull::Duration).not_null())
                    .col(big_integer(AvailabilityFull::RevCreated).not_null())
                    .col(ColumnDef::new(AvailabilityFull::RevDeleted).big_integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Availability_ResourceId")
                            .from(AvailabilityFull::Table, AvailabilityFull::ResourceId)
                            .to(ResourceHeader::Table, ResourceHeader::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Availability_RevCreated")
                            .from(AvailabilityFull::Table, AvailabilityFull::RevCreated)
                            .to(Revision::Table, Revision::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Availability_RevDeleted")
                            .from(AvailabilityFull::Table, AvailabilityFull::RevDeleted)
                            .to(Revision::Table, Revision::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        let insert_availabilities = Query::insert()
            .into_table(AvailabilityFull::Table)
            .columns([
                AvailabilityFull::Id,
                AvailabilityFull::ResourceId,
                AvailabilityFull::Weekday,
                AvailabilityFull::Duration,
                AvailabilityFull::RevCreated,
                AvailabilityFull::RevDeleted,
            ])
            .select_from(
                Query::select()
                    .column(Alias::new("id"))
                    .column(Alias::new("resource_id"))
                    .column(Alias::new("weekday"))
                    .column(Alias::new("duration"))
                    .expr(Expr::value(1))
                    .expr(Expr::value(Value::BigUnsigned(None)))
                    .from(Alias::new("_availability_old"))
                    .to_owned(),
            )
            .map_err(|e| DbErr::Custom(e.to_string()))?
            .to_owned();
        db.execute(backend.build(&insert_availabilities)).await?;

        manager.drop_table(Table::drop().table(Alias::new("_availability_old")).to_owned()).await?;

        // -- resource_constraint (task_id -> task_header) --
        manager
            .rename_table(
                Table::rename()
                    .table(
                        Alias::new("resource_constraint"),
                        Alias::new("_resource_constraint_old"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ResourceConstraintFull::Table)
                    .col(pk_auto(ResourceConstraintFull::Id))
                    .col(integer(ResourceConstraintFull::TaskId))
                    .col(string(ResourceConstraintFull::Type))
                    .col(boolean(ResourceConstraintFull::Optional))
                    .col(ColumnDef::new(ResourceConstraintFull::Speed).float().not_null())
                    .col(
                        ColumnDef::new(ResourceConstraintFull::RevCreated).big_integer().not_null(),
                    )
                    .col(ColumnDef::new(ResourceConstraintFull::RevDeleted).big_integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_ResourceConstraint_TaskId")
                            .from(ResourceConstraintFull::Table, ResourceConstraintFull::TaskId)
                            .to(TaskHeader::Table, TaskHeader::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_ResourceConstraint_RevCreated")
                            .from(ResourceConstraintFull::Table, ResourceConstraintFull::RevCreated)
                            .to(Revision::Table, Revision::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_ResourceConstraint_RevDeleted")
                            .from(ResourceConstraintFull::Table, ResourceConstraintFull::RevDeleted)
                            .to(Revision::Table, Revision::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        let insert_resource_constraints = Query::insert()
            .into_table(ResourceConstraintFull::Table)
            .columns([
                ResourceConstraintFull::Id,
                ResourceConstraintFull::TaskId,
                ResourceConstraintFull::Type,
                ResourceConstraintFull::Optional,
                ResourceConstraintFull::Speed,
                ResourceConstraintFull::RevCreated,
                ResourceConstraintFull::RevDeleted,
            ])
            .select_from(
                Query::select()
                    .column(Alias::new("id"))
                    .column(Alias::new("task_id"))
                    .column(Alias::new("type"))
                    .column(Alias::new("optional"))
                    .column(Alias::new("speed"))
                    .expr(Expr::value(1))
                    .expr(Expr::value(Value::BigUnsigned(None)))
                    .from(Alias::new("_resource_constraint_old"))
                    .to_owned(),
            )
            .map_err(|e| DbErr::Custom(e.to_string()))?
            .to_owned();
        db.execute(backend.build(&insert_resource_constraints)).await?;

        manager
            .drop_table(Table::drop().table(Alias::new("_resource_constraint_old")).to_owned())
            .await?;

        // -- resource_constraint_entry (resource_id -> resource_header) --
        manager
            .rename_table(
                Table::rename()
                    .table(
                        Alias::new("resource_constraint_entry"),
                        Alias::new("_resource_constraint_entry_old"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ResourceConstraintEntry::Table)
                    .col(pk_auto(ResourceConstraintEntry::Id))
                    .col(integer(ResourceConstraintEntry::ResourceConstraintId))
                    .col(integer(ResourceConstraintEntry::ResourceId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_ResourceConstraintEntry_ResourceConstraintId")
                            .from(
                                ResourceConstraintEntry::Table,
                                ResourceConstraintEntry::ResourceConstraintId,
                            )
                            .to(ResourceConstraintFull::Table, ResourceConstraintFull::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_ResourceConstraintEntry_ResourceId")
                            .from(
                                ResourceConstraintEntry::Table,
                                ResourceConstraintEntry::ResourceId,
                            )
                            .to(ResourceHeader::Table, ResourceHeader::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        let insert_resource_constraint_entries = Query::insert()
            .into_table(ResourceConstraintEntry::Table)
            .columns([
                ResourceConstraintEntry::Id,
                ResourceConstraintEntry::ResourceConstraintId,
                ResourceConstraintEntry::ResourceId,
            ])
            .select_from(
                Query::select()
                    .column(Alias::new("id"))
                    .column(Alias::new("resource_constraint_id"))
                    .column(Alias::new("resource_id"))
                    .from(Alias::new("_resource_constraint_entry_old"))
                    .to_owned(),
            )
            .map_err(|e| DbErr::Custom(e.to_string()))?
            .to_owned();
        db.execute(backend.build(&insert_resource_constraint_entries)).await?;

        manager
            .drop_table(
                Table::drop().table(Alias::new("_resource_constraint_entry_old")).to_owned(),
            )
            .await?;

        // -- allocation (task_id -> task_header, empty after cleanup, without allocation_type/final) --
        manager
            .rename_table(
                Table::rename()
                    .table(Alias::new("allocation"), Alias::new("_allocation_old"))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AllocationFull::Table)
                    .col(pk_auto(AllocationFull::Id))
                    .col(integer(AllocationFull::TaskId))
                    .col(timestamp(AllocationFull::Start))
                    .col(timestamp(AllocationFull::End))
                    .col(ColumnDef::new(AllocationFull::RevCreated).big_integer().not_null())
                    .col(ColumnDef::new(AllocationFull::RevDeleted).big_integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Allocation_TaskId")
                            .from(AllocationFull::Table, AllocationFull::TaskId)
                            .to(TaskHeader::Table, TaskHeader::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Allocation_RevCreated")
                            .from(AllocationFull::Table, AllocationFull::RevCreated)
                            .to(Revision::Table, Revision::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Allocation_RevDeleted")
                            .from(AllocationFull::Table, AllocationFull::RevDeleted)
                            .to(Revision::Table, Revision::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager.drop_table(Table::drop().table(Alias::new("_allocation_old")).to_owned()).await?;

        // -- allocated_resource (resource_id -> resource_header, empty after cleanup) --
        manager
            .rename_table(
                Table::rename()
                    .table(Alias::new("allocated_resource"), Alias::new("_allocated_resource_old"))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AllocatedResourceFull::Table)
                    .col(pk_auto(AllocatedResourceFull::Id))
                    .col(integer(AllocatedResourceFull::AllocationId))
                    .col(integer(AllocatedResourceFull::ResourceId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_AllocatedResource_AllocationId")
                            .from(AllocatedResourceFull::Table, AllocatedResourceFull::AllocationId)
                            .to(AllocationFull::Table, AllocationFull::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_AllocatedResource_ResourceId")
                            .from(AllocatedResourceFull::Table, AllocatedResourceFull::ResourceId)
                            .to(ResourceHeader::Table, ResourceHeader::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(Alias::new("_allocated_resource_old")).to_owned())
            .await?;

        // -- issue (task_id -> task_header) --
        manager
            .rename_table(
                Table::rename().table(Alias::new("issue"), Alias::new("_issue_old")).to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(IssueFull::Table)
                    .col(pk_auto(IssueFull::Id))
                    .col(integer(IssueFull::Code))
                    .col(string(IssueFull::Description))
                    .col(string(IssueFull::Type))
                    .col(ColumnDef::new(IssueFull::TaskId).integer().null())
                    .col(ColumnDef::new(IssueFull::RevCreated).big_integer().not_null())
                    .col(ColumnDef::new(IssueFull::RevDeleted).big_integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Issue_TaskId")
                            .from(IssueFull::Table, IssueFull::TaskId)
                            .to(TaskHeader::Table, TaskHeader::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Issue_RevCreated")
                            .from(IssueFull::Table, IssueFull::RevCreated)
                            .to(Revision::Table, Revision::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Issue_RevDeleted")
                            .from(IssueFull::Table, IssueFull::RevDeleted)
                            .to(Revision::Table, Revision::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        let insert_issues = Query::insert()
            .into_table(IssueFull::Table)
            .columns([
                IssueFull::Id,
                IssueFull::Code,
                IssueFull::Description,
                IssueFull::Type,
                IssueFull::TaskId,
                IssueFull::RevCreated,
                IssueFull::RevDeleted,
            ])
            .select_from(
                Query::select()
                    .column(Alias::new("id"))
                    .column(Alias::new("code"))
                    .column(Alias::new("description"))
                    .column(Alias::new("type"))
                    .column(Alias::new("task_id"))
                    .expr(Expr::value(1))
                    .expr(Expr::value(Value::BigUnsigned(None)))
                    .from(Alias::new("_issue_old"))
                    .to_owned(),
            )
            .map_err(|e| DbErr::Custom(e.to_string()))?
            .to_owned();
        db.execute(backend.build(&insert_issues)).await?;

        manager.drop_table(Table::drop().table(Alias::new("_issue_old")).to_owned()).await?;

        if is_sqlite {
            db.execute_unprepared("PRAGMA legacy_alter_table=OFF").await?;
            db.execute_unprepared("PRAGMA foreign_keys=ON").await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let db = manager.get_connection();

        let delete_booking = Query::delete()
            .from_table(Booking::Table)
            .and_where(Expr::col(Booking::RevDeleted).is_not_null())
            .to_owned();
        db.execute(backend.build(&delete_booking)).await?;

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

        let delete_issue = Query::delete()
            .from_table(Issue::Table)
            .and_where(Expr::col(Issue::RevDeleted).is_not_null())
            .to_owned();
        db.execute(backend.build(&delete_issue)).await?;
        Ok(())
    }
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum Revision {
    Table,
    Id,
    Timestamp,
    PlanState,
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum Task {
    Table,
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum Resource {
    Table,
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum TaskHeader {
    Table,
    Id,
    RevCreated,
    RevDeleted,
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum ResourceHeader {
    Table,
    Id,
    RevCreated,
    RevDeleted,
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum TaskIteration {
    Table,
    Id,
    ParentId,
    HeaderId,
    RevCreated,
    RevDeleted,
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum ResourceIteration {
    Table,
    Id,
    HeaderId,
    RevCreated,
    RevDeleted,
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum Dependency {
    Table,
    RevCreated,
    RevDeleted,
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum Vacation {
    Table,
    RevCreated,
    RevDeleted,
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum Availability {
    Table,
    RevCreated,
    RevDeleted,
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum ResourceConstraint {
    Table,
    RevCreated,
    RevDeleted,
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
    RevCreated,
    RevDeleted,
}

#[allow(dead_code)]
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

#[allow(dead_code)]
#[derive(DeriveIden)]
enum BookingResource {
    Table,
    Id,
    BookingId,
    ResourceId,
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum AllocatedResource {
    Table,
    AllocationId,
    ResourceId,
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum Issue {
    Table,
    RevCreated,
    RevDeleted,
}

// --- Full column enums for table recreation ---

#[allow(dead_code)]
#[derive(DeriveIden)]
enum Holiday {
    Table,
    Id,
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum TaskIterationFull {
    #[sea_orm(iden = "task_iteration")]
    Table,
    Id,
    ParentId,
    Title,
    Description,
    Designation,
    EarliestStart,
    ScheduleTarget,
    Effort,
    Priority,
    HeaderId,
    RevCreated,
    RevDeleted,
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum ResourceIterationFull {
    #[sea_orm(iden = "resource_iteration")]
    Table,
    Id,
    Name,
    Timezone,
    Added,
    Removed,
    HolidayId,
    HeaderId,
    RevCreated,
    RevDeleted,
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum DependencyFull {
    #[sea_orm(iden = "dependency")]
    Table,
    Id,
    PredecessorId,
    SuccessorId,
    RevCreated,
    RevDeleted,
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum VacationFull {
    #[sea_orm(iden = "vacation")]
    Table,
    Id,
    ResourceId,
    From,
    Until,
    RevCreated,
    RevDeleted,
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum AvailabilityFull {
    #[sea_orm(iden = "availability")]
    Table,
    Id,
    ResourceId,
    Weekday,
    Duration,
    RevCreated,
    RevDeleted,
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum ResourceConstraintFull {
    #[sea_orm(iden = "resource_constraint")]
    Table,
    Id,
    TaskId,
    Type,
    Optional,
    Speed,
    RevCreated,
    RevDeleted,
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum ResourceConstraintEntry {
    Table,
    Id,
    ResourceConstraintId,
    ResourceId,
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum AllocationFull {
    #[sea_orm(iden = "allocation")]
    Table,
    Id,
    TaskId,
    Start,
    End,
    RevCreated,
    RevDeleted,
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum AllocatedResourceFull {
    #[sea_orm(iden = "allocated_resource")]
    Table,
    Id,
    AllocationId,
    ResourceId,
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum IssueFull {
    #[sea_orm(iden = "issue")]
    Table,
    Id,
    Code,
    Description,
    Type,
    TaskId,
    RevCreated,
    RevDeleted,
}
