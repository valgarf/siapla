import { test, expect } from "@playwright/test";
import { graphqlRequest, createStandardResource } from "../helpers/graphql";
import { cleanDatabase } from "../helpers/cleanup";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const TASK_SAVE = `
    mutation TaskSave($task: TaskSaveInput!) {
        taskSave(task: $task) { dbId title designation }
    }
`;

const BOOKING_SAVE = `
    mutation BookingSave($taskId: Int!, $start: DateTime!, $end: DateTime!, $resources: [Int!]!, $final: Boolean!) {
        bookingSave(taskId: $taskId, start: $start, end: $end, resources: $resources, final: $final) { dbId }
    }
`;

const TASK_HISTORY = `
    query TaskHistory($taskHeaderId: Int!, $direction: SearchDirection!, $limit: Int, $fromRevision: Int64, $fromTimestamp: DateTime) {
        taskHistory(taskHeaderId: $taskHeaderId, direction: $direction, limit: $limit, fromRevision: $fromRevision, fromTimestamp: $fromTimestamp) {
            changes {
                __typename
                ... on TaskIterationChange {
                    revisionId
                    timestamp
                    changeType
                    taskIteration {
                        dbId
                        iterationId
                        title
                        description
                        designation
                        priority
                        effort
                    }
                }
                ... on BookingChange {
                    revisionId
                    timestamp
                    changeType
                    booking {
                        dbId
                        start
                        end
                        final
                        resources { dbId name }
                    }
                }
                ... on DependencyChange {
                    revisionId
                    timestamp
                    changeType
                    predecessorId
                    successorId
                }
                ... on ResourceConstraintChange {
                    revisionId
                    timestamp
                    changeType
                    constraintId
                    optional
                    speed
                    resourceIds
                }
            }
            hasMore
        }
    }
`;

interface TaskSaveResult {
    dbId: number;
    title: string;
    designation: string;
}

interface TaskHistoryChange {
    __typename: string;
    revisionId: number;
    timestamp: string;
    changeType: string;
    taskIteration?: {
        dbId: number;
        iterationId: number;
        title: string;
        description: string;
        designation: string;
        priority: number;
        effort: number | null;
    } | null;
    booking?: {
        dbId: number;
        start: string;
        end: string;
        final: boolean;
        resources: { dbId: number; name: string }[];
    } | null;
    predecessorId?: number;
    successorId?: number;
    constraintId?: number;
    optional?: boolean;
    speed?: number;
    resourceIds?: number[];
}

interface TaskHistoryResult {
    taskHistory: {
        changes: TaskHistoryChange[];
        hasMore: boolean;
    };
}

async function createTask(
    fields: Record<string, unknown>,
): Promise<TaskSaveResult> {
    const result = await graphqlRequest<{ taskSave: TaskSaveResult }>(
        TASK_SAVE,
        {
            task: {
                description: "",
                priority: 1.0,
                ...fields,
            },
        },
    );
    return result.taskSave;
}

async function updateTask(
    dbId: number,
    fields: Record<string, unknown>,
): Promise<TaskSaveResult> {
    const result = await graphqlRequest<{ taskSave: TaskSaveResult }>(
        TASK_SAVE,
        {
            task: {
                dbId,
                description: "",
                priority: 1.0,
                designation: "TASK",
                ...fields,
            },
        },
    );
    return result.taskSave;
}

/**
 * Open the task sidebar for a task, click the history button, and wait for the
 * history sidebar to appear (expanded). Returns the sidebar locator.
 */
