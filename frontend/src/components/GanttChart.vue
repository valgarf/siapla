<template>
    <div class="gantt-grid">
        <div class="gantt-corner">
            <slot name="corner" />
        </div>

        <div class="gantt-header" @mousedown="onPanStart" @mousemove="onPanMoveX" @mouseup="onPanEnd"
            @mouseleave="onPanEnd">
            <div class="gantt-header-scroll"
                :style="{ width: timelineWidth + 'px', left: '0px', transform: `translate(${-scrollX}px, 0)` }">
                <svg :width="timelineWidth" :height="headerHeight">
                    <g>
                        <template v-for="(month, i) in months" :key="i">
                            <rect :x="month.x" y="0" :width="month.width" :height="monthRowHeight" fill="#fff"
                                stroke="#ccc" stroke-width="1" />
                            <foreignObject v-if="month.width > dayWidth * 4" :x="month.x + 4" :y="0"
                                :width="month.width - 8" :height="monthRowHeight">
                                <div class="svg-text-ellipsis svg-text-month" xmlns="http://www.w3.org/1999/xhtml">{{
                                    month.label }}</div>
                            </foreignObject>
                        </template>
                    </g>
                    <g>
                        <template v-for="(day, i) in days" :key="i">
                            <rect v-if="day.date.getDay() === 0 || day.date.getDay() === 6" :x="day.x"
                                :y="monthRowHeight" :width="dayWidth" :height="dayRowHeight" :fill="weekendColor"
                                stroke="#ccc" stroke-width="1" />
                            <rect v-else :x="day.x" :y="monthRowHeight" :width="dayWidth" :height="dayRowHeight"
                                fill="#fff" stroke="#ccc" stroke-width="1" />
                            <foreignObject :x="day.x + 2" :y="monthRowHeight" :width="dayWidth - 4"
                                :height="dayRowHeight">
                                <div class="svg-text-ellipsis svg-text-day" xmlns="http://www.w3.org/1999/xhtml">{{
                                    day.label }}</div>
                            </foreignObject>
                        </template>
                    </g>
                </svg>
            </div>
        </div>

        <!-- rows list (left) - rendered by parent via slot #row -->
        <div class="gantt-descriptions-list" :style="{
            height: chartHeight + 'px', width: descriptionColWidth + 'px', position: 'relative',
            overflow: 'hidden'
        }" @mousedown="onPanStart" @mousemove="onPanMoveY" @mouseup="onPanEnd" @mouseleave="onPanEnd">
            <div :style="{ position: 'absolute', top: -scrollY + 'px', left: 0, width: '100%' }">
                <div v-for="rw in visibleRows" :key="rw.row.id" class="gantt-row-description"
                    :style="{ height: rowHeight + 'px', paddingLeft: (8 + (rw.row.depth ?? 0) * 12) + 'px' }">
                    <q-btn v-if="rw.row.designation == TaskDesignation.Group" flat dense size="sm" class="clickable"
                        @click.stop="() => toggleGroup(rw.row.id)"
                        :icon="collapsedGroups.has(rw.row.id) ? 'chevron_right' : 'expand_more'"
                        :style="{ padding: '0px' }" />
                    <span :style="{ marginLeft: rw.row.designation != TaskDesignation.Group ? '17.15px' : '0px' }"
                        class="clickable row-name" @click.stop="emitRowClick(rw.row.id)">{{
                            rw.row.name
                        }}</span>
                    <span v-if="props.rowSymbols && props.rowSymbols.find(s => s.rowId === rw.row.id)"
                        class="row-symbol" :title="(props.rowSymbols.find(s => s.rowId === rw.row.id)?.title) || ''">
                        {{props.rowSymbols.find(s => s.rowId === rw.row.id)?.symbol}}
                    </span>
                </div>
            </div>
        </div>

        <div class="gantt-chart-scroll" ref="scrollCell" @mousedown="onPanStart" @mousemove="onPanMove"
            @mouseup="onPanEnd" @mouseleave="onPanEnd" style="grid-column: 2; grid-row: 2;">
            <svg :width="timelineWidth" :height="chartHeight"
                :style="{ transform: `translate(${-scrollX}px, ${-scrollY}px)` }">
                <defs>
                    <marker id="arrow" markerWidth="10" markerHeight="10" refX="10" refY="5" orient="auto"
                        markerUnits="strokeWidth">
                        <path d="M0,0 L10,5 L0,10 z" fill="#333" />
                    </marker>
                </defs>

                <!-- weekend background -->
                <g>
                    <template v-for="(day, i) in days" :key="'w'+i">
                        <rect v-if="day.date.getDay() === 0 || day.date.getDay() === 6" :x="day.x" y="0"
                            :width="dayWidth" :height="chartHeight" :fill="weekendColor" opacity="1" stroke="none" />
                    </template>
                </g>

                <!-- availability segments -->
                <g>
                    <template v-for="(rw, ri) in visibleRows" :key="'avail'+rw.row.id">
                        <template v-for="seg in availabilityForRow(rw.row.id)"
                            :key="rw.row.id + '-' + (seg.start as any) + '-' + (seg.end as any)">
                            <rect :x="dateToX(seg.start)" :y="ri * rowHeight"
                                :width="dateToX(seg.end) - dateToX(seg.start)" :height="rowHeight" fill="#fff"
                                opacity="0.7" stroke="none" />
                        </template>
                    </template>
                </g>

                <!-- row separators -->
                <g>
                    <template v-for="(row, i) in visibleRows" :key="i">
                        <line :x1="0" :y1="i * rowHeight" :x2="timelineWidth" :y2="i * rowHeight" stroke="#ddd"
                            stroke-width="1" />
                    </template>
                    <line :x1="0" :y1="rows.length * rowHeight" :x2="timelineWidth" :y2="rows.length * rowHeight"
                        stroke="#ddd" stroke-width="1" />
                </g>

                <!-- vertical day lines -->
                <g>
                    <template v-for="(day, i) in days" :key="i">
                        <line :x1="day.x" :y1="0" :x2="day.x" :y2="chartHeight" stroke="#ddd" stroke-width="1" />
                    </template>
                </g>

                <!-- Milestone indication lines -->
                <g>
                    <template v-for="(rw, i) in visibleRows" :key="'milestone-line-'+rw.row.id">
                        <template
                            v-if="rw.row.designation == TaskDesignation.Milestone && rw.row.scheduleTarget && rw.row.allocations && rw.row.allocations.length > 0">
                            <line :x1="dateToX(firstAllocStart(rw.row)!)" :y1="i * rowHeight + rowHeight / 2"
                                :x2="dateToX(rw.row.scheduleTarget)" :y2="i * rowHeight + rowHeight / 2"
                                :stroke="firstAllocStart(rw.row)! <= rw.row.scheduleTarget! ? '#66bb6a' : '#ef5350'"
                                stroke-width="3" />
                        </template>
                    </template>
                </g>

                <!-- dependencies -->
                <g stroke="#333" stroke-width="1.2" fill="none" marker-end="url(#arrow)">
                    <template v-for="(dep, i) in dependencies" :key="'dep'+i">
                        <path v-if="allocArrow(dep.predId, dep.succId)" :d="allocArrow(dep.predId, dep.succId)" />
                    </template>
                </g>

                <!-- allocations -->
                <template v-for="(rw, i) in visibleRows" :key="'row-'+rw.row.id">
                    <!-- groups -->
                    <template
                        v-if="rw.row.designation === TaskDesignation.Group && rw.row.allocations && rw.row.allocations.length > 0">
                        <template v-if="collapsedGroups.has(rw.row.id)">
                            <rect :x="dateToX(firstAllocStart(rw.row)!)" :y="i * rowHeight + barPadding"
                                :width="dateToX(lastAllocEnd(rw.row)!) - dateToX(firstAllocStart(rw.row)!)"
                                :height="barHeight" fill="#6a1b9a" stroke="#2c0b41" rx="3"
                                @click.stop="() => emitRowClick(rw.row.id)" class="clickable" />
                            <foreignObject :x="dateToX(firstAllocStart(rw.row)!) + 4" :y="i * rowHeight + barPadding"
                                :width="((dateToX(lastAllocEnd(rw.row)!) - dateToX(firstAllocStart(rw.row)!)) > 20) ? (dateToX(lastAllocEnd(rw.row)!) - dateToX(firstAllocStart(rw.row)!) - 8) : 20"
                                :height="barHeight">
                                <div class="svg-text-ellipsis svg-text-bar clickable"
                                    xmlns="http://www.w3.org/1999/xhtml" @click.stop="() => emitRowClick(rw.row.id)">{{
                                        rw.row.name }}</div>
                            </foreignObject>
                        </template>
                        <template v-else>
                            <polygon
                                :points="makeGroupBar(dateToX(firstAllocStart(rw.row)!), dateToX(lastAllocEnd(rw.row)!), i * rowHeight + rowHeight * 0.5)"
                                fill="black" @click.stop="() => emitRowClick(rw.row.id)" class="clickable" />
                        </template>
                    </template>
                    <!-- requirements -->
                    <template v-if="rw.row.designation === TaskDesignation.Requirement && rw.row.earliestStart">
                        <g :transform="`translate(${dateToX(rw.row.earliestStart)}, ${i * rowHeight + rowHeight / 2})`">
                            <circle r="6" fill="#ffb74d" stroke="#b06b00" @click.stop="() => emitRowClick(rw.row.id)"
                                class="clickable" />
                        </g>
                    </template>

                    <!-- milestones -->
                    <template v-if="rw.row.designation === TaskDesignation.Milestone && rw.row.scheduleTarget">
                        <g
                            :transform="`translate(${dateToX(rw.row.scheduleTarget)}, ${i * rowHeight + rowHeight / 2})`">
                            <rect :x="-6" y="-6" width="12" height="12" fill="#ffb74d" transform="rotate(45)"
                                stroke="#b06b00" @click.stop="() => emitRowClick(rw.row.id)" class="clickable" />
                        </g>
                    </template>
                    <template
                        v-if="rw.row.designation === TaskDesignation.Milestone && rw.row.allocations && rw.row.allocations.length > 0">
                        <g
                            :transform="`translate(${dateToX(firstAllocStart(rw.row)!)}, ${i * rowHeight + rowHeight / 2})`">
                            <rect x="-6" y="-6" width="12" height="12"
                                :fill="allocBeforeTarget(rw.row) === true ? '#66bb6a' : '#ef5350'"
                                :stroke="allocBeforeTarget(rw.row) === true ? '#3f8d43' : '#d21714'"
                                transform="rotate(45)" @click.stop="() => emitRowClick(rw.row.id)" class="clickable" />
                        </g>
                    </template>

                    <!-- tasks -->
                    <template v-if="rw.row.designation === TaskDesignation.Task && rw.row.allocations">
                        <template v-for="alloc in rw.row.allocations" :key="rw.row.id + '-alloc-' + alloc.dbId">
                            <rect :x="dateToX(alloc.start)" :y="i * rowHeight + barPadding"
                                :width="dateToX(alloc.end) - dateToX(alloc.start)" :height="barHeight" fill="#42a5f5"
                                stroke="#0a6fc2" rx="3"
                                @click.stop="() => emitAllocClick(rw.row.id, alloc.dbId, alloc.task?.dbId ?? null)"
                                class="clickable" />
                            <foreignObject :x="dateToX(alloc.start) + 4" :y="i * rowHeight + barPadding"
                                :width="((dateToX(alloc.end) - dateToX(alloc.start)) > 20) ? (dateToX(alloc.end) - dateToX(alloc.start) - 8) : 20"
                                :height="barHeight">
                                <div class="svg-text-ellipsis svg-text-alloc clickable"
                                    xmlns="http://www.w3.org/1999/xhtml"
                                    @click.stop="() => emitAllocClick(rw.row.id, alloc.dbId, alloc.task?.dbId ?? null)">
                                    {{ alloc.task?.title ?? '' }}</div>
                            </foreignObject>
                        </template>
                    </template>
                </template>

            </svg>
        </div>


    </div>
