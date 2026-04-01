<template>
  <div class="task-history-change-list column no-wrap">
    <div class="history-toolbar q-gutter-xs row items-center">
      <q-btn dense flat icon="first_page" label="Start" size="sm" @click="jumpToStart" :loading="historyStore.loading" />
      <q-btn dense flat icon="last_page" label="Latest" size="sm" @click="jumpToLatest" :loading="historyStore.loading" />
      <q-btn dense flat icon="event" label="Jump to date" size="sm" @click="showDatePicker = true" />
    </div>
    <q-separator />

    <div v-if="historyStore.error" class="q-pa-md text-negative">
      <q-icon name="error" class="q-mr-xs" />
      {{ historyStore.error }}
    </div>

    <div v-if="!historyStore.loading && historyStore.revisionGroups.length === 0 && !historyStore.error" class="q-pa-md text-grey-6 text-center">
      No history available
    </div>

    <q-scroll-area ref="scrollAreaRef" class="col history-scroll-area" style="min-height: 0">
      <q-infinite-scroll @load="onLoadMore" :offset="100" :disable="!historyStore.hasMore">
        <q-list separator dense>
          <template v-for="group in historyStore.revisionGroups" :key="group.revisionId">
            <q-item
              clickable
              class="history-revision-item"
              :active="isSelected(group.revisionId)"
              active-class="history-revision-item--active"
              @click.exact="selectRevision(group.revisionId)"
              @click.ctrl.exact="toggleCompare(group.revisionId)"
            >
              <q-item-section side top class="history-revision-checkbox">
                <q-checkbox
                  :model-value="isCompareSelected(group.revisionId)"
                  @update:model-value="toggleCompare(group.revisionId)"
                  size="xs"
                  dense
                />
              </q-item-section>
              <q-item-section class="history-revision-content">
                <q-item-label class="text-weight-medium history-revision-timestamp">
                  {{ formatTimestamp(group.timestamp) }}
                </q-item-label>
                <q-item-label caption class="history-revision-caption">
                  Rev {{ group.revisionId }}
                </q-item-label>
                <div class="q-mt-xs q-gutter-xs row items-center">
                  <template v-for="(change, idx) in group.changes" :key="idx">
                    <q-badge
                      :color="changeTypeColor(change.changeType)"
                      text-color="white"
                      class="history-change-badge"
                    >
                      <q-icon :name="changeIcon(change.__typename)" size="12px" class="q-mr-xs" />
                      {{ changeLabel(change.__typename) }}
                    </q-badge>
                  </template>
                </div>
              </q-item-section>
              <q-item-section side top class="history-revision-dots">
                <div class="column items-center q-gutter-xs">
                  <template v-for="(change, idx) in uniqueChangeTypes(group.changes)" :key="idx">
                    <div
                      class="history-revision-dot"
                      :style="{ backgroundColor: dotColor(change) }"
                    />
                  </template>
                </div>
              </q-item-section>
            </q-item>
          </template>
        </q-list>

        <template #loading>
          <div class="row justify-center q-my-md">
            <q-spinner color="primary" size="30px" />
          </div>
        </template>
      </q-infinite-scroll>
    </q-scroll-area>

    <q-dialog v-model="showDatePicker">
      <q-card style="min-width: 320px">
        <q-card-section class="text-h6">Jump to date</q-card-section>
        <q-card-section>
          <q-date v-model="jumpDate" mask="YYYY-MM-DD" />
          <q-input v-model="jumpTime" label="Time (HH:mm)" mask="##:##" class="q-mt-sm" outlined dense />
        </q-card-section>
        <q-card-actions align="right">
          <q-btn flat label="Cancel" v-close-popup />
          <q-btn flat label="Go" color="primary" @click="performJumpToDate" v-close-popup />
        </q-card-actions>
      </q-card>
    </q-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { useTaskHistoryStore, type ChangeType, type ChangeData } from 'src/stores/taskHistory';

const props = defineProps<{
  taskHeaderId: number;
}>();

const emit = defineEmits<{
  select: [revisionId: number];
  compare: [revisionA: number, revisionB: number];
}>();

const historyStore = useTaskHistoryStore();

const selectedRevision = ref<number | null>(null);
const compareSet = ref<Set<number>>(new Set());
const showDatePicker = ref(false);
const jumpDate = ref('');
const jumpTime = ref('00:00');
const scrollAreaRef = ref<HTMLElement | null>(null);

const DEFAULT_LIMIT = 30;

function isSelected(revisionId: number): boolean {
  return selectedRevision.value === revisionId;
}

function isCompareSelected(revisionId: number): boolean {
  return compareSet.value.has(revisionId);
}

function selectRevision(revisionId: number) {
  compareSet.value.clear();
  selectedRevision.value = revisionId;
  emit('select', revisionId);
}

