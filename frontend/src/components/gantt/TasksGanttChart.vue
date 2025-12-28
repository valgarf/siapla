<template>
    <GanttChart :start="planStore.start" :end="planStore.end" :rows="ganttRows" :dependencies="dependencies"
        :rowSymbols="rowSymbols" :selectedRowIds="selectedRowIds" :selectedAllocIds="selectedAllocIds" dataKey="tasks"
        @alloc-click="onAllocClick" @row-click="onTaskClick" @alloc-drag-start="onAllocDragStart"
        @alloc-drag-move-start="onAllocDragMoveStart" @alloc-drag-move="onAllocDragMove"
        @alloc-drag-end="onAllocDragEnd" @delete-booking="onDeleteBooking" @split-booking="onSplitBooking"
        @join-bookings="onJoinBookings" key="gantt-tasks">
        <template #corner>
            <SortMenu :modelValue="taskSortOptions" @update:modelValue="updateSortOptions">
                <template #activator="{ toggle }">
                    <q-btn aria-label="Sort Order" flat icon="sort" @click.stop="toggle">
                        <q-tooltip>Sort Order</q-tooltip>
                    </q-btn>
                </template>
            </SortMenu>
            <q-btn aria-label="New task" flat @click.stop="onNewTask" icon="add_task">
                <q-tooltip>New Task</q-tooltip></q-btn>
            <q-btn aria-label="New resource" flat @click.stop="onNewResource" icon="person_add">
                <q-tooltip>New Resource</q-tooltip></q-btn>
        </template>
        <!-- left column (rows) and group toggle are rendered inside GanttChart now -->
    </GanttChart>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import SortMenu from './SortMenu.vue';


import { useIssueStore } from 'src/stores/issue';
import GanttChart from './GanttChart.vue';
import { usePlanStore } from 'src/stores/plan';
import { useTaskStore, type Task } from 'src/stores/task';
import { TaskDesignation } from 'src/gql/graphql';
import { useSidebarStore, TaskSidebarData, ResourceSidebarData, NewTaskSidebarData, NewResourceSidebarData } from 'src/stores/sidebar';
import { taskSortOptions, type SortOption } from './sortOptions'

const planStore = usePlanStore();
const taskStore = useTaskStore();
const sidebarStore = useSidebarStore();


// collapse state moved to GanttChart component

function onTaskClick(tid: number | null) {
    if (tid != null) {
        sidebarStore.toggleSidebar(new TaskSidebarData(tid));
    }
}
function onAllocClick(data: { rowId: number | null }) {
    onTaskClick(data.rowId)
}

// handlers for GanttChart events
const dragging = ref<{ rowId: number | null; allocId: number | null; edge: string | null } | null>(null);
const dragOriginals = ref(new Map<number, { start: Date; end: Date; grabOffsetMs?: number }>())

function onAllocDragStart(evt: { rowId: number | null; allocId: number | null; edge: 'start' | 'end' | 'move' | null; mouse?: MouseEvent }) {
    dragging.value = { rowId: evt.rowId ?? null, allocId: evt.allocId ?? null, edge: evt.edge ?? null };
}

function onAllocDragEnd() {
    dragging.value = null;
}

function onAllocDragMoveStart(evt: { rowId: number | null; allocId: number | null; mouse?: MouseEvent; date?: Date }) {
    if (!evt?.allocId) return;
    const alloc = planStore.allocation(evt.allocId);
    if (!alloc) return;
    const start = alloc.start instanceof Date ? alloc.start : new Date(alloc.start);
    const end = alloc.end instanceof Date ? alloc.end : new Date(alloc.end);
    const entry: { start: Date; end: Date; grabOffsetMs?: number } = { start, end };
    if (evt.date && evt.date instanceof Date) {
        if (evt && (dragging.value?.edge === 'move')) {
            entry.grabOffsetMs = evt.date.getTime() - start.getTime();
        }
    }
    dragOriginals.value.set(evt.allocId, entry);
}

async function onAllocDragMove(evt: { rowId: number | null; allocId: number | null; edge: 'start' | 'end' | 'move' | null; mouse?: MouseEvent; date?: Date }) {
    if (!evt?.allocId) return;
    const orig = dragOriginals.value.get(evt.allocId);
    const alloc = planStore.allocation(evt.allocId);
    if (!orig || !alloc) return;
    if (!evt.date) return;
    const d = evt.date;
    let newStart = orig.start;
    let newEnd = orig.end;
    if (evt.edge === 'start') {
        newStart = d;
    } else if (evt.edge === 'end') {
        newEnd = d;
    } else {
        const duration = orig.end.getTime() - orig.start.getTime();
        const offset = orig.grabOffsetMs ?? 0;
        newStart = new Date(d.getTime() - offset);
        newEnd = new Date(newStart.getTime() + duration);
    }
    await planStore.saveBooking({ ...alloc, start: newStart, end: newEnd });
}

async function onDeleteBooking(payload: { rowId: number | null; allocId: number | null }) {
    if (!payload?.allocId) return;
    await planStore.deleteBooking(payload.allocId);
}

async function onSplitBooking(payload: { rowId: number | null; allocId: number | null; zoom?: number }) {
    if (!payload?.allocId) return;
    await planStore.splitBooking(payload.allocId);
}

async function onJoinBookings(payload: { rowId: number | null; leftAllocId: number; rightAllocId: number }) {
    await planStore.joinBookings(payload.leftAllocId, payload.rightAllocId);
}
function onNewTask() {
    sidebarStore.pushSidebar(new NewTaskSidebarData());
}
function onNewResource() {
    sidebarStore.pushSidebar(new NewResourceSidebarData());
}


