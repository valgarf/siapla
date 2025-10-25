<template>

  <GanttChart :start="planStore.start" :end="planStore.end" :rows="resourceRows" :availability="availability"
    :dependencies="[]" @alloc-click="onAllocClick" @row-click="onResourceClick">
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
import { useDialogStore, ResourceDialogData, TaskDialogData, NewTaskDialogData, NewResourceDialogData } from 'src/stores/dialog';
import { computed } from 'vue';
import GanttChart from './GanttChart.vue';
import { TaskDesignation } from 'src/gql/graphql';

const planStore = usePlanStore();
const resourceStore = useResourceStore();
const dialogStore = useDialogStore();


const startDay = computed(() => {
  const d = planStore.start;
  return new Date(d.getFullYear(), d.getMonth(), d.getDate() - 1);
});
const endDay = computed(() => {
  const d = planStore.end;
  return new Date(d.getFullYear(), d.getMonth(), d.getDate() + 1);
});


const combinedAvailabiltyQuery = resourceStore.fetchCombinedAvailability(startDay, endDay);

const resourceRows = computed(() => {
  return Array.from(resourceStore.resources).map(r => ({
    id: r.dbId,
    name: resourceStore.resource(r.dbId)?.name ?? '<UNNAMED>',
    designation: TaskDesignation.Task,
    depth: 0,
    allocations: planStore.by_resource(r.dbId).map(a => ({
      dbId: a.dbId, start: a.start, end: a.end, task: a.task, allocationType: a.allocationType
    }))
  }));
});

const availability = computed(() => {
  const out: { rowId: number; segments: { start: string | Date; end: string | Date }[] }[] = [];
  const q = combinedAvailabiltyQuery;
  if (!q || !q.result || q.result.value == null) return out;
  const data = q.result.value;
  for (const r of data.resources) {
    out.push({ rowId: r.dbId, segments: r.combinedAvailability.map(s => ({ start: s.start, end: s.end })) });
  }
  return out;
});

function onResourceClick(rid: number) {
  dialogStore.pushDialog(new ResourceDialogData(rid));
}

function onAllocClick(data: { taskId: number | null }) {
  if (data.taskId != null) dialogStore.pushDialog(new TaskDialogData(data.taskId));
}

function onNewTask() {
  dialogStore.pushDialog(new NewTaskDialogData());
}

function onNewResource() {
  dialogStore.pushDialog(new NewResourceDialogData());
}

</script>