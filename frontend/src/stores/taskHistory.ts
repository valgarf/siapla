import { acceptHMRUpdate, defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { gql, type ApolloClient, type NormalizedCacheObject } from '@apollo/client/core';
import { useApolloClient } from '@vue/apollo-composable';

// --- Types ---

export type ChangeType = 'CREATED' | 'UPDATED' | 'DELETED';
export type SearchDirection = 'BACKWARD' | 'FORWARD';

export interface BaseChange {
  __typename: string;
  revisionId: number;
  timestamp: string;
  changeType: ChangeType;
}

export interface TaskSnapshotRef {
  dbId: number;
  title: string;
}

export interface TaskSnapshotResourceConstraint {
  optional: boolean;
  speed: number;
  entries: { resource: { dbId: number; name: string } }[];
}

export interface TaskSnapshotData {
  dbId: number;
  iterationId: number;
  title: string;
  description: string;
  designation: string;
  priority: number;
  effort: number | null;
  earliestStart: string | null;
  scheduleTarget: string | null;
  revCreated: number;
  revDeleted: number | null;
  predecessors: TaskSnapshotRef[];
  successors: TaskSnapshotRef[];
  children: TaskSnapshotRef[];
  parent: TaskSnapshotRef | null;
  resourceConstraints: TaskSnapshotResourceConstraint[];
}

export interface BookingSnapshotData {
  dbId: number;
  start: string;
  end: string;
  final: boolean;
  resources: { dbId: number; name: string }[];
}

export interface TaskIterationChangeData extends BaseChange {
  __typename: 'TaskIterationChange';
  taskIteration: TaskSnapshotData | null;
}

export interface BookingChangeData extends BaseChange {
  __typename: 'BookingChange';
  booking: BookingSnapshotData | null;
}

export interface DependencyChangeData extends BaseChange {
  __typename: 'DependencyChange';
  predecessorId: number;
  successorId: number;
  predecessorTitle: string;
  successorTitle: string;
}

export interface ResourceConstraintChangeData extends BaseChange {
  __typename: 'ResourceConstraintChange';
  constraintId: number;
  optional: boolean;
  speed: number;
  resourceIds: number[];
  resourceNames: string[];
}

export type ChangeData =
  | TaskIterationChangeData
  | BookingChangeData
  | DependencyChangeData
  | ResourceConstraintChangeData;

export interface RevisionGroup {
  revisionId: number;
  timestamp: string;
  changes: ChangeData[];
}

interface TaskHistoryQueryResult {
  taskHistory: {
    changes: ChangeData[];
    hasMore: boolean;
  };
}

interface TaskHistoryQueryVariables {
  taskHeaderId: number;
  fromRevision?: number | null;
  fromTimestamp?: string | null;
  direction: SearchDirection;
  limit?: number | null;
}

// --- GraphQL Query ---

const TASK_HISTORY_QUERY = gql`
  query TaskHistory(
    $taskHeaderId: Int!
    $fromRevision: Int64
    $fromTimestamp: DateTime
    $direction: SearchDirection!
    $limit: Int
  ) {
    taskHistory(
      taskHeaderId: $taskHeaderId
      fromRevision: $fromRevision
      fromTimestamp: $fromTimestamp
      direction: $direction
      limit: $limit
    ) {
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
            earliestStart
            scheduleTarget
            revCreated
            revDeleted
            predecessors {
              dbId
              title
            }
            successors {
              dbId
              title
            }
            children {
              dbId
              title
            }
            parent {
              dbId
              title
            }
            resourceConstraints {
              optional
              speed
              entries {
                resource {
                  dbId
                  name
                }
              }
            }
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
            resources {
              dbId
              name
            }
          }
        }
        ... on DependencyChange {
          revisionId
          timestamp
          changeType
          predecessorId
          successorId
          predecessorTitle
          successorTitle
        }
        ... on ResourceConstraintChange {
          revisionId
          timestamp
          changeType
          constraintId
          optional
          speed
          resourceIds
          resourceNames
        }
      }
      hasMore
    }
  }
`;

// --- Helpers ---

function groupByRevision(changes: ChangeData[]): RevisionGroup[] {
  const map = new Map<number, RevisionGroup>();
  for (const change of changes) {
    const existing = map.get(change.revisionId);
    if (existing) {
      existing.changes.push(change);
    } else {
      map.set(change.revisionId, {
        revisionId: change.revisionId,
        timestamp: change.timestamp,
        changes: [change],
      });
    }
  }
  return Array.from(map.values());
}

// --- Store ---

export const useTaskHistoryStore = defineStore('taskHistoryStore', () => {
  const changes = ref<ChangeData[]>([]);
  const hasMore = ref(false);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const currentTaskHeaderId = ref<number | null>(null);
  const currentDirection = ref<SearchDirection>('BACKWARD');

  let apolloClient: ApolloClient<NormalizedCacheObject> | null = null;

  function getClient(): ApolloClient<NormalizedCacheObject> {
    if (apolloClient) return apolloClient;
    const { client } = useApolloClient();
    apolloClient = client as ApolloClient<NormalizedCacheObject>;
    return apolloClient;
  }

  const revisionGroups = computed(() => groupByRevision(changes.value));

  const lastRevisionId = computed<number | null>(() => {
    const last = changes.value[changes.value.length - 1];
    return last !== undefined ? last.revisionId : null;
  });

  const firstRevisionId = computed<number | null>(() => {
    const first = changes.value[0];
    return first !== undefined ? first.revisionId : null;
  });

  async function loadHistory(
    taskHeaderId: number,
    direction: SearchDirection,
    fromRevision?: number | null,
    fromTimestamp?: string | null,
    limit?: number | null,
  ): Promise<void> {
    loading.value = true;
    error.value = null;
    currentTaskHeaderId.value = taskHeaderId;
    currentDirection.value = direction;

    try {
      const client = getClient();
      const result = await client.query<TaskHistoryQueryResult, TaskHistoryQueryVariables>({
        query: TASK_HISTORY_QUERY,
        variables: {
          taskHeaderId,
          fromRevision: fromRevision ?? null,
          fromTimestamp: fromTimestamp ?? null,
          direction,
          limit: limit ?? null,
        },
        fetchPolicy: 'no-cache',
      });

      changes.value = result.data.taskHistory.changes;
      hasMore.value = result.data.taskHistory.hasMore;
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      error.value = message;
      changes.value = [];
      hasMore.value = false;
    } finally {
      loading.value = false;
    }
  }

  async function loadMore(limit?: number | null): Promise<void> {
    if (!hasMore.value || currentTaskHeaderId.value == null || loading.value) return;

    const edgeRevision =
      currentDirection.value === 'BACKWARD' ? lastRevisionId.value : firstRevisionId.value;
    if (edgeRevision == null) return;

    loading.value = true;
    error.value = null;

    try {
      const client = getClient();
      const result = await client.query<TaskHistoryQueryResult, TaskHistoryQueryVariables>({
        query: TASK_HISTORY_QUERY,
        variables: {
          taskHeaderId: currentTaskHeaderId.value,
          fromRevision: edgeRevision,
          fromTimestamp: null,
          direction: currentDirection.value,
          limit: limit ?? null,
        },
        fetchPolicy: 'no-cache',
      });

      const newChanges = result.data.taskHistory.changes;
      // Deduplicate: remove changes whose revisionId already exists at the boundary
      const existingRevisionIds = new Set(changes.value.map((c) => c.revisionId));
      const filtered = newChanges.filter((c) => !existingRevisionIds.has(c.revisionId));

      if (currentDirection.value === 'BACKWARD') {
        changes.value = [...changes.value, ...filtered];
      } else {
        changes.value = [...filtered, ...changes.value];
      }
      hasMore.value = result.data.taskHistory.hasMore;
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      error.value = message;
    } finally {
      loading.value = false;
    }
  }

  async function jumpToTimestamp(taskHeaderId: number, timestamp: string): Promise<void> {
    await loadHistory(taskHeaderId, 'BACKWARD', null, timestamp, 30);
  }

  async function jumpToStart(taskHeaderId: number): Promise<void> {
    await loadHistory(taskHeaderId, 'FORWARD', null, null, 30);
  }

  function reset(): void {
    changes.value = [];
    hasMore.value = false;
    loading.value = false;
    error.value = null;
    currentTaskHeaderId.value = null;
    currentDirection.value = 'BACKWARD';
  }

  return {
    changes,
    hasMore,
    loading,
    error,
    currentTaskHeaderId,
    currentDirection,
    revisionGroups,
    lastRevisionId,
    firstRevisionId,
    loadHistory,
    loadMore,
    jumpToTimestamp,
    jumpToStart,
    reset,
  };
});

if (import.meta.hot) {
  import.meta.hot.accept(acceptHMRUpdate(useTaskHistoryStore, import.meta.hot));
}
