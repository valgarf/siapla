<template>
  <div class="task-history-compare q-pa-md">
    <div v-if="loading" class="column items-center q-pa-xl">
      <q-spinner color="primary" size="40px" />
      <div class="q-mt-sm text-grey-6">Loading revision data…</div>
    </div>

    <div v-else-if="errorMsg" class="q-pa-md text-negative">
      <q-icon name="error" class="q-mr-xs" />
      {{ errorMsg }}
    </div>

    <div
      v-else-if="!changesA.length && !changesB.length"
      class="q-pa-lg text-grey-6 text-center"
    >
      No data available for these revisions.
    </div>

    <template v-else>
      <div class="row items-center q-mb-sm q-gutter-sm">
        <q-chip dense color="red-2" text-color="red-10" icon="arrow_back">
          Rev {{ revisionA }} — {{ formatDate(timestampA) }}
        </q-chip>
        <q-icon name="compare_arrows" size="sm" color="grey-6" />
        <q-chip dense color="green-2" text-color="green-10" icon="arrow_forward">
          Rev {{ revisionB }} — {{ formatDate(timestampB) }}
        </q-chip>
      </div>

      <div class="text-caption text-grey-6 q-mb-md">
        Comparing revision {{ revisionA }} with revision {{ revisionB }}
      </div>

      <div class="comparison-columns row q-col-gutter-md">
        <div class="col-12 col-md-6">
          <div class="text-subtitle2 text-grey-7 q-mb-sm">
            Revision {{ revisionA }}
          </div>
          <TaskSnapshotReadonly
            v-if="snapshotForViewA"
            :snapshot="snapshotForViewA"
            :show-requirements="true"
            :show-milestones="true"
          />
          <div v-else class="text-grey-5">No task snapshot available.</div>
        </div>

        <div class="col-12 col-md-6">
          <div class="text-subtitle2 text-grey-7 q-mb-sm">
            Revision {{ revisionB }}
          </div>
          <TaskSnapshotReadonly
            v-if="snapshotForViewB"
            :snapshot="snapshotForViewB"
            :show-requirements="true"
            :show-milestones="true"
          />
          <div v-else class="text-grey-5">No task snapshot available.</div>
        </div>
      </div>

      <q-separator class="q-my-md" />

      <section class="compare-section">
        <div class="text-subtitle1 text-weight-bold q-mb-sm">Changed Fields</div>

        <div v-if="taskFields.length > 0" class="column q-gutter-sm">
          <div
            v-for="field in taskFields"
            :key="field.key"
            class="compare-field-entry"
          >
            <div class="text-subtitle2 text-grey-8">{{ field.label }}</div>
            <div class="row q-gutter-sm items-start">
              <div class="diff-removed q-pa-xs rounded-borders compare-value-box">
                {{ field.oldVal ?? '—' }}
              </div>
              <q-icon
                name="arrow_forward"
                size="xs"
                class="self-center q-mt-xs"
              />
              <div class="diff-added q-pa-xs rounded-borders compare-value-box">
                {{ field.newVal ?? '—' }}
              </div>
            </div>
          </div>
        </div>

        <div v-else class="text-grey-5">No scalar field changes.</div>
      </section>

      <section v-if="descriptionChanged" class="compare-section q-mt-md">
        <div class="text-subtitle2 text-grey-8 q-mb-xs">Description</div>
        <div class="row q-gutter-sm">
          <div class="col diff-removed detail-description">
            {{ taskSnapshotA?.description ?? '' }}
          </div>
          <div class="col diff-added detail-description">
            {{ taskSnapshotB?.description ?? '' }}
          </div>
        </div>
      </section>

      <q-separator class="q-my-md" />

      <section class="compare-section">
        <div class="text-subtitle1 text-weight-bold q-mb-sm">Dependency Changes</div>
        <div class="text-subtitle2 text-grey-8 q-mb-xs">Dependencies</div>

        <div
          v-if="
            dependencyDiff.removed.length > 0 ||
            dependencyDiff.unchanged.length > 0 ||
            dependencyDiff.added.length > 0
          "
          class="q-gutter-xs"
        >
          <q-chip
            v-for="item in dependencyDiff.removed"
            :key="'dep-removed-' + item.key"
            class="diff-removed"
            text-color="white"
          >
            {{ item.label }}
          </q-chip>
          <q-chip
            v-for="item in dependencyDiff.unchanged"
            :key="'dep-unchanged-' + item.key"
            color="primary"
            text-color="white"
          >
            {{ item.label }}
          </q-chip>
          <q-chip
            v-for="item in dependencyDiff.added"
            :key="'dep-added-' + item.key"
            class="diff-added"
            text-color="white"
          >
            {{ item.label }}
          </q-chip>
        </div>

        <div v-else class="text-grey-5">—</div>
      </section>

      <q-separator class="q-my-md" />

      <section class="compare-section">
        <div class="text-subtitle1 text-weight-bold q-mb-sm">
          Resource Constraint Changes
        </div>
        <div class="text-subtitle2 text-grey-8 q-mb-xs">Constraints</div>

        <div
          v-if="
            constraintDiff.removed.length > 0 ||
            constraintDiff.unchanged.length > 0 ||
            constraintDiff.added.length > 0
          "
          class="column q-gutter-md"
        >
          <div
            v-for="item in constraintDiff.removed"
            :key="'constraint-removed-' + item.key"
            class="row items-start q-gutter-sm"
          >
            <div class="text-subtitle2 text-grey-8">Removed</div>
            <div class="constraint-pill-group diff-removed rounded-borders q-pa-sm">
              <q-chip color="warning" text-color="white" dense>
                {{ item.optional ? 'Optional' : 'Required' }}
              </q-chip>
              <span class="text-body2">Speed: {{ item.speed }}</span>
              <div class="q-gutter-xs q-mt-xs">
                <q-chip
                  v-for="resourceName in item.resourceNames"
                  :key="resourceName"
                  color="secondary"
                  text-color="white"
                >
                  {{ resourceName }}
                </q-chip>
                <span v-if="item.resourceNames.length === 0" class="text-grey-5">—</span>
              </div>
            </div>
          </div>

          <div
            v-for="item in constraintDiff.unchanged"
            :key="'constraint-unchanged-' + item.key"
            class="row items-start q-gutter-sm"
          >
            <div class="text-subtitle2 text-grey-8">Unchanged</div>
            <div class="constraint-pill-group">
              <q-chip color="warning" text-color="white" dense>
                {{ item.optional ? 'Optional' : 'Required' }}
              </q-chip>
              <span class="text-body2">Speed: {{ item.speed }}</span>
              <div class="q-gutter-xs q-mt-xs">
                <q-chip
                  v-for="resourceName in item.resourceNames"
                  :key="resourceName"
                  color="secondary"
                  text-color="white"
                >
                  {{ resourceName }}
                </q-chip>
                <span v-if="item.resourceNames.length === 0" class="text-grey-5">—</span>
              </div>
            </div>
          </div>

          <div
            v-for="item in constraintDiff.added"
            :key="'constraint-added-' + item.key"
            class="row items-start q-gutter-sm"
          >
            <div class="text-subtitle2 text-grey-8">Added</div>
            <div class="constraint-pill-group diff-added rounded-borders q-pa-sm">
              <q-chip color="warning" text-color="white" dense>
                {{ item.optional ? 'Optional' : 'Required' }}
              </q-chip>
              <span class="text-body2">Speed: {{ item.speed }}</span>
              <div class="q-gutter-xs q-mt-xs">
                <q-chip
                  v-for="resourceName in item.resourceNames"
                  :key="resourceName"
                  color="secondary"
                  text-color="white"
                >
                  {{ resourceName }}
                </q-chip>
                <span v-if="item.resourceNames.length === 0" class="text-grey-5">—</span>
              </div>
            </div>
          </div>
        </div>

        <div v-else class="text-grey-5">—</div>
      </section>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { TaskDesignation } from 'src/gql/graphql';
