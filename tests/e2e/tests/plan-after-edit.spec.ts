import { test, expect } from "@playwright/test";
import {
    graphqlRequest,
    MUTATION_TASK_SAVE,
    MUTATION_RECALCULATE,
    createStandardResource,
} from "../helpers/graphql";
import { cleanDatabase } from "../helpers/cleanup";

test.beforeEach(async () => {
    await cleanDatabase();
});

test.describe("Plan visibility after task edit", () => {
    test("plan allocations persist after editing a planned task", async ({
        page,
    }) => {
        // Step 1: Create a resource with standard availability
        const resource = await createStandardResource("Engineer");
        expect(resource.dbId).toBeTruthy();

        // Step 2: Create a requirement with earliestStart set to tomorrow
        const tomorrow = new Date();
        tomorrow.setDate(tomorrow.getDate() + 1);
        tomorrow.setHours(8, 0, 0, 0);

        const requirement = await graphqlRequest<{
            taskSave: { dbId: number; title: string; designation: string };
        }>(MUTATION_TASK_SAVE, {
            task: {
                title: "Project Start",
                description: "Earliest start marker",
                designation: "REQUIREMENT",
                priority: 100.0,
                earliestStart: tomorrow.toISOString(),
            },
        });
        expect(requirement.taskSave.dbId).toBeTruthy();
        const requirementId = requirement.taskSave.dbId;

        // Step 3: Create a task with effort=16, predecessor on the requirement, and resource constraint
        const taskTitle = "Editable Task";
        const task = await graphqlRequest<{
            taskSave: { dbId: number; title: string; designation: string };
        }>(MUTATION_TASK_SAVE, {
            task: {
                title: taskTitle,
                description: "Task that will be edited after planning",
                designation: "TASK",
                priority: 50.0,
                effort: 16.0,
                predecessors: [requirementId],
                resourceConstraints: [
                    {
                        optional: false,
                        speed: 1.0,
                        entries: [{ resourceId: resource.dbId }],
                    },
                ],
            },
        });
        expect(task.taskSave.dbId).toBeTruthy();
        const originalTaskId = task.taskSave.dbId;

        // Step 4: Create a milestone with scheduleTarget 30 days from now
        const deadline = new Date();
        deadline.setDate(deadline.getDate() + 30);
        deadline.setHours(17, 0, 0, 0);

        await graphqlRequest<{
            taskSave: { dbId: number; title: string; designation: string };
        }>(MUTATION_TASK_SAVE, {
            task: {
                title: "Project Deadline",
                description: "Project end milestone",
                designation: "MILESTONE",
                priority: 50.0,
                scheduleTarget: deadline.toISOString(),
                predecessors: [originalTaskId],
            },
        });

        // Step 5: Trigger recalculation
        await graphqlRequest<{ recalculateNow: boolean }>(
            MUTATION_RECALCULATE,
        );

        // Step 6: Poll until allocations appear for the task (by dbId, since this is the first plan)
        await expect
            .poll(
                async () => {
                    const plan = await graphqlRequest<{
                        currentPlan: {
                            allocations: Array<{
                                task: { dbId: number; title: string };
                            }>;
                        };
                    }>(
                        `query CurrentPlan {
                            currentPlan {
                                allocations { task { dbId title } }
                            }
                        }`,
                    );
                    return plan.currentPlan.allocations.filter(
                        (a) => a.task.dbId === originalTaskId,
                    ).length;
                },
                {
                    message:
                        "Waiting for initial plan allocations for the task",
                    timeout: 30_000,
                    intervals: [500, 1000, 2000],
                },
            )
            .toBeGreaterThanOrEqual(1);

        // Step 7: Modify the task — reduce effort from 16 to 8
        // taskSave with dbId creates a NEW iteration (new dbId) while keeping the same header_id
        const updatedTask = await graphqlRequest<{
            taskSave: { dbId: number; title: string; designation: string };
        }>(MUTATION_TASK_SAVE, {
            task: {
                dbId: originalTaskId,
                title: taskTitle,
                description: "Task with reduced effort",
                designation: "TASK",
                priority: 50.0,
                effort: 8.0,
                predecessors: [requirementId],
                resourceConstraints: [
                    {
                        optional: false,
                        speed: 1.0,
                        entries: [{ resourceId: resource.dbId }],
                    },
                ],
            },
        });
        expect(updatedTask.taskSave.dbId).toBeTruthy();

        // Step 8: Trigger recalculation again
        await graphqlRequest<{ recalculateNow: boolean }>(
            MUTATION_RECALCULATE,
        );

        // Step 9: Poll for allocations after edit — match by title, NOT by dbId,
        // because the task iteration id changes after modification
        await expect
            .poll(
                async () => {
                    const plan = await graphqlRequest<{
                        currentPlan: {
                            allocations: Array<{
                                task: { dbId: number; title: string };
                            }>;
                        };
                    }>(
                        `query CurrentPlan {
                            currentPlan {
                                allocations { task { dbId title } }
                            }
                        }`,
                    );
                    return plan.currentPlan.allocations.filter(
                        (a) => a.task.title === taskTitle,
                    ).length;
                },
                {
                    message:
                        "Waiting for plan allocations after task edit (matched by title)",
                    timeout: 30_000,
                    intervals: [500, 1000, 2000],
                },
            )
            .toBeGreaterThanOrEqual(1);

        // Step 10: Navigate to the Gantt chart and verify the task is visible
        await page.goto("/#/");
        await page.waitForLoadState("networkidle");
        await expect(
            page.locator(".gantt-row-description", { hasText: taskTitle }),
        ).toBeVisible({ timeout: 15_000 });
    });
});
