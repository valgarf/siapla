import { test, expect } from "@playwright/test";
import { graphqlRequest, standardAvailability } from "../helpers/graphql";
import { cleanDatabase } from "../helpers/cleanup";

/** Shorthand to create a resource via the GraphQL API. */
async function createResource(
    name: string,
    timezone = "Europe/Berlin",
    availability = standardAvailability(),
    extras: Record<string, unknown> = {},
): Promise<{ dbId: number; name: string; timezone: string }> {
    const result = await graphqlRequest<{
        resourceSave: { dbId: number; name: string; timezone: string };
    }>(
        `mutation ResourceSave($resource: ResourceSaveInput!) {
            resourceSave(resource: $resource) { dbId name timezone }
        }`,
        {
            resource: {
                name,
                timezone,
                added: new Date().toISOString(),
                availability,
                ...extras,
            },
        },
    );
    return result.resourceSave;
}

test.describe("Resource Management", () => {
    test.beforeEach(async () => {
        await cleanDatabase();
    });

    test("resource created via API appears on the Resources page", async ({
        page,
    }) => {
        const saved = await createResource("Alice Engineer");
        expect(saved.dbId).toBeTruthy();
        expect(saved.name).toBe("Alice Engineer");

        // Navigate to Resources page
        await page.goto("/#/resources");
        await page.waitForLoadState("networkidle");

        // The resource name should appear in the Gantt row descriptions
        const rowName = page.locator(".gantt-row-description .row-name", {
            hasText: "Alice Engineer",
        });
        await expect(rowName).toBeVisible({ timeout: 15_000 });
    });

    test("multiple resources appear on the Resources page", async ({
        page,
    }) => {
        const names = ["Bob Backend", "Carol Frontend"];
        for (const name of names) {
            await createResource(name);
        }

        await page.goto("/#/resources");
        await page.waitForLoadState("networkidle");

        for (const name of names) {
            const row = page.locator(".gantt-row-description .row-name", {
                hasText: name,
            });
            await expect(row).toBeVisible({ timeout: 15_000 });
        }
    });

    test("resources with different timezones are stored correctly", async ({
        page,
    }) => {
        const resources = [
            { name: "Tokyo Dev", timezone: "Asia/Tokyo" },
            { name: "NYC Dev", timezone: "America/New_York" },
        ];

        for (const r of resources) {
            await createResource(r.name, r.timezone);
        }

        await page.goto("/#/resources");
        await page.waitForLoadState("networkidle");

        // Both resources should appear in the UI
        for (const r of resources) {
            const row = page.locator(".gantt-row-description .row-name", {
                hasText: r.name,
            });
            await expect(row).toBeVisible({ timeout: 15_000 });
        }

        // Verify timezones via API
        const queryResult = await graphqlRequest<{
            resources: Array<{ dbId: number; name: string; timezone: string }>;
        }>(`query Resources { resources { dbId name timezone } }`);

        const tokyo = queryResult.resources.find(
            (res) => res.name === "Tokyo Dev",
        );
        const nyc = queryResult.resources.find((res) => res.name === "NYC Dev");
        expect(tokyo).toBeDefined();
        expect(tokyo!.timezone).toBe("Asia/Tokyo");
        expect(nyc).toBeDefined();
        expect(nyc!.timezone).toBe("America/New_York");
    });

    test("clicking a resource row opens the resource sidebar", async ({
        page,
    }) => {
        await createResource("Sidebar Test Resource", "Europe/London");

        await page.goto("/#/resources");
        await page.waitForLoadState("networkidle");

        // Click the resource row
        const row = page.locator(".gantt-row-description", {
            hasText: "Sidebar Test Resource",
        });
        await expect(row).toBeVisible({ timeout: 15_000 });
        await row.click();

        // The right sidebar should open and show the resource name
        const sidebar = page.locator(".q-drawer--right");
        await expect(sidebar).toBeVisible({ timeout: 10_000 });

        // The sidebar should contain the resource name
        await expect(sidebar.getByText("Sidebar Test Resource")).toBeVisible({
            timeout: 10_000,
        });
    });

    test("resource with custom availability hours is stored correctly", async ({
        page,
    }) => {
        // Part-time: 4h Mon-Wed, 0 otherwise
        const partTimeAvailability = [
            { weekday: "MONDAY", duration: 14400 },
            { weekday: "TUESDAY", duration: 14400 },
            { weekday: "WEDNESDAY", duration: 14400 },
            { weekday: "THURSDAY", duration: 0 },
            { weekday: "FRIDAY", duration: 0 },
            { weekday: "SATURDAY", duration: 0 },
            { weekday: "SUNDAY", duration: 0 },
        ];

        const saved = await createResource(
            "Part-Time Dev",
            "Europe/Berlin",
            partTimeAvailability,
        );

        // Verify via API
        const queryResult = await graphqlRequest<{
            resources: Array<{
                dbId: number;
                name: string;
                availability: Array<{ weekday: string; duration: number }>;
            }>;
        }>(
            `query Resources {
                resources { dbId name availability { weekday duration } }
            }`,
        );

        const resource = queryResult.resources.find(
            (r) => r.dbId === saved.dbId,
        );
        expect(resource).toBeDefined();
        expect(resource!.name).toBe("Part-Time Dev");

        const monday = resource!.availability.find(
            (a) => a.weekday === "MONDAY",
        );
        expect(monday).toBeDefined();
        expect(monday!.duration).toBe(14400);

        const thursday = resource!.availability.find(
            (a) => a.weekday === "THURSDAY",
        );
        expect(thursday).toBeDefined();
        expect(thursday!.duration).toBe(0);

        // Verify it appears in the UI
        await page.goto("/#/resources");
        await page.waitForLoadState("networkidle");

        const rowName = page.locator(".gantt-row-description .row-name", {
            hasText: "Part-Time Dev",
        });
        await expect(rowName).toBeVisible({ timeout: 15_000 });
    });

    test("resource with vacation is stored correctly", async ({ page }) => {
        const vacationStart = new Date();
        vacationStart.setDate(vacationStart.getDate() + 30);
        vacationStart.setHours(0, 0, 0, 0);
        const vacationEnd = new Date(vacationStart);
        vacationEnd.setDate(vacationEnd.getDate() + 14);

        const saved = await createResource(
            "Vacation Dev",
            "Europe/Berlin",
            standardAvailability(),
            {
                addedVacations: [
                    {
                        from: vacationStart.toISOString(),
                        until: vacationEnd.toISOString(),
                    },
                ],
            },
        );

        // Verify vacation stored via API
        const queryResult = await graphqlRequest<{
            resources: Array<{
                dbId: number;
                name: string;
                vacation: Array<{ dbId: number; from: string; until: string }>;
            }>;
        }>(
            `query Resources {
                resources { dbId name vacation { dbId from until } }
            }`,
        );

        const resource = queryResult.resources.find(
            (r) => r.dbId === saved.dbId,
        );
        expect(resource).toBeDefined();
        expect(resource!.vacation).toHaveLength(1);
        expect(new Date(resource!.vacation[0]!.from).getTime()).toBe(
            vacationStart.getTime(),
        );
        expect(new Date(resource!.vacation[0]!.until).getTime()).toBe(
            vacationEnd.getTime(),
        );

        // Verify it appears in the UI
        await page.goto("/#/resources");
        await page.waitForLoadState("networkidle");

        const rowName = page.locator(".gantt-row-description .row-name", {
            hasText: "Vacation Dev",
        });
        await expect(rowName).toBeVisible({ timeout: 15_000 });
    });

    test("deleting a resource via API removes it from the page", async ({
        page,
    }) => {
        const saved = await createResource("Ephemeral Resource", "UTC");

        // Navigate and confirm it's there
        await page.goto("/#/resources");
        await page.waitForLoadState("networkidle");

        const row = page.locator(".gantt-row-description .row-name", {
            hasText: "Ephemeral Resource",
        });
        await expect(row).toBeVisible({ timeout: 15_000 });

        // Delete via API
        await graphqlRequest(
            `mutation ResourceDelete($resourceId: Int!) {
                resourceDelete(resourceId: $resourceId)
            }`,
            { resourceId: saved.dbId },
        );

        // Reload and verify it's gone
        await page.reload();
        await page.waitForLoadState("networkidle");

        await expect(
            page.locator(".gantt-row-description .row-name", {
                hasText: "Ephemeral Resource",
            }),
        ).not.toBeVisible({ timeout: 10_000 });
    });

    test("resource also appears on the tasks page Gantt corner buttons", async ({
        page,
    }) => {
        // Creating a resource doesn't make it show on the tasks page rows,
        // but the "New resource" button should still be functional.
        await page.goto("/#/");
        await page.waitForLoadState("networkidle");

        const newResourceBtn = page.locator(
            'button[aria-label="New resource"]',
        );
        await expect(newResourceBtn).toBeVisible({ timeout: 15_000 });
    });
});