const issueStore = useIssueStore();
const rowSymbols = computed(() => {
    const map: { [rowid: number]: { symbolUTF8: string; title?: string } } = {};
    for (const i of issueStore.issues) {
        if (i.taskId != null) {
            const existing = map[i.taskId];
            const desc = existing ? existing.title + '\n' + i.description : i.description;
            map[i.taskId] = { symbolUTF8: '⚠', title: desc };
        }
    }
    return map;
});

// Build flattened rows for the left list and the Gantt rows structure
function updateSortOptions(newOpts: SortOption[]) {
    // replace reactive array contents to preserve reactivity
    taskSortOptions.splice(0, taskSortOptions.length, ...newOpts);
}

const rows = computed(() => {
    const tasks = taskStore.tasks.slice();

    function isBookingType(v: unknown): boolean {
        return v === 'BOOKING' || v === 1
    }

    function cmp(a: Task, b: Task) {
        for (const opt of taskSortOptions) {
            if (opt.key === 'isRequirement') {
                const ab = a.designation == TaskDesignation.Requirement;
                const bb = b.designation == TaskDesignation.Requirement;
                if (ab !== bb) return ((ab ? 1 : 0) - (bb ? 1 : 0)) * (opt.asc ? 1 : -1);
            }
            if (opt.key === 'isMilestone') {
                const ab = a.designation == TaskDesignation.Milestone;
                const bb = b.designation == TaskDesignation.Milestone;
                if (ab !== bb) return ((ab ? 1 : 0) - (bb ? 1 : 0)) * (opt.asc ? 1 : -1);
            }
            if (opt.key === 'isGroup') {
                const ab = a.designation == TaskDesignation.Group;
                const bb = b.designation == TaskDesignation.Group;
                if (ab !== bb) return ((ab ? 1 : 0) - (bb ? 1 : 0)) * (opt.asc ? 1 : -1);
            }
            if (opt.key === 'name') {
                const ca = a.title ?? '';
                const cb = b.title ?? '';
                if (ca !== cb) return (ca.localeCompare(cb)) * (opt.asc ? 1 : -1);
            }
            else if (opt.key === 'start') {
                const as = planStore.by_task(a.dbId).map(x => new Date(x.start).getTime()).sort((x, y) => x - y)[0] ?? Infinity;
                const bs = planStore.by_task(b.dbId).map(x => new Date(x.start).getTime()).sort((x, y) => x - y)[0] ?? Infinity;
                if (as !== bs) return (as - bs) * (opt.asc ? 1 : -1);
            }
            else if (opt.key === 'end') {
                const ae = planStore.by_task(a.dbId).map(x => new Date(x.end).getTime()).sort((x, y) => y - x)[0] ?? -Infinity;
                const be = planStore.by_task(b.dbId).map(x => new Date(x.end).getTime()).sort((x, y) => y - x)[0] ?? -Infinity;
                if (ae !== be) return (ae - be) * (opt.asc ? 1 : -1);
            }
            else if (opt.key === 'isBooked') {
                const ab = planStore.by_task(a.dbId).some(x => isBookingType(x.allocationType));
                const bb = planStore.by_task(b.dbId).some(x => isBookingType(x.allocationType));
                if (ab !== bb) return ((ab ? 1 : 0) - (bb ? 1 : 0)) * (opt.asc ? 1 : -1);
            }
            else if (opt.key === 'effort') {
                const ae = planStore.by_task(a.dbId).reduce((s, x) => s + (new Date(x.end).getTime() - new Date(x.start).getTime()), 0);
                const be = planStore.by_task(b.dbId).reduce((s, x) => s + (new Date(x.end).getTime() - new Date(x.start).getTime()), 0);
                if (ae !== be) return (ae - be) * (opt.asc ? 1 : -1);
            }
        }
        return 0;
    }

    const roots = tasks.filter((t) => t.parent == null).sort((a, b) => cmp(a, b));
    const result: { task: Task; depth: number }[] = [];
    function walk(t: Task, depth: number) {
        result.push({ task: t, depth });
        if (t.designation == TaskDesignation.Group) {
            const children = t.children.slice().sort((a, b) => cmp(a, b));
            for (const c of children) walk(c, depth + 1);
        }
    }
    for (const r of roots) walk(r, 0);
    return result;
});

// Build rows formatted for Gantt component
const ganttRows = computed(() => {
    return rows.value.map((r) => ({
        id: r.task.dbId,
        name: r.task.title,
        depth: r.depth,
        designation: r.task.designation,
        allocations: planStore.by_task(r.task.dbId).map((a) => ({ dbId: a.dbId, start: a.start, end: a.end, task: r.task, allocationType: a.allocationType })),
        scheduleTarget: r.task.scheduleTarget,
        earliestStart: r.task.earliestStart,
        availability: [],
        symbol: rowSymbols.value[r.task.dbId]
    }));
});

// compute selections based on sidebar
const selectedRowIds = computed(() => {
    const active = sidebarStore.activeSidebar;
    if (!active || !sidebarStore.isSelected) return [] as number[];
    if (active instanceof TaskSidebarData) {
        return [active.taskId];
    }
    return [] as number[];
});

const selectedAllocIds = computed(() => {
    const active = sidebarStore.activeSidebar;
    if (!active || !sidebarStore.isSelected) return [] as number[];
    if (active instanceof ResourceSidebarData) {
        const resId = active.resourceId;
        return planStore.by_resource(resId).map(a => a.dbId);
    }
    return [] as number[];
});


// dependencies: extract predecessor relationships
const dependencies = computed(() => {
    const deps: { predId: number; succId: number }[] = [];
    for (const t of taskStore.tasks) {
        for (const p of t.predecessors) deps.push({ predId: p.dbId, succId: t.dbId });
    }
    return deps;
});


// row clicks are handled inside GanttChart now

</script>