import { useResourceStore, type Resource } from 'src/stores/resource';
import { useTaskStore, type Task } from 'src/stores/task';
import {
  useTaskHistoryStore,
  type ChangeData,
  type DependencyChangeData,
  type ResourceConstraintChangeData,
  type TaskIterationChangeData,
  type TaskSnapshotData,
} from 'src/stores/taskHistory';
import TaskSnapshotReadonly from 'src/components/task/TaskSnapshotReadonly.vue';
import type {
  TaskSnapshotReadonlyBooking,
  TaskSnapshotReadonlyData,
  TaskSnapshotReadonlyResourceConstraint,
} from 'src/components/task/TaskSnapshotReadonlyTypes';

const props = defineProps<{
  taskHeaderId: number;
  revisionA: number;
  revisionB: number;
}>();

const historyStore = useTaskHistoryStore();
const taskStore = useTaskStore();
const resourceStore = useResourceStore();

const loading = computed(() => historyStore.loading);
const errorMsg = computed(() => historyStore.error);

const changesA = computed<ChangeData[]>(() =>
  historyStore.changes.filter((c) => c.revisionId === props.revisionA),
);
const changesB = computed<ChangeData[]>(() =>
  historyStore.changes.filter((c) => c.revisionId === props.revisionB),
);

const timestampA = computed(() => changesA.value[0]?.timestamp ?? '');
const timestampB = computed(() => changesB.value[0]?.timestamp ?? '');

