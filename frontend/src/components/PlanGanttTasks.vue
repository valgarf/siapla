<template>
    <div class="gantt-grid">
        <!-- Top left: new task button -->
        <div class="gantt-corner">
            <div class="corner-buttons">
                <q-btn aria-label="New task" flat @click.stop="onNewTask" icon="add_task">
                </q-btn>
            </div>
        </div>
        <!-- Top right: header -->
        <div class="gantt-header" @mousedown="onPanStart" @mousemove="onPanMoveX" @mouseup="onPanEnd"
            @mouseleave="onPanEnd">
            <div class="gantt-header-scroll"
                :style="{ width: timelineWidth + 'px', left: '0px', transform: `translate(${-scrollX}px, 0)` }">
                <svg :width="timelineWidth" :height="headerHeight">
                    <!-- Year/Month row -->
                    <g>
                        <template v-for="(month, i) in months" :key="i">
                            <rect :x="month.x" y="0" :width="month.width" :height="monthRowHeight" fill="#fff"
                                stroke="#ccc" stroke-width="1" />
                            <text :x="month.x + 4" :y="monthRowHeight - 6" font-size="12" fill="#333">{{ month.label
                                }}</text>
                        </template>
                    </g>
                    <!-- Day row -->
                    <g>
                        <template v-for="(day, i) in days" :key="i">
                            <rect :x="day.x" :y="monthRowHeight" :width="dayWidth" :height="dayRowHeight" fill="#fff"
                                stroke="#ccc" stroke-width="1" />
                            <text :x="day.x + 2" :y="monthRowHeight + dayRowHeight - 6" font-size="10" fill="#666">{{
                                day.label
                                }}</text>
                        </template>
                    </g>
                </svg>
            </div>
        </div>
        <!-- Bottom left: task list -->
        <div class="gantt-resources"
            :style="{ height: chartHeight + 'px', width: resourceColWidth + 'px', position: 'relative', overflow: 'hidden' }"
            @mousedown="onPanStart" @mousemove="onPanMoveY" @mouseup="onPanEnd" @mouseleave="onPanEnd">
            <div :style="{ position: 'absolute', top: -scrollY + 'px', left: 0, width: '100%' }">
                <div v-for="row in visibleRows" :key="row.task.dbId" class="gantt-resource-row"
                    :style="{ height: rowHeight + 'px', paddingLeft: (8 + row.depth * 12) + 'px' }">
                    <q-btn flat dense size="sm" v-if="row.task.designation == TaskDesignation.Group"
                        @click.stop="toggleGroup(row.task.dbId)"
                        :icon="collapsedGroups.has(row.task.dbId) ? 'chevron_right' : 'expand_more'" />
                    <span @click.stop="() => onTaskClick(row.task.dbId)">{{ row.task.title }}</span>
                </div>
            </div>
        </div>
        <!-- Bottom right: scrollable chart -->
        <div class="gantt-chart-scroll" ref="scrollCell" @mousedown="onPanStart" @mousemove="onPanMove"
            @mouseup="onPanEnd" @mouseleave="onPanEnd">
            <svg :width="timelineWidth" :height="chartHeight"
                :style="{ transform: `translate(${-scrollX}px, ${-scrollY}px)` }">
                <defs>
                    <marker id="arrow" markerWidth="10" markerHeight="10" refX="10" refY="5" orient="auto"
                        markerUnits="strokeWidth">
                        <path d="M0,0 L10,5 L0,10 z" fill="#333" />
                    </marker>
                </defs>

                <!-- Horizontal separators -->
                <g>
                    <template v-for="(row, i) in visibleRows" :key="'line'+row.task.dbId">
                        <line :x1="0" :y1="i * rowHeight" :x2="timelineWidth" :y2="i * rowHeight" stroke="#ddd"
                            stroke-width="1" />
                    </template>
                    <line :x1="0" :y1="visibleRows.length * rowHeight" :x2="timelineWidth"
                        :y2="visibleRows.length * rowHeight" stroke="#ddd" stroke-width="1" />
                </g>

                <!-- Vertical day lines -->
                <g>
                    <template v-for="(day, i) in days" :key="i">
                        <line :x1="day.x" :y1="0" :x2="day.x" :y2="chartHeight" stroke="#ddd" stroke-width="1" />
                    </template>
                </g>

                <!-- Group bars: span from first child start to last child end -->
                <g>
                    <template v-for="(row, i) in visibleRows" :key="'group'+row.task.dbId">
                        <rect
                            v-if="row.task.designation == TaskDesignation.Group && planStore.by_task(row.task.dbId).length > 0"
                            :x="dateToX(planStore.by_task(row.task.dbId)[0]?.start)" :y="i * rowHeight + barPadding"
                            :width="dateToX(planStore.by_task(row.task.dbId)[0]?.end) - dateToX(planStore.by_task(row.task.dbId)[0]?.start)"
                            :height="barHeight" fill="#66bb6a" rx="3" />
                    </template>
                </g>

                <!-- Task and milestone allocation bars -->
                <g>
                    <template v-for="(row, i) in visibleRows" :key="'taskalloc'+row.task.dbId">
                        <template v-if="TaskDesignation.Task == row.task.designation">
                            <template v-for="alloc in planStore.by_task(row.task.dbId)" :key="alloc.dbId">
                                <rect :x="dateToX(alloc.start)" :y="i * rowHeight + barPadding"
                                    :width="dateToX(alloc.end) - dateToX(alloc.start)" :height="barHeight"
                                    fill="#42a5f5" rx="3" @click.stop="() => onTaskClick(row.task.dbId)" />
                                <text :x="dateToX(alloc.start) + 4" :y="i * rowHeight + barPadding + barHeight / 2 + 4"
                                    font-size="11" fill="#fff">{{ row.task.title }}</text>
                            </template>
                        </template>
                    </template>
                </g>

                <!-- Milestones and requirements symbols -->
                <g>
                    <template v-for="(row, i) in visibleRows" :key="'symbol'+row.task.dbId">
                        <template v-if="row.task.designation == TaskDesignation.Milestone && row.task.scheduleTarget">
                            <g
                                :transform="`translate(${dateToX(row.task.scheduleTarget)}, ${i * rowHeight + rowHeight / 2})`">
                                <rect x="-6" y="-6" width="12" height="12" fill="#ffb74d" transform="rotate(45)"
                                    stroke="#b06b00" />
                            </g>
                        </template>
                        <template
                            v-if="row.task.designation == TaskDesignation.Milestone && planStore.by_task(row.task.dbId).length > 0">
                            <g
                                :transform="`translate(${dateToX(planStore.by_task(row.task.dbId)[0]!.start)}, ${i * rowHeight + rowHeight / 2})`">
                                <rect x="-6" y="-6" width="12" height="12"
                                    :fill="planStore.by_task(row.task.dbId)[0]!.start <= row.task.scheduleTarget! ? '#66bb6a' : '#ef5350'"
                                    transform="rotate(45)" stroke="#b06b00" />
                            </g>
                        </template>
                        <template v-if="row.task.designation == TaskDesignation.Requirement && row.task.earliestStart">
                            <g
                                :transform="`translate(${dateToX(row.task.earliestStart)}, ${i * rowHeight + rowHeight / 2})`">
                                <circle r="6" fill="#ab47bc" stroke="#6a1b9a" />
                            </g>
                        </template>
                    </template>
                </g>

                <!-- Dependency arrows (predecessor -> successor) -->
                <g stroke="#333" stroke-width="1.2" fill="none" marker-end="url(#arrow)">
                    <template v-for="row in visibleRows" :key="'deps'+row.task.dbId">
                        <template v-for="pred of row.task.predecessors" :key="row.task.dbId + '-pred-' + pred.dbId">
                            <path v-if="allocArrow(pred.dbId, row.task.dbId)"
                                :d="allocArrow(pred.dbId, row.task.dbId)" />
                        </template>
                    </template>
                </g>

            </svg>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue';