</template>

<script setup lang="ts">
import { TaskDesignation } from 'src/gql/graphql';
import { ref, computed } from 'vue'

type Allocation = { dbId: number; start: string | Date; end: string | Date; task?: { dbId?: number; title?: string } | null }
type Row = {
    id: number;
    name: string;
    designation?: TaskDesignation;
    allocations?: Allocation[];
    scheduleTarget?: string | Date | null;
    earliestStart?: string | Date | null;
    depth: number
}
type AvailabilitySegment = { start: string | Date; end: string | Date }
type Availability = { rowId: number; segments: AvailabilitySegment[] }
type Dependency = { predId: number; succId: number }
type RowWrapper = { visible: boolean, lastVisibleId: number, visibleIdx: number, idx: number, row: Row };

const props = defineProps<{ start: string | Date; end: string | Date; rows: Row[]; availability?: Availability[]; dependencies?: Dependency[]; rowHeight?: number; dayWidth?: number; barPadding?: number, rowSymbols?: { rowId: number; symbol: string; title?: string }[] }>()
const emit = defineEmits<{
    (e: 'alloc-click', data: { rowId: number | null, allocId: number | null, taskId: number | null }): void
    (e: 'row-click', id: number): void
}>()

const weekendColor = "#fff7ce"
const descriptionColWidth = computed(() => 240);
const rowHeight = computed(() => props.rowHeight ?? 36)
const dayWidth = computed(() => props.dayWidth ?? 24)
const barPadding = computed(() => props.barPadding ?? 8)
const barHeight = computed(() => rowHeight.value - barPadding.value * 2);
const monthRowHeight = computed(() => 28);
const dayRowHeight = computed(() => 22);
const headerHeight = computed(() => monthRowHeight.value + dayRowHeight.value);

