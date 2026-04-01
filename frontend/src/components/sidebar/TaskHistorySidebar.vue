<template>
  <div class="history-root">
    <!-- Toolbar - same as SidebarLayout but without the margins wrapper -->
    <q-toolbar>
      <q-btn v-if="!sidebarStore.currentEditing" :disable="sidebarStore.atFirst()" flat round icon="arrow_back"
        @click="sidebarStore.back" />
      <q-btn v-if="!sidebarStore.currentEditing" :disable="sidebarStore.atLast()" flat round icon="arrow_forward"
        @click="sidebarStore.next" class="q-mr-sm" />
      <div class="col">
        <q-breadcrumbs>
          <q-breadcrumbs-el label="Tasks" disable />
          <q-breadcrumbs-el :label="taskTitle" class="clickable" @click="goBackToTask" />
          <q-breadcrumbs-el label="History" disable />
        </q-breadcrumbs>
      </div>
      <q-space />
      <q-btn flat round icon="fullscreen" @click="sidebarStore.toggleExpand" aria-label="Expand" />
    </q-toolbar>

    <div class="history-body row no-wrap">
      <!-- Left: change list panel, flush against left edge -->
      <div class="history-change-list-panel">
        <TaskHistoryChangeList
          :task-header-id="taskId"
          @select="onSelect"
          @compare="onCompare"
        />
      </div>

      <!-- Right: detail panel, with max-width constraint like the normal sidebar -->
      <div class="col history-main-panel">
        <div class="history-main-content">
          <TaskHistoryCompare
            v-if="viewMode === 'compare' && compareRevA != null && compareRevB != null"
            :task-header-id="taskId"
            :revision-a="compareRevA"
            :revision-b="compareRevB"
          />
          <TaskHistoryDetail
            v-else-if="selectedRevision != null"
            :task-header-id="taskId"
            :revision-id="selectedRevision"
          />
          <div v-else class="q-pa-xl text-center text-grey-6">
            <q-icon name="history" size="64px" class="q-mb-md" />
            <div class="text-h6">Select a revision</div>
            <div class="text-body2 q-mt-xs">
              Click a revision in the sidebar to view its details.<br>
              Ctrl+Click or use checkboxes to select two for comparison.
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import TaskHistoryChangeList from 'src/components/history/TaskHistoryChangeList.vue';
import TaskHistoryDetail from 'src/components/history/TaskHistoryDetail.vue';
import TaskHistoryCompare from 'src/components/history/TaskHistoryCompare.vue';
import { TaskSidebarData, useSidebarStore } from 'src/stores/sidebar';
import { useTaskStore } from 'src/stores/task';
import { useTaskHistoryStore } from 'src/stores/taskHistory';

const props = defineProps<{ taskId: number }>();

const sidebarStore = useSidebarStore();
const taskStore = useTaskStore();
const historyStore = useTaskHistoryStore();

const viewMode = ref<'view' | 'compare'>('view');
const selectedRevision = ref<number | null>(null);
const compareRevA = ref<number | null>(null);
const compareRevB = ref<number | null>(null);

const taskTitle = computed(() => {
  const task = taskStore.task(props.taskId);
  return task?.title ?? `Task #${props.taskId}`;
});

function onSelect(revisionId: number) {
  selectedRevision.value = revisionId;
  compareRevA.value = null;
  compareRevB.value = null;
  viewMode.value = 'view';
}

function onCompare(revA: number, revB: number) {
  compareRevA.value = revA;
  compareRevB.value = revB;
  viewMode.value = 'compare';
}

function goBackToTask() {
  sidebarStore.replaceTop(new TaskSidebarData(props.taskId));
}

let exitingViaCollapse = false;

// Watch for sidebar collapse → exit history mode
watch(() => sidebarStore.isExpanded, (newVal, oldVal) => {
  if (oldVal === true && newVal === false) {
    exitingViaCollapse = true;
    goBackToTask();
  }
});

onMounted(() => {
  // Auto-expand sidebar for history view
  if (!sidebarStore.isExpanded) {
    sidebarStore.toggleExpand();
  }
});

onUnmounted(() => {
  historyStore.reset();
  // Only collapse if we're NOT already exiting via the collapse button
  if (!exitingViaCollapse && sidebarStore.isExpanded) {
    sidebarStore.toggleExpand();
  }
});
</script>

<style scoped>
.q-btn.disabled {
  opacity: 0.3 !important;
}

.history-root {
  height: 100%;
  display: flex;
  flex-direction: column;
}
.history-body {
  flex: 1;
  min-height: 0;
}
.history-change-list-panel {
  width: 300px;
  min-width: 300px;
  border-right: 1px solid rgba(0, 0, 0, 0.12);
  overflow-y: auto;
}
.history-main-panel {
  overflow-y: auto;
}
.history-main-content {
  max-width: 1000px;
  margin-left: auto;
  margin-right: auto;
}
</style>