const taskSnapshotA = computed<TaskSnapshotData | null>(() => {
  const found = changesA.value.find(
    (c): c is TaskIterationChangeData => c.__typename === 'TaskIterationChange',
  );
  return found?.taskIteration ?? null;
});

const taskSnapshotB = computed<TaskSnapshotData | null>(() => {
  const found = changesB.value.find(
    (c): c is TaskIterationChangeData => c.__typename === 'TaskIterationChange',
  );
  return found?.taskIteration ?? null;
});

interface TaskField {
  key: string;
  label: string;
  oldVal: string | null;
  newVal: string | null;
}

interface ConstraintDiffItem {
  key: string;
  optional: boolean;
  speed: string;
  resourceNames: string[];
}

interface TextDiffItem {
  key: string;
  label: string;
}

function formatDate(value: string | null | undefined): string {
  if (!value) return '—';
  try {
    return new Date(value).toLocaleString();
  } catch {
    return value;
  }
}

function fieldStr(val: string | number | null | undefined): string | null {
  if (val == null) return null;
  return String(val);
}

function taskRefToTask(refTask: { dbId: number; title: string }): Task {
  return {
    dbId: refTask.dbId,
    title: refTask.title,
    description: '',
    parent: null,
    children: [],
    predecessors: [],
    successors: [],
    earliestStart: null,
    scheduleTarget: null,
    priority: 0,
    effort: null,
    designation: taskStore.task(refTask.dbId)?.designation ?? TaskDesignation.Task,
    resourceConstraints: [],
  };
}

function resourceRefToResource(refResource: {
  dbId: number;
  name: string;
}): Resource {
  return {
    dbId: refResource.dbId,
    name: refResource.name,
    timezone: resourceStore.resource(refResource.dbId)?.timezone ?? 'UTC',
    added: resourceStore.resource(refResource.dbId)?.added ?? new Date(0),
    removed: resourceStore.resource(refResource.dbId)?.removed ?? null,
    holiday: resourceStore.resource(refResource.dbId)?.holiday ?? null,
    availability: resourceStore.resource(refResource.dbId)?.availability ?? null,
    vacations: resourceStore.resource(refResource.dbId)?.vacations ?? [],
  };
}

function deriveRequirements(snapshot: TaskSnapshotData): Task[] {
  return snapshot.predecessors
    .map(taskRefToTask)
    .filter((task) => task.designation === TaskDesignation.Requirement);
}

function deriveMilestones(snapshot: TaskSnapshotData): Task[] {
  return snapshot.successors
    .map(taskRefToTask)
    .filter((task) => task.designation === TaskDesignation.Milestone);
}