function toggleCompare(revisionId: number) {
  const s = new Set(compareSet.value);
  if (s.has(revisionId)) {
    s.delete(revisionId);
  } else {
    if (s.size >= 2) {
      // Replace the oldest one
      const iter = s.values();
      const first = iter.next().value;
      if (first !== undefined) {
        s.delete(first);
      }
    }
    s.add(revisionId);
  }
  compareSet.value = s;
  if (s.size === 2) {
    const ids = Array.from(s).sort((a, b) => a - b);
    if (ids[0] !== undefined && ids[1] !== undefined) {
      emit('compare', ids[0], ids[1]);
    }
  } else if (s.size === 1) {
    const id = Array.from(s)[0];
    if (id !== undefined) {
      selectedRevision.value = id;
      emit('select', id);
    }
  }
}

function changeTypeColor(changeType: ChangeType): string {
  switch (changeType) {
    case 'CREATED': return 'positive';
    case 'UPDATED': return 'info';
    case 'DELETED': return 'negative';
    default: return 'grey';
  }
}

function dotColor(changeType: ChangeType): string {
  switch (changeType) {
    case 'CREATED': return '#4caf50';
    case 'UPDATED': return '#2196f3';
    case 'DELETED': return '#f44336';
    default: return '#9e9e9e';
  }
}

function changeIcon(typename: string): string {
  switch (typename) {
    case 'TaskIterationChange': return 'task_alt';
    case 'BookingChange': return 'event';
    case 'DependencyChange': return 'link';
    case 'ResourceConstraintChange': return 'person';
    default: return 'info';
  }
}

function changeLabel(typename: string): string {
  switch (typename) {
    case 'TaskIterationChange': return 'Task';
    case 'BookingChange': return 'Booking';
    case 'DependencyChange': return 'Dep';
    case 'ResourceConstraintChange': return 'Constraint';
    default: return 'Change';
  }
}

function uniqueChangeTypes(changes: ChangeData[]): ChangeType[] {
  const seen = new Set<ChangeType>();
  for (const c of changes) {
    seen.add(c.changeType);
  }
  return Array.from(seen);
}

function formatTimestamp(ts: string): string {
  try {
    return new Date(ts).toLocaleString();
  } catch {
    return ts;
  }
}

async function onLoadMore(index: number, done: (stop?: boolean) => void) {
  try {
    await historyStore.loadMore(DEFAULT_LIMIT);
    done(!historyStore.hasMore);
  } catch {
    done(true);
  }
}

async function jumpToStart() {
  await historyStore.jumpToStart(props.taskHeaderId);
  const first = historyStore.revisionGroups[0];
  if (first !== undefined) {
    selectRevision(first.revisionId);
  }
}

async function jumpToLatest() {
  await historyStore.loadHistory(props.taskHeaderId, 'BACKWARD', null, null, DEFAULT_LIMIT);
  const first = historyStore.revisionGroups[0];
  if (first !== undefined) {
    selectRevision(first.revisionId);
  }
}

async function performJumpToDate() {
  if (!jumpDate.value) return;
  const parts = jumpTime.value.split(':');
  const hours = parseInt(parts[0] || '0', 10);
  const minutes = parseInt(parts[1] || '0', 10);
  const dt = new Date(jumpDate.value);
  dt.setHours(hours, minutes, 0, 0);
  await historyStore.jumpToTimestamp(props.taskHeaderId, dt.toISOString());
  const first = historyStore.revisionGroups[0];
  if (first !== undefined) {
    selectRevision(first.revisionId);
  }
}

async function initialLoad() {
  await historyStore.loadHistory(props.taskHeaderId, 'BACKWARD', null, null, DEFAULT_LIMIT);
  const first = historyStore.revisionGroups[0];
  if (first !== undefined) {
    selectRevision(first.revisionId);
  }
}

watch(() => props.taskHeaderId, () => {
  historyStore.reset();
  compareSet.value.clear();
  selectedRevision.value = null;
  void initialLoad();
});

onMounted(() => {
  void initialLoad();
});
</script>

<style scoped>
.task-history-change-list {
  height: 100%;
  border-right: 1px solid rgba(0, 0, 0, 0.12);
  background: transparent;
}

.history-toolbar {
  padding: 8px 12px;
}

.history-scroll-area {
  width: 100%;
}

.history-revision-item {
  padding-left: 8px;
  padding-right: 8px;
  margin: 0;
  border-radius: 0;
  background: transparent;
}

.history-revision-item--active {
  background: rgba(25, 118, 210, 0.12);
}

.history-revision-checkbox {
  min-width: 24px;
  padding-right: 4px;
}

.history-revision-content {
  padding-left: 0;
}

.history-revision-timestamp {
  font-size: 0.8rem;
}

.history-revision-caption {
  font-size: 0.7rem;
}

.history-change-badge {
  font-size: 0.65rem;
}

.history-revision-dots {
  min-width: 16px;
  padding-left: 6px;
}

.history-revision-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}
</style>