import { usePlanStore } from 'src/stores/plan';
import { useTaskStore, type Task } from 'src/stores/task';
import { TaskDesignation } from 'src/gql/graphql';
import { useDialogStore, TaskDialogData, NewTaskDialogData } from 'src/stores/dialog';

const planStore = usePlanStore();
const taskStore = useTaskStore();
const dialogStore = useDialogStore();

const resourceColWidth = 240;
const rowHeight = 40;
const barPadding = 8;
const barHeight = rowHeight - barPadding * 2;
const dayWidth = 32;
const monthRowHeight = 28;
const dayRowHeight = 22;
const headerHeight = monthRowHeight + dayRowHeight;

const startDay = computed(() => {
    const d = planStore.start;
    return new Date(d.getFullYear(), d.getMonth(), d.getDate() - 1);
});
const endDay = computed(() => {
    const d = planStore.end;
    return new Date(d.getFullYear(), d.getMonth(), d.getDate() + 1);
});

function parseDate(d: string | Date | null | undefined): Date | null {
    if (d == null) return null;
    return typeof d === 'string' ? new Date(d) : d;
}

const days = computed(() => {
    const arr: { date: Date; label: number; x: number }[] = [];
    const d = new Date(startDay.value);
    const end = new Date(endDay.value);
    let i = 0;
    while (d <= end) {
        arr.push({ date: new Date(d), label: d.getDate(), x: i * dayWidth });
        d.setDate(d.getDate() + 1);
        i++;
    }
    return arr;
});