function parseDate(d: string | Date) {
    return d instanceof Date ? new Date(d) : new Date(d)
}

const startDate = computed(() => {
    const d = parseDate(props.start)
    return new Date(d.getFullYear(), d.getMonth(), d.getDate() - 1);
})
const endDate = computed(() => {
    const d = parseDate(props.end)
    return new Date(d.getFullYear(), d.getMonth(), d.getDate() + 1);
})
const msPerDay = 24 * 60 * 60 * 1000

const days = computed(() => {
    const arr: { date: Date; x: number; label: string }[] = []
    const cur = new Date(startDate.value)
    let idx = 0
    while (cur <= endDate.value) {
        arr.push({ date: new Date(cur), x: idx * dayWidth.value, label: `${cur.getDate()}` })
        cur.setDate(cur.getDate() + 1)
        idx++
    }
    return arr
})

const timelineWidth = computed(() => days.value.length * dayWidth.value)

const months = computed(() => {
    const map = new Map<string, { x: number; width: number; label: string }>()
    days.value.forEach((d) => {
        const key = `${d.date.getFullYear()}-${d.date.getMonth()}`
        if (!map.has(key)) map.set(key, { x: d.x, width: dayWidth.value, label: `${d.date.toLocaleString(undefined, { month: 'short' })} ${d.date.getFullYear()}` })
        else map.get(key)!.width += dayWidth.value
    })
    return Array.from(map.values())
})

