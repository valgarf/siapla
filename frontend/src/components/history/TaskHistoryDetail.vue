<template>
  <div class="task-history-detail q-pa-md">
    <div v-if="loading" class="column items-center q-pa-xl">
      <q-spinner color="primary" size="40px" />
      <div class="q-mt-sm text-grey-6">Loading revision data…</div>
    </div>

    <div v-else-if="errorMsg" class="q-pa-md text-negative">
      <q-icon name="error" class="q-mr-xs" />
      {{ errorMsg }}
    </div>

    <div
      v-else-if="!taskSnapshot && changes.length === 0"
      class="q-pa-lg text-grey-6 text-center"
    >
      No data available for this revision.
    </div>

    <template v-else>
      <div class="text-caption text-grey-6 q-mb-md">
        Revision {{ revisionId }} · {{ formattedTimestamp }}
      </div>

      <TaskSnapshotReadonly
        v-if="taskSnapshot"
        :snapshot="readonlySnapshot"
        :show-requirements="true"
        :show-milestones="true"
      />

      <template v-if="bookingChanges.length > 0">
        <section class="sidebar-section">
          <div class="text-subtitle2 q-mb-xs">Booking Changes</div>
          <q-list dense bordered separator class="rounded-borders q-mb-sm">
            <q-item v-for="(bc, idx) in bookingChanges" :key="idx">
              <q-item-section side>
                <q-badge
                  :color="changeTypeColor(bc.changeType)"
                  text-color="white"
                >
                  {{ bc.changeType }}
                </q-badge>
              </q-item-section>
              <q-item-section v-if="bc.booking">
                <q-item-label>
                  {{ formatDate(bc.booking.start) }} —
                  {{ formatDate(bc.booking.end) }}
                </q-item-label>
                <q-item-label caption>
                  <q-badge
                    v-if="bc.booking.final"
                    color="positive"
                    text-color="white"
                    class="q-mr-xs"
                  >
                    Final
                  </q-badge>
                  Resources:
                  {{
                    bc.booking.resources.map((r) => r.name).join(', ') || '—'
                  }}
                </q-item-label>
              </q-item-section>
              <q-item-section v-else>
                <q-item-label class="text-grey-5">
                  Booking data not available (deleted)
                </q-item-label>
              </q-item-section>
            </q-item>
          </q-list>
        </section>
      </template>

      <template v-if="dependencyChanges.length > 0">
        <section class="sidebar-section">
          <div class="text-subtitle2 q-mb-xs">Dependency Changes</div>
          <q-list dense bordered separator class="rounded-borders q-mb-sm">
            <q-item v-for="(dc, idx) in dependencyChanges" :key="idx">
              <q-item-section side>
                <q-badge
                  :color="changeTypeColor(dc.changeType)"
                  text-color="white"
                >
                  {{ dc.changeType }}
                </q-badge>
              </q-item-section>
              <q-item-section>
                <q-item-label>
                  {{ dependencyLabel(dc) }}
                </q-item-label>
              </q-item-section>
            </q-item>
          </q-list>
        </section>
      </template>

      <template v-if="constraintChanges.length > 0">
        <section class="sidebar-section">
          <div class="text-subtitle2 q-mb-xs">Resource Constraint Changes</div>
          <q-list dense bordered separator class="rounded-borders q-mb-sm">
            <q-item v-for="(rc, idx) in constraintChanges" :key="idx">
              <q-item-section side>
                <q-badge
                  :color="changeTypeColor(rc.changeType)"
                  text-color="white"
                >
                  {{ rc.changeType }}
                </q-badge>
              </q-item-section>
              <q-item-section>
                <q-item-label>
                  <q-badge
                    :color="rc.optional ? 'warning' : 'primary'"
                    class="q-mr-xs"
                  >
                    {{ rc.optional ? 'Optional' : 'Required' }}
                  </q-badge>
                  Speed: {{ Number(rc.speed).toFixed(2) }}
                </q-item-label>
                <q-item-label caption>
                  {{ constraintLabel(rc) }}
                </q-item-label>
              </q-item-section>
            </q-item>
          </q-list>
        </section>
      </template>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useTaskStore, type Task } from 'src/stores/task';
import { TaskDesignation } from 'src/gql/graphql';
import { useResourceStore, type Resource } from 'src/stores/resource';
import {
  useTaskHistoryStore,
  type ChangeType,
  type ChangeData,
  type TaskSnapshotData,
  type TaskIterationChangeData,
  type BookingChangeData,
  type DependencyChangeData,
  type ResourceConstraintChangeData,
} from 'src/stores/taskHistory';
import TaskSnapshotReadonly, {
  type TaskSnapshotReadonlyData,
  type TaskSnapshotReadonlyResourceConstraint,
  type TaskSnapshotReadonlyBooking,
} from 'src/components/task/TaskSnapshotReadonly.vue';