const months = computed(() => {
    const arr: { label: string; x: number; width: number }[] = [];
    const d = new Date(startDay.value);
    const end = new Date(endDay.value);
    let curMonth = d.getMonth();
    let curYear = d.getFullYear();
    let startIdx = 0;
    let i = 0;
    while (d <= end) {
        if (d.getMonth() !== curMonth || d.getFullYear() !== curYear) {
            arr.push({ label: `${curYear}-${String(curMonth + 1).padStart(2, '0')}`, x: startIdx * dayWidth, width: (i - startIdx) * dayWidth });
            curMonth = d.getMonth();
            curYear = d.getFullYear();
            startIdx = i;
        }
        d.setDate(d.getDate() + 1);
        i++;
    }
    arr.push({ label: `${curYear}-${String(curMonth + 1).padStart(2, '0')}`, x: startIdx * dayWidth, width: (i - startIdx) * dayWidth });
    return arr;
});

const timelineWidth = computed(() => days.value.length * dayWidth);
const scrollX = ref(0);
const scrollY = ref(0);
const isPanning = ref(false);
const panStartX = ref(0);
const panStartY = ref(0);
const panOrigX = ref(0);
const panOrigY = ref(0);
const scrollCell = ref<HTMLElement | null>(null);

function dateToX(date: string | Date | null | undefined): number {
    const d = parseDate(date);
    if (d == null) return 0;
    return (d.getTime() - startDay.value.getTime()) / (1000 * 60 * 60 * 24) * dayWidth;
}

function onPanStart(e: MouseEvent) {
    isPanning.value = true;
    panStartX.value = e.clientX;
    panStartY.value = e.clientY;
    panOrigX.value = scrollX.value;
    panOrigY.value = scrollY.value;
}
function onPanMove(e: MouseEvent) {
    onPanMoveX(e);
    onPanMoveY(e);
}
function onPanMoveX(e: MouseEvent) {
    if (!isPanning.value) return;
    const dx = e.clientX - panStartX.value;
    let newX = panOrigX.value - dx;
    if (scrollCell.value) {
        const rect = scrollCell.value.getBoundingClientRect();
        const visibleWidth = rect.width;
        newX = Math.max(0, Math.min(newX, timelineWidth.value - visibleWidth));
    }
    scrollX.value = newX;
}
function onPanMoveY(e: MouseEvent) {
    if (!isPanning.value) return;
    const dy = e.clientY - panStartY.value;
    let newY = panOrigY.value - dy;
    if (scrollCell.value) {
        const rect = scrollCell.value.getBoundingClientRect();
        let visibleHeight = rect.height;
        const viewportHeight = window.innerHeight;
        if (rect.bottom > viewportHeight) {
            visibleHeight -= (rect.bottom - viewportHeight);
        }
        newY = Math.max(0, Math.min(newY, chartHeight.value - visibleHeight));
    }
    scrollY.value = newY;
}
function onPanEnd() {
    isPanning.value = false;
}

const collapsedGroups = ref(new Set<number>());

function toggleGroup(id: number) {
    if (collapsedGroups.value.has(id)) collapsedGroups.value.delete(id);
    else collapsedGroups.value.add(id);
}

function onTaskClick(tid: number) {
    dialogStore.pushDialog(new TaskDialogData(tid));
}
function onNewTask() {
    dialogStore.pushDialog(new NewTaskDialogData());
}

// Build a flattened list of tasks grouped by parent groups, with depth
const rows = computed(() => {
    const tasks = taskStore.tasks.slice();
    // Build tree
    const roots = tasks.filter(t => t.parent == null).sort((a, b) => a.title.localeCompare(b.title));
    const result: { task: Task; depth: number }[] = [];
    function walk(t: Task, depth: number) {
        result.push({ task: t, depth });
        if (t.designation == TaskDesignation.Group && !collapsedGroups.value.has(t.dbId)) {
            const children = t.children.slice().sort((a, b) => a.title.localeCompare(b.title));
            for (const c of children) walk(c, depth + 1);
        }
    }
    for (const r of roots) walk(r, 0);
    return result;
});

const visibleRows = computed(() => rows.value);
const chartHeight = computed(() => visibleRows.value.length * rowHeight);