// internal collapsed groups state (moved from parent)
const collapsedGroups = ref(new Set<number>())

const rowMap = computed(() => {
    let idx = 0;
    let visibleIdx = 0;
    const rows = props.rows ?? []
    const out: Map<number, RowWrapper> = new Map()
    let lastCollapsed: RowWrapper | null = null;
    for (const r of rows) {
        const depth = r.depth ?? 0
        if (lastCollapsed != null && (lastCollapsed.row.depth ?? 0) >= depth) {
            // we left the collapsed group. From here on out everything is visible again
            lastCollapsed = null;
        }
        const wrapper: RowWrapper = {
            visible: lastCollapsed == null,
            lastVisibleId: lastCollapsed?.row.id ?? r.id,
            idx: idx,
            visibleIdx: visibleIdx,
            row: r
        };
        idx += 1;
        if (lastCollapsed == null) {
            visibleIdx += 1
        }
        out.set(r.id, wrapper);
        if (collapsedGroups.value.has(r.id)) {
            // we entered a collapsed group. All rows are visible until we leave the group
            lastCollapsed = wrapper;
        }

    }
    return out
})

const visibleRows = computed(() => [...rowMap.value.values()].filter((r) => r.visible));

const chartHeight = computed(() => (visibleRows.value.length ?? 0) * rowHeight.value)


// panning logic
const scrollX = ref(0)
const scrollY = ref(0)
const isPanning = ref(false)
let panStartX = 0
let panStartY = 0
let panOrigX = 0
let panOrigY = 0
const scrollCell = ref<HTMLElement | null>(null);

function onPanStart(e: MouseEvent) {
    isPanning.value = true
    panStartX = e.clientX
    panStartY = e.clientY
    panOrigX = scrollX.value
    panOrigY = scrollY.value
}
function onPanMove(e: MouseEvent) {
    onPanMoveX(e);
    onPanMoveY(e);
}
function onPanMoveX(e: MouseEvent) {
    if (!isPanning.value) return;
    const dx = e.clientX - panStartX;
    let newX = panOrigX - dx;
    if (scrollCell.value) {
        const rect = scrollCell.value.getBoundingClientRect();
        const visibleWidth = rect.width;
        newX = Math.max(0, Math.min(newX, timelineWidth.value - visibleWidth));
    }
    scrollX.value = newX;
}
function onPanMoveY(e: MouseEvent) {
    if (!isPanning.value) return;
    const dy = e.clientY - panStartY;
    let newY = panOrigY - dy;
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
    isPanning.value = false
}