function buildReadonlySnapshot(
  snapshot: TaskSnapshotData | null,
  changes: ChangeData[],
): TaskSnapshotReadonlyData | null {
  if (!snapshot) {
    return null;
  }

  const resourceConstraints: TaskSnapshotReadonlyResourceConstraint[] =
    snapshot.resourceConstraints.map((constraint) => ({
      resources: constraint.entries.map((entry) =>
        resourceRefToResource(entry.resource),
      ),
      optional: constraint.optional,
      speed: constraint.speed,
    }));

  const bookings: TaskSnapshotReadonlyBooking[] = changes
    .filter(
      (
        change,
      ): change is Extract<ChangeData, { __typename: 'BookingChange' }> =>
        change.__typename === 'BookingChange' && change.booking != null,
    )
    .map((change) => ({
      dbId: change.booking!.dbId,
      start: change.booking?.start ? new Date(change.booking.start) : null,
      end: change.booking?.end ? new Date(change.booking.end) : null,
      final: change.booking?.final ?? false,
      resources:
        change.booking?.resources.map((resource) =>
          resourceRefToResource(resource),
        ) ?? [],
    }));

  return {
    dbId: snapshot.dbId,
    title: snapshot.title,
    description: snapshot.description,
    designation: snapshot.designation,
    earliestStart: snapshot.earliestStart
      ? new Date(snapshot.earliestStart)
      : null,
    scheduleTarget: snapshot.scheduleTarget
      ? new Date(snapshot.scheduleTarget)
      : null,
    effort: snapshot.effort,
    priority: snapshot.priority,
    predecessors: snapshot.predecessors.map(taskRefToTask),
    successors: snapshot.successors.map(taskRefToTask),
    parent: snapshot.parent ? taskRefToTask(snapshot.parent) : null,
    children: snapshot.children.map(taskRefToTask),
    resourceConstraints,
    requirements: deriveRequirements(snapshot),
    milestones: deriveMilestones(snapshot),
    bookings,
  };
}

const snapshotForViewA = computed(() =>
  buildReadonlySnapshot(taskSnapshotA.value, changesA.value),
);
const snapshotForViewB = computed(() =>
  buildReadonlySnapshot(taskSnapshotB.value, changesB.value),
);

const taskFields = computed<TaskField[]>(() => {
  const a = taskSnapshotA.value;
  const b = taskSnapshotB.value;

  return [
    { key: 'title', label: 'Title', oldVal: fieldStr(a?.title), newVal: fieldStr(b?.title) },
    {
      key: 'designation',
      label: 'Designation',
      oldVal: fieldStr(a?.designation),
      newVal: fieldStr(b?.designation),
    },
    {
      key: 'priority',
      label: 'Priority',
      oldVal: fieldStr(a?.priority),
      newVal: fieldStr(b?.priority),
    },
    {
      key: 'effort',
      label: 'Effort',
      oldVal: fieldStr(a?.effort),
      newVal: fieldStr(b?.effort),
    },
    {
      key: 'earliestStart',
      label: 'Earliest Start',
      oldVal: formatDate(a?.earliestStart),
      newVal: formatDate(b?.earliestStart),
    },
    {
      key: 'scheduleTarget',
      label: 'Schedule Target',
      oldVal: formatDate(a?.scheduleTarget),
      newVal: formatDate(b?.scheduleTarget),
    },
  ].filter((field) => field.oldVal !== field.newVal);
});

const descriptionChanged = computed(() => {
  const a = taskSnapshotA.value?.description ?? '';
  const b = taskSnapshotB.value?.description ?? '';
  return a !== b;
});

function dependencyKey(change: DependencyChangeData): string {
  return `${change.predecessorId}->${change.successorId}`;
}

function dependencyLabel(change: DependencyChangeData): string {
  const predecessor =
    change.predecessorTitle && change.predecessorTitle.length > 0
      ? change.predecessorTitle
      : `Task #${change.predecessorId}`;
  const successor =
    change.successorTitle && change.successorTitle.length > 0
      ? change.successorTitle
      : `Task #${change.successorId}`;
  return `${predecessor} → ${successor}`;
}

