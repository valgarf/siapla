import { test, expect } from "@playwright/test";
import {
    graphqlRequest,
    standardAvailability,
    getLatestRevision,
    queryTasksAtRevision,
    queryResourcesAtRevision,
    queryBookingsAtRevision,
} from "../helpers/graphql";
import { cleanDatabase } from "../helpers/cleanup";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const TASK_SAVE = `
    mutation TaskSave($task: TaskSaveInput!) {
        taskSave(task: $task) { dbId title designation }
    }
`;

const RESOURCE_SAVE = `
    mutation ResourceSave($resource: ResourceSaveInput!) {
        resourceSave(resource: $resource) { dbId name timezone }
    }
`;

async function createTask(
    fields: Record<string, unknown>,
): Promise<{ dbId: number; title: string; designation: string }> {
    const result = await graphqlRequest<{
        taskSave: { dbId: number; title: string; designation: string };
    }>(TASK_SAVE, {
        task: {
            description: "",
            priority: 1.0,
            ...fields,
        },
    });
    return result.taskSave;
}

async function createResource(
    name: string,
    timezone = "Europe/Berlin",
): Promise<{ dbId: number; name: string; timezone: string }> {
    const result = await graphqlRequest<{
        resourceSave: { dbId: number; name: string; timezone: string };
    }>(RESOURCE_SAVE, {
        resource: {
            name,
            timezone,
            added: new Date().toISOString(),
            availability: standardAvailability(),
        },
    });
    return result.resourceSave;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

test.beforeEach(async () => {
    await cleanDatabase();
});

test.describe("Revision System", () => {
    // -----------------------------------------------------------------------
    // Basic revision tracking
    // -----------------------------------------------------------------------

    test.describe("Basic revision tracking", () => {
        test("latestRevision increases after creating a task", async () => {
            const rev1 = await getLatestRevision();

            await createTask({
                title: "Rev Test Task",
                designation: "TASK",
                effort: 4.0,
            });

            const rev2 = await getLatestRevision();
            expect(rev2).toBeGreaterThan(rev1);
        });

        test("latestRevision increases after creating a resource", async () => {
            const rev1 = await getLatestRevision();

            await createResource("Rev Test Resource");

            const rev2 = await getLatestRevision();
            expect(rev2).toBeGreaterThan(rev1);
        });

        test("latestRevision increases after deleting a task", async () => {
            const task = await createTask({
                title: "Delete Me",
                designation: "TASK",
                effort: 4.0,
            });

            const rev1 = await getLatestRevision();

            await graphqlRequest(
                `mutation TaskDelete($taskId: Int!) { taskDelete(taskId: $taskId) }`,
                { taskId: task.dbId },
            );

            const rev2 = await getLatestRevision();
            expect(rev2).toBeGreaterThan(rev1);
        });

        test("latestRevision increases after deleting a resource", async () => {
            const resource = await createResource("Delete Me Resource");
            const rev1 = await getLatestRevision();

            await graphqlRequest(
                `mutation ResourceDelete($resourceId: Int!) { resourceDelete(resourceId: $resourceId) }`,
                { resourceId: resource.dbId },
            );

            const rev2 = await getLatestRevision();
            expect(rev2).toBeGreaterThan(rev1);
        });
    });

    // -----------------------------------------------------------------------
    // Task revision: old state queryable after changes
    // -----------------------------------------------------------------------

    test.describe("Task revisions: old state queryable after changes", () => {
        test("after modifying a task, old revision returns old data", async () => {
            // Create a task
            const task = await createTask({
                title: "Original Title",
                description: "Original description",
                designation: "TASK",
                effort: 8.0,
            });

            const revAfterCreate = await getLatestRevision();

            // Verify task exists at this revision
            const tasksAtOldRev = await queryTasksAtRevision(revAfterCreate);
            const taskAtOld = tasksAtOldRev.tasks.find(
                (t) => t.dbId === task.dbId,
            );
            expect(taskAtOld).toBeDefined();
            expect(taskAtOld!.title).toBe("Original Title");
            expect(taskAtOld!.effort).toBe(8.0);

            // Update the task (creates new iteration with new dbId)
            const updated = await graphqlRequest<{
                taskSave: { dbId: number; title: string };
            }>(TASK_SAVE, {
                task: {
                    dbId: task.dbId,
                    title: "Updated Title",
                    description: "Updated description",
                    designation: "TASK",
                    priority: 1.0,
                    effort: 16.0,
                },
            });

            const revAfterUpdate = await getLatestRevision();
            expect(revAfterUpdate).toBeGreaterThan(revAfterCreate);

            // Query the OLD revision: should still return original data
            const tasksAtOldRevAgain =
                await queryTasksAtRevision(revAfterCreate);
            const taskAtOldAgain = tasksAtOldRevAgain.tasks.find(
                (t) => t.dbId === task.dbId,
            );
            expect(taskAtOldAgain).toBeDefined();
            expect(taskAtOldAgain!.title).toBe("Original Title");
            expect(taskAtOldAgain!.effort).toBe(8.0);

            // Query the NEW revision: should return updated data
            const tasksAtNewRev = await queryTasksAtRevision(revAfterUpdate);
            const taskAtNew = tasksAtNewRev.tasks.find(
                (t) => t.dbId === updated.taskSave.dbId,
            );
            expect(taskAtNew).toBeDefined();
            expect(taskAtNew!.title).toBe("Updated Title");
            expect(taskAtNew!.effort).toBe(16.0);
        });

        test("after deleting a task, old revision still returns it", async () => {
            const task = await createTask({
                title: "Will Be Deleted",
                designation: "TASK",
                effort: 4.0,
            });

            const revAfterCreate = await getLatestRevision();

            // Verify task exists at this revision
            const tasksAtCreate = await queryTasksAtRevision(revAfterCreate);
            expect(tasksAtCreate.tasks.some((t) => t.dbId === task.dbId)).toBe(
                true,
            );

            // Delete the task
            await graphqlRequest(
                `mutation TaskDelete($taskId: Int!) { taskDelete(taskId: $taskId) }`,
                { taskId: task.dbId },
            );

            const revAfterDelete = await getLatestRevision();

            // Old revision: task still present
            const tasksAtOldRev = await queryTasksAtRevision(revAfterCreate);
            expect(tasksAtOldRev.tasks.some((t) => t.dbId === task.dbId)).toBe(
                true,
            );

            // New revision: task gone
            const tasksAtNewRev = await queryTasksAtRevision(revAfterDelete);
            expect(tasksAtNewRev.tasks.some((t) => t.dbId === task.dbId)).toBe(
                false,
            );
        });

        test("deleted task is not returned for latest (no revision param)", async () => {
            const task = await createTask({
                title: "Ephemeral Task",
                designation: "TASK",
                effort: 4.0,
            });

            // Query without revision param (latest) – should have the task
            const before = await graphqlRequest<{
                tasks: Array<{ dbId: number; title: string }>;
            }>(`query { tasks { dbId title } }`);
            expect(before.tasks.some((t) => t.dbId === task.dbId)).toBe(true);

            // Delete
            await graphqlRequest(
                `mutation TaskDelete($taskId: Int!) { taskDelete(taskId: $taskId) }`,
                { taskId: task.dbId },
            );

            // Query latest again – task should be gone
            const after = await graphqlRequest<{
                tasks: Array<{ dbId: number; title: string }>;
            }>(`query { tasks { dbId title } }`);
            expect(after.tasks.some((t) => t.dbId === task.dbId)).toBe(false);
        });
    });

    // -----------------------------------------------------------------------
    // Resource revision: old state queryable after changes
    // -----------------------------------------------------------------------

    test.describe("Resource revisions: old state queryable after changes", () => {
        test("after modifying a resource, old revision returns old data", async () => {
            const resource = await createResource("Original Resource", "UTC");
            const revAfterCreate = await getLatestRevision();

            // Verify resource exists
            const resourcesAtOldRev =
                await queryResourcesAtRevision(revAfterCreate);
            const resAtOld = resourcesAtOldRev.resources.find(
                (r) => r.dbId === resource.dbId,
            );
            expect(resAtOld).toBeDefined();
            expect(resAtOld!.name).toBe("Original Resource");
            expect(resAtOld!.timezone).toBe("UTC");

            // Update the resource
            const updated = await graphqlRequest<{
                resourceSave: {
                    dbId: number;
                    name: string;
                    timezone: string;
                };
            }>(RESOURCE_SAVE, {
                resource: {
                    dbId: resource.dbId,
                    name: "Updated Resource",
                    timezone: "Europe/Berlin",
                    added: new Date().toISOString(),
                    availability: standardAvailability(),
                },
            });

            const revAfterUpdate = await getLatestRevision();

            // Old revision: original data
            const resourcesAtOldRevAgain =
                await queryResourcesAtRevision(revAfterCreate);
            const resAtOldAgain = resourcesAtOldRevAgain.resources.find(
                (r) => r.dbId === resource.dbId,
            );
            expect(resAtOldAgain).toBeDefined();
            expect(resAtOldAgain!.name).toBe("Original Resource");
            expect(resAtOldAgain!.timezone).toBe("UTC");

            // New revision: updated data
            const resourcesAtNewRev =
                await queryResourcesAtRevision(revAfterUpdate);
            const resAtNew = resourcesAtNewRev.resources.find(
                (r) => r.dbId === updated.resourceSave.dbId,
            );
            expect(resAtNew).toBeDefined();
            expect(resAtNew!.name).toBe("Updated Resource");
            expect(resAtNew!.timezone).toBe("Europe/Berlin");
        });

        test("after deleting a resource, old revision still returns it", async () => {
            const resource = await createResource("Will Be Deleted", "UTC");
            const revAfterCreate = await getLatestRevision();

            // Delete
            await graphqlRequest(
                `mutation ResourceDelete($resourceId: Int!) { resourceDelete(resourceId: $resourceId) }`,
                { resourceId: resource.dbId },
            );

            const revAfterDelete = await getLatestRevision();

            // Old revision: resource present
            const resourcesAtOldRev =
                await queryResourcesAtRevision(revAfterCreate);
            expect(
                resourcesAtOldRev.resources.some(
                    (r) => r.dbId === resource.dbId,
                ),
            ).toBe(true);

            // New revision: resource gone
            const resourcesAtNewRev =
                await queryResourcesAtRevision(revAfterDelete);
            expect(
                resourcesAtNewRev.resources.some(
                    (r) => r.dbId === resource.dbId,
                ),
            ).toBe(false);
        });

        test("deleted resource is not returned for latest (no revision param)", async () => {
            const resource = await createResource("Ephemeral Resource", "UTC");

            const before = await graphqlRequest<{
                resources: Array<{ dbId: number; name: string }>;
            }>(`query { resources { dbId name } }`);
            expect(before.resources.some((r) => r.dbId === resource.dbId)).toBe(
                true,
            );

            await graphqlRequest(
                `mutation ResourceDelete($resourceId: Int!) { resourceDelete(resourceId: $resourceId) }`,
                { resourceId: resource.dbId },
            );

            const after = await graphqlRequest<{
                resources: Array<{ dbId: number; name: string }>;
            }>(`query { resources { dbId name } }`);
            expect(after.resources.some((r) => r.dbId === resource.dbId)).toBe(
                false,
            );
        });
    });

    // -----------------------------------------------------------------------
    // No duplicates on modification
    // -----------------------------------------------------------------------

    test.describe("No duplicates on modification", () => {
        test("modifying a task does not create duplicates at the new revision", async () => {
            const task = await createTask({
                title: "Unique Task",
                designation: "TASK",
                effort: 8.0,
            });

            // Update
            await graphqlRequest(TASK_SAVE, {
                task: {
                    dbId: task.dbId,
                    title: "Unique Task Modified",
                    description: "",
                    designation: "TASK",
                    priority: 1.0,
                    effort: 12.0,
                },
            });

            const revAfterUpdate = await getLatestRevision();
            const tasksAtNew = await queryTasksAtRevision(revAfterUpdate);

            // Should have exactly 1 task, not 2
            expect(tasksAtNew.tasks).toHaveLength(1);
            expect(tasksAtNew.tasks[0]!.title).toBe("Unique Task Modified");
        });

        test("modifying a resource does not create duplicates at the new revision", async () => {
            const resource = await createResource("Unique Resource", "UTC");

            // Update
            await graphqlRequest(RESOURCE_SAVE, {
                resource: {
                    dbId: resource.dbId,
                    name: "Unique Resource Modified",
                    timezone: "UTC",
                    added: new Date().toISOString(),
                    availability: standardAvailability(),
                },
            });

            const revAfterUpdate = await getLatestRevision();
            const resourcesAtNew =
                await queryResourcesAtRevision(revAfterUpdate);

            // Should have exactly 1 resource, not 2
            expect(resourcesAtNew.resources).toHaveLength(1);
            expect(resourcesAtNew.resources[0]!.name).toBe(
                "Unique Resource Modified",
            );
        });

        test("modifying a task preserves correct count at old revision", async () => {
            const t1 = await createTask({
                title: "Task A",
                designation: "TASK",
                effort: 4.0,
            });
            const t2 = await createTask({
                title: "Task B",
                designation: "TASK",
                effort: 4.0,
            });

            const revBefore = await getLatestRevision();

            // Modify task A
            await graphqlRequest(TASK_SAVE, {
                task: {
                    dbId: t1.dbId,
                    title: "Task A v2",
                    description: "",
                    designation: "TASK",
                    priority: 1.0,
                    effort: 8.0,
                },
            });

            const revAfter = await getLatestRevision();

            // Old revision should still show 2 tasks with original names
            const tasksOld = await queryTasksAtRevision(revBefore);
            expect(tasksOld.tasks).toHaveLength(2);
            expect(tasksOld.tasks.map((t) => t.title).sort()).toEqual(
                ["Task A", "Task B"].sort(),
            );

            // New revision should also show 2 tasks, with updated name
            const tasksNew = await queryTasksAtRevision(revAfter);
            expect(tasksNew.tasks).toHaveLength(2);
            const titles = tasksNew.tasks.map((t) => t.title).sort();
            expect(titles).toEqual(["Task A v2", "Task B"].sort());
        });
    });

    // -----------------------------------------------------------------------
    // Booking revisions
    // -----------------------------------------------------------------------

    test.describe("Booking revisions", () => {
        test("booking is visible at creation revision and deleted at next", async () => {
            // Create a task and resource for the booking
            const resource = await createResource("Booking Resource", "UTC");
            const task = await createTask({
                title: "Booking Task",
                designation: "TASK",
                effort: 8.0,
            });

            const start = new Date();
            start.setDate(start.getDate() + 1);
            start.setHours(9, 0, 0, 0);
            const end = new Date(start);
            end.setHours(17, 0, 0, 0);

            // Create booking
            const bookingResult = await graphqlRequest<{
                bookingSave: { dbId: number };
            }>(
                `mutation BookingSave($taskId: Int!, $start: DateTime!, $end: DateTime!, $resources: [Int!]!, $final: Boolean!) {
                    bookingSave(taskId: $taskId, start: $start, end: $end, resources: $resources, final: $final) { dbId }
                }`,
                {
                    taskId: task.dbId,
                    start: start.toISOString(),
                    end: end.toISOString(),
                    resources: [resource.dbId],
                    final: false,
                },
            );

            const revAfterCreate = await getLatestRevision();

            // Booking should be visible at creation revision
            const bookingsAtCreate =
                await queryBookingsAtRevision(revAfterCreate);
            expect(
                bookingsAtCreate.bookings.some(
                    (b) => b.dbId === bookingResult.bookingSave.dbId,
                ),
            ).toBe(true);

            // Delete the booking
            await graphqlRequest(
                `mutation BookingDelete($dbId: Int!) { bookingDelete(dbId: $dbId) }`,
                { dbId: bookingResult.bookingSave.dbId },
            );

            const revAfterDelete = await getLatestRevision();

            // Old revision: booking still there
            const bookingsAtOld = await queryBookingsAtRevision(revAfterCreate);
            expect(
                bookingsAtOld.bookings.some(
                    (b) => b.dbId === bookingResult.bookingSave.dbId,
                ),
            ).toBe(true);

            // New revision: booking gone
            const bookingsAtNew = await queryBookingsAtRevision(revAfterDelete);
            expect(
                bookingsAtNew.bookings.some(
                    (b) => b.dbId === bookingResult.bookingSave.dbId,
                ),
            ).toBe(false);
        });

        test("modifying a booking does not create duplicates", async () => {
            const resource = await createResource("Booking Resource 2", "UTC");
            const task = await createTask({
                title: "Booking Task 2",
                designation: "TASK",
                effort: 8.0,
            });

            const start = new Date();
            start.setDate(start.getDate() + 1);
            start.setHours(9, 0, 0, 0);
            const end = new Date(start);
            end.setHours(17, 0, 0, 0);

            // Create booking
            const bookingResult = await graphqlRequest<{
                bookingSave: { dbId: number };
            }>(
                `mutation BookingSave($taskId: Int!, $start: DateTime!, $end: DateTime!, $resources: [Int!]!, $final: Boolean!) {
                    bookingSave(taskId: $taskId, start: $start, end: $end, resources: $resources, final: $final) { dbId }
                }`,
                {
                    taskId: task.dbId,
                    start: start.toISOString(),
                    end: end.toISOString(),
                    resources: [resource.dbId],
                    final: false,
                },
            );

            const revAfterCreate = await getLatestRevision();

            // Update booking (extend end by 1 hour)
            const newEnd = new Date(end);
            newEnd.setHours(newEnd.getHours() + 1);
            await graphqlRequest(
                `mutation BookingSave($dbId: Int, $taskId: Int!, $start: DateTime!, $end: DateTime!, $resources: [Int!]!, $final: Boolean!) {
                    bookingSave(dbId: $dbId, taskId: $taskId, start: $start, end: $end, resources: $resources, final: $final) { dbId }
                }`,
                {
                    dbId: bookingResult.bookingSave.dbId,
                    taskId: task.dbId,
                    start: start.toISOString(),
                    end: newEnd.toISOString(),
                    resources: [resource.dbId],
                    final: false,
                },
            );

            const revAfterUpdate = await getLatestRevision();

            // Old revision: 1 booking
            const bookingsOld = await queryBookingsAtRevision(revAfterCreate);
            expect(bookingsOld.bookings).toHaveLength(1);

            // New revision: still 1 booking (not 2)
            const bookingsNew = await queryBookingsAtRevision(revAfterUpdate);
            expect(bookingsNew.bookings).toHaveLength(1);
        });
    });

    // -----------------------------------------------------------------------
    // Relationship preservation across revisions
    // -----------------------------------------------------------------------

    test.describe("Relationship preservation across revisions", () => {
        test("updating a task preserves its predecessor/successor relationships", async () => {
            const taskA = await createTask({
                title: "Predecessor Task",
                designation: "TASK",
                effort: 4.0,
            });

            const taskB = await createTask({
                title: "Successor Task",
                designation: "TASK",
                effort: 4.0,
                predecessors: [taskA.dbId],
            });

            const revBefore = await getLatestRevision();

            // Update task A (creates new iteration)
            const updatedA = await graphqlRequest<{
                taskSave: { dbId: number; title: string };
            }>(TASK_SAVE, {
                task: {
                    dbId: taskA.dbId,
                    title: "Predecessor Task v2",
                    description: "",
                    designation: "TASK",
                    priority: 1.0,
                    effort: 8.0,
                },
            });

            // Query current tasks and verify relationships
            const result = await graphqlRequest<{
                tasks: Array<{
                    dbId: number;
                    title: string;
                    predecessors: Array<{ dbId: number }>;
                    successors: Array<{ dbId: number }>;
                }>;
            }>(
                `query { tasks { dbId title predecessors { dbId } successors { dbId } } }`,
            );

            const newA = result.tasks.find(
                (t) => t.dbId === updatedA.taskSave.dbId,
            );
            const currentB = result.tasks.find(
                (t) => t.title === "Successor Task",
            );

            expect(newA).toBeDefined();
            expect(currentB).toBeDefined();

            // A should still have B as successor
            expect(newA!.successors.length).toBeGreaterThanOrEqual(1);
            expect(
                newA!.successors.some((s) => s.dbId === currentB!.dbId),
            ).toBe(true);

            // B should have the new A as predecessor
            expect(currentB!.predecessors.length).toBeGreaterThanOrEqual(1);
            expect(
                currentB!.predecessors.some(
                    (p) => p.dbId === updatedA.taskSave.dbId,
                ),
            ).toBe(true);
        });

        test("updating a task preserves its resource constraints", async () => {
            const resource = await createResource("Constraint Resource", "UTC");

            const task = await createTask({
                title: "Constrained Task",
                designation: "TASK",
                effort: 8.0,
                resourceConstraints: [
                    {
                        optional: false,
                        speed: 1.0,
                        entries: [{ resourceId: resource.dbId }],
                    },
                ],
            });

            // Update the task (without providing resource constraints → should migrate them)
            const updated = await graphqlRequest<{
                taskSave: { dbId: number; title: string };
            }>(TASK_SAVE, {
                task: {
                    dbId: task.dbId,
                    title: "Constrained Task v2",
                    description: "",
                    designation: "TASK",
                    priority: 1.0,
                    effort: 12.0,
                },
            });

            // Query the updated task's resource constraints
            const result = await graphqlRequest<{
                tasks: Array<{
                    dbId: number;
                    title: string;
                    resourceConstraints: Array<{
                        optional: boolean;
                        speed: number;
                        entries: Array<{ resource: { dbId: number } }>;
                    }>;
                }>;
            }>(
                `query { tasks { dbId title resourceConstraints { optional speed entries { resource { dbId } } } } }`,
            );

            const updatedTask = result.tasks.find(
                (t) => t.dbId === updated.taskSave.dbId,
            );
            expect(updatedTask).toBeDefined();
            expect(updatedTask!.resourceConstraints).toHaveLength(1);
            expect(
                updatedTask!.resourceConstraints[0]!.entries[0]!.resource.dbId,
            ).toBe(resource.dbId);
        });

        test("updating a parent task preserves its children", async () => {
            const group = await createTask({
                title: "Parent Group",
                designation: "GROUP",
            });

            await createTask({
                title: "Child 1",
                designation: "TASK",
                effort: 4.0,
                parentId: group.dbId,
            });

            await createTask({
                title: "Child 2",
                designation: "TASK",
                effort: 4.0,
                parentId: group.dbId,
            });

            // Update the parent (creates new iteration)
            const updatedGroup = await graphqlRequest<{
                taskSave: { dbId: number; title: string };
            }>(TASK_SAVE, {
                task: {
                    dbId: group.dbId,
                    title: "Parent Group v2",
                    description: "",
                    designation: "GROUP",
                    priority: 1.0,
                },
            });

            // Query current tasks
            const result = await graphqlRequest<{
                tasks: Array<{
                    dbId: number;
                    title: string;
                    children: Array<{ dbId: number }>;
                    parent: { dbId: number } | null;
                }>;
            }>(
                `query { tasks { dbId title children { dbId } parent { dbId } } }`,
            );

            const newGroup = result.tasks.find(
                (t) => t.dbId === updatedGroup.taskSave.dbId,
            );
            expect(newGroup).toBeDefined();
            expect(newGroup!.children).toHaveLength(2);

            // Children should reference the new parent
            const child1 = result.tasks.find((t) => t.title === "Child 1");
            const child2 = result.tasks.find((t) => t.title === "Child 2");
            expect(child1).toBeDefined();
            expect(child2).toBeDefined();
            expect(child1!.parent).toBeDefined();
            expect(child1!.parent!.dbId).toBe(updatedGroup.taskSave.dbId);
            expect(child2!.parent).toBeDefined();
            expect(child2!.parent!.dbId).toBe(updatedGroup.taskSave.dbId);
        });

        test("old and new revisions resolve parent relationships via parent header id", async () => {
            const group = await createTask({
                title: "Parent Group",
                designation: "GROUP",
            });

            const child = await createTask({
                title: "Child Task",
                designation: "TASK",
                effort: 4.0,
                parentId: group.dbId,
            });

            const revBeforeParentUpdate = await getLatestRevision();

            const updatedGroup = await graphqlRequest<{
                taskSave: { dbId: number; title: string };
            }>(TASK_SAVE, {
                task: {
                    dbId: group.dbId,
                    title: "Parent Group v2",
                    description: "",
                    designation: "GROUP",
                    priority: 1.0,
                },
            });

            const revAfterParentUpdate = await getLatestRevision();

            const oldRevision = await queryTasksAtRevision(
                revBeforeParentUpdate,
            );
            const oldParent = oldRevision.tasks.find(
                (t) => t.title === "Parent Group",
            );
            const oldChild = oldRevision.tasks.find(
                (t) => t.dbId === child.dbId,
            );

            expect(oldParent).toBeDefined();
            expect(oldChild).toBeDefined();
            expect(oldChild!.parent).toBeDefined();
            expect(oldChild!.parent!.dbId).toBe(group.dbId);
            expect(oldParent!.children.some((c) => c.dbId === child.dbId)).toBe(
                true,
            );

            const newRevision =
                await queryTasksAtRevision(revAfterParentUpdate);
            const newParent = newRevision.tasks.find(
                (t) => t.dbId === updatedGroup.taskSave.dbId,
            );
            const currentChild = newRevision.tasks.find(
                (t) => t.title === "Child Task",
            );

            expect(newParent).toBeDefined();
            expect(currentChild).toBeDefined();
            expect(currentChild!.parent).toBeDefined();
            expect(currentChild!.parent!.dbId).toBe(updatedGroup.taskSave.dbId);
            expect(
                newParent!.children.some((c) => c.dbId === currentChild!.dbId),
            ).toBe(true);
        });
    });

    // -----------------------------------------------------------------------
    // Revision-aware predecessors / successors
    // -----------------------------------------------------------------------

    test.describe("Revision-aware predecessors and successors", () => {
        test("old revision returns original predecessors after they are changed", async () => {
            // Create three tasks: A → B (A is predecessor of B)
            const taskA = await createTask({
                title: "Task A",
                designation: "TASK",
                effort: 4.0,
            });
            const taskB = await createTask({
                title: "Task B",
                designation: "TASK",
                effort: 4.0,
                predecessors: [taskA.dbId],
            });
            const taskC = await createTask({
                title: "Task C",
                designation: "TASK",
                effort: 4.0,
            });

            const revOriginal = await getLatestRevision();

            // Verify at revOriginal: B has predecessor A, A has successor B
            const atOriginal = await queryTasksAtRevision(revOriginal);
            const bOrig = atOriginal.tasks.find((t) => t.title === "Task B");
            expect(bOrig).toBeDefined();
            expect(bOrig!.predecessors).toHaveLength(1);
            expect(bOrig!.predecessors![0]!.dbId).toBe(taskA.dbId);

            const aOrig = atOriginal.tasks.find((t) => t.title === "Task A");
            expect(aOrig).toBeDefined();
            expect(aOrig!.successors).toHaveLength(1);
            expect(aOrig!.successors![0]!.dbId).toBe(taskB.dbId);

            // Now change B's predecessor from A to C
            await graphqlRequest(TASK_SAVE, {
                task: {
                    dbId: taskB.dbId,
                    title: "Task B",
                    description: "",
                    designation: "TASK",
                    priority: 1.0,
                    effort: 4.0,
                    predecessors: [taskC.dbId],
                },
            });

            const revAfterChange = await getLatestRevision();
            expect(revAfterChange).toBeGreaterThan(revOriginal);

            // Old revision still shows A as predecessor of B
            const atOld = await queryTasksAtRevision(revOriginal);
            const bOld = atOld.tasks.find((t) => t.title === "Task B");
            expect(bOld).toBeDefined();
            expect(bOld!.predecessors).toHaveLength(1);
            expect(bOld!.predecessors![0]!.dbId).toBe(taskA.dbId);

            const aOld = atOld.tasks.find((t) => t.title === "Task A");
            expect(aOld).toBeDefined();
            expect(aOld!.successors).toHaveLength(1);
            expect(aOld!.successors![0]!.dbId).toBe(bOld!.dbId);

            // New revision shows C as predecessor of B
            const atNew = await queryTasksAtRevision(revAfterChange);
            const bNew = atNew.tasks.find((t) => t.title === "Task B");
            expect(bNew).toBeDefined();
            expect(bNew!.predecessors).toHaveLength(1);
            expect(bNew!.predecessors![0]!.dbId).toBe(taskC.dbId);

            // A should no longer be a predecessor of B at the new revision
            const aNew = atNew.tasks.find((t) => t.title === "Task A");
            expect(aNew).toBeDefined();
            expect(aNew!.successors).toHaveLength(0);

            // C should now be a predecessor of B at the new revision
            const cNew = atNew.tasks.find((t) => t.title === "Task C");
            expect(cNew).toBeDefined();
            expect(cNew!.successors).toHaveLength(1);
            expect(cNew!.successors![0]!.dbId).toBe(bNew!.dbId);
        });

        test("old revision returns original successors after they are removed", async () => {
            const taskA = await createTask({
                title: "Task A",
                designation: "TASK",
                effort: 4.0,
            });
            const taskB = await createTask({
                title: "Task B",
                designation: "TASK",
                effort: 4.0,
                predecessors: [taskA.dbId],
            });

            const revWithDep = await getLatestRevision();

            // Remove the dependency by saving B without predecessors
            const updatedB = await graphqlRequest<{
                taskSave: { dbId: number };
            }>(TASK_SAVE, {
                task: {
                    dbId: taskB.dbId,
                    title: "Task B",
                    description: "",
                    designation: "TASK",
                    priority: 1.0,
                    effort: 4.0,
                    predecessors: [],
                },
            });

            const revWithoutDep = await getLatestRevision();

            // Old revision: dependency still exists
            const atOld = await queryTasksAtRevision(revWithDep);
            const bOld = atOld.tasks.find((t) => t.title === "Task B");
            expect(bOld).toBeDefined();
            expect(bOld!.predecessors).toHaveLength(1);

            const aOld = atOld.tasks.find((t) => t.title === "Task A");
            expect(aOld).toBeDefined();
            expect(aOld!.successors).toHaveLength(1);

            // New revision: dependency is gone
            const atNew = await queryTasksAtRevision(revWithoutDep);
            const bNew = atNew.tasks.find(
                (t) => t.dbId === updatedB.taskSave.dbId,
            );
            expect(bNew).toBeDefined();
            expect(bNew!.predecessors).toHaveLength(0);

            const aNew = atNew.tasks.find((t) => t.title === "Task A");
            expect(aNew).toBeDefined();
            expect(aNew!.successors).toHaveLength(0);
        });

        test("old revision returns original predecessors after predecessor task is updated", async () => {
            // A → B. Then update A (creates new iteration). Old rev should still work.
            const taskA = await createTask({
                title: "Task A",
                designation: "TASK",
                effort: 4.0,
            });
            const taskB = await createTask({
                title: "Task B",
                designation: "TASK",
                effort: 4.0,
                predecessors: [taskA.dbId],
            });

            const revBefore = await getLatestRevision();

            // Update task A (creates new iteration, migrates deps)
            const updatedA = await graphqlRequest<{
                taskSave: { dbId: number; title: string };
            }>(TASK_SAVE, {
                task: {
                    dbId: taskA.dbId,
                    title: "Task A v2",
                    description: "updated",
                    designation: "TASK",
                    priority: 1.0,
                    effort: 8.0,
                },
            });

            const revAfter = await getLatestRevision();

            // Old revision: B's predecessor is the OLD iteration of A
            const atOld = await queryTasksAtRevision(revBefore);
            const bOld = atOld.tasks.find((t) => t.title === "Task B");
            expect(bOld).toBeDefined();
            expect(bOld!.predecessors).toHaveLength(1);
            // The predecessor should be A at the old revision (old iteration id)
            expect(bOld!.predecessors![0]!.dbId).toBe(taskA.dbId);

            const aOld = atOld.tasks.find((t) => t.title === "Task A");
            expect(aOld).toBeDefined();
            expect(aOld!.successors).toHaveLength(1);
            expect(aOld!.successors![0]!.dbId).toBe(taskB.dbId);

            // New revision: B's predecessor is the NEW iteration of A
            const atNew = await queryTasksAtRevision(revAfter);
            const bNew = atNew.tasks.find((t) => t.title === "Task B");
            expect(bNew).toBeDefined();
            expect(bNew!.predecessors).toHaveLength(1);
            expect(bNew!.predecessors![0]!.dbId).toBe(updatedA.taskSave.dbId);

            const aNew = atNew.tasks.find((t) => t.title === "Task A v2");
            expect(aNew).toBeDefined();
            expect(aNew!.successors).toHaveLength(1);
            expect(aNew!.successors![0]!.dbId).toBe(bNew!.dbId);
        });

        test("multiple predecessor changes across revisions are all queryable", async () => {
            const taskA = await createTask({
                title: "Task A",
                designation: "TASK",
                effort: 4.0,
            });
            const taskB = await createTask({
                title: "Task B",
                designation: "TASK",
                effort: 4.0,
            });
            const taskC = await createTask({
                title: "Task C",
                designation: "TASK",
                effort: 4.0,
            });

            // Rev 1: no dependencies
            const rev1 = await getLatestRevision();

            // Add A as predecessor of C
            await graphqlRequest(TASK_SAVE, {
                task: {
                    dbId: taskC.dbId,
                    title: "Task C",
                    description: "",
                    designation: "TASK",
                    priority: 1.0,
                    effort: 4.0,
                    predecessors: [taskA.dbId],
                },
            });
            const rev2 = await getLatestRevision();

            // Change predecessor of C from A to B
            // Need to use the new dbId of C from the previous save
            const tasksAtRev2 = await queryTasksAtRevision(rev2);
            const cAtRev2 = tasksAtRev2.tasks.find((t) => t.title === "Task C");
            expect(cAtRev2).toBeDefined();

            await graphqlRequest(TASK_SAVE, {
                task: {
                    dbId: cAtRev2!.dbId,
                    title: "Task C",
                    description: "",
                    designation: "TASK",
                    priority: 1.0,
                    effort: 4.0,
                    predecessors: [taskB.dbId],
                },
            });
            const rev3 = await getLatestRevision();

            // Rev 1: C has no predecessors
            const atRev1 = await queryTasksAtRevision(rev1);
            const cR1 = atRev1.tasks.find((t) => t.title === "Task C");
            expect(cR1).toBeDefined();
            expect(cR1!.predecessors).toHaveLength(0);

            // Rev 2: C has A as predecessor
            const atRev2 = await queryTasksAtRevision(rev2);
            const cR2 = atRev2.tasks.find((t) => t.title === "Task C");
            expect(cR2).toBeDefined();
            expect(cR2!.predecessors).toHaveLength(1);
            const aR2 = atRev2.tasks.find((t) => t.title === "Task A");
            expect(cR2!.predecessors![0]!.dbId).toBe(aR2!.dbId);

            // Rev 3: C has B as predecessor
            const atRev3 = await queryTasksAtRevision(rev3);
            const cR3 = atRev3.tasks.find((t) => t.title === "Task C");
            expect(cR3).toBeDefined();
            expect(cR3!.predecessors).toHaveLength(1);
            const bR3 = atRev3.tasks.find((t) => t.title === "Task B");
            expect(cR3!.predecessors![0]!.dbId).toBe(bR3!.dbId);
        });

        test("adding new successors does not affect predecessors at old revision", async () => {
            const taskA = await createTask({
                title: "Task A",
                designation: "TASK",
                effort: 4.0,
            });
            const taskB = await createTask({
                title: "Task B",
                designation: "TASK",
                effort: 4.0,
                predecessors: [taskA.dbId],
            });

            const revBefore = await getLatestRevision();

            // Create task D as a new successor of A (via successors field)
            const taskD = await createTask({
                title: "Task D",
                designation: "TASK",
                effort: 4.0,
                predecessors: [taskA.dbId],
            });

            const revAfter = await getLatestRevision();

            // Old revision: A has exactly one successor (B)
            const atOld = await queryTasksAtRevision(revBefore);
            const aOld = atOld.tasks.find((t) => t.title === "Task A");
            expect(aOld).toBeDefined();
            expect(aOld!.successors).toHaveLength(1);
            expect(aOld!.successors![0]!.dbId).toBe(taskB.dbId);

            // New revision: A has two successors (B and D)
            const atNew = await queryTasksAtRevision(revAfter);
            const aNew = atNew.tasks.find((t) => t.title === "Task A");
            expect(aNew).toBeDefined();
            expect(aNew!.successors).toHaveLength(2);
            const succTitles = aNew!.successors!.map((s) => {
                const match = atNew.tasks.find((t) => t.dbId === s.dbId);
                return match?.title;
            });
            expect(succTitles.sort()).toEqual(["Task B", "Task D"]);
        });

        test("deleted predecessor disappears at new revision but remains at old", async () => {
            const taskA = await createTask({
                title: "Task A",
                designation: "TASK",
                effort: 4.0,
            });
            const taskB = await createTask({
                title: "Task B",
                designation: "TASK",
                effort: 4.0,
                predecessors: [taskA.dbId],
            });

            const revBefore = await getLatestRevision();

            // Delete task A
            await graphqlRequest(
                `mutation TaskDelete($taskId: Int!) { taskDelete(taskId: $taskId) }`,
                { taskId: taskA.dbId },
            );

            const revAfter = await getLatestRevision();

            // Old revision: B has predecessor A
            const atOld = await queryTasksAtRevision(revBefore);
            const bOld = atOld.tasks.find((t) => t.title === "Task B");
            expect(bOld).toBeDefined();
            expect(bOld!.predecessors).toHaveLength(1);
            expect(bOld!.predecessors![0]!.dbId).toBe(taskA.dbId);

            // Task A is also present at old revision
            const aOld = atOld.tasks.find((t) => t.title === "Task A");
            expect(aOld).toBeDefined();

            // New revision: A is deleted, so it's not in the task list
            const atNew = await queryTasksAtRevision(revAfter);
            const aNew = atNew.tasks.find((t) => t.title === "Task A");
            expect(aNew).toBeUndefined();

            // B still exists at new revision; the dependency link to A
            // should no longer resolve (A's iteration is deleted at this rev)
            const bNew = atNew.tasks.find((t) => t.title === "Task B");
            expect(bNew).toBeDefined();
            // The predecessor iteration was soft-deleted, so it should not
            // appear in the predecessor list at the new revision
            expect(bNew!.predecessors).toHaveLength(0);
        });

        test("complex chain: A → B → C, update B, verify all revisions", async () => {
            const taskA = await createTask({
                title: "Chain A",
                designation: "TASK",
                effort: 2.0,
            });
            const taskB = await createTask({
                title: "Chain B",
                designation: "TASK",
                effort: 2.0,
                predecessors: [taskA.dbId],
            });
            const taskC = await createTask({
                title: "Chain C",
                designation: "TASK",
                effort: 2.0,
                predecessors: [taskB.dbId],
            });

            const revChain = await getLatestRevision();

            // Update B (new iteration created, deps migrated)
            const updatedB = await graphqlRequest<{
                taskSave: { dbId: number };
            }>(TASK_SAVE, {
                task: {
                    dbId: taskB.dbId,
                    title: "Chain B v2",
                    description: "updated",
                    designation: "TASK",
                    priority: 1.0,
                    effort: 6.0,
                },
            });

            const revUpdated = await getLatestRevision();

            // Old revision: full chain A → B → C with original IDs
            const atOld = await queryTasksAtRevision(revChain);
            const aOld = atOld.tasks.find((t) => t.title === "Chain A");
            const bOld = atOld.tasks.find((t) => t.title === "Chain B");
            const cOld = atOld.tasks.find((t) => t.title === "Chain C");
            expect(aOld).toBeDefined();
            expect(bOld).toBeDefined();
            expect(cOld).toBeDefined();

            expect(aOld!.successors).toHaveLength(1);
            expect(aOld!.successors![0]!.dbId).toBe(bOld!.dbId);
            expect(bOld!.predecessors).toHaveLength(1);
            expect(bOld!.predecessors![0]!.dbId).toBe(aOld!.dbId);
            expect(bOld!.successors).toHaveLength(1);
            expect(bOld!.successors![0]!.dbId).toBe(cOld!.dbId);
            expect(cOld!.predecessors).toHaveLength(1);
            expect(cOld!.predecessors![0]!.dbId).toBe(bOld!.dbId);

            // New revision: chain A → B v2 → C with new B iteration
            const atNew = await queryTasksAtRevision(revUpdated);
            const aNew = atNew.tasks.find((t) => t.title === "Chain A");
            const bNew = atNew.tasks.find((t) => t.title === "Chain B v2");
            const cNew = atNew.tasks.find((t) => t.title === "Chain C");
            expect(aNew).toBeDefined();
            expect(bNew).toBeDefined();
            expect(cNew).toBeDefined();
            expect(bNew!.dbId).toBe(updatedB.taskSave.dbId);

            expect(aNew!.successors).toHaveLength(1);
            expect(aNew!.successors![0]!.dbId).toBe(bNew!.dbId);
            expect(bNew!.predecessors).toHaveLength(1);
            expect(bNew!.predecessors![0]!.dbId).toBe(aNew!.dbId);
            expect(bNew!.successors).toHaveLength(1);
            expect(bNew!.successors![0]!.dbId).toBe(cNew!.dbId);
            expect(cNew!.predecessors).toHaveLength(1);
            expect(cNew!.predecessors![0]!.dbId).toBe(bNew!.dbId);
        });

        test("resource constraints are correct at old revision after task update", async () => {
            const resource = await createResource("Rev RC Resource", "UTC");

            const task = await createTask({
                title: "RC Task",
                designation: "TASK",
                effort: 8.0,
                resourceConstraints: [
                    {
                        optional: false,
                        speed: 1.0,
                        entries: [{ resourceId: resource.dbId }],
                    },
                ],
            });

            const revBefore = await getLatestRevision();

            // Update task (no resource constraints provided → migrated from old)
            const updated = await graphqlRequest<{
                taskSave: { dbId: number };
            }>(TASK_SAVE, {
                task: {
                    dbId: task.dbId,
                    title: "RC Task v2",
                    description: "",
                    designation: "TASK",
                    priority: 1.0,
                    effort: 12.0,
                },
            });

            const revAfter = await getLatestRevision();

            // Old revision: task has the resource constraint
            const atOld = await queryTasksAtRevision(revBefore);
            const tOld = atOld.tasks.find((t) => t.title === "RC Task");
            expect(tOld).toBeDefined();
            expect(tOld!.resourceConstraints).toHaveLength(1);
            expect(
                tOld!.resourceConstraints![0]!.entries[0]!.resource.dbId,
            ).toBe(resource.dbId);

            // New revision: updated task still has the resource constraint
            const atNew = await queryTasksAtRevision(revAfter);
            const tNew = atNew.tasks.find((t) => t.title === "RC Task v2");
            expect(tNew).toBeDefined();
            expect(tNew!.dbId).toBe(updated.taskSave.dbId);
            expect(tNew!.resourceConstraints).toHaveLength(1);
        });
    });

    // -----------------------------------------------------------------------
    // Previous revision state remains unchanged after further edits
    // -----------------------------------------------------------------------

    test.describe("Previous revision state remains unchanged", () => {
        test("creating more tasks does not affect a previous revision snapshot", async () => {
            const task1 = await createTask({
                title: "Initial Task",
                designation: "TASK",
                effort: 4.0,
            });

            const revSnapshot = await getLatestRevision();

            // Create more tasks
            await createTask({
                title: "Added Task 1",
                designation: "TASK",
                effort: 4.0,
            });
            await createTask({
                title: "Added Task 2",
                designation: "TASK",
                effort: 4.0,
            });

            // Old revision should still show exactly 1 task
            const tasksAtSnapshot = await queryTasksAtRevision(revSnapshot);
            expect(tasksAtSnapshot.tasks).toHaveLength(1);
            expect(tasksAtSnapshot.tasks[0]!.title).toBe("Initial Task");

            // Latest should have 3
            const tasksLatest = await graphqlRequest<{
                tasks: Array<{ dbId: number; title: string }>;
            }>(`query { tasks { dbId title } }`);
            expect(tasksLatest.tasks).toHaveLength(3);
        });

        test("deleting tasks does not affect a previous revision snapshot", async () => {
            const t1 = await createTask({
                title: "Stable Task",
                designation: "TASK",
                effort: 4.0,
            });
            const t2 = await createTask({
                title: "Doomed Task",
                designation: "TASK",
                effort: 4.0,
            });

            const revSnapshot = await getLatestRevision();

            // Delete one task
            await graphqlRequest(
                `mutation TaskDelete($taskId: Int!) { taskDelete(taskId: $taskId) }`,
                { taskId: t2.dbId },
            );

            // Old revision should still have both tasks
            const tasksAtSnapshot = await queryTasksAtRevision(revSnapshot);
            expect(tasksAtSnapshot.tasks).toHaveLength(2);
            expect(tasksAtSnapshot.tasks.map((t) => t.title).sort()).toEqual([
                "Doomed Task",
                "Stable Task",
            ]);

            // Latest should have only one
            const tasksLatest = await graphqlRequest<{
                tasks: Array<{ dbId: number; title: string }>;
            }>(`query { tasks { dbId title } }`);
            expect(tasksLatest.tasks).toHaveLength(1);
            expect(tasksLatest.tasks[0]!.title).toBe("Stable Task");
        });

        test("complex sequence: create, update, delete – all revisions consistent", async () => {
            // Step 1: Create task A
            const taskA = await createTask({
                title: "Task A",
                designation: "TASK",
                effort: 4.0,
            });
            const rev1 = await getLatestRevision();

            // Step 2: Create task B
            const taskB = await createTask({
                title: "Task B",
                designation: "TASK",
                effort: 8.0,
            });
            const rev2 = await getLatestRevision();

            // Step 3: Update task A
            const updatedA = await graphqlRequest<{
                taskSave: { dbId: number; title: string };
            }>(TASK_SAVE, {
                task: {
                    dbId: taskA.dbId,
                    title: "Task A v2",
                    description: "",
                    designation: "TASK",
                    priority: 1.0,
                    effort: 12.0,
                },
            });
            const rev3 = await getLatestRevision();

            // Step 4: Delete task B
            await graphqlRequest(
                `mutation TaskDelete($taskId: Int!) { taskDelete(taskId: $taskId) }`,
                { taskId: taskB.dbId },
            );
            const rev4 = await getLatestRevision();

            // Verify rev1: only Task A (original)
            const atRev1 = await queryTasksAtRevision(rev1);
            expect(atRev1.tasks).toHaveLength(1);
            expect(atRev1.tasks[0]!.title).toBe("Task A");
            expect(atRev1.tasks[0]!.effort).toBe(4.0);

            // Verify rev2: Task A + Task B
            const atRev2 = await queryTasksAtRevision(rev2);
            expect(atRev2.tasks).toHaveLength(2);
            expect(atRev2.tasks.map((t) => t.title).sort()).toEqual([
                "Task A",
                "Task B",
            ]);

            // Verify rev3: Task A v2 + Task B
            const atRev3 = await queryTasksAtRevision(rev3);
            expect(atRev3.tasks).toHaveLength(2);
            expect(atRev3.tasks.map((t) => t.title).sort()).toEqual([
                "Task A v2",
                "Task B",
            ]);

            // Verify rev4: only Task A v2
            const atRev4 = await queryTasksAtRevision(rev4);
            expect(atRev4.tasks).toHaveLength(1);
            expect(atRev4.tasks[0]!.title).toBe("Task A v2");
            expect(atRev4.tasks[0]!.effort).toBe(12.0);
        });

        test("resource revisions are also consistent across complex operations", async () => {
            const r1 = await createResource("Resource Alpha", "UTC");
            const rev1 = await getLatestRevision();

            const r2 = await createResource("Resource Beta", "US/Eastern");
            const rev2 = await getLatestRevision();

            // Update Resource Alpha
            await graphqlRequest(RESOURCE_SAVE, {
                resource: {
                    dbId: r1.dbId,
                    name: "Resource Alpha v2",
                    timezone: "Europe/Berlin",
                    added: new Date().toISOString(),
                    availability: standardAvailability(),
                },
            });
            const rev3 = await getLatestRevision();

            // Delete Resource Beta
            await graphqlRequest(
                `mutation ResourceDelete($resourceId: Int!) { resourceDelete(resourceId: $resourceId) }`,
                { resourceId: r2.dbId },
            );
            const rev4 = await getLatestRevision();

            // Rev1: only Alpha
            const atRev1 = await queryResourcesAtRevision(rev1);
            expect(atRev1.resources).toHaveLength(1);
            expect(atRev1.resources[0]!.name).toBe("Resource Alpha");

            // Rev2: Alpha + Beta
            const atRev2 = await queryResourcesAtRevision(rev2);
            expect(atRev2.resources).toHaveLength(2);

            // Rev3: Alpha v2 + Beta
            const atRev3 = await queryResourcesAtRevision(rev3);
            expect(atRev3.resources).toHaveLength(2);
            expect(atRev3.resources.map((r) => r.name).sort()).toEqual([
                "Resource Alpha v2",
                "Resource Beta",
            ]);

            // Rev4: only Alpha v2
            const atRev4 = await queryResourcesAtRevision(rev4);
            expect(atRev4.resources).toHaveLength(1);
            expect(atRev4.resources[0]!.name).toBe("Resource Alpha v2");
        });
    });

    // -----------------------------------------------------------------------
    // resetDatabase cleanup
    // -----------------------------------------------------------------------

    test.describe("Database reset", () => {
        test("resetDatabase removes all tasks and resources", async () => {
            // Create some data
            await createTask({
                title: "Test Task",
                designation: "TASK",
                effort: 4.0,
            });
            await createResource("Test Resource");

            // Verify data exists
            const before = await graphqlRequest<{
                tasks: Array<{ dbId: number }>;
                resources: Array<{ dbId: number }>;
            }>(`query { tasks { dbId } resources { dbId } }`);
            expect(before.tasks.length).toBeGreaterThan(0);
            expect(before.resources.length).toBeGreaterThan(0);

            // Reset
            await graphqlRequest(`mutation ResetDatabase { resetDatabase }`);

            // Verify everything is gone
            const after = await graphqlRequest<{
                tasks: Array<{ dbId: number }>;
                resources: Array<{ dbId: number }>;
            }>(`query { tasks { dbId } resources { dbId } }`);
            expect(after.tasks).toHaveLength(0);
            expect(after.resources).toHaveLength(0);
        });

        test("resetDatabase creates a fresh revision", async () => {
            await createTask({
                title: "Before Reset",
                designation: "TASK",
                effort: 4.0,
            });

            await graphqlRequest(`mutation ResetDatabase { resetDatabase }`);

            const rev = await getLatestRevision();
            expect(rev).toBeGreaterThan(0);
        });
    });
});
