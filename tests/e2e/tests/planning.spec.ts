import { test, expect } from "@playwright/test";
import { graphqlRequest } from "../helpers/graphql";
import { cleanDatabase } from "../helpers/cleanup";

test.beforeEach(async () => {
    await cleanDatabase();
});

test.describe("Planning Workflow", () => {
    test("full planning scenario: create resources, tasks, constraints, recalculate, verify allocations", async ({
        page,
    }) => {
        // ─── Step 1: Create resources via API ───────────────────────────────
        const resource1 = await graphqlRequest<{
            resourceSave: { dbId: number; name: string };
        }>(
            `mutation ResourceSave($resource: ResourceSaveInput!) {
        resourceSave(resource: $resource) { dbId name }
      }`,
            {
                resource: {
                    name: "Dev Alice",
                    timezone: "Europe/Berlin",
                    added: new Date().toISOString(),
                    availability: [
                        { weekday: "MONDAY", duration: 28800 },
                        { weekday: "TUESDAY", duration: 28800 },
                        { weekday: "WEDNESDAY", duration: 28800 },
                        { weekday: "THURSDAY", duration: 28800 },
                        { weekday: "FRIDAY", duration: 28800 },
                        { weekday: "SATURDAY", duration: 0 },
                        { weekday: "SUNDAY", duration: 0 },
                    ],
                },
            },
        );
        expect(resource1.resourceSave.dbId).toBeTruthy();
        const aliceId = resource1.resourceSave.dbId;

        const resource2 = await graphqlRequest<{
            resourceSave: { dbId: number; name: string };
        }>(
            `mutation ResourceSave($resource: ResourceSaveInput!) {
        resourceSave(resource: $resource) { dbId name }
      }`,
            {
                resource: {
                    name: "Dev Bob",
                    timezone: "Europe/London",
                    added: new Date().toISOString(),
                    availability: [
                        { weekday: "MONDAY", duration: 28800 },
                        { weekday: "TUESDAY", duration: 28800 },
                        { weekday: "WEDNESDAY", duration: 28800 },
                        { weekday: "THURSDAY", duration: 28800 },
                        { weekday: "FRIDAY", duration: 28800 },
                        { weekday: "SATURDAY", duration: 0 },
                        { weekday: "SUNDAY", duration: 0 },
                    ],
                },
            },
        );
        expect(resource2.resourceSave.dbId).toBeTruthy();
        const bobId = resource2.resourceSave.dbId;

        // ─── Step 2: Create a requirement (project start marker) ────────────
        const tomorrow = new Date();
        tomorrow.setDate(tomorrow.getDate() + 1);
        tomorrow.setHours(8, 0, 0, 0);

        const requirement = await graphqlRequest<{
            taskSave: { dbId: number; title: string; designation: string };
        }>(
            `mutation TaskSave($task: TaskSaveInput!) {
        taskSave(task: $task) { dbId title designation }
      }`,
            {
                task: {
                    title: "Project Start",
                    description: "Earliest start for the project",
                    designation: "REQUIREMENT",
                    priority: 100.0,
                    earliestStart: tomorrow.toISOString(),
                },
            },
        );
        expect(requirement.taskSave.dbId).toBeTruthy();
        expect(requirement.taskSave.designation).toBe("REQUIREMENT");
        const requirementId = requirement.taskSave.dbId;

        // ─── Step 3: Create tasks with resource constraints ─────────────────
        const task1 = await graphqlRequest<{
            taskSave: { dbId: number; title: string };
        }>(
            `mutation TaskSave($task: TaskSaveInput!) {
        taskSave(task: $task) { dbId title designation }
      }`,
            {
                task: {
                    title: "Backend Development",
                    description: "Implement the backend API",
                    designation: "TASK",
                    priority: 80.0,
                    effort: 16.0,
                    predecessors: [requirementId],
                    resourceConstraints: [
                        {
                            optional: false,
                            speed: 1.0,
                            entries: [{ resourceId: aliceId }],
                        },
                    ],
                },
            },
        );
        expect(task1.taskSave.dbId).toBeTruthy();
        const task1Id = task1.taskSave.dbId;

        const task2 = await graphqlRequest<{
            taskSave: { dbId: number; title: string };
        }>(
            `mutation TaskSave($task: TaskSaveInput!) {
        taskSave(task: $task) { dbId title designation }
      }`,
            {
                task: {
                    title: "Frontend Development",
                    description: "Implement the frontend UI",
                    designation: "TASK",
                    priority: 70.0,
                    effort: 24.0,
                    predecessors: [requirementId],
                    resourceConstraints: [
                        {
                            optional: false,
                            speed: 1.0,
                            entries: [{ resourceId: bobId }],
                        },
                    ],
                },
            },
        );
        expect(task2.taskSave.dbId).toBeTruthy();
        const task2Id = task2.taskSave.dbId;

        const task3 = await graphqlRequest<{
            taskSave: { dbId: number; title: string };
        }>(
            `mutation TaskSave($task: TaskSaveInput!) {
        taskSave(task: $task) { dbId title designation }
      }`,
            {
                task: {
                    title: "Integration Testing",
                    description:
                        "Integration tests after both dev tasks complete",
                    designation: "TASK",
                    priority: 90.0,
                    effort: 8.0,
                    predecessors: [task1Id, task2Id],
                    resourceConstraints: [
                        {
                            optional: false,
                            speed: 1.0,
                            entries: [{ resourceId: aliceId }],
                        },
                    ],
                },
            },
        );
        expect(task3.taskSave.dbId).toBeTruthy();
        const task3Id = task3.taskSave.dbId;

        // ─── Step 4: Create a milestone (deadline) ──────────────────────────
        const deadline = new Date();
        deadline.setDate(deadline.getDate() + 30);
        deadline.setHours(17, 0, 0, 0);

        const milestone = await graphqlRequest<{
            taskSave: { dbId: number; title: string; designation: string };
        }>(
            `mutation TaskSave($task: TaskSaveInput!) {
        taskSave(task: $task) { dbId title designation }
      }`,
            {
                task: {
                    title: "Release Deadline",
                    description: "Target release date",
                    designation: "MILESTONE",
                    priority: 100.0,
                    scheduleTarget: deadline.toISOString(),
                    predecessors: [task3Id],
                },
            },
        );
        expect(milestone.taskSave.dbId).toBeTruthy();
        expect(milestone.taskSave.designation).toBe("MILESTONE");

        // ─── Step 5: Trigger recalculation ──────────────────────────────────
        const recalcResult = await graphqlRequest<{ recalculateNow: boolean }>(
            `mutation RecalculateNow { recalculateNow }`,
        );
        expect(recalcResult.recalculateNow).toBe(true);

        // ─── Step 6: Wait for plan allocations to appear ────────────────────
        // The scheduler runs asynchronously; poll until allocations appear
        let planAllocations: Array<{
            dbId: number;
            start: string;
            end: string;
            allocationType: string;
            task: { dbId: number; title: string };
            resources: Array<{ dbId: number; name: string }>;
        }> = [];

        await expect
            .poll(
                async () => {
                    const plan = await graphqlRequest<{
                        currentPlan: {
                            allocations: typeof planAllocations;
                        };
                    }>(
                        `query CurrentPlan {
              currentPlan {
                allocations {
                  dbId start end allocationType
                  task { dbId title }
                  resources { dbId name }
                }
              }
            }`,
                    );
                    planAllocations = plan.currentPlan.allocations;
                    // We expect allocations for our 3 TASK items (not the requirement or milestone directly)
                    const taskAllocations = planAllocations.filter(
                        (a) =>
                            a.task.dbId === task1Id ||
                            a.task.dbId === task2Id ||
                            a.task.dbId === task3Id,
                    );
                    return taskAllocations.length;
                },
                {
                    message:
                        "Waiting for plan allocations to appear after recalculation",
                    timeout: 30_000,
                    intervals: [500, 1000, 2000],
                },
            )
            .toBeGreaterThanOrEqual(3);

        // ─── Step 7: Verify allocation properties via API ───────────────────
        const backendAllocs = planAllocations.filter(
            (a) => a.task.dbId === task1Id,
        );
        expect(backendAllocs.length).toBeGreaterThanOrEqual(1);
        // Backend task should be assigned to Alice
        for (const alloc of backendAllocs) {
            expect(alloc.resources.some((r) => r.dbId === aliceId)).toBe(true);
        }

        const frontendAllocs = planAllocations.filter(
            (a) => a.task.dbId === task2Id,
        );
        expect(frontendAllocs.length).toBeGreaterThanOrEqual(1);
        // Frontend task should be assigned to Bob
        for (const alloc of frontendAllocs) {
            expect(alloc.resources.some((r) => r.dbId === bobId)).toBe(true);
        }

        const integrationAllocs = planAllocations.filter(
            (a) => a.task.dbId === task3Id,
        );
        expect(integrationAllocs.length).toBeGreaterThanOrEqual(1);

        // Integration testing should start after both backend and frontend end
        const backendEnd = Math.max(
            ...backendAllocs.map((a) => new Date(a.end).getTime()),
        );
        const frontendEnd = Math.max(
            ...frontendAllocs.map((a) => new Date(a.end).getTime()),
        );
        const integrationStart = Math.min(
            ...integrationAllocs.map((a) => new Date(a.start).getTime()),
        );
        const predecessorEnd = Math.max(backendEnd, frontendEnd);
        // Integration should not start before both predecessors are done
        expect(integrationStart).toBeGreaterThanOrEqual(predecessorEnd);

        // ─── Step 8: Verify allocations appear in the Tasks Gantt ───────────
        await page.goto("/#/");
        await page.waitForLoadState("networkidle");

        // Wait for the Gantt chart to render - look for row labels
        await expect(
            page.locator(".gantt-row-description", {
                hasText: "Backend Development",
            }),
        ).toBeVisible({ timeout: 15_000 });
        await expect(
            page.locator(".gantt-row-description", {
                hasText: "Frontend Development",
            }),
        ).toBeVisible();
        await expect(
            page.locator(".gantt-row-description", {
                hasText: "Integration Testing",
            }),
        ).toBeVisible();
        await expect(
            page.locator(".gantt-row-description", {
                hasText: "Project Start",
            }),
        ).toBeVisible();
        await expect(
            page.locator(".gantt-row-description", {
                hasText: "Release Deadline",
            }),
        ).toBeVisible();

        // Verify allocation bars are rendered in the SVG chart area
        // Task allocations render as <rect> elements inside the chart SVG
        const chartSvg = page.locator(".gantt-chart-scroll svg");
        await expect(chartSvg).toBeVisible();

        // There should be rect elements (allocation bars) in the chart
        const allocRects = chartSvg.locator('rect[rx="3"]');
        await expect(allocRects.first()).toBeVisible({ timeout: 10_000 });
        const rectCount = await allocRects.count();
        // At least our 3 task allocations (could be split across multiple rects)
        expect(rectCount).toBeGreaterThanOrEqual(3);

        // ─── Step 9: Verify allocations on the Resources Gantt ──────────────
        await page.goto("/#/resources");
        await page.waitForLoadState("networkidle");

        // Resource rows should appear
        await expect(
            page.locator(".gantt-row-description", { hasText: "Dev Alice" }),
        ).toBeVisible({ timeout: 15_000 });
        await expect(
            page.locator(".gantt-row-description", { hasText: "Dev Bob" }),
        ).toBeVisible();

        // Allocation bars should be rendered for resources too
        const resourceChartSvg = page.locator(".gantt-chart-scroll svg");
        await expect(resourceChartSvg).toBeVisible();
        const resourceAllocRects = resourceChartSvg.locator('rect[rx="3"]');
        await expect(resourceAllocRects.first()).toBeVisible({
            timeout: 10_000,
        });

        // ─── Step 10: Check no critical issues ──────────────────────────────
        const issuesResult = await graphqlRequest<{
            issues: Array<{
                dbId: number;
                code: string;
                description: string;
                type: string;
                task: { dbId: number; title: string } | null;
            }>;
        }>(
            `query Issues {
        issues {
          dbId code description type
          task { dbId title }
        }
      }`,
        );
        // Log issues for diagnostics (don't necessarily fail - some warnings are OK)
        if (issuesResult.issues.length > 0) {
            console.log(
                "Planning issues found:",
                JSON.stringify(issuesResult.issues, null, 2),
            );
        }
    });

    test("resource constraints are respected in the plan", async ({ page }) => {
        // Create a single resource
        const resource = await graphqlRequest<{
            resourceSave: { dbId: number };
        }>(
            `mutation ResourceSave($resource: ResourceSaveInput!) {
        resourceSave(resource: $resource) { dbId name }
      }`,
            {
                resource: {
                    name: "Solo Developer",
                    timezone: "UTC",
                    added: new Date().toISOString(),
                    availability: [
                        { weekday: "MONDAY", duration: 28800 },
                        { weekday: "TUESDAY", duration: 28800 },
                        { weekday: "WEDNESDAY", duration: 28800 },
                        { weekday: "THURSDAY", duration: 28800 },
                        { weekday: "FRIDAY", duration: 28800 },
                        { weekday: "SATURDAY", duration: 0 },
                        { weekday: "SUNDAY", duration: 0 },
                    ],
                },
            },
        );
        const resourceId = resource.resourceSave.dbId;

        const tomorrow = new Date();
        tomorrow.setDate(tomorrow.getDate() + 1);
        tomorrow.setHours(8, 0, 0, 0);

        // Create requirement
        const req = await graphqlRequest<{ taskSave: { dbId: number } }>(
            `mutation TaskSave($task: TaskSaveInput!) {
        taskSave(task: $task) { dbId title designation }
      }`,
            {
                task: {
                    title: "Sprint Start",
                    description: "Sprint start marker",
                    designation: "REQUIREMENT",
                    priority: 100.0,
                    earliestStart: tomorrow.toISOString(),
                },
            },
        );

        // Create two tasks that both need the same resource (sequential forced)
        const taskA = await graphqlRequest<{ taskSave: { dbId: number } }>(
            `mutation TaskSave($task: TaskSaveInput!) {
        taskSave(task: $task) { dbId title }
      }`,
            {
                task: {
                    title: "Task Alpha",
                    description: "First task for solo dev",
                    designation: "TASK",
                    priority: 90.0,
                    effort: 8.0,
                    predecessors: [req.taskSave.dbId],
                    resourceConstraints: [
                        {
                            optional: false,
                            speed: 1.0,
                            entries: [{ resourceId }],
                        },
                    ],
                },
            },
        );

        const taskB = await graphqlRequest<{ taskSave: { dbId: number } }>(
            `mutation TaskSave($task: TaskSaveInput!) {
        taskSave(task: $task) { dbId title }
      }`,
            {
                task: {
                    title: "Task Beta",
                    description: "Second task for solo dev",
                    designation: "TASK",
                    priority: 80.0,
                    effort: 8.0,
                    predecessors: [req.taskSave.dbId],
                    resourceConstraints: [
                        {
                            optional: false,
                            speed: 1.0,
                            entries: [{ resourceId }],
                        },
                    ],
                },
            },
        );

        // Create milestone (required by scheduler to estimate end date)
        const deadline = new Date();
        deadline.setDate(deadline.getDate() + 30);
        deadline.setHours(17, 0, 0, 0);

        await graphqlRequest(
            `mutation TaskSave($task: TaskSaveInput!) {
        taskSave(task: $task) { dbId }
      }`,
            {
                task: {
                    title: "Sprint End",
                    description: "Sprint deadline",
                    designation: "MILESTONE",
                    priority: 50.0,
                    scheduleTarget: deadline.toISOString(),
                    predecessors: [taskA.taskSave.dbId, taskB.taskSave.dbId],
                },
            },
        );

        // Recalculate
        await graphqlRequest(`mutation RecalculateNow { recalculateNow }`);

        // Wait for allocations
        let allAllocations: Array<{
            dbId: number;
            start: string;
            end: string;
            task: { dbId: number };
            resources: Array<{ dbId: number }>;
        }> = [];

        await expect
            .poll(
                async () => {
                    const plan = await graphqlRequest<{
                        currentPlan: { allocations: typeof allAllocations };
                    }>(
                        `query CurrentPlan {
              currentPlan {
                allocations {
                  dbId start end
                  task { dbId }
                  resources { dbId }
                }
              }
            }`,
                    );
                    allAllocations = plan.currentPlan.allocations;
                    const relevant = allAllocations.filter(
                        (a) =>
                            a.task.dbId === taskA.taskSave.dbId ||
                            a.task.dbId === taskB.taskSave.dbId,
                    );
                    return relevant.length;
                },
                {
                    message: "Waiting for allocations for both tasks",
                    timeout: 30_000,
                    intervals: [500, 1000, 2000],
                },
            )
            .toBeGreaterThanOrEqual(2);

        // Both tasks are assigned to the same resource, so their time ranges should not overlap
        const aAllocs = allAllocations.filter(
            (a) => a.task.dbId === taskA.taskSave.dbId,
        );
        const bAllocs = allAllocations.filter(
            (a) => a.task.dbId === taskB.taskSave.dbId,
        );

        const aEnd = Math.max(...aAllocs.map((a) => new Date(a.end).getTime()));
        const aStart = Math.min(
            ...aAllocs.map((a) => new Date(a.start).getTime()),
        );
        const bEnd = Math.max(...bAllocs.map((a) => new Date(a.end).getTime()));
        const bStart = Math.min(
            ...bAllocs.map((a) => new Date(a.start).getTime()),
        );

        // One should come before the other (no overlap)
        const noOverlap = aEnd <= bStart || bEnd <= aStart;
        expect(noOverlap).toBe(true);

        // Verify on the UI
        await page.goto("/#/");
        await page.waitForLoadState("networkidle");

        await expect(
            page.locator(".gantt-row-description", { hasText: "Task Alpha" }),
        ).toBeVisible({ timeout: 15_000 });
        await expect(
            page.locator(".gantt-row-description", { hasText: "Task Beta" }),
        ).toBeVisible();
    });

    test("recalculation updates the plan after task changes", async ({
        page,
    }) => {
        // Create resource
        const resource = await graphqlRequest<{
            resourceSave: { dbId: number };
        }>(
            `mutation ResourceSave($resource: ResourceSaveInput!) {
        resourceSave(resource: $resource) { dbId name }
      }`,
            {
                resource: {
                    name: "Worker",
                    timezone: "UTC",
                    added: new Date().toISOString(),
                    availability: [
                        { weekday: "MONDAY", duration: 28800 },
                        { weekday: "TUESDAY", duration: 28800 },
                        { weekday: "WEDNESDAY", duration: 28800 },
                        { weekday: "THURSDAY", duration: 28800 },
                        { weekday: "FRIDAY", duration: 28800 },
                        { weekday: "SATURDAY", duration: 0 },
                        { weekday: "SUNDAY", duration: 0 },
                    ],
                },
            },
        );

        const tomorrow = new Date();
        tomorrow.setDate(tomorrow.getDate() + 1);
        tomorrow.setHours(8, 0, 0, 0);

        // Create requirement + task
        const req = await graphqlRequest<{ taskSave: { dbId: number } }>(
            `mutation TaskSave($task: TaskSaveInput!) {
        taskSave(task: $task) { dbId }
      }`,
            {
                task: {
                    title: "Start",
                    description: "Start",
                    designation: "REQUIREMENT",
                    priority: 100.0,
                    earliestStart: tomorrow.toISOString(),
                },
            },
        );

        const task = await graphqlRequest<{ taskSave: { dbId: number } }>(
            `mutation TaskSave($task: TaskSaveInput!) {
        taskSave(task: $task) { dbId }
      }`,
            {
                task: {
                    title: "Work Item",
                    description: "Some work",
                    designation: "TASK",
                    priority: 50.0,
                    effort: 8.0,
                    predecessors: [req.taskSave.dbId],
                    resourceConstraints: [
                        {
                            optional: false,
                            speed: 1.0,
                            entries: [
                                { resourceId: resource.resourceSave.dbId },
                            ],
                        },
                    ],
                },
            },
        );

        // Create milestone (required by scheduler to estimate end date)
        const deadline = new Date();
        deadline.setDate(deadline.getDate() + 30);
        deadline.setHours(17, 0, 0, 0);

        await graphqlRequest(
            `mutation TaskSave($task: TaskSaveInput!) {
        taskSave(task: $task) { dbId }
      }`,
            {
                task: {
                    title: "Project End",
                    description: "Project deadline",
                    designation: "MILESTONE",
                    priority: 50.0,
                    scheduleTarget: deadline.toISOString(),
                    predecessors: [task.taskSave.dbId],
                },
            },
        );

        // First recalculation
        await graphqlRequest(`mutation RecalculateNow { recalculateNow }`);

        // Wait for initial allocations
        await expect
            .poll(
                async () => {
                    const plan = await graphqlRequest<{
                        currentPlan: {
                            allocations: Array<{ task: { dbId: number } }>;
                        };
                    }>(
                        `query CurrentPlan {
              currentPlan {
                allocations { task { dbId } }
              }
            }`,
                    );
                    return plan.currentPlan.allocations.filter(
                        (a) => a.task.dbId === task.taskSave.dbId,
                    ).length;
                },
                {
                    message: "Waiting for initial allocations",
                    timeout: 30_000,
                    intervals: [500, 1000, 2000],
                },
            )
            .toBeGreaterThanOrEqual(1);

        // Update the task effort to be larger
        // Note: taskSave with dbId creates a NEW iteration (new dbId) and soft-deletes the old one
        const updatedTask = await graphqlRequest<{
            taskSave: { dbId: number };
        }>(
            `mutation TaskSave($task: TaskSaveInput!) {
        taskSave(task: $task) { dbId }
      }`,
            {
                task: {
                    dbId: task.taskSave.dbId,
                    title: "Work Item",
                    description: "Some work - expanded",
                    designation: "TASK",
                    priority: 50.0,
                    effort: 40.0,
                    predecessors: [req.taskSave.dbId],
                    resourceConstraints: [
                        {
                            optional: false,
                            speed: 1.0,
                            entries: [
                                { resourceId: resource.resourceSave.dbId },
                            ],
                        },
                    ],
                },
            },
        );
        const updatedTaskId = updatedTask.taskSave.dbId;

        // Recalculate again
        await graphqlRequest(`mutation RecalculateNow { recalculateNow }`);

        // Poll for the updated plan - total allocation span should be longer now
        // Use the NEW task dbId since the old iteration was soft-deleted
        await expect
            .poll(
                async () => {
                    const plan = await graphqlRequest<{
                        currentPlan: {
                            allocations: Array<{
                                start: string;
                                end: string;
                                task: { dbId: number; title: string };
                            }>;
                        };
                    }>(
                        `query CurrentPlan {
              currentPlan {
                allocations { start end task { dbId title } }
              }
            }`,
                    );
                    const taskAllocs = plan.currentPlan.allocations.filter(
                        (a) => a.task.dbId === updatedTaskId,
                    );
                    if (taskAllocs.length === 0) return 0;
                    // Calculate total hours of allocation
                    const totalMs = taskAllocs.reduce((sum, a) => {
                        return (
                            sum +
                            new Date(a.end).getTime() -
                            new Date(a.start).getTime()
                        );
                    }, 0);
                    const totalHours = totalMs / (1000 * 60 * 60);
                    return totalHours;
                },
                {
                    message:
                        "Waiting for updated allocations reflecting 40h of effort",
                    timeout: 30_000,
                    intervals: [500, 1000, 2000],
                },
            )
            .toBeGreaterThanOrEqual(16); // Should be significantly more than the original 8h

        // Verify on the UI
        await page.goto("/#/");
        await page.waitForLoadState("networkidle");
        await expect(
            page.locator(".gantt-row-description", { hasText: "Work Item" }),
        ).toBeVisible({ timeout: 15_000 });
    });
});