function constraintChangeKey(change: ResourceConstraintChangeData): string {
  const resources = [...change.resourceIds].sort((a, b) => a - b).join(',');
  return `${change.optional}:${change.speed}:${resources}`;
}

function constraintDiffItem(change: ResourceConstraintChangeData): ConstraintDiffItem {
  return {
    key: constraintChangeKey(change),
    optional: change.optional,
    speed: Number(change.speed).toFixed(2),
    resourceNames: change.resourceNames,
  };
}

function latestDependencyState(changes: ChangeData[]): DependencyChangeData[] {
  const latestByKey = new Map<string, DependencyChangeData>();
  const sorted = [...changes]
    .filter((c): c is DependencyChangeData => c.__typename === 'DependencyChange')
    .sort((a, b) => a.revisionId - b.revisionId);

  for (const change of sorted) {
    const key = dependencyKey(change);
    if (change.changeType === 'DELETED') {
      latestByKey.delete(key);
    } else {
      latestByKey.set(key, change);
    }
  }

  return Array.from(latestByKey.values());
}

function latestConstraintState(changes: ChangeData[]): ResourceConstraintChangeData[] {
  const latestByKey = new Map<string, ResourceConstraintChangeData>();
  const sorted = [...changes]
    .filter(
      (c): c is ResourceConstraintChangeData =>
        c.__typename === 'ResourceConstraintChange',
    )
    .sort((a, b) => a.revisionId - b.revisionId);

  for (const change of sorted) {
    const key = constraintChangeKey(change);
    if (change.changeType === 'DELETED') {
      latestByKey.delete(key);
    } else {
      latestByKey.set(key, change);
    }
  }

  return Array.from(latestByKey.values());
}

function diffTextItems(
  oldItems: TextDiffItem[],
  newItems: TextDiffItem[],
): { added: TextDiffItem[]; removed: TextDiffItem[]; unchanged: TextDiffItem[] } {
  const oldKeys = new Set(oldItems.map((item) => item.key));
  const newKeys = new Set(newItems.map((item) => item.key));

  return {
    removed: oldItems.filter((item) => !newKeys.has(item.key)),
    unchanged: oldItems.filter((item) => newKeys.has(item.key)),
    added: newItems.filter((item) => !oldKeys.has(item.key)),
  };
}

function diffConstraintItems(
  oldItems: ConstraintDiffItem[],
  newItems: ConstraintDiffItem[],
): {
  added: ConstraintDiffItem[];
  removed: ConstraintDiffItem[];
  unchanged: ConstraintDiffItem[];
} {
  const oldKeys = new Set(oldItems.map((item) => item.key));
  const newKeys = new Set(newItems.map((item) => item.key));

  return {
    removed: oldItems.filter((item) => !newKeys.has(item.key)),
    unchanged: oldItems.filter((item) => newKeys.has(item.key)),
    added: newItems.filter((item) => !oldKeys.has(item.key)),
  };
}

const dependencyDiff = computed(() => {
  const oldItems = latestDependencyState(changesA.value).map((change) => ({
    key: dependencyKey(change),
    label: dependencyLabel(change),
  }));
  const newItems = latestDependencyState(changesB.value).map((change) => ({
    key: dependencyKey(change),
    label: dependencyLabel(change),
  }));
  return diffTextItems(oldItems, newItems);
});

const constraintDiff = computed(() => {
  const oldItems = latestConstraintState(changesA.value).map(constraintDiffItem);
  const newItems = latestConstraintState(changesB.value).map(constraintDiffItem);
  return diffConstraintItems(oldItems, newItems);
});
</script>

<style scoped>
.diff-removed {
  background-color: #ffcdd2;
}

.diff-added {
  background-color: #c8e6c9;
}

.compare-field-entry {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.compare-value-box {
  min-width: 0;
  flex: 1 1 0;
}

.detail-description {
  white-space: pre-wrap;
  border-radius: 4px;
  padding: 8px 12px;
  font-size: 0.85rem;
  min-height: 40px;
}

.compare-section {
  display: flex;
  flex-direction: column;
}

.comparison-columns {
  align-items: start;
}

.constraint-pill-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
  flex: 1 1 auto;
}
</style>
