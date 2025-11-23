<template>

  <GanttChart :start="planStore.start" :end="planStore.end" :rows="resourceRows" hasAvailability :dependencies="[]"
    @alloc-click="onAllocClick" @row-click="onResourceClick">
    <template #corner>
      <div class="corner-buttons">
        <q-btn aria-label="New task" flat @click.stop="onNewTask" icon="add_task" />
        <q-btn aria-label="New resource" flat @click.stop="onNewResource" icon="person_add" />
      </div>
    </template>
  </GanttChart>
</template>

<style scoped>
.corner-buttons {
  display: flex;
  gap: 6px;
  justify-content: center;
  align-content: center;
  height: 100%;
}
</style>
<script setup lang="ts">

import { usePlanStore } from 'src/stores/plan';
import { useResourceStore } from 'src/stores/resource';
import { useSidebarStore, ResourceSidebarData, TaskSidebarData, NewTaskSidebarData, NewResourceSidebarData } from 'src/stores/sidebar';
import { computed } from 'vue';
import GanttChart from './GanttChart.vue';
import { TaskDesignation } from 'src/gql/graphql';

const planStore = usePlanStore();
const resourceStore = useResourceStore();
const sidebarStore = useSidebarStore();


const startDay = computed(() => {
  const d = planStore.start;
  return new Date(d.getFullYear(), d.getMonth(), d.getDate() - 1);
});
const endDay = computed(() => {
  const d = planStore.end;
  return new Date(d.getFullYear(), d.getMonth(), d.getDate() + 1);
});


const combinedAvailabiltyQuery = resourceStore.fetchCombinedAvailability(startDay, endDay);


const availability = computed(() => {
  const out: { [rowid: number]: { start: string | Date; end: string | Date }[] } = {}
  const q = combinedAvailabiltyQuery;
  if (!q || !q.result || q.result.value == null) return out;
  const data = q.result.value;
  for (const r of data.resources) {
    out[r.dbId] = r.combinedAvailability.map(s => ({ start: s.start, end: s.end }));
  }
  return out;
});

const resourceRows = computed(() => {
  return Array.from(resourceStore.resources).map(r => ({
    id: r.dbId,
    name: resourceStore.resource(r.dbId)?.name ?? '<UNNAMED>',
    designation: TaskDesignation.Task,
    depth: 0,
    allocations: planStore.by_resource(r.dbId).map(a => ({
      dbId: a.dbId, start: a.start, end: a.end, task: a.task, allocationType: a.allocationType
    })),
    availability: availability.value[r.dbId] ?? []
  }));
});


function onResourceClick(rid: number) {
  sidebarStore.selectedRow = rid;
  sidebarStore.selectedElements = [{ type: 'resource', id: rid }];
  sidebarStore.pushSidebar(new ResourceSidebarData(rid));
}

function onAllocClick(data: { taskId: number | null }) {
  if (data.taskId != null) {
    sidebarStore.selectedRow = data.taskId;
    sidebarStore.selectedElements = [{ type: 'task', id: data.taskId }];
    sidebarStore.pushSidebar(new TaskSidebarData(data.taskId));
  }
}

function onNewTask() {
  sidebarStore.selectedRow = null;
  sidebarStore.selectedElements = [{ type: 'new' }];
  sidebarStore.pushSidebar(new NewTaskSidebarData());
}

function onNewResource() {
  sidebarStore.pushSidebar(new NewResourceSidebarData());
}

</script>