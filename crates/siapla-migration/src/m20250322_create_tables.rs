use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts

        manager
            .create_table(
                Table::create()
                    .table(Task::Table)
                    .if_not_exists()
                    .col(pk_auto(Task::Id))
                    .col(integer_null(Task::ParentId))
                    .col(string(Task::Title))
                    .col(string(Task::Description))
                    .col(string(Task::Designation))
                    .col(timestamp_null(Task::EarliestStart))
                    .col(timestamp_null(Task::ScheduleTarget))
                    .col(float_null(Task::Effort))
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Task_Task")
                            .from(Task::Table, Task::ParentId)
                            .to(Task::Table, Task::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Dependency::Table)
                    .if_not_exists()
                    .col(pk_auto(Dependency::Id))
                    .col(integer(Dependency::PredecessorId))
                    .col(integer(Dependency::SuccessorId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Dependency_Predecessor")
                            .from(Dependency::Table, Dependency::PredecessorId)
                            .to(Task::Table, Task::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Dependency_Successor")
                            .from(Dependency::Table, Dependency::SuccessorId)
                            .to(Task::Table, Task::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("IDX_Dependency_PredecessorId")
                    .table(Dependency::Table)
                    .col(Dependency::PredecessorId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("IDX_Dependency_SuccessorId")
                    .table(Dependency::Table)
                    .col(Dependency::SuccessorId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Holiday::Table)
                    .if_not_exists()
                    .col(pk_auto(Holiday::Id))
                    .col(string(Holiday::Name))
                    .col(date(Holiday::Start))
                    .col(date(Holiday::End))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(HolidayEntry::Table)
                    .if_not_exists()
                    .col(pk_auto(HolidayEntry::Id))
                    .col(integer(HolidayEntry::HolidayId))
                    .col(string_null(HolidayEntry::Name))
                    .col(date(HolidayEntry::Date))
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_HolidayEntry_Holiday")
                            .from(HolidayEntry::Table, HolidayEntry::HolidayId)
                            .to(Holiday::Table, Holiday::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Resource::Table)
                    .if_not_exists()
                    .col(pk_auto(Resource::Id))
                    .col(string(Resource::Name))
                    .col(string(Resource::Timezone))
                    .col(timestamp(Resource::Added))
                    .col(timestamp_null(Resource::Removed))
                    .col(integer_null(Resource::HolidayId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Resource_Holiday")
                            .from(Resource::Table, Resource::HolidayId)
                            .to(Holiday::Table, Holiday::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Availability::Table)
                    .if_not_exists()
                    .col(pk_auto(Availability::Id))
                    .col(integer(Availability::ResourceId))
                    .col(string_len(Availability::Weekday, 2)) // MO, TU, WE, TH, FR, SA, SU
                    .col(decimal(Availability::Duration)) // hours
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Availability_Resource")
                            .from(Availability::Table, Availability::ResourceId)
                            .to(Resource::Table, Resource::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Vacation::Table)
                    .if_not_exists()
                    .col(pk_auto(Vacation::Id))
                    .col(integer(Vacation::ResourceId))
                    .col(timestamp(Vacation::From))
                    .col(timestamp(Vacation::Until))
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Vacation_Resource")
                            .from(Vacation::Table, Vacation::ResourceId)
                            .to(Resource::Table, Resource::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ResourceConstraint::Table)
                    .if_not_exists()
                    .col(pk_auto(ResourceConstraint::Id))
                    .col(integer(ResourceConstraint::TaskId))
                    .col(string(ResourceConstraint::Type))
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_ResourceConstraint_Resource")
                            .from(ResourceConstraint::Table, ResourceConstraint::TaskId)
                            .to(Resource::Table, Resource::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ResourceConstraintEntry::Table)
                    .if_not_exists()
                    .col(pk_auto(ResourceConstraintEntry::Id))
                    .col(integer(ResourceConstraintEntry::ResourceConstraintId))
                    .col(string(ResourceConstraintEntry::ResourceId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_ResourceConstraintEntry_ResourceConstraint")
                            .from(
                                ResourceConstraintEntry::Table,
                                ResourceConstraintEntry::ResourceConstraintId,
                            )
                            .to(ResourceConstraint::Table, ResourceConstraint::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_ResourceConstraintEntry_Resource")
                            .from(
                                ResourceConstraintEntry::Table,
                                ResourceConstraintEntry::ResourceId,
                            )
                            .to(Resource::Table, Resource::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Allocation::Table)
                    .if_not_exists()
                    .col(pk_auto(Allocation::Id))
                    .col(integer(Allocation::TaskId))
                    .col(timestamp(Allocation::Start))
                    .col(timestamp(Allocation::End))
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_Allocation_Task")
                            .from(Allocation::Table, Allocation::TaskId)
                            .to(Task::Table, Task::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AllocatedResource::Table)
                    .if_not_exists()
                    .col(pk_auto(AllocatedResource::Id))
                    .col(integer(AllocatedResource::AllocationId))
                    .col(integer(AllocatedResource::ResourceId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_AllocatedResource_Allocation")
                            .from(AllocatedResource::Table, AllocatedResource::AllocationId)
                            .to(Allocation::Table, Allocation::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_AllocatedResource_Allocation")
                            .from(AllocatedResource::Table, AllocatedResource::ResourceId)
                            .to(Resource::Table, Resource::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(AllocatedResource::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(Allocation::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(ResourceConstraintEntry::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(ResourceConstraint::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(Resource::Table).if_exists().to_owned())
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(Dependency::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(Task::Table).if_exists().to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Task {
    Table,
    Id,
    ParentId,
    Title,
    Description,
    Designation,
    EarliestStart,
    ScheduleTarget,
    Effort,
}

#[derive(DeriveIden)]
enum Dependency {
    Table,
    Id,
    PredecessorId,
    SuccessorId,
}

#[derive(DeriveIden)]
enum Holiday {
    Table,
    Id,
    Name,
    Start,
    End,
}

#[derive(DeriveIden)]
enum HolidayEntry {
    Table,
    Id,
    HolidayId,
    Name,
    Date,
}
#[derive(DeriveIden)]
enum Resource {
    Table,
    Id,
    Name,
    Timezone,
    Added,
    Removed,
    HolidayId,
}

#[derive(DeriveIden)]
enum Vacation {
    Table,
    Id,
    ResourceId,
    From,
    Until,
}

#[derive(DeriveIden)]
enum Availability {
    Table,
    Id,
    ResourceId,
    Weekday,
    Duration,
}

#[derive(DeriveIden)]
enum ResourceConstraint {
    Table,
    Id,
    TaskId,
    Type,
}

#[derive(DeriveIden)]
enum ResourceConstraintEntry {
    Table,
    Id,
    ResourceConstraintId,
    ResourceId,
}

#[derive(DeriveIden)]
enum Allocation {
    Table,
    Id,
    TaskId,
    Start,
    End,
}

#[derive(DeriveIden)]
enum AllocatedResource {
    Table,
    Id,
    AllocationId,
    ResourceId,
}