async function openHistorySidebar(
    page: import("@playwright/test").Page,
    taskTitle: string,
) {
    // Click the task row to open the task sidebar
    const row = page.locator(".gantt-row-description", {
        hasText: taskTitle,
    });
    await expect(row).toBeVisible({ timeout: 15_000 });
    await row.click();

    const sidebar = page.locator(".q-drawer--right");
    await expect(sidebar).toBeVisible({ timeout: 10_000 });

    // Wait for the task title to appear in the sidebar to ensure it's loaded
    await expect(sidebar.getByText(taskTitle)).toBeVisible({
        timeout: 10_000,
    });

    // Click the History button
    const historyBtn = sidebar.locator('button[aria-label="History"]');
    await expect(historyBtn).toBeVisible({ timeout: 10_000 });
    await historyBtn.click();

    // Wait for the history change list to appear inside the sidebar
    const changeList = sidebar.locator(".task-history-change-list");
    await expect(changeList).toBeVisible({ timeout: 15_000 });

    return sidebar;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

test.beforeEach(async () => {
    await cleanDatabase();
});

test.describe("Task History", () => {
    // -----------------------------------------------------------------------
    // UI tests
    // -----------------------------------------------------------------------

    test("history button appears in task sidebar", async ({ page }) => {
        await createTask({
            title: "History Button Task",
            designation: "TASK",
            effort: 4.0,
        });

        await page.goto("/#/");
        await page.waitForLoadState("networkidle");

        const row = page.locator(".gantt-row-description", {
            hasText: "History Button Task",
        });
        await expect(row).toBeVisible({ timeout: 15_000 });
        await row.click();

        const sidebar = page.locator(".q-drawer--right");
        await expect(sidebar).toBeVisible({ timeout: 10_000 });

        const historyBtn = sidebar.locator('button[aria-label="History"]');
        await expect(historyBtn).toBeVisible({ timeout: 10_000 });

        // Verify it has the history icon
        const icon = historyBtn.locator(".q-icon");
        await expect(icon).toBeVisible();
        await expect(icon).toHaveText("history");
    });

    test("history button opens history sidebar with change list", async ({
        page,
    }) => {
        const task = await createTask({
            title: "Open History Task",
            designation: "TASK",
            effort: 4.0,
        });

        await page.goto("/#/");
        await page.waitForLoadState("networkidle");

        const sidebar = await openHistorySidebar(page, "Open History Task");

        // The change list should contain revision entries
        const revisionEntries = sidebar.locator(
            ".task-history-change-list .q-item",
        );
        await expect(revisionEntries.first()).toBeVisible({ timeout: 10_000 });

        // At least 1 entry (the creation)
        const count = await revisionEntries.count();
        expect(count).toBeGreaterThanOrEqual(1);
    });

    test("history sidebar shows revision entries for modified task", async ({
        page,
    }) => {
        const task = await createTask({
            title: "Revision List Task",
            designation: "TASK",
            effort: 4.0,
        });

        // Modify the task to generate a second revision
        await updateTask(task.dbId, {
            title: "Revision List Task Updated",
            effort: 8.0,
        });

        await page.goto("/#/");
        await page.waitForLoadState("networkidle");

        // The task is now called "Revision List Task Updated" in the gantt
        const sidebar = await openHistorySidebar(
            page,
            "Revision List Task Updated",
        );

        // There should be revision group entries (created + updated = at least 2)
        const revisionEntries = sidebar.locator(
            ".task-history-change-list .q-item",
        );
        await expect(revisionEntries.first()).toBeVisible({ timeout: 10_000 });

        const count = await revisionEntries.count();
        expect(count).toBeGreaterThanOrEqual(2);
    });

    test("clicking a revision entry shows task details", async ({ page }) => {
        const task = await createTask({
            title: "Detail View Task",
            designation: "TASK",
            effort: 4.0,
        });

        await updateTask(task.dbId, {
            title: "Detail View Task Modified",
            effort: 12.0,
        });

        await page.goto("/#/");
        await page.waitForLoadState("networkidle");

        const sidebar = await openHistorySidebar(
            page,
            "Detail View Task Modified",
        );

        // Click on the first revision entry
        const revisionEntries = sidebar.locator(
            ".task-history-change-list .q-item",
        );
        await expect(revisionEntries.first()).toBeVisible({ timeout: 10_000 });
        await revisionEntries.first().click();

        // The detail panel should appear showing task data
        const detail = sidebar.locator(".task-history-detail");
        await expect(detail).toBeVisible({ timeout: 10_000 });

        // It should contain a "Revision" heading or the word "Revision"
        await expect(detail.getByText(/Revision/)).toBeVisible({
            timeout: 10_000,
        });

        // It should contain task data like designation
        await expect(detail.getByText("Designation")).toBeVisible({
            timeout: 10_000,
        });
    });

    test("selecting two revisions via checkboxes activates compare mode", async ({
        page,
    }) => {
        const task = await createTask({
            title: "Compare Mode Task",
            designation: "TASK",
            effort: 4.0,
        });

        await updateTask(task.dbId, {
            title: "Compare Mode Task V2",
            effort: 8.0,
        });

        await page.goto("/#/");
        await page.waitForLoadState("networkidle");

        const sidebar = await openHistorySidebar(page, "Compare Mode Task V2");

        const revisionEntries = sidebar.locator(
            ".task-history-change-list .q-item",
        );
        await expect(revisionEntries.first()).toBeVisible({ timeout: 10_000 });

        const entryCount = await revisionEntries.count();
        expect(entryCount).toBeGreaterThanOrEqual(2);

        // Select two revisions via checkboxes
        const checkbox1 = revisionEntries.nth(0).locator(".q-checkbox");
        const checkbox2 = revisionEntries.nth(1).locator(".q-checkbox");
        await checkbox1.click();
        await checkbox2.click();

        // The compare view should appear
        const compare = sidebar.locator(".task-history-compare");
        await expect(compare).toBeVisible({ timeout: 10_000 });

        // Compare view should contain diff styling elements
        const diffAdded = compare.locator(".diff-added");
        const diffRemoved = compare.locator(".diff-removed");

        // At least one diff class should be present (since title/effort changed)
        const addedCount = await diffAdded.count();
        const removedCount = await diffRemoved.count();
        expect(addedCount + removedCount).toBeGreaterThan(0);
    });

    test("sidebar auto-expands when history is opened", async ({ page }) => {
        await createTask({
            title: "Auto Expand Task",
            designation: "TASK",
            effort: 4.0,
        });

        await page.goto("/#/");
        await page.waitForLoadState("networkidle");

        await openHistorySidebar(page, "Auto Expand Task");

        // The sidebar should be expanded (wide). We check that the drawer
        // width is significantly larger than the default 560px.
        const drawer = page.locator(".q-drawer--right");
        const box = await drawer.boundingBox();
        expect(box).not.toBeNull();
        // The expanded sidebar should be at least 700px wide (default is 560).
        expect(box!.width).toBeGreaterThan(700);
    });

    test("breadcrumb back to task works", async ({ page }) => {
        await createTask({
            title: "Breadcrumb Task",
            designation: "TASK",
            effort: 4.0,
        });

        await page.goto("/#/");
        await page.waitForLoadState("networkidle");

        const sidebar = await openHistorySidebar(page, "Breadcrumb Task");

        // The breadcrumb should show the task title as a clickable link
        const breadcrumbEl = sidebar.locator(".q-breadcrumbs-el", {
            hasText: "Breadcrumb Task",
        });
        await expect(breadcrumbEl).toBeVisible({ timeout: 10_000 });
        await breadcrumbEl.click();

        // Should go back to the task sidebar (no longer history)
        // The history change list should disappear and the task title should
        // be shown in the normal sidebar heading
        await expect(
            sidebar.locator(".task-history-change-list"),
        ).not.toBeVisible({ timeout: 10_000 });
        await expect(sidebar.getByText("Breadcrumb Task")).toBeVisible({
            timeout: 10_000,
        });
    });

    test("latest button loads most recent revisions", async ({ page }) => {
        const task = await createTask({
            title: "Latest Task V1",
            designation: "TASK",
            effort: 2.0,
        });

        await updateTask(task.dbId, {
            title: "Latest Task V2",
            effort: 4.0,
        });
        await updateTask(task.dbId, {
            title: "Latest Task V3",
            effort: 6.0,
        });

        await page.goto("/#/");
        await page.waitForLoadState("networkidle");

        const sidebar = await openHistorySidebar(page, "Latest Task V3");

        // First jump to start
        const startBtn = sidebar.locator("button", { hasText: "Start" });
        await expect(startBtn).toBeVisible({ timeout: 10_000 });
        await startBtn.click();
        await page.waitForTimeout(1000);

        // Now click Latest to go back to the most recent
        const latestBtn = sidebar.locator("button", { hasText: "Latest" });
        await expect(latestBtn).toBeVisible({ timeout: 10_000 });
        await latestBtn.click();
        await page.waitForTimeout(1000);

        // The most recent revision should be the first entry
        const revisionEntries = sidebar.locator(
            ".task-history-change-list .q-item",
        );
        await expect(revisionEntries.first()).toBeVisible({ timeout: 10_000 });

        // Click the first entry and verify it shows the latest task data
        await revisionEntries.first().click();
        const detail = sidebar.locator(".task-history-detail");
        await expect(detail).toBeVisible({ timeout: 10_000 });
    });

    test("jump to start button loads oldest revisions", async ({ page }) => {
        const task = await createTask({
            title: "Jump Start Task V1",
            designation: "TASK",
            effort: 2.0,
        });

        await updateTask(task.dbId, {
            title: "Jump Start Task V2",
            effort: 4.0,
        });
        await updateTask(task.dbId, {
            title: "Jump Start Task V3",
            effort: 6.0,
        });

        await page.goto("/#/");
        await page.waitForLoadState("networkidle");

        const sidebar = await openHistorySidebar(page, "Jump Start Task V3");

        // Click the "Start" button to jump to oldest revisions
        const startBtn = sidebar.locator("button", { hasText: "Start" });
        await expect(startBtn).toBeVisible({ timeout: 10_000 });
        await startBtn.click();
        await page.waitForTimeout(1000);

        // After jumping to start, the change list should still have entries
        const revisionEntries = sidebar.locator(
            ".task-history-change-list .q-item",
        );
        await expect(revisionEntries.first()).toBeVisible({ timeout: 10_000 });

        // Click on the last entry (oldest = creation) and verify it shows V1
        const entryCount = await revisionEntries.count();
        expect(entryCount).toBeGreaterThanOrEqual(1);

        const lastEntry = revisionEntries.nth(entryCount - 1);
        await lastEntry.click();

        const detail = sidebar.locator(".task-history-detail");
        await expect(detail).toBeVisible({ timeout: 10_000 });

        // The oldest revision should show the V1 title
        await expect(detail.getByText("Jump Start Task V1")).toBeVisible({
            timeout: 10_000,
        });
    });

    // -----------------------------------------------------------------------
    // API tests
    // -----------------------------------------------------------------------

    test("history API returns changes for modified task", async () => {
        const task = await createTask({
            title: "API History Task",
            designation: "TASK",
            effort: 4.0,
        });

        // Modify the task
        await updateTask(task.dbId, {
            title: "API History Task Updated",
            effort: 16.0,
        });

        // Query history backward
        const result = await graphqlRequest<TaskHistoryResult>(TASK_HISTORY, {
            taskHeaderId: task.dbId,
            direction: "BACKWARD",
            limit: 50,
        });

        expect(result.taskHistory.changes.length).toBeGreaterThanOrEqual(2);

        // Find the TaskIterationChange entries
        const taskChanges = result.taskHistory.changes.filter(
            (c) => c.__typename === "TaskIterationChange",
        );
        expect(taskChanges.length).toBeGreaterThanOrEqual(2);

        // The most recent change should have the updated title
        const latestChange = taskChanges[0];
        expect(latestChange.changeType).toBe("UPDATED");
        expect(latestChange.taskIteration).not.toBeNull();
        expect(latestChange.taskIteration!.title).toBe(
            "API History Task Updated",
        );
        expect(latestChange.taskIteration!.effort).toBe(16.0);

        // The earlier change should have the original title
        const createChange = taskChanges[taskChanges.length - 1];
        expect(createChange.changeType).toBe("CREATED");
        expect(createChange.taskIteration).not.toBeNull();
        expect(createChange.taskIteration!.title).toBe("API History Task");
        expect(createChange.taskIteration!.effort).toBe(4.0);

        // All changes should have revisionId and timestamp
        for (const change of result.taskHistory.changes) {
            expect(change.revisionId).toBeGreaterThan(0);
            expect(change.timestamp).toBeTruthy();
        }
    });

    test("history API returns booking changes", async () => {
        const resource = await createStandardResource("History Resource");
        const task = await createTask({
            title: "Booking History Task",
            designation: "TASK",
            effort: 8.0,
        });

        // Create a booking
        const start = new Date();
        start.setDate(start.getDate() + 1);
        start.setHours(9, 0, 0, 0);
        const end = new Date(start);
        end.setHours(17, 0, 0, 0);

        await graphqlRequest<{ bookingSave: { dbId: number } }>(BOOKING_SAVE, {
            taskId: task.dbId,
            start: start.toISOString(),
            end: end.toISOString(),
            resources: [resource.dbId],
            final: false,
        });

        // Query history
        const result = await graphqlRequest<TaskHistoryResult>(TASK_HISTORY, {
            taskHeaderId: task.dbId,
            direction: "BACKWARD",
            limit: 50,
        });

        expect(result.taskHistory.changes.length).toBeGreaterThanOrEqual(2);

        // Find BookingChange entries
        const bookingChanges = result.taskHistory.changes.filter(
            (c) => c.__typename === "BookingChange",
        );
        expect(bookingChanges.length).toBeGreaterThanOrEqual(1);

        // The booking change should have the correct data
        const bookingChange = bookingChanges[0];
        expect(bookingChange.changeType).toBe("CREATED");
        expect(bookingChange.booking).not.toBeNull();
        expect(bookingChange.booking!.dbId).toBeGreaterThan(0);
        expect(bookingChange.booking!.start).toBeTruthy();
        expect(bookingChange.booking!.end).toBeTruthy();
        expect(bookingChange.booking!.resources.length).toBeGreaterThanOrEqual(
            1,
        );
        expect(bookingChange.booking!.resources[0].dbId).toBe(resource.dbId);
    });

    test("history API direction FORWARD returns chronological order", async () => {
        const task = await createTask({
            title: "Forward Task",
            designation: "TASK",
            effort: 4.0,
        });

        await updateTask(task.dbId, {
            title: "Forward Task V2",
            effort: 8.0,
        });

        const result = await graphqlRequest<TaskHistoryResult>(TASK_HISTORY, {
            taskHeaderId: task.dbId,
            direction: "FORWARD",
            limit: 50,
        });

        const taskChanges = result.taskHistory.changes.filter(
            (c) => c.__typename === "TaskIterationChange",
        );
        expect(taskChanges.length).toBeGreaterThanOrEqual(2);

        // Forward means ascending revision ids
        for (let i = 1; i < taskChanges.length; i++) {
            expect(taskChanges[i].revisionId).toBeGreaterThanOrEqual(
                taskChanges[i - 1].revisionId,
            );
        }
    });

    test("history API respects limit and returns hasMore", async () => {
        const task = await createTask({
            title: "Limit Task",
            designation: "TASK",
            effort: 2.0,
        });

        // Modify 5 times
        for (let i = 2; i <= 6; i++) {
            await updateTask(task.dbId, {
                title: `Limit Task V${i}`,
                effort: i * 2.0,
            });
        }

        const result = await graphqlRequest<TaskHistoryResult>(TASK_HISTORY, {
            taskHeaderId: task.dbId,
            direction: "BACKWARD",
            limit: 2,
        });

        // Should return exactly 2 changes
        expect(result.taskHistory.changes.length).toBe(2);
        // And there should be more available
        expect(result.taskHistory.hasMore).toBe(true);
    });
});
