/**
 * GraphQL helper for direct API calls.
 * Used for test data setup, teardown, and verification — bypassing the UI.
 */

const GRAPHQL_ENDPOINT =
    process.env.GRAPHQL_ENDPOINT ?? "http://localhost:8880/graphql";

// ---------------------------------------------------------------------------
// Core request function
// ---------------------------------------------------------------------------

/**
 * Execute a GraphQL query or mutation against the backend.
 *
 * @param query   - The GraphQL query/mutation string.
 * @param variables - Optional variables object.
 * @returns The `data` portion of the response, typed as `T`.
 * @throws On HTTP errors or GraphQL-level errors.
 *
 * @example
 *   const result = await graphqlRequest<{ tasks: Array<{ dbId: number }> }>(
 *     `query { tasks { dbId } }`
 *   );
 *   console.log(result.tasks);
 *
 * @example
 *   const result = await graphqlRequest<{ taskSave: { dbId: number } }>(
 *     `mutation ($task: TaskSaveInput!) { taskSave(task: $task) { dbId } }`,
 *     { task: { title: 'Hello', description: '', designation: 'TASK', priority: 1 } }
 *   );
 *   console.log(result.taskSave.dbId);
 */
export async function graphqlRequest<T = Record<string, unknown>>(
    query: string,
    variables?: Record<string, unknown>,
): Promise<T> {
    const res = await fetch(GRAPHQL_ENDPOINT, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ query, variables }),
    });

    if (!res.ok) {
        const body = await res.text().catch(() => "<unreadable>");
        throw new Error(
            `GraphQL HTTP error: ${res.status} ${res.statusText}\n${body}`,
        );
    }

    const json = (await res.json()) as {
        data?: T;
        errors?: Array<{ message: string; path?: string[] }>;
    };

    if (json.errors?.length) {
        const messages = json.errors.map((e) => e.message).join("; ");
        throw new Error(`GraphQL errors: ${messages}`);
    }

    if (json.data === undefined || json.data === null) {
        throw new Error("GraphQL response contained no data");
    }

    return json.data;
}

// ---------------------------------------------------------------------------
// Revision helpers
// ---------------------------------------------------------------------------

export const QUERY_LATEST_REVISION = `
  query LatestRevision {
    latestRevision
  }
`;

/**
 * Get the current latest revision number.
 */
export async function getLatestRevision(): Promise<number> {
    const res = await graphqlRequest<{ latestRevision: number }>(
        QUERY_LATEST_REVISION,
    );
    return res.latestRevision;
}

/**
 * Query tasks at a specific revision.
 */
export async function queryTasksAtRevision(revision: number) {
    return graphqlRequest<{ tasks: Array<TaskFields> }>(
        `query Tasks($revision: Int64) {
            tasks(revision: $revision) {
                dbId title description designation priority effort
                earliestStart scheduleTarget
                predecessors { dbId }
                successors { dbId }
                parent { dbId }
                children { dbId }
                resourceConstraints {
                    optional speed
                    entries { resource { dbId } }
                }
            }
        }`,
        { revision },
    );
}

/**
 * Query resources at a specific revision.
 */
export async function queryResourcesAtRevision(revision: number) {
    return graphqlRequest<{ resources: Array<ResourceFields> }>(
        `query Resources($revision: Int64) {
            resources(revision: $revision) {
                dbId name timezone added removed
                availability { weekday duration }
                vacation { dbId from until }
            }
        }`,
        { revision },
    );
}

/**
 * Query bookings at a specific revision.
 */
export async function queryBookingsAtRevision(revision: number) {
    return graphqlRequest<{
        bookings: Array<{
            dbId: number;
            start: string;
            end: string;
            final: boolean;
            task: { dbId: number };
            resources: Array<{ dbId: number }>;
        }>;
    }>(
        `query Bookings($revision: Int64) {
            bookings(revision: $revision) {
                dbId start end final
                task { dbId }
                resources { dbId }
            }
        }`,
        { revision },
    );
}

// ---------------------------------------------------------------------------
// Commonly-used query / mutation strings
// ---------------------------------------------------------------------------

export const QUERY_TASKS = `
  query Tasks {
    tasks {
      dbId
      title
      description
      designation
      priority
      effort
      earliestStart
      scheduleTarget
      predecessors { dbId }
      successors { dbId }
      parent { dbId }
      children { dbId }
      resourceConstraints {
        optional
        speed
        entries { resource { dbId } }
      }
    }
  }
`;

