use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{big_unsigned, boolean, integer, pk_auto, string, timestamp};
use sea_query::{Expr, OnConflict, Query};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let db = manager.get_connection();

        db.execute_unprepared("PRAGMA foreign_keys=OFF").await?;
        db.execute_unprepared("PRAGMA legacy_alter_table=ON").await?;

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
        db.execute_unprepared("ALTER TABLE \"task\" RENAME TO \"_task_old\"")
            .await?;

        db.execute_unprepared(concat!(
            "CREATE TABLE \"task_iteration\" (",
            " \"id\" integer NOT NULL PRIMARY KEY AUTOINCREMENT,",
            " \"parent_id\" integer NULL,",
            " \"title\" varchar NOT NULL,",
            " \"description\" varchar NOT NULL,",
            " \"designation\" varchar NOT NULL,",
            " \"earliest_start\" timestamp_text NULL,",
            " \"schedule_target\" timestamp_text NULL,",
            " \"effort\" float NULL,",
            " \"priority\" float NOT NULL DEFAULT 1,",
            " \"header_id\" integer NOT NULL,",
            " \"rev_created\" bigint NOT NULL,",
            " \"rev_deleted\" bigint NULL,",
            " FOREIGN KEY (\"parent_id\") REFERENCES \"task_header\" (\"id\") ON DELETE SET NULL ON UPDATE CASCADE,",
            " FOREIGN KEY (\"header_id\") REFERENCES \"task_header\" (\"id\") ON DELETE CASCADE ON UPDATE CASCADE,",
            " FOREIGN KEY (\"rev_created\") REFERENCES \"revision\" (\"id\") ON DELETE RESTRICT ON UPDATE CASCADE,",
            " FOREIGN KEY (\"rev_deleted\") REFERENCES \"revision\" (\"id\") ON DELETE RESTRICT ON UPDATE CASCADE",
            ")"
        ))
        .await?;

        db.execute_unprepared(concat!(
            "INSERT INTO \"task_header\" (\"id\", \"rev_created\", \"rev_deleted\")",
            " SELECT \"id\", 1, NULL FROM \"_task_old\""
        ))
        .await?;

        db.execute_unprepared(concat!(
            "INSERT INTO \"task_iteration\"",
            " (\"id\", \"parent_id\", \"title\", \"description\", \"designation\",",
            " \"earliest_start\", \"schedule_target\", \"effort\", \"priority\",",
            " \"header_id\", \"rev_created\", \"rev_deleted\")",
            " SELECT \"id\", \"parent_id\", \"title\", \"description\", \"designation\",",
            " \"earliest_start\", \"schedule_target\", \"effort\", \"priority\",",
            " \"id\", 1, NULL FROM \"_task_old\""
        ))
        .await?;

        db.execute_unprepared(concat!(
            "UPDATE \"task_iteration\" SET \"parent_id\" = (",
            " SELECT parent.\"header_id\" FROM \"task_iteration\" AS parent",
            " WHERE parent.\"id\" = \"task_iteration\".\"parent_id\"",
            ") WHERE \"parent_id\" IS NOT NULL"
        ))
        .await?;

        db.execute_unprepared("DROP TABLE \"_task_old\"").await?;

        // === Phase 4: Recreate resource -> resource_iteration with proper FKs ===
        db.execute_unprepared("ALTER TABLE \"resource\" RENAME TO \"_resource_old\"")
            .await?;

        db.execute_unprepared(concat!(
            "CREATE TABLE \"resource_iteration\" (",
            " \"id\" integer NOT NULL PRIMARY KEY AUTOINCREMENT,",
            " \"name\" varchar NOT NULL,",
            " \"timezone\" varchar NOT NULL,",
            " \"added\" timestamp_text NOT NULL,",
            " \"removed\" timestamp_text NULL,",
            " \"holiday_id\" integer NULL,",
            " \"header_id\" integer NOT NULL,",
            " \"rev_created\" bigint NOT NULL,",
            " \"rev_deleted\" bigint NULL,",
            " FOREIGN KEY (\"holiday_id\") REFERENCES \"holiday\" (\"id\") ON DELETE SET NULL ON UPDATE SET NULL,",
            " FOREIGN KEY (\"header_id\") REFERENCES \"resource_header\" (\"id\") ON DELETE CASCADE ON UPDATE CASCADE,",
            " FOREIGN KEY (\"rev_created\") REFERENCES \"revision\" (\"id\") ON DELETE RESTRICT ON UPDATE CASCADE,",
            " FOREIGN KEY (\"rev_deleted\") REFERENCES \"revision\" (\"id\") ON DELETE RESTRICT ON UPDATE CASCADE",
            ")"
        ))
        .await?;

        db.execute_unprepared(concat!(
            "INSERT INTO \"resource_header\" (\"id\", \"rev_created\", \"rev_deleted\")",
            " SELECT \"id\", 1, NULL FROM \"_resource_old\""
        ))
        .await?;

        db.execute_unprepared(concat!(
            "INSERT INTO \"resource_iteration\"",
            " (\"id\", \"name\", \"timezone\", \"added\", \"removed\", \"holiday_id\",",
            " \"header_id\", \"rev_created\", \"rev_deleted\")",
            " SELECT \"id\", \"name\", \"timezone\", \"added\", \"removed\", \"holiday_id\",",
            " \"id\", 1, NULL FROM \"_resource_old\""
        ))
        .await?;

        db.execute_unprepared("DROP TABLE \"_resource_old\"").await?;

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
        db.execute_unprepared("ALTER TABLE \"dependency\" RENAME TO \"_dependency_old\"")
            .await?;
        db.execute_unprepared(concat!(
            "CREATE TABLE \"dependency\" (",
            " \"id\" integer NOT NULL PRIMARY KEY AUTOINCREMENT,",
            " \"predecessor_id\" integer NOT NULL,",
            " \"successor_id\" integer NOT NULL,",
            " \"rev_created\" bigint NOT NULL,",
            " \"rev_deleted\" bigint NULL,",
            " FOREIGN KEY (\"predecessor_id\") REFERENCES \"task_header\" (\"id\") ON DELETE CASCADE ON UPDATE CASCADE,",
            " FOREIGN KEY (\"successor_id\") REFERENCES \"task_header\" (\"id\") ON DELETE CASCADE ON UPDATE CASCADE,",
            " FOREIGN KEY (\"rev_created\") REFERENCES \"revision\" (\"id\") ON DELETE RESTRICT ON UPDATE CASCADE,",
            " FOREIGN KEY (\"rev_deleted\") REFERENCES \"revision\" (\"id\") ON DELETE RESTRICT ON UPDATE CASCADE",
            ")"
        ))
        .await?;
        db.execute_unprepared(concat!(
            "INSERT INTO \"dependency\" (\"id\", \"predecessor_id\", \"successor_id\", \"rev_created\", \"rev_deleted\")",
            " SELECT \"id\", \"predecessor_id\", \"successor_id\", 1, NULL FROM \"_dependency_old\""
        ))
        .await?;
        db.execute_unprepared("DROP TABLE \"_dependency_old\"").await?;
        db.execute_unprepared(
            "CREATE INDEX \"IDX_Dependency_PredecessorId\" ON \"dependency\" (\"predecessor_id\")",
        )
        .await?;
        db.execute_unprepared(
            "CREATE INDEX \"IDX_Dependency_SuccessorId\" ON \"dependency\" (\"successor_id\")",
        )
        .await?;

        // -- vacation (resource_id -> resource_header) --
        db.execute_unprepared("ALTER TABLE \"vacation\" RENAME TO \"_vacation_old\"")
            .await?;
        db.execute_unprepared(concat!(
            "CREATE TABLE \"vacation\" (",
            " \"id\" integer NOT NULL PRIMARY KEY AUTOINCREMENT,",
            " \"resource_id\" integer NOT NULL,",
            " \"from\" timestamp_text NOT NULL,",
            " \"until\" timestamp_text NOT NULL,",
            " \"rev_created\" bigint NOT NULL,",
            " \"rev_deleted\" bigint NULL,",
            " FOREIGN KEY (\"resource_id\") REFERENCES \"resource_header\" (\"id\") ON DELETE CASCADE ON UPDATE CASCADE,",
            " FOREIGN KEY (\"rev_created\") REFERENCES \"revision\" (\"id\") ON DELETE RESTRICT ON UPDATE CASCADE,",
            " FOREIGN KEY (\"rev_deleted\") REFERENCES \"revision\" (\"id\") ON DELETE RESTRICT ON UPDATE CASCADE",
            ")"
        ))
        .await?;
        db.execute_unprepared(concat!(
            "INSERT INTO \"vacation\" (\"id\", \"resource_id\", \"from\", \"until\", \"rev_created\", \"rev_deleted\")",
            " SELECT \"id\", \"resource_id\", \"from\", \"until\", 1, NULL FROM \"_vacation_old\""
        ))
        .await?;
        db.execute_unprepared("DROP TABLE \"_vacation_old\"").await?;

        // -- availability (resource_id -> resource_header) --
        db.execute_unprepared("ALTER TABLE \"availability\" RENAME TO \"_availability_old\"")
            .await?;
        db.execute_unprepared(concat!(
            "CREATE TABLE \"availability\" (",
            " \"id\" integer NOT NULL PRIMARY KEY AUTOINCREMENT,",
            " \"resource_id\" integer NOT NULL,",
            " \"weekday\" varchar(2) NOT NULL,",
            " \"duration\" real NOT NULL,",
            " \"rev_created\" bigint NOT NULL,",
            " \"rev_deleted\" bigint NULL,",
            " FOREIGN KEY (\"resource_id\") REFERENCES \"resource_header\" (\"id\") ON DELETE CASCADE ON UPDATE CASCADE,",
            " FOREIGN KEY (\"rev_created\") REFERENCES \"revision\" (\"id\") ON DELETE RESTRICT ON UPDATE CASCADE,",
            " FOREIGN KEY (\"rev_deleted\") REFERENCES \"revision\" (\"id\") ON DELETE RESTRICT ON UPDATE CASCADE",
            ")"
        ))
        .await?;
        db.execute_unprepared(concat!(
            "INSERT INTO \"availability\" (\"id\", \"resource_id\", \"weekday\", \"duration\", \"rev_created\", \"rev_deleted\")",
            " SELECT \"id\", \"resource_id\", \"weekday\", \"duration\", 1, NULL FROM \"_availability_old\""
        ))
        .await?;
        db.execute_unprepared("DROP TABLE \"_availability_old\"").await?;

        // -- resource_constraint (task_id -> task_header) --
        db.execute_unprepared(
            "ALTER TABLE \"resource_constraint\" RENAME TO \"_resource_constraint_old\"",
        )
        .await?;
        db.execute_unprepared(concat!(
            "CREATE TABLE \"resource_constraint\" (",
            " \"id\" integer NOT NULL PRIMARY KEY AUTOINCREMENT,",
            " \"task_id\" integer NOT NULL,",
            " \"type\" varchar NOT NULL,",
            " \"optional\" boolean NOT NULL,",
            " \"speed\" float NOT NULL,",
            " \"rev_created\" bigint NOT NULL,",
            " \"rev_deleted\" bigint NULL,",
            " FOREIGN KEY (\"task_id\") REFERENCES \"task_header\" (\"id\") ON DELETE CASCADE ON UPDATE CASCADE,",
            " FOREIGN KEY (\"rev_created\") REFERENCES \"revision\" (\"id\") ON DELETE RESTRICT ON UPDATE CASCADE,",
            " FOREIGN KEY (\"rev_deleted\") REFERENCES \"revision\" (\"id\") ON DELETE RESTRICT ON UPDATE CASCADE",
            ")"
        ))
        .await?;
        db.execute_unprepared(concat!(
            "INSERT INTO \"resource_constraint\"",
            " (\"id\", \"task_id\", \"type\", \"optional\", \"speed\", \"rev_created\", \"rev_deleted\")",
            " SELECT \"id\", \"task_id\", \"type\", \"optional\", \"speed\", 1, NULL",
            " FROM \"_resource_constraint_old\""
        ))
        .await?;
        db.execute_unprepared("DROP TABLE \"_resource_constraint_old\"").await?;

        // -- resource_constraint_entry (resource_id -> resource_header) --
        db.execute_unprepared(
            "ALTER TABLE \"resource_constraint_entry\" RENAME TO \"_resource_constraint_entry_old\"",
        )
        .await?;
        db.execute_unprepared(concat!(
            "CREATE TABLE \"resource_constraint_entry\" (",
            " \"id\" integer NOT NULL PRIMARY KEY AUTOINCREMENT,",
            " \"resource_constraint_id\" integer NOT NULL,",
            " \"resource_id\" integer NOT NULL,",
            " FOREIGN KEY (\"resource_constraint_id\") REFERENCES \"resource_constraint\" (\"id\") ON DELETE CASCADE ON UPDATE CASCADE,",
            " FOREIGN KEY (\"resource_id\") REFERENCES \"resource_header\" (\"id\") ON DELETE RESTRICT ON UPDATE RESTRICT",
            ")"
        ))
        .await?;
        db.execute_unprepared(concat!(
            "INSERT INTO \"resource_constraint_entry\" (\"id\", \"resource_constraint_id\", \"resource_id\")",
            " SELECT \"id\", \"resource_constraint_id\", \"resource_id\" FROM \"_resource_constraint_entry_old\""
        ))
        .await?;
        db.execute_unprepared("DROP TABLE \"_resource_constraint_entry_old\"").await?;

        // -- allocation (task_id -> task_header, empty after cleanup, without allocation_type/final) --
        db.execute_unprepared("ALTER TABLE \"allocation\" RENAME TO \"_allocation_old\"")
            .await?;
        db.execute_unprepared(concat!(
            "CREATE TABLE \"allocation\" (",
            " \"id\" integer NOT NULL PRIMARY KEY AUTOINCREMENT,",
            " \"task_id\" integer NOT NULL,",
            " \"start\" timestamp_text NOT NULL,",
            " \"end\" timestamp_text NOT NULL,",
            " \"rev_created\" bigint NOT NULL,",
            " \"rev_deleted\" bigint NULL,",
            " FOREIGN KEY (\"task_id\") REFERENCES \"task_header\" (\"id\") ON DELETE CASCADE ON UPDATE CASCADE,",
            " FOREIGN KEY (\"rev_created\") REFERENCES \"revision\" (\"id\") ON DELETE RESTRICT ON UPDATE CASCADE,",
            " FOREIGN KEY (\"rev_deleted\") REFERENCES \"revision\" (\"id\") ON DELETE RESTRICT ON UPDATE CASCADE",
            ")"
        ))
        .await?;
        db.execute_unprepared("DROP TABLE \"_allocation_old\"").await?;

        // -- allocated_resource (resource_id -> resource_header, empty after cleanup) --
        db.execute_unprepared(
            "ALTER TABLE \"allocated_resource\" RENAME TO \"_allocated_resource_old\"",
        )
        .await?;
        db.execute_unprepared(concat!(
            "CREATE TABLE \"allocated_resource\" (",
            " \"id\" integer NOT NULL PRIMARY KEY AUTOINCREMENT,",
            " \"allocation_id\" integer NOT NULL,",
            " \"resource_id\" integer NOT NULL,",
            " FOREIGN KEY (\"allocation_id\") REFERENCES \"allocation\" (\"id\") ON DELETE CASCADE ON UPDATE CASCADE,",
            " FOREIGN KEY (\"resource_id\") REFERENCES \"resource_header\" (\"id\") ON DELETE RESTRICT ON UPDATE CASCADE",
            ")"
        ))
        .await?;
        db.execute_unprepared("DROP TABLE \"_allocated_resource_old\"").await?;

        // -- issue (task_id -> task_header) --
        db.execute_unprepared("ALTER TABLE \"issue\" RENAME TO \"_issue_old\"").await?;
        db.execute_unprepared(concat!(
            "CREATE TABLE \"issue\" (",
            " \"id\" integer NOT NULL PRIMARY KEY AUTOINCREMENT,",
            " \"code\" integer NOT NULL,",
            " \"description\" varchar NOT NULL,",
            " \"type\" varchar NOT NULL,",
            " \"task_id\" integer NULL,",
            " \"rev_created\" bigint NOT NULL,",
            " \"rev_deleted\" bigint NULL,",
            " FOREIGN KEY (\"task_id\") REFERENCES \"task_header\" (\"id\") ON DELETE SET NULL ON UPDATE CASCADE,",
            " FOREIGN KEY (\"rev_created\") REFERENCES \"revision\" (\"id\") ON DELETE RESTRICT ON UPDATE CASCADE,",
            " FOREIGN KEY (\"rev_deleted\") REFERENCES \"revision\" (\"id\") ON DELETE RESTRICT ON UPDATE CASCADE",
            ")"
        ))
        .await?;
        db.execute_unprepared(concat!(
            "INSERT INTO \"issue\"",
            " (\"id\", \"code\", \"description\", \"type\", \"task_id\", \"rev_created\", \"rev_deleted\")",
            " SELECT \"id\", \"code\", \"description\", \"type\", \"task_id\", 1, NULL FROM \"_issue_old\""
        ))
        .await?;
        db.execute_unprepared("DROP TABLE \"_issue_old\"").await?;

        db.execute_unprepared("PRAGMA legacy_alter_table=OFF").await?;
        db.execute_unprepared("PRAGMA foreign_keys=ON").await?;

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
