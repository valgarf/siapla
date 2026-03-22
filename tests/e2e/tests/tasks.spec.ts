import { test, expect } from "@playwright/test";
import { graphqlRequest, standardAvailability } from "../helpers/graphql";
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

/** Query all tasks from the API with relationship info. */
async function queryTasks() {
    return graphqlRequest<{
        tasks: Array<{
            dbId: number;
            title: string;
            designation: string;
            predecessors: Array<{ dbId: number }>;
            successors: Array<{ dbId: number }>;
            parent: { dbId: number } | null;
            children: Array<{ dbId: number }>;
            resourceConstraints: Array<{
                optional: boolean;
                speed: number;
                entries: Array<{ resource: { dbId: number } }>;
            }>;
        }>;
    }>(
        `query {
            tasks {
                dbId title designation
                predecessors { dbId }
                successors { dbId }
                parent { dbId }
                children { dbId }
                resourceConstraints { optional speed entries { resource { dbId } } }
            }
        }`,
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

test.beforeEach(async () => {
    await cleanDatabase();
});

test.describe("Task Management", () => {
    // -----------------------------------------------------------------------
    // Task creation and display
    // -----------------------------------------------------------------------

    test.describe("Task creation and display", () => {
        test("task created via API appears on the Tasks Gantt page", async ({
            page,
        }) => {
            const resource = await createResource("Dev Team Lead");
            const task = await createTask({
                title: "Implement Login Feature",
                description: "Build the login page with OAuth support",
                designation: "TASK",
                priority: 1.0,
                effort: 16.0,
                resourceConstraints: [
                    {
                        optional: false,
                        speed: 1.0,
                        entries: [{ resourceId: resource.dbId }],
                    },
                ],
            });
            expect(task.dbId).toBeGreaterThan(0);

            await page.goto("/#/");
            await page.waitForLoadState("networkidle");

            const rowName = page.locator(".gantt-row-description .row-name", {
                hasText: "Implement Login Feature",
            });
            await expect(rowName).toBeVisible({ timeout: 10_000 });
        });

        test("multiple tasks appear in the Gantt chart", async ({ page }) => {
            const taskNames = ["Task Alpha", "Task Beta", "Task Gamma"];
            for (const name of taskNames) {
                await createTask({
                    title: name,
                    description: `Description for ${name}`,
                    designation: "TASK",
                    effort: 8.0,
                });
            }

            await page.goto("/#/");
            await page.waitForLoadState("networkidle");

            for (const name of taskNames) {
                const row = page.locator(".gantt-row-description .row-name", {
                    hasText: name,
                });
                await expect(row).toBeVisible({ timeout: 10_000 });
            }
        });
    });

    // -----------------------------------------------------------------------
    // Designation types
    // -----------------------------------------------------------------------

    test.describe("Designation types render differently", () => {
        test("REQUIREMENT designation is displayed", async ({ page }) => {
            const futureDate = new Date();
            futureDate.setDate(futureDate.getDate() + 7);

            await createTask({
                title: "Project Kickoff Requirement",
                description: "The project cannot start before this date",
                designation: "REQUIREMENT",
                earliestStart: futureDate.toISOString(),
            });

            await page.goto("/#/");
            await page.waitForLoadState("networkidle");

            const row = page.locator(".gantt-row-description .row-name", {
                hasText: "Project Kickoff Requirement",
            });
            await expect(row).toBeVisible({ timeout: 10_000 });
        });

        test("MILESTONE designation is displayed", async ({ page }) => {
            const targetDate = new Date();
            targetDate.setDate(targetDate.getDate() + 30);

            await createTask({
                title: "Sprint 1 Delivery",
                description: "All sprint 1 deliverables must be complete",
                designation: "MILESTONE",
                scheduleTarget: targetDate.toISOString(),
            });

            await page.goto("/#/");
            await page.waitForLoadState("networkidle");

            const row = page.locator(".gantt-row-description .row-name", {
                hasText: "Sprint 1 Delivery",
            });
            await expect(row).toBeVisible({ timeout: 10_000 });
        });

        test("GROUP designation is displayed with expand/collapse toggle", async ({
            page,
        }) => {
            const group = await createTask({
                title: "Backend Module",
                description: "All backend tasks",
                designation: "GROUP",
            });

            await createTask({
                title: "Setup Database",
                description: "Initialize database schema",
                designation: "TASK",
                effort: 4.0,
                parentId: group.dbId,
            });

            await createTask({
                title: "Build API Endpoints",
                description: "Implement REST API",
                designation: "TASK",
                effort: 12.0,
                parentId: group.dbId,
            });

            await page.goto("/#/");
            await page.waitForLoadState("networkidle");

            // Group row should be visible
            const groupRow = page.locator(".gantt-row-description .row-name", {
                hasText: "Backend Module",
            });
            await expect(groupRow).toBeVisible({ timeout: 10_000 });

            // Children should be visible
            const child1 = page.locator(".gantt-row-description .row-name", {
                hasText: "Setup Database",
            });
            const child2 = page.locator(".gantt-row-description .row-name", {
                hasText: "Build API Endpoints",
            });
            await expect(child1).toBeVisible({ timeout: 10_000 });
            await expect(child2).toBeVisible({ timeout: 10_000 });

            // The group row should have a collapse toggle button
            const groupRowContainer = page.locator(".gantt-row-description", {
                hasText: "Backend Module",
            });
            const toggleBtn = groupRowContainer.locator(".q-btn").first();
            await expect(toggleBtn).toBeVisible();

            // Click the toggle to collapse the group
            await toggleBtn.click();

            // After collapsing, children should be hidden
            await expect(child1).toBeHidden({ timeout: 5_000 });
            await expect(child2).toBeHidden({ timeout: 5_000 });

            // Click again to expand
            await toggleBtn.click();

            // Children should be visible again
            await expect(child1).toBeVisible({ timeout: 5_000 });
            await expect(child2).toBeVisible({ timeout: 5_000 });
        });

        test("all four designation types appear together", async ({ page }) => {
            const futureDate = new Date();
            futureDate.setDate(futureDate.getDate() + 7);
            const targetDate = new Date();
            targetDate.setDate(targetDate.getDate() + 30);

            await createTask({
                title: "Req: Start Date",
                designation: "REQUIREMENT",
                earliestStart: futureDate.toISOString(),
            });

            const group = await createTask({
                title: "Group: Feature Set",
                designation: "GROUP",
            });

            await createTask({
                title: "Task: Do Work",
                designation: "TASK",
                effort: 8.0,
                parentId: group.dbId,
            });

            await createTask({
                title: "Mile: Delivery",
                designation: "MILESTONE",
                scheduleTarget: targetDate.toISOString(),
            });

            await page.goto("/#/");
            await page.waitForLoadState("networkidle");

            for (const title of [
                "Req: Start Date",
                "Group: Feature Set",
                "Task: Do Work",
                "Mile: Delivery",
            ]) {
                await expect(
                    page.locator(".gantt-row-description .row-name", {
                        hasText: title,
                    }),
                ).toBeVisible({ timeout: 10_000 });
            }
        });
    });

    // -----------------------------------------------------------------------
    // Dependencies between tasks
    // -----------------------------------------------------------------------

    test.describe("Dependencies between tasks", () => {
        test("predecessor/successor relationships are created via API", async ({
            page,
        }) => {
            const taskA = await createTask({
                title: "Design Phase",
                description: "Complete design documentation",
                designation: "TASK",
                effort: 8.0,
            });

            const taskB = await createTask({
                title: "Implementation Phase",
                description: "Implement the design",
                designation: "TASK",
                effort: 16.0,
                predecessors: [taskA.dbId],
            });

            // Verify the dependency via API
            const result = await queryTasks();
            const implTask = result.tasks.find((t) => t.dbId === taskB.dbId);
            expect(implTask).toBeDefined();
            expect(implTask!.predecessors).toEqual(
                expect.arrayContaining([
                    expect.objectContaining({ dbId: taskA.dbId }),
                ]),
            );

            const designTask = result.tasks.find((t) => t.dbId === taskA.dbId);
            expect(designTask).toBeDefined();
            expect(designTask!.successors).toEqual(
                expect.arrayContaining([
                    expect.objectContaining({ dbId: taskB.dbId }),
                ]),
            );

            // Navigate and verify both tasks are shown
            await page.goto("/#/");
            await page.waitForLoadState("networkidle");

            await expect(
                page.locator(".gantt-row-description .row-name", {
                    hasText: "Design Phase",
                }),
            ).toBeVisible({ timeout: 10_000 });
            await expect(
                page.locator(".gantt-row-description .row-name", {
                    hasText: "Implementation Phase",
                }),
            ).toBeVisible({ timeout: 10_000 });
        });

        test("chain of dependencies: A -> B -> C", async ({ page }) => {
            const taskA = await createTask({
                title: "Step 1: Requirements",
                description: "Gather requirements",
                designation: "TASK",
                effort: 4.0,
            });

            const taskB = await createTask({
                title: "Step 2: Development",
                description: "Write the code",
                designation: "TASK",
                effort: 12.0,
                predecessors: [taskA.dbId],
            });

            await createTask({
                title: "Step 3: Testing",
                description: "QA and testing",
                designation: "TASK",
                effort: 6.0,
                predecessors: [taskB.dbId],
            });

            await page.goto("/#/");
            await page.waitForLoadState("networkidle");

            for (const title of [
                "Step 1: Requirements",
                "Step 2: Development",
                "Step 3: Testing",
            ]) {
                await expect(
                    page.locator(".gantt-row-description .row-name", {
                        hasText: title,
                    }),
                ).toBeVisible({ timeout: 10_000 });
            }

            // Verify dependency chain via API
            const result = await queryTasks();
            const step1 = result.tasks.find(
                (t) => t.title === "Step 1: Requirements",
            );
            const step2 = result.tasks.find(
                (t) => t.title === "Step 2: Development",
            );
            const step3 = result.tasks.find(
                (t) => t.title === "Step 3: Testing",
            );

            expect(step1).toBeDefined();
            expect(step2).toBeDefined();
            expect(step3).toBeDefined();

            // Step 1 has no predecessors and one successor (Step 2)
            expect(step1!.predecessors).toHaveLength(0);
            expect(step1!.successors).toHaveLength(1);
            expect(step1!.successors[0]!.dbId).toBe(step2!.dbId);

            // Step 2 has one predecessor (Step 1) and one successor (Step 3)
            expect(step2!.predecessors).toHaveLength(1);
            expect(step2!.predecessors[0]!.dbId).toBe(step1!.dbId);
            expect(step2!.successors).toHaveLength(1);
            expect(step2!.successors[0]!.dbId).toBe(step3!.dbId);

            // Step 3 has one predecessor (Step 2) and no successors
            expect(step3!.predecessors).toHaveLength(1);
            expect(step3!.predecessors[0]!.dbId).toBe(step2!.dbId);
            expect(step3!.successors).toHaveLength(0);
        });

        test("multiple predecessors converging on one task", async ({
            page,
        }) => {
            const taskA = await createTask({
                title: "Frontend Work",
                description: "Build the UI",
                designation: "TASK",
                effort: 10.0,
            });

            const taskB = await createTask({
                title: "Backend Work",
                description: "Build the API",
                designation: "TASK",
                effort: 10.0,
            });

            await createTask({
                title: "Integration Testing",
                description: "Test frontend + backend together",
                designation: "TASK",
                effort: 6.0,
                predecessors: [taskA.dbId, taskB.dbId],
            });

            await page.goto("/#/");
            await page.waitForLoadState("networkidle");

            for (const title of [
                "Frontend Work",
                "Backend Work",
                "Integration Testing",
            ]) {
                await expect(
                    page.locator(".gantt-row-description .row-name", {
                        hasText: title,
                    }),
                ).toBeVisible({ timeout: 10_000 });
            }

            // Verify via API
            const result = await queryTasks();
            const integration = result.tasks.find(
                (t) => t.title === "Integration Testing",
            );
            expect(integration).toBeDefined();
            expect(integration!.predecessors).toHaveLength(2);
        });

        test("one task with multiple successors", async ({ page }) => {
            const shared = await createTask({
                title: "Core Library",
                description: "Build the shared library",
                designation: "TASK",
                effort: 8.0,
            });

            await createTask({
                title: "Module A",
                description: "First consumer",
                designation: "TASK",
                effort: 4.0,
                predecessors: [shared.dbId],
            });

            await createTask({
                title: "Module B",
                description: "Second consumer",
                designation: "TASK",
                effort: 4.0,
                predecessors: [shared.dbId],
            });

            await page.goto("/#/");
            await page.waitForLoadState("networkidle");

            for (const title of ["Core Library", "Module A", "Module B"]) {
                await expect(
                    page.locator(".gantt-row-description .row-name", {
                        hasText: title,
                    }),
                ).toBeVisible({ timeout: 10_000 });
            }

            // Verify via API
            const result = await queryTasks();
            const core = result.tasks.find((t) => t.title === "Core Library");
            expect(core).toBeDefined();
            expect(core!.successors).toHaveLength(2);
        });
    });

    // -----------------------------------------------------------------------
    // Groups with children
    // -----------------------------------------------------------------------

    test.describe("Groups with children", () => {
        test("group with multiple children appears with correct hierarchy", async ({
            page,
        }) => {
            const group = await createTask({
                title: "Epic: User Management",
                description: "All user management tasks",
                designation: "GROUP",
            });

            const childTitles = [
                "Create User",
                "Edit User",
                "Delete User",
                "User Permissions",
            ];
            for (const title of childTitles) {
                await createTask({
                    title,
                    description: `${title} implementation`,
                    designation: "TASK",
                    effort: 4.0,
                    parentId: group.dbId,
                });
            }

            await page.goto("/#/");
            await page.waitForLoadState("networkidle");

            // Group should be visible
            await expect(
                page.locator(".gantt-row-description .row-name", {
                    hasText: "Epic: User Management",
                }),
            ).toBeVisible({ timeout: 10_000 });

            // All children should be visible
            for (const title of childTitles) {
                await expect(
                    page.locator(".gantt-row-description .row-name", {
                        hasText: title,
                    }),
                ).toBeVisible({ timeout: 10_000 });
            }

            // Verify parent-child relationship via API
            const result = await queryTasks();
            const epic = result.tasks.find(
                (t) => t.title === "Epic: User Management",
            );
            expect(epic).toBeDefined();
            expect(epic!.children).toHaveLength(4);
            expect(epic!.parent).toBeNull();

            for (const title of childTitles) {
                const child = result.tasks.find((t) => t.title === title);
                expect(child).toBeDefined();
                expect(child!.parent).toBeDefined();
                expect(child!.parent!.dbId).toBe(epic!.dbId);
            }
        });

        test("nested groups: group inside a group", async ({ page }) => {
            // Create outer group
            const outerGroup = await createTask({
                title: "Project Alpha",
                description: "Top-level project",
                designation: "GROUP",
            });

            // Create inner group
            const innerGroup = await createTask({
                title: "Sprint 1",
                description: "First sprint",
                designation: "GROUP",
                parentId: outerGroup.dbId,
            });

            // Create leaf tasks in inner group
            await createTask({
                title: "Task in Sprint 1",
                description: "Actual work item",
                designation: "TASK",
                effort: 4.0,
                parentId: innerGroup.dbId,
            });

            // Create another task directly under outer group
            await createTask({
                title: "Standalone Task in Alpha",
                description: "Direct child of outer group",
                designation: "TASK",
                effort: 2.0,
                parentId: outerGroup.dbId,
            });

            await page.goto("/#/");
            await page.waitForLoadState("networkidle");

            // All items should be visible
            for (const title of [
                "Project Alpha",
                "Sprint 1",
                "Task in Sprint 1",
                "Standalone Task in Alpha",
            ]) {
                await expect(
                    page.locator(".gantt-row-description .row-name").filter({
                        hasText: new RegExp(`^${title}$`),
                    }),
                ).toBeVisible({ timeout: 10_000 });
            }

            // Verify nesting via API
            const result = await queryTasks();
            const alpha = result.tasks.find((t) => t.title === "Project Alpha");
            const sprint = result.tasks.find((t) => t.title === "Sprint 1");
            const leaf = result.tasks.find(
                (t) => t.title === "Task in Sprint 1",
            );

            expect(alpha).toBeDefined();
            expect(sprint).toBeDefined();
            expect(leaf).toBeDefined();

            // Outer group has 2 children (Sprint 1 + Standalone Task)
            expect(alpha!.children).toHaveLength(2);
            expect(alpha!.parent).toBeNull();

            // Inner group is a child of outer group
            expect(sprint!.parent!.dbId).toBe(alpha!.dbId);
            expect(sprint!.children).toHaveLength(1);

            // Leaf is a child of inner group
            expect(leaf!.parent!.dbId).toBe(sprint!.dbId);
            expect(leaf!.children).toHaveLength(0);
        });

        test("collapsing outer group hides inner group and its children", async ({
            page,
        }) => {
            const outerGroup = await createTask({
                title: "Outer Group",
                designation: "GROUP",
            });

            const innerGroup = await createTask({
                title: "Inner Group",
                designation: "GROUP",
                parentId: outerGroup.dbId,
            });

            await createTask({
                title: "Deep Leaf Task",
                designation: "TASK",
                effort: 2.0,
                parentId: innerGroup.dbId,
            });

            await page.goto("/#/");
            await page.waitForLoadState("networkidle");

            // Everything should be visible initially
            const outerRow = page.locator(".gantt-row-description .row-name", {
                hasText: "Outer Group",
            });
            const innerRow = page.locator(".gantt-row-description .row-name", {
                hasText: "Inner Group",
            });
            const deepLeaf = page.locator(".gantt-row-description .row-name", {
                hasText: "Deep Leaf Task",
            });

            await expect(outerRow).toBeVisible({ timeout: 10_000 });
            await expect(innerRow).toBeVisible({ timeout: 10_000 });
            await expect(deepLeaf).toBeVisible({ timeout: 10_000 });

            // Collapse the outer group
            const outerRowContainer = page.locator(".gantt-row-description", {
                hasText: "Outer Group",
            });
            const outerToggle = outerRowContainer.locator(".q-btn").first();
            await outerToggle.click();

            // Inner group and deep leaf should become hidden
            await expect(innerRow).toBeHidden({ timeout: 5_000 });
            await expect(deepLeaf).toBeHidden({ timeout: 5_000 });

            // Outer group should remain visible
            await expect(outerRow).toBeVisible();
        });
    });

    // -----------------------------------------------------------------------
    // Full project structure via API
    // -----------------------------------------------------------------------

    test.describe("Full project structure via API", () => {
        test("create a complete project with all task types and verify in UI", async ({
            page,
        }) => {
            const futureDate = new Date();
            futureDate.setDate(futureDate.getDate() + 1);
            const milestoneDate = new Date();
            milestoneDate.setDate(milestoneDate.getDate() + 60);

            // Create resource
            const resource = await createResource("Alice Developer");

            // Create requirement
            const requirement = await createTask({
                title: "Project Start",
                description: "Project cannot start before this date",
                designation: "REQUIREMENT",
                priority: 1.0,
                earliestStart: futureDate.toISOString(),
            });

            // Create group
            const group = await createTask({
                title: "Feature: Authentication",
                description: "Authentication feature group",
                designation: "GROUP",
            });

            // Create tasks within group with dependencies
            const task1 = await createTask({
                title: "Design Auth Flow",
                description: "Design the authentication flow",
                designation: "TASK",
                priority: 2.0,
                effort: 8.0,
                parentId: group.dbId,
                predecessors: [requirement.dbId],
                resourceConstraints: [
                    {
                        optional: false,
                        speed: 1.0,
                        entries: [{ resourceId: resource.dbId }],
                    },
                ],
            });

            const task2 = await createTask({
                title: "Implement Auth Backend",
                description: "Backend auth implementation",
                designation: "TASK",
                effort: 16.0,
                parentId: group.dbId,
                predecessors: [task1.dbId],
                resourceConstraints: [
                    {
                        optional: false,
                        speed: 1.0,
                        entries: [{ resourceId: resource.dbId }],
                    },
                ],
            });

            const task3 = await createTask({
                title: "Implement Auth Frontend",
                description: "Frontend auth implementation",
                designation: "TASK",
                effort: 12.0,
                parentId: group.dbId,
                predecessors: [task1.dbId],
                resourceConstraints: [
                    {
                        optional: false,
                        speed: 1.0,
                        entries: [{ resourceId: resource.dbId }],
                    },
                ],
            });

            // Create milestone depending on both implementation tasks
            await createTask({
                title: "Auth Complete",
                description: "Authentication must be complete by this date",
                designation: "MILESTONE",
                scheduleTarget: milestoneDate.toISOString(),
                predecessors: [task2.dbId, task3.dbId],
            });

            // Navigate and verify everything shows up
            await page.goto("/#/");
            await page.waitForLoadState("networkidle");

            const expectedTitles = [
                "Project Start",
                "Feature: Authentication",
                "Design Auth Flow",
                "Implement Auth Backend",
                "Implement Auth Frontend",
                "Auth Complete",
            ];

            for (const title of expectedTitles) {
                await expect(
                    page.locator(".gantt-row-description .row-name", {
                        hasText: title,
                    }),
                ).toBeVisible({ timeout: 10_000 });
            }

            // Verify full structure via API
            const tasksResult = await queryTasks();
            expect(tasksResult.tasks).toHaveLength(6);

            // Verify resource constraint was applied
            const designTask = tasksResult.tasks.find(
                (t) => t.title === "Design Auth Flow",
            );
            expect(designTask).toBeDefined();
            expect(designTask!.resourceConstraints).toHaveLength(1);
            expect(
                designTask!.resourceConstraints[0]!.entries[0]!.resource.dbId,
            ).toBe(resource.dbId);
        });
    });

    // -----------------------------------------------------------------------
    // Task sidebar interaction
    // -----------------------------------------------------------------------

    test.describe("Task sidebar interaction", () => {
        test("clicking a task row opens the sidebar with task details", async ({
            page,
        }) => {
            await createTask({
                title: "Clickable Task",
                description: "Click me to open sidebar",
                designation: "TASK",
                effort: 4.0,
            });

            await page.goto("/#/");
            await page.waitForLoadState("networkidle");

            // Click the task row
            const row = page.locator(".gantt-row-description", {
                hasText: "Clickable Task",
            });
            await expect(row).toBeVisible({ timeout: 10_000 });
            await row.click();

            // The right-side drawer should open and display the task title
            const sidebar = page.locator(".q-drawer--right");
            await expect(sidebar).toBeVisible({ timeout: 10_000 });
            await expect(sidebar.getByText("Clickable Task")).toBeVisible({
                timeout: 10_000,
            });
        });

        test("clicking a different task switches the sidebar content", async ({
            page,
        }) => {
            await createTask({
                title: "First Task",
                description: "First task description",
                designation: "TASK",
                effort: 4.0,
            });

            await createTask({
                title: "Second Task",
                description: "Second task description",
                designation: "TASK",
                effort: 8.0,
            });

            await page.goto("/#/");
            await page.waitForLoadState("networkidle");

            // Click the first task
            const row1 = page.locator(".gantt-row-description", {
                hasText: "First Task",
            });
            await expect(row1).toBeVisible({ timeout: 10_000 });
            await row1.click();

            const sidebar = page.locator(".q-drawer--right");
            await expect(sidebar).toBeVisible({ timeout: 10_000 });
            await expect(
                sidebar.locator(".text-h5", { hasText: "First Task" }),
            ).toBeVisible({
                timeout: 10_000,
            });

            // Click the second task
            const row2 = page.locator(".gantt-row-description", {
                hasText: "Second Task",
            });
            await row2.click();

            await expect(
                sidebar.locator(".text-h5", { hasText: "Second Task" }),
            ).toBeVisible({
                timeout: 10_000,
            });
        });
    });
});
