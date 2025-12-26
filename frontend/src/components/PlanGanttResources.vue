<template>

  <GanttChart :start="planStore.start" :end="planStore.end" :rows="resourceRows" hasAvailability :dependencies="[]"
    :selectedRowIds="selectedRowIds" :selectedAllocIds="selectedAllocIds" scrollYKey="resources"
    @alloc-click="onAllocClick" @row-click="onResourceClick" key="gantt-resources">
    <template #corner>
      <SortMenu :modelValue="resourceSortOptions" @update:modelValue="updateSortOptions">
        <template #activator="{ toggle }">
          <q-btn aria-label="Sort Order" flat icon="sort" @click.stop="toggle" />
        </template>
      </SortMenu>
      <q-btn aria-label="New task" flat @click.stop="onNewTask" icon="add_task">
        <q-tooltip>New Task</q-tooltip></q-btn>
      <q-btn aria-label="New resource" flat @click.stop="onNewResource" icon="person_add">
        <q-tooltip>New Resource</q-tooltip></q-btn>
    </template>
  </GanttChart>
</template>

<script setup lang="ts">

import { usePlanStore } from 'src/stores/plan';
import { useResourceStore } from 'src/stores/resource';
import { useSidebarStore, ResourceSidebarData, TaskSidebarData, NewTaskSidebarData, NewResourceSidebarData } from 'src/stores/sidebar';
import { computed } from 'vue';
import GanttChart from './GanttChart.vue';
import type { Row } from './GanttChart.vue';
import { TaskDesignation } from 'src/gql/graphql';
import SortMenu from './SortMenu.vue';
import { resourceSortOptions, type SortOption } from './sortOptions'


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

function updateSortOptions(newOpts: SortOption[]) {
  resourceSortOptions.splice(0, resourceSortOptions.length, ...newOpts);
}

const resourceRows = computed(() => {
  const arr = Array.from(resourceStore.resources).map(r => ({
    id: r.dbId,
    name: resourceStore.resource(r.dbId)?.name ?? '<UNNAMED>',
    designation: TaskDesignation.Task,
    depth: 0,
    allocations: planStore.by_resource(r.dbId).map(a => ({ dbId: a.dbId, start: a.start, end: a.end, task: a.task, allocationType: a.allocationType })),
    availability: availability.value[r.dbId] ?? []
  }));

  function cmp(a: Row, b: Row) {
    for (const opt of resourceSortOptions) {
      if (opt.key === 'name') {
        const ca = a.name ?? '';
        const cb = b.name ?? '';
        if (ca !== cb) return (ca.localeCompare(cb)) * (opt.asc ? 1 : -1);
      }
      else if (opt.key === 'added') {
        const aa = resourceStore.resource(a.id)?.added?.getTime() ?? 0;
        const ba = resourceStore.resource(b.id)?.added?.getTime() ?? 0;
        if (aa !== ba) return (aa - ba) * (opt.asc ? 1 : -1);
      }
      else if (opt.key === 'removed') {
        const ar = resourceStore.resource(a.id)?.removed ? resourceStore.resource(a.id)!.removed!.getTime() : 0;
        const br = resourceStore.resource(b.id)?.removed ? resourceStore.resource(b.id)!.removed!.getTime() : 0;
        if (ar !== br) return (ar - br) * (opt.asc ? 1 : -1);
      }
      else if (opt.key === 'totalHours') {
        const ah = planStore.by_resource(a.id).reduce((s, x) => s + (new Date(x.end).getTime() - new Date(x.start).getTime()), 0);
        const bh = planStore.by_resource(b.id).reduce((s, x) => s + (new Date(x.end).getTime() - new Date(x.start).getTime()), 0);
        if (ah !== bh) return (ah - bh) * (opt.asc ? 1 : -1);
      }
      else if (opt.key === 'earliestStart') {
        const ae = planStore.by_resource(a.id).map(x => new Date(x.start).getTime()).sort((x, y) => x - y)[0] ?? Infinity;
        const be = planStore.by_resource(b.id).map(x => new Date(x.start).getTime()).sort((x, y) => x - y)[0] ?? Infinity;
        if (ae !== be) return (ae - be) * (opt.asc ? 1 : -1);
      }
      else if (opt.key === 'lastEnd') {
        const ae = planStore.by_resource(a.id).map(x => new Date(x.end).getTime()).sort((x, y) => y - x)[0] ?? -Infinity;
        const be = planStore.by_resource(b.id).map(x => new Date(x.end).getTime()).sort((x, y) => y - x)[0] ?? -Infinity;
        if (ae !== be) return (ae - be) * (opt.asc ? 1 : -1);
      }
    }
    return 0;
  }

  arr.sort((a, b) => cmp(a, b));
  return arr;
});

// compute selections from sidebar
const selectedRowIds = computed(() => {
  const active = sidebarStore.activeSidebar;
  if (!active || !sidebarStore.isSelected) return [] as number[];
  // if active sidebar is a resource, highlight that row
  if (active instanceof ResourceSidebarData) {
    return [active.resourceId];
  }
  return [] as number[];
});

const selectedAllocIds = computed(() => {
  const active = sidebarStore.activeSidebar;
  if (!active || !sidebarStore.isSelected) return [] as number[];
  // if active sidebar is a task, highlight allocations for that task
  if (active instanceof TaskSidebarData) {
    const taskId = active.taskId;
    return planStore.by_task(taskId).map(a => a.dbId);
  }
  return [] as number[];
});


function onResourceClick(rid: number) {
  sidebarStore.toggleSidebar(new ResourceSidebarData(rid));
}

function onAllocClick(data: { taskId: number | null }) {
  if (data.taskId != null) {
    sidebarStore.toggleSidebar(new TaskSidebarData(data.taskId));
  }
}

function onNewTask() {
  sidebarStore.pushSidebar(new NewTaskSidebarData());
}

function onNewResource() {
  sidebarStore.pushSidebar(new NewResourceSidebarData());
}

</script>