function dateToX(d: string | Date | undefined) {
    if (!d) return 0
    const dt = parseDate(d)
    return (dt.getTime() - startDate.value.getTime()) / msPerDay * dayWidth.value
}

function fallbackTimestamp(row: Row): string | Date | null {
    if (row.designation == TaskDesignation.Requirement) {
        return row.earliestStart ?? null;
    }
    if (row.designation == TaskDesignation.Milestone) {
        return row.scheduleTarget ?? null;
    }
    return null;
}

function firstAllocStart(row: Row) {
    return row.allocations?.[0]?.start ?? fallbackTimestamp(row)
}
function lastAllocEnd(row: Row) {
    const allocs: Allocation[] = row.allocations ?? []
    return (allocs.length > 0 ? allocs[allocs.length - 1]!.end : fallbackTimestamp(row))
}
function allocBeforeTarget(row: Row) {
    const first = row.allocations?.[0]?.start
    if (!row.scheduleTarget || !first) return false
    return parseDate(first).getTime() <= parseDate(row.scheduleTarget).getTime()
}

function availabilityForRow(rowId: number) {
    const avail = props.availability ?? []
    const found = avail.find(a => a.rowId === rowId)
    return found ? found.segments : []
}

function allocArrow(predId: number, succId: number): string {
    // get allocations or build pseudo allocations for milestones/requirements when missing
    const predRw = rowMap.value.get(predId);
    const succRw = rowMap.value.get(succId);
    if (predRw == null || succRw == null) {
        return "";
    }
    const predAllocEnd = lastAllocEnd(predRw.row);
    const succAllocStart = firstAllocStart(succRw.row);

    if (predAllocEnd == null || succAllocStart == null) {
        return "";
    }

    const predCollapsedGroup = predRw?.lastVisibleId != predRw.row.id ? rowMap.value.get(predRw?.lastVisibleId) : null;
    const succCollapsedGroup = succRw?.lastVisibleId != succRw.row.id ? rowMap.value.get(succRw?.lastVisibleId) : null;
    if (predCollapsedGroup != null && predCollapsedGroup == succCollapsedGroup) {
        // both in the same collapsed group, nothing to draw
        return ""
    }
    if (predCollapsedGroup == null && succCollapsedGroup != null && succCollapsedGroup == predRw) {
        // both in the same collapsed group, nothing to draw
        return ""
    }
    if (predCollapsedGroup != null && succCollapsedGroup == null && predCollapsedGroup == succRw) {
        // both in the same collapsed group, nothing to draw
        return ""
    }
    // start and end positions
    const x1 = dateToX(predAllocEnd);
    const y1 = ((predCollapsedGroup?.visibleIdx ?? predRw.visibleIdx) + 0.5) * rowHeight.value;
    const x2 = dateToX(succAllocStart);
    const y2 = ((succCollapsedGroup?.visibleIdx ?? succRw.visibleIdx) + 0.5) * rowHeight.value;
    const start = [x1, y1];
    const coords = [];
    let lastX = x1;
    let targetX = x2;
    let targetY = y2;
    if (succCollapsedGroup != null) {
        // target is a collapsed group, draw arrow towards the side of the group
        targetY = y1 < y2 ? y2 - barHeight.value / 2 : y2 + barHeight.value / 2;
    }
    if (succRw.row.designation != null && [TaskDesignation.Milestone].includes(succRw.row.designation) && succCollapsedGroup == null) {
        // target is a milestone, leave a little more space to not collide with the milestone symbol
        targetX = x2 - 8;
    }

    if (predRw.row.designation != null && [TaskDesignation.Task, TaskDesignation.Group].includes(predRw.row.designation) && predCollapsedGroup == null) {
        // when starting from a normal task / group bar: move right for a very short line
        lastX = lastX + 5;
        coords.push([lastX, y1]);
    }

    if (succCollapsedGroup != null) {
        // target is a collapsed group, handle a vertical arrow
        if (lastX != targetX) {
            let y = y1
            if (targetX < lastX) {
                y = y1 < y2 ? y1 + 15 : y1 - 15;
            }
            coords.push([lastX, y]);
            lastX = targetX
            coords.push([lastX, y]);
        }
        coords.push([lastX, targetY]);
    }
    else {
        if (lastX > targetX - 15) {
            // last x position too large, we need to have a few extra coordinates to draw the line
            // to a smaller x value
            const y = y1 < y2 ? y2 - 15 : y2 + 15;
            coords.push([lastX, y]);
            lastX = targetX - 15
            coords.push([lastX, y]);
        }
        coords.push([lastX, targetY]);
        coords.push([targetX, targetY]);
    }
    return `M ${start[0]} ${start[1]}` + coords.map(c => `L ${c[0]} ${c[1]}`).join(' ');
}

