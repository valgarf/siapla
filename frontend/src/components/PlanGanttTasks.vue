<template>
    <GanttChart :start="planStore.start" :end="planStore.end" :rows="ganttRows" :availability="availability"
        :dependencies="dependencies" :rowSymbols="rowSymbols" @alloc-click="onAllocClick" @row-click="onTaskClick">
        <template #corner>
            <div class="corner-buttons">
                <q-btn aria-label="New task" flat @click.stop="onNewTask" icon="add_task" />
                <q-btn aria-label="New resource" flat @click.stop="onNewResource" icon="person_add" />
            </div>
        </template>
        <!-- left column (rows) and group toggle are rendered inside GanttChart now -->
    </GanttChart>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useIssueStore } from 'src/stores/issue';
import GanttChart from './GanttChart.vue';
import { usePlanStore } from 'src/stores/plan';
import { useTaskStore, type Task } from 'src/stores/task';
import { TaskDesignation } from 'src/gql/graphql';
import { useSidebarStore, TaskSidebarData, NewTaskSidebarData, NewResourceSidebarData } from 'src/stores/sidebar';

const planStore = usePlanStore();
const taskStore = useTaskStore();
const sidebarStore = useSidebarStore();


// collapse state moved to GanttChart component

function onTaskClick(tid: number | null) {
    if (tid != null) {
        sidebarStore.selectedRow = tid;
        sidebarStore.selectedElements = [{ type: 'task', id: tid }];
        sidebarStore.pushSidebar(new TaskSidebarData(tid));
    }
}
function onAllocClick(data: { rowId: number | null }) {
    onTaskClick(data.rowId)
}
function onNewTask() {
    sidebarStore.selectedRow = null;
    sidebarStore.selectedElements = [{ type: 'new' }];
    sidebarStore.pushSidebar(new NewTaskSidebarData());
}
function onNewResource() {
    sidebarStore.pushSidebar(new NewResourceSidebarData());
}

// Build flattened rows for the left list and the Gantt rows structure
const rows = computed(() => {
    const tasks = taskStore.tasks.slice();
    const roots = tasks.filter((t) => t.parent == null).sort((a, b) => a.title.localeCompare(b.title));
    const result: { task: Task; depth: number }[] = [];
    function walk(t: Task, depth: number) {
        result.push({ task: t, depth });
        if (t.designation == TaskDesignation.Group) {
            const children = t.children.slice().sort((a, b) => a.title.localeCompare(b.title));
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
    }));
});

const issueStore = useIssueStore();
const rowSymbols = computed(() => {
    const map = new Map<number, { rowId: number; symbol: string; title?: string }>();
    for (const i of issueStore.issues) {
        if (i.taskId != null) {
            const existing = map.get(i.taskId);
            const desc = existing ? existing.title + '\n' + i.description : i.description;
            map.set(i.taskId, { rowId: i.taskId, symbol: '⚠', title: desc });
        }
    }
    return Array.from(map.values());
});

// availability per row: tasks do not have availability in this view; provide empty
const availability = computed(() => {
    return [] as { rowId: number; segments: { start: string | Date; end: string | Date }[] }[];
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

<style scoped>
.corner-buttons {
    display: flex;
    gap: 6px;
    justify-content: center;
    align-content: center;
    height: 100%;
}
</style>
