import { graphqlRequest } from "./graphql";

/**
 * Clean the database by deleting all tasks and resources.
 *
 * Order matters: tasks must be deleted before resources due to FK constraints
 * (resource_constraints reference resources). Within tasks, children must be
 * deleted before parents and leaf tasks before groups, so we do multiple
 * passes until the list is empty.
 */
export async function cleanDatabase(): Promise<void> {
    await deleteAllTasks();
    await deleteAllResources();
}

async function deleteAllTasks(): Promise<void> {
    const MAX_PASSES = 20;

    for (let pass = 0; pass < MAX_PASSES; pass++) {
        const result = await graphqlRequest<{
            tasks: Array<{
                dbId: number;
                children: Array<{ dbId: number }>;
                parent: { dbId: number } | null;
            }>;
        }>(
            `query Tasks {
                tasks {
                    dbId
                    children { dbId }
                    parent { dbId }
                }
            }`,
        );

        const tasks = result.tasks;
        if (tasks.length === 0) break;

        // Delete leaf tasks first (tasks with no children)
        const leaves = tasks.filter((t) => t.children.length === 0);

        // If there are no leaves but there are still tasks, something is cyclic —
        // just try to delete the first one to make progress.
        const toDelete = leaves.length > 0 ? leaves : [tasks[0]!];

        for (const task of toDelete) {
            try {
                await graphqlRequest(
                    `mutation TaskDelete($taskId: Int!) {
                        taskDelete(taskId: $taskId)
                    }`,
                    { taskId: task.dbId },
                );
            } catch (err) {
                // Ignore errors for individual deletes — the task might have been
                // cascade-deleted already. We'll catch stragglers in the next pass.
                console.warn(
                    `[cleanup] Failed to delete task ${task.dbId}:`,
                    err,
                );
            }
        }
    }

    // Final verification
    const verify = await graphqlRequest<{ tasks: Array<{ dbId: number }> }>(
        `query Tasks { tasks { dbId } }`,
    );

    if (verify.tasks.length > 0) {
        console.warn(
            `[cleanup] ${verify.tasks.length} task(s) remain after cleanup: ${verify.tasks.map((t) => t.dbId).join(", ")}`,
        );
    }
}

async function deleteAllResources(): Promise<void> {
    const result = await graphqlRequest<{
        resources: Array<{ dbId: number; name: string }>;
    }>(
        `query Resources {
            resources { dbId name }
        }`,
    );

    for (const resource of result.resources) {
        try {
            await graphqlRequest(
                `mutation ResourceDelete($resourceId: Int!) {
                    resourceDelete(resourceId: $resourceId)
                }`,
                { resourceId: resource.dbId },
            );
        } catch (err) {
            console.warn(
                `[cleanup] Failed to delete resource ${resource.dbId} (${resource.name}):`,
                err,
            );
        }
    }

    // Final verification
    const verify = await graphqlRequest<{
        resources: Array<{ dbId: number }>;
    }>(`query Resources { resources { dbId } }`);

    if (verify.resources.length > 0) {
        console.warn(
            `[cleanup] ${verify.resources.length} resource(s) remain after cleanup: ${verify.resources.map((r) => r.dbId).join(", ")}`,
        );
    }
}