// Arrow path between predecessor and successor allocations (choose last end of pred, first start of succ)
function allocArrow(predId: number, succId: number): string {
    // get allocations or build pseudo allocations for milestones/requirements when missing
    const pred_allocs_raw = planStore.by_task(predId).slice().sort((a, b) => a.end.getTime() - b.end.getTime());
    const succ_allocs_raw = planStore.by_task(succId).slice().sort((a, b) => a.start.getTime() - b.start.getTime());

    const predTask = taskStore.task(predId);
    const succTask = taskStore.task(succId);

    const pred_allocs = pred_allocs_raw.length > 0 ? pred_allocs_raw : (predTask ? [
        // milestone fallback
        ...(predTask.designation == TaskDesignation.Milestone && predTask.scheduleTarget ? [{ dbId: -predId, start: predTask.scheduleTarget, end: predTask.scheduleTarget, task: predTask, resources: [] }] : []),
        // requirement fallback
        ...(predTask.designation == TaskDesignation.Requirement && predTask.earliestStart ? [{ dbId: -predId, start: predTask.earliestStart, end: predTask.earliestStart, task: predTask, resources: [] }] : []),
    ] : []);

    const succ_allocs = succ_allocs_raw.length > 0 ? succ_allocs_raw : (succTask ? [
        ...(succTask.designation == TaskDesignation.Milestone && succTask.scheduleTarget ? [{ dbId: -succId, start: succTask.scheduleTarget, end: succTask.scheduleTarget, task: succTask, resources: [] }] : []),
        ...(succTask.designation == TaskDesignation.Requirement && succTask.earliestStart ? [{ dbId: -succId, start: succTask.earliestStart, end: succTask.earliestStart, task: succTask, resources: [] }] : []),
    ] : []);

    if (pred_allocs.length == 0 || succ_allocs.length == 0) return "";
    const aPred = pred_allocs[pred_allocs.length - 1]!;
    const aSucc = succ_allocs[0]!;
    const x1 = dateToX(aPred.end);
    const y1 = (rowIndexForTask(predId) + 0.5) * rowHeight;
    const x2 = dateToX(aSucc.start);
    const y2 = (rowIndexForTask(succId) + 0.5) * rowHeight;
    // simple elbow path: horizontal from x1 to midX, vertical to y2, horizontal to x2
    const midX = x1 + Math.max(12, (x2 - x1) / 2);
    return `M ${x1} ${y1} L ${midX} ${y1} L ${midX} ${y2} L ${x2} ${y2}`;
}

function rowIndexForTask(tid: number) {
    for (let i = 0; i < visibleRows.value.length; i++) {
        const r = visibleRows.value[i];
        if (r && r.task.dbId == tid) return i;
    }
    return -1;
}

// Helpers for template: nothing extra required, imports are available in script setup

</script>

<style scoped>
/* reuse styles from PlanGanttResources.vue with slight adjustments */
.gantt-grid {
    display: grid;
    grid-template-columns: auto 1fr;
    grid-template-rows: auto 1fr;
    width: 100%;
    height: 100%;
    min-height: 0;
    overflow: hidden;
}

.gantt-corner,
.gantt-header,
.gantt-resources,
.gantt-chart-scroll {
    min-height: 0;
    line-height: 0;
}

.gantt-corner {
    grid-column: 1;
    grid-row: 1;
    background: #fff;
    border-bottom: 1px solid #ddd;
    border-right: 1px solid #ddd;
    width: var(--resource-col-width, 240px);
    height: var(--header-height, 50px);
}

.gantt-header {
    grid-column: 2;
    grid-row: 1;
    background: #fff;
    border-bottom: 1px solid #ddd;
    position: relative;
    overflow: hidden;
}

.gantt-header-scroll {
    position: absolute;
    left: 0;
    top: 0;
    will-change: transform;
    background: #fff;
}

.gantt-resources {
    grid-column: 1;
    grid-row: 2;
    background: #fff;
    border-right: 1px solid #ddd;
    z-index: 1;
}

.gantt-resource-row {
    display: flex;
    align-items: center;
    height: 40px;
    border-bottom: 1px solid #eee;
    padding-left: 8px;
    font-size: 15px;
    color: #333;
}

.gantt-chart-scroll {
    grid-column: 2;
    grid-row: 2;
    overflow: hidden;
    cursor: grab;
    background: #f3f4f5;
    position: relative;
}

.gantt-resources,
.gantt-header,
.gantt-chart-scroll {
    cursor: grab;
}

.gantt-resources:active,
.gantt-header:active,
.gantt-chart-scroll:active {
    cursor: grabbing;
}

.corner-buttons {
    display: flex;
    gap: 6px;
    padding: 8px;
    justify-content: center;
}
</style>
