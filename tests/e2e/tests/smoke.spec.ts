import { test, expect } from "@playwright/test";
import { cleanDatabase } from "../helpers/cleanup";

test.describe("Smoke Tests", () => {
    test.beforeEach(async () => {
        await cleanDatabase();
    });

    test("page loads with SIAPLA title in header", async ({ page }) => {
        await page.goto("/");
        await page.waitForLoadState("networkidle");

        // The header toolbar should contain the SIAPLA title
        const title = page.locator(".q-toolbar__title").first();
        await expect(title).toBeVisible({ timeout: 15_000 });
        await expect(title).toHaveText("SIAPLA");
    });

    test("tasks page is the default route", async ({ page }) => {
        await page.goto("/");
        await page.waitForLoadState("networkidle");

        // Hash-based routing: default route is /#/
        await expect(page).toHaveURL(/\/#\//);

        // The Gantt chart container should be present on the tasks page
        const ganttGrid = page.locator(".gantt-grid");
        await expect(ganttGrid).toBeVisible({ timeout: 15_000 });
    });

    test("can navigate to resources page via left drawer", async ({ page }) => {
        await page.goto("/");
        await page.waitForLoadState("networkidle");

        // The left drawer has navigation links rendered as <a> tags via PageLink.
        // In mini mode, only the icon is visible; the tooltip says "Resources".
        // The person icon corresponds to the Resources link.
        const resourceLinkByIcon = page
            .locator(".q-drawer .q-item .q-icon")
            .filter({ hasText: "person" })
            .first();

        // Click the parent q-item (the <a> tag) of the icon
        await resourceLinkByIcon.locator("xpath=ancestor::a[1]").click();

        await page.waitForURL(/\/#\/resources/);
        await expect(page).toHaveURL(/\/#\/resources/);

        // Gantt chart should also be visible on the resources page
        const ganttGrid = page.locator(".gantt-grid");
        await expect(ganttGrid).toBeVisible({ timeout: 15_000 });
    });

    test("can navigate to tasks page from resources page", async ({ page }) => {
        // Start on the resources page
        await page.goto("/#/resources");
        await page.waitForLoadState("networkidle");

        await expect(page).toHaveURL(/\/#\/resources/);

        // Navigate to Tasks using the left drawer link with the task_alt icon
        const taskLinkByIcon = page
            .locator(".q-drawer .q-item .q-icon")
            .filter({ hasText: "task_alt" })
            .first();

        await taskLinkByIcon.locator("xpath=ancestor::a[1]").click();

        // Should be back at the tasks page (root hash route)
        await page.waitForURL((url) => {
            const hash = new URL(url).hash;
            return hash === "#/" || hash === "#";
        });
    });

    test("header has toggle sidebar button", async ({ page }) => {
        await page.goto("/");
        await page.waitForLoadState("networkidle");

        // The header has a toggle sidebar button with aria-label "Toggle sidebar"
        const sidebarToggle = page.locator(
            'button[aria-label="Toggle sidebar"]',
        );
        await expect(sidebarToggle).toBeVisible({ timeout: 10_000 });
    });

    test("header has menu button to toggle left drawer", async ({ page }) => {
        await page.goto("/");
        await page.waitForLoadState("networkidle");

        // The header has a menu button with aria-label "Menu"
        const menuButton = page.locator('button[aria-label="Menu"]');
        await expect(menuButton).toBeVisible({ timeout: 10_000 });
    });

    test("tasks page has new task and new resource buttons", async ({
        page,
    }) => {
        await page.goto("/");
        await page.waitForLoadState("networkidle");

        // The corner area of the Gantt chart has "New task" and "New resource" buttons
        const newTaskBtn = page.locator('button[aria-label="New task"]');
        const newResourceBtn = page.locator(
            'button[aria-label="New resource"]',
        );

        await expect(newTaskBtn).toBeVisible({ timeout: 15_000 });
        await expect(newResourceBtn).toBeVisible({ timeout: 15_000 });
    });

    test("resources page has new task and new resource buttons", async ({
        page,
    }) => {
        await page.goto("/#/resources");
        await page.waitForLoadState("networkidle");

        const newTaskBtn = page.locator('button[aria-label="New task"]');
        const newResourceBtn = page.locator(
            'button[aria-label="New resource"]',
        );

        await expect(newTaskBtn).toBeVisible({ timeout: 15_000 });
        await expect(newResourceBtn).toBeVisible({ timeout: 15_000 });
    });

    test("gantt chart has reset zoom button", async ({ page }) => {
        await page.goto("/");
        await page.waitForLoadState("networkidle");

        const resetZoomBtn = page.locator('button[aria-label="Reset Zoom"]');
        await expect(resetZoomBtn).toBeVisible({ timeout: 15_000 });
    });
});