export const QUERY_RESOURCES = `
  query Resources {
    resources {
      dbId
      name
      timezone
      added
      removed
      availability { weekday duration }
      vacation { dbId from until }
    }
  }
`;

export const QUERY_CURRENT_PLAN = `
  query CurrentPlan {
    currentPlan {
      allocations {
        dbId
        start
        end
        allocationType
        resources { dbId name }
        task { dbId title }
      }
    }
  }
`;

export const QUERY_ISSUES = `
  query Issues {
    issues {
      dbId
      code
      description
      type
      task { dbId title }
    }
  }
`;

export const MUTATION_TASK_SAVE = `
  mutation TaskSave($task: TaskSaveInput!) {
    taskSave(task: $task) {
      dbId
      title
      designation
    }
  }
`;

export const MUTATION_TASK_DELETE = `
  mutation TaskDelete($taskId: Int!) {
    taskDelete(taskId: $taskId)
  }
`;

export const MUTATION_RESOURCE_SAVE = `
  mutation ResourceSave($resource: ResourceSaveInput!) {
    resourceSave(resource: $resource) {
      dbId
      name
      timezone
    }
  }
`;

export const MUTATION_RESOURCE_DELETE = `
  mutation ResourceDelete($resourceId: Int!) {
    resourceDelete(resourceId: $resourceId)
  }
`;

export const MUTATION_RECALCULATE = `
  mutation RecalculateNow {
    recalculateNow
  }
`;

export const MUTATION_RESET_DATABASE = `
  mutation ResetDatabase {
    resetDatabase
  }
`;

// ---------------------------------------------------------------------------
// Typed result interfaces (for use with graphqlRequest<T>)
// ---------------------------------------------------------------------------

export interface TaskFields {
    dbId: number;
    title: string;
    description?: string;
    designation: string;
    priority?: number;
    effort?: number | null;
    earliestStart?: string | null;
    scheduleTarget?: string | null;
    predecessors?: Array<{ dbId: number }>;
    successors?: Array<{ dbId: number }>;
    parent?: { dbId: number } | null;
    children?: Array<{ dbId: number }>;
    resourceConstraints?: Array<{
        optional: boolean;
        speed: number;
        entries: Array<{ resource: { dbId: number } }>;
    }>;
}

export interface ResourceFields {
    dbId: number;
    name: string;
    timezone: string;
    added?: string;
    removed?: string | null;
    availability?: Array<{ weekday: string; duration: number }>;
    vacation?: Array<{ dbId: number; from: string; until: string }>;
}

export interface AllocationFields {
    dbId: number;
    start: string;
    end: string;
    allocationType: string | null;
    resources: Array<{ dbId: number; name: string }>;
    task: { dbId: number; title: string };
}

export interface IssueFields {
    dbId: number;
    code: string;
    description: string;
    type: string;
    task: { dbId: number; title: string } | null;
}

// ---------------------------------------------------------------------------
// Convenience helpers — thin wrappers around graphqlRequest
// ---------------------------------------------------------------------------

/**
 * Create a standard 8 h/weekday availability array (Mon-Fri, 0 on weekends).
 */
export function standardAvailability(
    hoursPerDay = 8,
): Array<{ weekday: string; duration: number }> {
    const weekdays = ["MONDAY", "TUESDAY", "WEDNESDAY", "THURSDAY", "FRIDAY"];
    const weekend = ["SATURDAY", "SUNDAY"];
    return [
        ...weekdays.map((weekday) => ({
            weekday,
            duration: hoursPerDay * 3600,
        })),
        ...weekend.map((weekday) => ({ weekday, duration: 0 })),
    ];
}

/**
 * Create a resource with standard 8 h/day Mon-Fri availability.
 */
export async function createStandardResource(
    name: string,
    timezone = "UTC",
): Promise<{ dbId: number; name: string; timezone: string }> {
    const res = await graphqlRequest<{
        resourceSave: { dbId: number; name: string; timezone: string };
    }>(MUTATION_RESOURCE_SAVE, {
        resource: {
            name,
            timezone,
            added: new Date().toISOString(),
            availability: standardAvailability(),
        },
    });
    return res.resourceSave;
}

/**
 * Trigger plan recalculation and return immediately (the scheduler runs async).
 */
export async function triggerRecalculate(): Promise<boolean> {
    const res = await graphqlRequest<{ recalculateNow: boolean }>(
        MUTATION_RECALCULATE,
    );
    return res.recalculateNow;
}