const props = defineProps<{
  taskHeaderId: number;
  revisionId: number;
}>();

const historyStore = useTaskHistoryStore();
const taskStore = useTaskStore();
const resourceStore = useResourceStore();

const loading = ref(false);
const errorMsg = ref<string | null>(null);

const changes = computed<ChangeData[]>(() => {
  return historyStore.changes.filter((c) => c.revisionId === props.revisionId);
});

const taskIterationChange = computed<TaskIterationChangeData | null>(() => {
  const found = changes.value.find(
    (c): c is TaskIterationChangeData => c.__typename === 'TaskIterationChange',
  );
  return found ?? null;
});

const taskSnapshot = computed<TaskSnapshotData | null>(() => {
  return taskIterationChange.value?.taskIteration ?? null;
});

const bookingChanges = computed<BookingChangeData[]>(() => {
  return changes.value.filter(
    (c): c is BookingChangeData => c.__typename === 'BookingChange',
  );
});

const dependencyChanges = computed<DependencyChangeData[]>(() => {
  return changes.value.filter(
    (c): c is DependencyChangeData => c.__typename === 'DependencyChange',
  );
});

const constraintChanges = computed<ResourceConstraintChangeData[]>(() => {
  return changes.value.filter(
    (c): c is ResourceConstraintChangeData =>
      c.__typename === 'ResourceConstraintChange',
  );
});

const formattedTimestamp = computed(() => {
  const first = changes.value[0];
  if (!first) return '';
  return formatDate(first.timestamp);
});

function formatDate(value: string | null | undefined): string {
  if (!value) return '—';
  try {
    return new Date(value).toLocaleString();
  } catch {
    return value;
  }
}

function changeTypeColor(changeType: ChangeType): string {
  switch (changeType) {
    case 'CREATED':
      return 'positive';
    case 'UPDATED':
      return 'info';
    case 'DELETED':
      return 'negative';
    default:
      return 'grey';
  }
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
    .filter((task) => task.designation === 'REQUIREMENT');
}

function deriveMilestones(snapshot: TaskSnapshotData): Task[] {
  return snapshot.successors
    .map(taskRefToTask)
    .filter((task) => task.designation === 'MILESTONE');
}

const readonlySnapshot = computed<TaskSnapshotReadonlyData>(() => {
  const snapshot = taskSnapshot.value;
  if (!snapshot) {
    return {
      dbId: props.taskHeaderId,
      title: '',
      description: '',
      designation: 'TASK',
      earliestStart: null,
      scheduleTarget: null,
      effort: null,
      priority: null,
      predecessors: [],
      successors: [],
      parent: null,
      children: [],
      resourceConstraints: [],
      requirements: [],
      milestones: [],
      bookings: [],
    };
  }

  const resourceConstraints: TaskSnapshotReadonlyResourceConstraint[] =
    snapshot.resourceConstraints.map((constraint) => ({
      resources: constraint.entries.map((entry) =>
        resourceRefToResource(entry.resource),
      ),
      optional: constraint.optional,
      speed: constraint.speed,
    }));

  const bookings: TaskSnapshotReadonlyBooking[] = bookingChanges.value
    .filter((change) => change.booking != null)
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
});

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

function constraintLabel(change: ResourceConstraintChangeData): string {
  return change.resourceNames.join(', ') || '—';
}

async function loadIfNeeded() {
  const hasData = historyStore.changes.some(
    (c) => c.revisionId === props.revisionId,
  );
  if (hasData) {
    loading.value = false;
    errorMsg.value = null;
    return;
  }

  loading.value = true;
  errorMsg.value = null;
  try {
    await historyStore.loadHistory(
      props.taskHeaderId,
      'BACKWARD',
      props.revisionId,
      null,
      30,
    );
    const stillHas = historyStore.changes.some(
      (c) => c.revisionId === props.revisionId,
    );
    if (!stillHas) {
      errorMsg.value = 'Revision data not found.';
    }
  } catch (err: unknown) {
    errorMsg.value = err instanceof Error ? err.message : String(err);
  } finally {
    loading.value = false;
  }
}

watch(
  () => [props.taskHeaderId, props.revisionId],
  () => {
    void loadIfNeeded();
  },
);

onMounted(() => {
  void loadIfNeeded();
});
</script>

<style scoped>
.sidebar-section {
  padding: 12px 12px;
  border-bottom: 1px solid #f0f0f0;
}
</style>