function makeGroupBar(x1: number, x2: number, y: number) {
    const height = 8;
    const arrowWidth = 7;
    const arrowHeight = 7;

    const points = [];
    points.push([x1, y - height / 2]);
    points.push([x1, y + height / 2 + arrowHeight]);
    points.push([x1 + arrowWidth, y + height / 2]);
    points.push([x2 - arrowWidth, y + height / 2]);
    points.push([x2, y + height / 2 + arrowHeight]);
    points.push([x2, y - height / 2]);
    return points.map(p => `${p[0]},${p[1]}`).join(' ')
}

function emitAllocClick(rowId: number | null, allocId: number | null, taskId: number | null) {
    emit('alloc-click', { rowId, allocId, taskId })
}

function emitRowClick(id: number) {
    emit('row-click', id)
}

function toggleGroup(id: number) {
    if (collapsedGroups.value.has(id)) collapsedGroups.value.delete(id)
    else collapsedGroups.value.add(id)
}
</script>

<style scoped>
.gantt-corner,
.gantt-header,
.gantt-descriptions-list,
.gantt-chart-scroll {
    min-height: 0;
    line-height: 0;
}


.gantt-descriptions-list,
.gantt-header,
.gantt-chart-scroll {
    cursor: grab;
}

.gantt-descriptions-list:active,
.gantt-header:active,
.gantt-chart-scroll:active {
    cursor: grabbing;
}

.clickable {
    cursor: pointer;
}

.row-symbol {
    margin-left: 6px;
    color: #b58900;
    font-weight: bold;
}

.svg-text-ellipsis {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    display: block;
    box-sizing: border-box;
    height: 100%;
    line-height: 1;
    align-content: center;
}

.svg-text-month {
    font-size: 12px;
    color: #333;
}

.svg-text-day {
    font-size: 10px;
    color: #666;
}

.svg-text-bar,
.svg-text-alloc {
    font-size: 11px;
    color: #fff;
}

.row-name {
    text-overflow: ellipsis;
    white-space: nowrap;
    height: 100%;
    line-height: 1;
    overflow: hidden;
    align-content: center;
}

.gantt-header-and-chart {
    display: block;
}

.gantt-header {
    grid-column: 2;
    grid-row: 1;
    background: #fff;
    border-bottom: 1px solid #ddd;
    position: relative;
    overflow: hidden;
    height: 100%;
}

.gantt-header-scroll {
    position: absolute;
    left: 0;
    top: 0;
    will-change: transform;
    background: #fff;
}

.gantt-chart-scroll {
    overflow: hidden;
    cursor: grab;
    background: v-bind('(props.availability?.length ?? 0) > 0 ? "#f1f2f3" : "#fff"');
    position: relative;
}

.gantt-grid {
    display: grid;
    grid-template-columns: auto 1fr;
    grid-template-rows: v-bind('headerHeight + "px"') 1fr;
    width: 100%;
    height: 100%;
    gap: 0;
    min-height: 0;
    overflow: hidden;
}

.gantt-corner {
    grid-column: 1;
    grid-row: 1;
    background: #fff;
    border-bottom: 1px solid #ddd;
    border-right: 1px solid #ddd;
    width: v-bind(descriptionColWidth);
    height: v-bind(headerHeight);
}

.gantt-descriptions-list {
    grid-column: 1;
    grid-row: 2;
    background: #fff;
    border-right: 1px solid #ddd;
    z-index: 1;
}

.gantt-row-description {
    display: flex;
    align-items: center;
    padding-left: 8px;
    font-size: 12px;
    color: #333;
    border-bottom: 1px solid #f0f0f0;
}
</style>
