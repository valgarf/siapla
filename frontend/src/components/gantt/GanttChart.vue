<template>
    <div class="gantt-grid" :class="{ 'dragging': draggingState != null || isPanning }" @mousedown.stop.prevent>
        <div class="gantt-corner corner-buttons" style="display:flex;align-items:center;">
            <q-btn aria-label="Reset Zoom" flat @click.stop="resetZoom" icon="refresh">
                <q-tooltip>Reset Zoom</q-tooltip></q-btn>
            <slot name="corner" />
        </div>

        <div class="gantt-header" @mousedown.stop.prevent="onPanStart" @mousemove="onPanMoveX" @mouseup="onPanEnd"
            @mouseleave="onPanEnd" @wheel.prevent="onWheel">
            <div class="gantt-header-scroll"
                :style="{ width: timelineWidth + 'px', left: '0px', transform: `translate(${-scrollX}px, 0)` }">
                <svg :width="timelineWidth" :height="headerHeight">
                    <!-- header top row (year/month) -->
                    <g>
                        <template v-for="(seg, i) in topRowSegments" :key="i">
                            <rect :x="seg.x" y="0" :width="seg.width" :height="monthRowHeight" fill="#fff" stroke="#ccc"
                                stroke-width="1" />
                            <foreignObject v-if="seg.width > 20" :x="seg.x + 4" :y="0"
                                :width="Math.max(seg.width - 8, 0)" :height="monthRowHeight">
                                <div class="svg-text-ellipsis svg-text-month" xmlns="http://www.w3.org/1999/xhtml">{{
                                    seg.label }}</div>
                            </foreignObject>
                        </template>
                    </g>
                    <!-- header bottom row (day/week/hour) -->
                    <g>
                        <template v-for="(seg, i) in bottomRowSegments" :key="i">
                            <rect :x="seg.x" :y="monthRowHeight" :width="seg.width" :height="dayRowHeight" fill="#fff"
                                stroke="#ccc" stroke-width="1" />
                            <foreignObject :x="seg.x + 2" :y="monthRowHeight" :width="Math.max(seg.width - 4, 0)"
                                :height="dayRowHeight">
                                <div class="svg-text-ellipsis svg-text-day" xmlns="http://www.w3.org/1999/xhtml">{{
                                    seg.label }}</div>
                            </foreignObject>
                        </template>
                    </g>
                    <!-- current datetime indicator -->
                    <g>
                        <line v-if="now > startDate && now < endDate" :x1="dateToX(now)" :y1="0" :x2="dateToX(now)"
                            :y2="headerHeight" stroke="#d77b7b" stroke-width="2" />
                    </g>
                </svg>
            </div>
        </div>

        <!-- rows list (left) - rendered by parent via slot #row -->
        <div class="gantt-descriptions-list" :style="{
            height: chartHeight + 'px', width: descriptionColWidth + 'px', position: 'relative',
            overflow: 'hidden'
        }" @mousedown.stop.prevent="onPanStart" @mousemove="onPanMoveY" @mouseup="onPanEnd" @mouseleave="onPanEnd">
            <div :style="{ position: 'absolute', top: -scrollY + 'px', left: 0, width: '100%' }">
                <div v-for="rw in visibleRows" :key="rw.row.id" :class="{
                    'gantt-row-description': true,
                    'gantt-row-description-highlight': rowIsSelected(rw),
                    'gantt-row-description-selected': (selectionMode && rw.row.selected),
                    'gantt-row-description-not-selectable': selectionMode && rw.row.selectable === false,
                    'clickable': true
                }" :style="{ height: rowHeight + 'px', paddingLeft: (8 + (rw.row.depth ?? 0) * 12) + 'px' }"
                    @click.stop="selectionMode ? (rw.row.selectable ? emit('toggle-selection', rw.row.id) : undefined) : emitRowClick(rw.row.id)">
                    <q-btn v-if="rw.row.designation == TaskDesignation.Group" flat dense size="sm" class="clickable"
                        @click.stop="() => toggleGroup(rw.row.id)"
                        :icon="collapsedGroups.has(rw.row.id) ? 'chevron_right' : 'expand_more'"
                        :style="{ padding: '0px' }" />
                    <span :style="{ marginLeft: rw.row.designation != TaskDesignation.Group ? '17.15px' : '0px' }"
                        class="row-name">{{
                            rw.row.name
                        }}</span>
                    <span v-if="rw.row.symbol != null" class="row-symbol" :title="rw.row.symbol.title || ''">
                        {{ rw.row.symbol.symbolUTF8 }}
                    </span>
                </div>
            </div>
        </div>

        <div class="gantt-chart-scroll" ref="scrollCell" @mousedown.stop.prevent="onPanStart" @mousemove="onPanMove"
            @mouseup="onPanEnd" @mouseleave="onPanEnd" @wheel.prevent="onWheel" style="grid-column: 2; grid-row: 2;">
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
                        <rect v-if="day.date.getDay() === 0 || day.date.getDay() === 6" :x="dateToX(day.date)" y="0"
                            :width="dayWidthAtDate(day.date)" :height="chartHeight" :fill="weekendColor" opacity="1"
                            stroke="none" />
                    </template>
                </g>

                <!-- availability segments -->
                <g>
                    <template v-for="(rw, ri) in visibleRows" :key="'avail'+rw.row.id">
                        <template v-for="seg in rw.row.availability"
                            :key="rw.row.id + '-' + (seg.start as any) + '-' + (seg.end as any)">
                            <rect :x="dateToX(seg.start)" :y="ri * rowHeight"
                                :width="dateToX(seg.end) - dateToX(seg.start)" :height="rowHeight" fill="#fff"
                                opacity="0.7" stroke="none" />
                        </template>
                    </template>
                </g>


                <!-- vertical day lines -->
                <g>
                    <template v-for="(seg, i) in bottomRowSegments" :key="i">
                        <line :x1="seg.x" :y1="0" :x2="seg.x" :y2="chartHeight" stroke="#ddd" stroke-width="1" />
                    </template>

                </g>

                <!-- highlighted row-->
                <g>
                    <template v-for="(rw, ri) in visibleRows" :key="ri">
                        <rect v-if="rowIsSelected(rw)" :x="dateToX(startDate)" :y="ri * rowHeight"
                            :width="timelineWidth" :height="rowHeight" fill="#0074d330" stroke="none" />
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

                <!-- current datetime indicator -->
                <g>
                    <line v-if="now > startDate && now < endDate" :x1="dateToX(now)" :y1="0" :x2="dateToX(now)"
                        :y2="chartHeight" stroke="#d77b7b" stroke-width="2" />
                </g>


                <!-- Milestone indication lines -->
                <g>
                    <template v-for="(rw, i) in visibleRows" :key="'milestone-line-'+rw.row.id">
                        <template
                            v-if="rw.row.designation == TaskDesignation.Milestone && rw.row.scheduleTarget && rw.row.allocations && rw.row.allocations.length > 0">
                            <line :x1="dateToX(firstAllocStart(rw.row)!)" :y1="i * rowHeight + rowHeight / 2"
                                :x2="rowTimestampX(rw.row)" :y2="i * rowHeight + rowHeight / 2"
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
                                @click.stop="() => emitRowClick(rw.row.id)" class="clickable"
                                :class="{ 'selected-alloc': allocationInGroupIsSelected(rw) }" />
                            <!-- only render text when there's enough space -->
                            <template v-if="(dateToX(lastAllocEnd(rw.row)!) - dateToX(firstAllocStart(rw.row)!)) > 23">
                                <foreignObject :x="dateToX(firstAllocStart(rw.row)!) + 4"
                                    :y="i * rowHeight + barPadding"
                                    :width="(dateToX(lastAllocEnd(rw.row)!) - dateToX(firstAllocStart(rw.row)!) - 8)"
                                    :height="barHeight">
                                    <div class="svg-text-ellipsis svg-text-bar clickable"
                                        xmlns="http://www.w3.org/1999/xhtml"
                                        @click.stop="() => emitRowClick(rw.row.id)">{{ rw.row.name }}</div>
                                </foreignObject>
                            </template>
                        </template>
                        <template v-else>
                            <polygon
                                :points="makeGroupBar(dateToX(firstAllocStart(rw.row)!), dateToX(lastAllocEnd(rw.row)!), i * rowHeight + rowHeight * 0.5)"
                                fill="black" @click.stop="() => emitRowClick(rw.row.id)" class="clickable" />
                        </template>
                    </template>
                    <!-- requirements -->
                    <template v-if="rw.row.designation === TaskDesignation.Requirement && rw.row.earliestStart">
                        <g :transform="`translate(${rowTimestampX(rw.row)}, ${i * rowHeight + rowHeight / 2})`">
                            <circle r="6" fill="#ffb74d" stroke="#b06b00" @click.stop="() => emitRowClick(rw.row.id)"
                                class="clickable" />
                            <!-- drag handle for requirement (on top of symbol) -->
                            <g v-if="selectedRowIdsSet.has(rw.row.id)" class="drag-handle move"
                                @mousedown.stop.prevent="(e) => onAllocDragStart(rw.row.id, null, 'move', e)">
                                <circle r="6" fill="#ffffff00" />
                            </g>
                        </g>
                    </template>

                    <!-- milestones -->
                    <template v-if="rw.row.designation === TaskDesignation.Milestone && rw.row.scheduleTarget">
                        <g :transform="`translate(${rowTimestampX(rw.row)}, ${i * rowHeight + rowHeight / 2})`">
                            <rect :x="-6" y="-6" width="12" height="12" fill="#ffb74d" transform="rotate(45)"
                                stroke="#b06b00" @click.stop="() => emitRowClick(rw.row.id)" class="clickable" />
                            <!-- drag handle for milestone (on top of symbol) -->
                            <g v-if="selectedRowIdsSet.has(rw.row.id)" class="drag-handle move"
                                @mousedown.stop.prevent="(e) => onAllocDragStart(rw.row.id, null, 'move', e)">
                                <rect :x="-7" y="-7" width="14" height="14" fill="#ffffff00" transform="rotate(45)" />
                            </g>
                        </g>
                    </template>
                    <template
                        v-if="rw.row.designation === TaskDesignation.Milestone && rw.row.allocations && rw.row.allocations.length > 0">
                        <g
                            :transform="`translate(${dateToX(firstAllocStart(rw.row)!)}, ${i * rowHeight + rowHeight / 2})`">
                            <rect x="-7" y="-7" width="14" height="14"
                                :fill="allocBeforeTarget(rw.row) === true ? '#66bb6a' : '#ef5350'"
                                :stroke="allocBeforeTarget(rw.row) === true ? '#3f8d43' : '#d21714'"
                                transform="rotate(45)" @click.stop="() => emitRowClick(rw.row.id)" class="clickable" />
                        </g>
                    </template>

                    <!-- tasks -->
                    <template v-if="rw.row.designation === TaskDesignation.Task && rw.row.allocations">
                        <template v-for="alloc in rw.row.allocations" :key="rw.row.id + '-alloc-' + alloc.dbId">
                            <rect :x="allocStartX(alloc)" :y="i * rowHeight + barPadding"
                                :width="allocEndX(alloc) - allocStartX(alloc)" :height="barHeight" rx="3"
                                :fill="alloc.allocationType === AllocationType.Booking ? '#ffb74d' : '#42a5f5'"
                                :stroke="alloc.allocationType === AllocationType.Booking ? '#b06b00' : '#0a6fc2'"
                                @click.stop="() => emitAllocClick(rw.row.id, alloc.dbId, alloc.task?.dbId ?? null)"
                                class="clickable" :class="{ 'selected-alloc': allocationIsSelected(rw, alloc) }" />
                            <!-- only render allocation text if the bar is wide enough -->
                            <template v-if="(allocEndX(alloc) - allocStartX(alloc)) > 23">
                                <foreignObject :x="allocStartX(alloc) + 4" :y="i * rowHeight + barPadding"
                                    :width="(allocEndX(alloc) - allocStartX(alloc) - 8)" :height="barHeight">
                                    <div class="svg-text-ellipsis svg-text-alloc clickable"
                                        xmlns="http://www.w3.org/1999/xhtml"
                                        @click.stop="() => emitAllocClick(rw.row.id, alloc.dbId, alloc.task?.dbId ?? null)">
                                        {{ alloc.task?.title ?? '' }}</div>
                                </foreignObject>
                            </template>
                            <!-- handles and action buttons are rendered in the overlay group at the end of the SVG -->
                        </template>
                    </template>
                </template>

                <!-- overlay handles and action buttons rendered last so they appear above chart elements -->
                <g class="overlay-handles">
                    <template v-for="(rw) in visibleRows" :key="'overlay-'+rw.row.id">
                        <template v-for="(alloc, ai) in rw.row.allocations ?? []" :key="'ov-'+rw.row.id+'-'+alloc.dbId">
                            <g v-if="alloc.allocationType === AllocationType.Booking && (selectedAllocIdsSet.has(alloc.dbId) || (rw.row.designation === TaskDesignation.Task && selectedRowIdsSet.has(rw.row.id) && alloc.task?.dbId === rw.row.id))"
                                :transform="`translate(${allocStartX(alloc)}, ${visibleRows.findIndex(x => x.row.id === rw.row.id) * rowHeight + barPadding})`">
                                <!-- start handle -->
                                <g class="drag-handle start"
                                    @mousedown.stop.prevent="(e) => onAllocDragStart(rw.row.id, alloc.dbId, 'start', e)">
                                    <rect :x="-barHeight - 2" y="0" :width="barHeight" :height="barHeight"
                                        fill="#1e88e5" rx="2" />
                                    <text :x="-barHeight / 2 - 3" :y="barHeight / 2 + 3" font-size="10" fill="#fff"
                                        text-anchor="middle">◀</text>
                                </g>
                                <!-- move handle (center) -->
                                <g class="drag-handle move"
                                    @mousedown.stop.prevent="(e) => onAllocDragStart(rw.row.id, alloc.dbId, 'move', e)">
                                    <rect x="0" y="0" :width="allocEndX(alloc) - allocStartX(alloc)" :height="barHeight"
                                        fill="#ffffff00" />
                                </g>
                                <!-- end handle -->
                                <g class="drag-handle end"
                                    @mousedown.stop.prevent="(e) => onAllocDragStart(rw.row.id, alloc.dbId, 'end', e)"
                                    :transform="`translate(${allocEndX(alloc) - allocStartX(alloc)},0)`">
                                    <rect x="2" y="0" :width="barHeight" :height="barHeight" fill="#1e88e5" rx="2" />
                                    <text :x="barHeight / 2 + 3" :y="barHeight / 2 + 3" font-size="10" fill="#fff"
                                        text-anchor="middle">▶</text>
                                </g>
                            </g>

                            <!-- centered action buttons for selected task's bookings -->
                            <g v-if="alloc.allocationType === AllocationType.Booking && (selectedAllocIdsSet.has(alloc.dbId) || (rw.row.designation === TaskDesignation.Task && selectedRowIdsSet.has(rw.row.id) && alloc.task?.dbId === rw.row.id))"
                                :transform="`translate(${allocStartX(alloc) + (allocEndX(alloc) - allocStartX(alloc)) / 2}, ${visibleRows.findIndex(x => x.row.id === rw.row.id) * rowHeight + barPadding + barHeight + 6})`">
                                <!-- delete button -->
                                <g class="action-btn clickable" :transform="`translate(${-barHeight / 2 - 2},0)`"
                                    @click.stop.prevent="() => onDeleteBooking(rw.row.id, alloc.dbId)">
                                    <rect :x="-barHeight / 2" y="0" :width="barHeight" :height="barHeight" rx="3"
                                        fill="#e53935" />
                                    <text x="0" :y="4 + barHeight / 2" font-size="12" fill="#fff"
                                        text-anchor="middle">🗑</text>
                                </g>
                                <!-- split button -->
                                <g class="action-btn clickable" :transform="`translate(${barHeight / 2 + 2},0)`"
                                    @click.stop.prevent="() => onSplitBooking(rw.row.id, alloc.dbId)">
                                    <rect :x="-barHeight / 2" y="0" :width="barHeight" :height="barHeight" rx="3"
                                        fill="#fb8c00" />
                                    <text x="0" :y="4 + barHeight / 2" font-size="12" fill="#fff"
                                        text-anchor="middle">✂</text>
                                </g>
                            </g>
                            <!-- join button between this and previous booking -->
                            <g v-if="prevAlloc(rw, ai) && (selectedAllocIdsSet.has(alloc.dbId) || (rw.row.designation === TaskDesignation.Task && selectedRowIdsSet.has(rw.row.id) && alloc.task?.dbId === rw.row.id)) && alloc.allocationType === AllocationType.Booking && prevAlloc(rw, ai)!.allocationType === AllocationType.Booking"
                                :transform="`translate(${allocEndX(prevAlloc(rw, ai)!) + (allocStartX(alloc) - allocEndX(prevAlloc(rw, ai)!)) / 2}, ${visibleRows.findIndex(x => x.row.id === rw.row.id) * rowHeight + barPadding + barHeight + 6})`">
                                <g class="action-btn clickable" @click.stop.prevent="() => onJoinBookings(rw.row.id, prevAlloc(rw, ai)!.dbId,
                                    alloc.dbId)">
                                    <rect :x="-barHeight / 2" y="0" :width="barHeight" :height="barHeight" rx="3"
                                        fill="#4caf50" />
                                    <text x="0" :y="4 + barHeight / 2" font-size="12" fill="#fff"
                                        text-anchor="middle">🔗</text>
                                </g>
                            </g>
                        </template>
                    </template>
                </g>

            </svg>
        </div>


    </div>
</template>

<script setup lang="ts">
import { AllocationType, TaskDesignation } from 'src/gql/graphql';
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { scrollX, scrollYMap, panInitialized, zoomX, collapsedGroupsMap } from './ganttShared'
import { nextTick } from 'process';
import { formatDate } from 'src/common/datetime';

export type Allocation = { dbId: number; start: string | Date; end: string | Date; task?: { dbId?: number; title?: string } | null; allocationType: AllocationType | null; final?: boolean }
export type Row = {
    id: number;
    name: string;
    designation?: TaskDesignation;
    allocations?: Allocation[];
    scheduleTarget?: string | Date | null;
    earliestStart?: string | Date | null;
    symbol?: { symbolUTF8: string; title?: string } | undefined | null
    availability: Availability[]
    depth: number
    selectable?: boolean
    selected?: boolean
}
export type Availability = { start: string | Date; end: string | Date }
export type Dependency = { predId: number; succId: number }
type RowWrapper = { visible: boolean, lastVisibleId: number, visibleIdx: number, idx: number, row: Row };
interface Props {
    start: string | Date
    end: string | Date
    rows: Row[]
    hasAvailability?: boolean,
    dependencies?: Dependency[]
    rowHeight?: number
    dayWidth?: number
    barPadding?: number
    // ids of rows that should be highlighted
    selectedRowIds?: number[]
    // ids of allocations that should be highlighted
    selectedAllocIds?: number[]
    // key used to store state information for, e.g. scrolling or collapsed status
    dataKey: string
    // whether gantt is in selection mode; when true rows may have `selectable` and `selected`
    selectionMode?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{
    (e: 'alloc-click', data: { rowId: number | null, allocId: number | null, taskId: number | null }): void
    (e: 'row-click', id: number): void
    (e: 'toggle-selection', id: number): void
    (e: 'alloc-drag-end', data: { rowId: number, allocId: number | null, start: Date, end: Date }): void
    (e: 'delete-booking', data: { rowId: number | null, allocId: number | null }): void
    (e: 'split-booking', data: { rowId: number | null, allocId: number | null, zoom: number }): void
    (e: 'join-bookings', data: { rowId: number | null, leftAllocId: number, rightAllocId: number }): void
}>()

const weekendColor = "#fff7ce"
const descriptionColWidth = computed(() => 240);
const rowHeight = computed(() => props.rowHeight ?? 36)
const dayWidth = computed(() => (props.dayWidth ?? 24) * zoomX.value)

const barPadding = computed(() => props.barPadding ?? 8)
const barHeight = computed(() => rowHeight.value - barPadding.value * 2);
const monthRowHeight = computed(() => 28);
const dayRowHeight = computed(() => 22);
const headerHeight = computed(() => monthRowHeight.value + dayRowHeight.value);
const now = ref<Date>(new Date(Date.now()));

// let _activeDrag: { rowId: number | null; allocId: number | null; edge: 'start' | 'end' | 'move'; moveListener: (e: MouseEvent) => void; upListener: (e: MouseEvent) => void } | null = null;

// internal drag state and overwrites
const dragOverwrites = ref(new Map<number, { start: Date; end: Date }>());
// row-level single-date overwrites (for requirements / milestones)
const dragRowOverwrites = ref(new Map<number, Date>());
const draggingState = ref<{ rowId: number | null; allocId: number | null; edge: 'start' | 'end' | 'move'; origStart?: Date | undefined; origEnd?: Date | undefined; grabOffsetMs?: number | undefined, moveListener: (e: MouseEvent) => void; upListener: (e: MouseEvent) => void } | null>(null);

function clientXToDate(clientX: number): Date {
    const rect = scrollCell.value?.getBoundingClientRect();
    const offset = rect ? clientX - rect.left : clientX;
    const x = scrollX.value + offset;
    const ms = startDate.value.getTime() + x / dayWidth.value * msPerDay;
    return new Date(ms);
}

function parseDate(d: string | Date) {
    return d instanceof Date ? d : new Date(d)
}

const startDate = computed(() => {
    const d = parseDate(props.start)
    return new Date(d.getFullYear(), d.getMonth(), d.getDate() - 7);
})
const endDate = computed(() => {
    const d = parseDate(props.end)
    return new Date(d.getFullYear(), d.getMonth(), d.getDate() + 7);
})
const msPerDay = 24 * 60 * 60 * 1000

// helper: return the next calendar day (preserving local date semantics)
function nextDayDate(d: Date | string) {
    const dt = parseDate(d)
    const nd = new Date(dt)
    nd.setDate(nd.getDate() + 1)
    return nd
}

function dayWidthAtDate(d: Date | string) {
    const dt = parseDate(d)
    const nd = nextDayDate(dt)
    return dateToX(nd) - dateToX(dt)
}

const days = computed(() => {
    const arr: { date: Date; x: number; label: string, labelFull: string }[] = []
    const cur = new Date(startDate.value)
    // determine visible bounds in pixels (include 1000px margin outside)
    const rect = scrollCell.value?.getBoundingClientRect()
    const visibleWidth = rect ? rect.width : (window?.innerWidth ?? 0)
    const leftBound = Math.max(0, scrollX.value - 4000)
    const rightBound = scrollX.value + visibleWidth + 4000
    while (cur <= endDate.value) {
        const x = dateToX(cur)
        if (x >= leftBound && x <= rightBound) {
            arr.push({ date: new Date(cur), x, label: `${cur.getDate()}`, labelFull: formatDate(cur) })
        }
        cur.setDate(cur.getDate() + 1)
    }
    return arr
})

const timelineWidth = computed(() => {
    // width up to (endDate + 1 day) so the last day column is included
    const endNext = nextDayDate(endDate.value)
    return dateToX(endNext)
})

// helper to group segments for header rows depending on zoom
function getWeekNumber(dt: Date) {
    const d = new Date(Date.UTC(dt.getFullYear(), dt.getMonth(), dt.getDate()));
    const dayNum = d.getUTCDay() || 7;
    d.setUTCDate(d.getUTCDate() + 4 - dayNum);
    const yearStart = new Date(Date.UTC(d.getUTCFullYear(), 0, 1));
    return Math.ceil((((d.getTime() - yearStart.getTime()) / 86400000) + 1) / 7);
}

const topRowSegments = computed(() => {
    // zoom > 5 => show days on top row
    if (zoomX.value > 5) {
        return days.value.map(d => ({ x: d.x, width: dayWidthAtDate(d.date), label: d.labelFull }))
    }
    // zoom < 0.25 => group by year
    if (zoomX.value < 0.25) {
        const map = new Map<number, { x: number; width: number; label: string }>()
        days.value.forEach(d => {
            const y = d.date.getFullYear()
            if (!map.has(y)) map.set(y, { x: d.x, width: dayWidthAtDate(d.date), label: `${y}` })
            else map.get(y)!.width += dayWidthAtDate(d.date)
        })
        return Array.from(map.values())
    }
    // default: months
    const map = new Map<string, { x: number; width: number; label: string }>()
    days.value.forEach((d) => {
        const key = `${d.date.getFullYear()}-${d.date.getMonth()}`
        if (!map.has(key)) map.set(key, { x: d.x, width: dayWidthAtDate(d.date), label: `${d.date.toLocaleString(undefined, { month: 'short' })} ${d.date.getFullYear()}` })
        else map.get(key)!.width += dayWidthAtDate(d.date)
    })
    return Array.from(map.values())
})

const zoomTimeResMs = computed(() => {
    if (zoomX.value > 20) {
        // 1/4 hour
        return 3600 * 1000 / 4
    }
    if (zoomX.value > 5) {
        // 1/2 hour
        return 3600 * 1000 / 2
    }
    if (zoomX.value > 0.6) {
        // 1 hour
        return 3600 * 1000
    }
    // 1 day
    return 24 * 3600 * 1000
})

function roundToZoomRes(d: Date): Date {
    const res = zoomTimeResMs.value
    const result = new Date(Math.round((d.getTime() - startDate.value.getTime()) / res) * res + startDate.value.getTime());
    if (res >= 24 * 3600 * 1000) {
        if (d.getHours() > 12) {
            return new Date(d.getFullYear(), d.getMonth(), d.getDate() + 1);
        }
        else {
            return new Date(d.getFullYear(), d.getMonth(), d.getDate());
        }
    }
    return result
}

const bottomRowSegments = computed(() => {
    // zoom < 0.25 => months
    if (zoomX.value < 0.25) {
        const map = new Map<string, { x: number; width: number; label: string }>()
        days.value.forEach(d => {
            const key = `${d.date.getFullYear()}-${d.date.getMonth()}`
            if (!map.has(key)) map.set(key, { x: d.x, width: dayWidthAtDate(d.date), label: `${d.date.toLocaleString(undefined, { month: 'short' })}` })
            else map.get(key)!.width += dayWidthAtDate(d.date)
        })
        return Array.from(map.values())
    }
    // zoom > 5 => hours
    if (zoomX.value > 5) {
        const step = zoomX.value < 20 ? 3 : 1;
        const segs: { x: number; width: number; label: string }[] = []
        days.value.forEach(d => {
            for (let h = 0; h < 24; h += step) {
                const dt = new Date(d.date)
                dt.setHours(h, 0, 0, 0)
                const x = dateToX(dt)
                const width = dayWidthAtDate(d.date) / 24 * step
                const label = `${h}`
                segs.push({ x, width, label })
            }
        })
        return segs
    }
    // zoom < 0.6 => weeks
    if (zoomX.value < 0.6) {
        const map = new Map<string, { x: number; width: number; label: string }>()
        days.value.forEach(d => {
            const wk = `${d.date.getFullYear()}-${getWeekNumber(d.date)}`
            if (!map.has(wk)) map.set(wk, { x: d.x, width: dayWidthAtDate(d.date), label: `W${getWeekNumber(d.date)}` })
            else map.get(wk)!.width += dayWidthAtDate(d.date)
        })
        return Array.from(map.values())
    }
    // default: days
    return days.value.map(d => ({ x: d.x, width: dayWidthAtDate(d.date), label: d.label }))
})

const selectedRowIdsSet = computed(() => new Set((props.selectedRowIds) ?? []));
const selectedAllocIdsSet = computed(() => new Set((props.selectedAllocIds) ?? []));

function* iterateHidden(rw: RowWrapper): IterableIterator<RowWrapper> {
    if (collapsedGroups.value.has(rw.row.id) && rw.row.designation == TaskDesignation.Group) {
        let idx = rw.idx + 1
        let value = rowMap.value.get(props.rows[idx]?.id ?? -1);
        while (value != null && !value.visible) {
            yield value
            idx += 1
            value = rowMap.value.get(props.rows[idx]?.id ?? -1);
        }
    }
}

function rowIsSelected(rw: RowWrapper): boolean {
    if (selectedRowIdsSet.value.has(rw.row.id)) {
        return true
    }
    for (const hiddenRw of iterateHidden(rw)) {
        if (selectedRowIdsSet.value.has(hiddenRw.row.id)) {
            return true
        }
    }
    return false
}

function allocationInGroupIsSelected(rw: RowWrapper): boolean {
    for (const hiddenRw of iterateHidden(rw)) {
        const alloc_ids = new Set(hiddenRw.row.allocations?.map(a => a.dbId) ?? [])
        if (selectedAllocIdsSet.value.intersection(alloc_ids).size > 0) {
            return true
        }
    }
    return false
}

function allocationIsSelected(rw: RowWrapper, alloc: Allocation): boolean {
    return selectedAllocIdsSet.value.has(alloc.dbId)
}

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
        if (collapsedGroups.value.has(r.id) && r.designation == TaskDesignation.Group) {
            // we entered a collapsed group. All rows are visible until we leave the group
            lastCollapsed = wrapper;
        }

    }
    return out
})

const visibleRows = computed(() => [...rowMap.value.values()].filter((r) => r.visible));

const chartHeight = computed(() => (visibleRows.value.length ?? 0) * rowHeight.value)

// when external data arrives after a drag ended, clear the overwrite and only keep the overwrite
// for any active drag
watch(() => props.rows, () => {
    const allocMap: Map<number, { start: Date, end: Date }> = new Map();
    const rowMap: Map<number, Date> = new Map();
    if (draggingState.value?.allocId != null) {
        const allocOver = dragOverwrites.value.get(draggingState.value.allocId);
        if (allocOver) {
            allocMap.set(draggingState.value.allocId, allocOver);
        }
    }
    else if (draggingState.value?.rowId != null) {
        const rowOver = dragRowOverwrites.value.get(draggingState.value.rowId);
        if (rowOver) {
            rowMap.set(draggingState.value.rowId, rowOver);
        }
    }
    dragRowOverwrites.value = rowMap;
    dragOverwrites.value = allocMap;
}, { deep: true })


// panning logic

const scrollY = computed({
    // getter
    get(): number {
        let result = scrollYMap.value[props.dataKey]
        if (result == null) {
            result = 0
            scrollYMap.value[props.dataKey] = result
        }
        return result
    },
    // setter
    set(newValue) {
        scrollYMap.value[props.dataKey] = newValue
    }
})
// internal collapsed groups state
const collapsedGroups = computed({
    // getter
    get(): Set<number> {
        let result = collapsedGroupsMap.value[props.dataKey]
        if (result == null) {
            result = new Set<number>()
            collapsedGroupsMap.value[props.dataKey] = result
        }
        return result
    },
    // setter
    set(newValue) {
        collapsedGroupsMap.value[props.dataKey] = newValue
    }
})

// const collapsedGroups = ref(new Set<number>())

const isPanning = ref(false)
let panStartX = 0
let panStartY = 0
let panOrigX = 0
let panOrigY = 0
const scrollCell = ref<HTMLElement | null>(null);

function onPanStart(e: MouseEvent) {
    panInitialized.value = true
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

function initPan() {
    if (scrollCell.value && !panInitialized.value) {
        const rect = scrollCell.value.getBoundingClientRect();
        const visibleWidth = rect.width;
        const xnow = dateToX(now.value);
        _assignClampedScrollX(xnow - visibleWidth / 2);
    }
    autoClampScroll()
}

watch([timelineWidth, panInitialized, scrollCell], initPan)

function _assignClampedScrollX(value: number) {
    // make sure we cannot scroll outside of the visible range
    if (scrollCell.value) {
        const rect = scrollCell.value.getBoundingClientRect();
        const visibleWidth = rect.width;
        scrollX.value = Math.max(0, Math.min(value, Math.max(0, timelineWidth.value - visibleWidth)));
    }
}

function _assignClampedScrollY(value: number) {
    // make sure we cannot scroll outside of the visible range
    if (scrollCell.value) {
        const rect = scrollCell.value.getBoundingClientRect();
        let visibleHeight = rect.height;
        const viewportHeight = window.innerHeight;
        if (rect.bottom > viewportHeight) {
            visibleHeight -= (rect.bottom - viewportHeight);
        }
        scrollY.value = Math.max(0, Math.min(value, Math.max(0, chartHeight.value - visibleHeight)));
    }

}

function onPanMoveX(e: MouseEvent) {
    if (!isPanning.value) return;
    const dx = e.clientX - panStartX;
    const newX = panOrigX - dx;
    _assignClampedScrollX(newX);
}
function onPanMoveY(e: MouseEvent) {
    if (!isPanning.value) return;
    const dy = e.clientY - panStartY;
    const newY = panOrigY - dy;
    _assignClampedScrollY(newY)
}
function onPanEnd() {
    isPanning.value = false
}

let clampingInterval: NodeJS.Timeout | null = null

function autoClampScroll() {
    _assignClampedScrollX(scrollX.value)
    _assignClampedScrollY(scrollY.value)
}

function startAutoClamping() {
    clampingInterval = setInterval(autoClampScroll, 5)
}

function stopAutoClamping() {
    if (clampingInterval != null) {
        clearInterval(clampingInterval)
    }
    autoClampScroll()
}

onMounted(() => {
    window.addEventListener('sidebarClosing', startAutoClamping);
    window.addEventListener('sidebarClosed', stopAutoClamping);
    window.addEventListener('resize', autoClampScroll);
    nextTick(() => {
        if (panInitialized.value) {
            autoClampScroll()
        }
    })
});
onUnmounted(() => {
    window.removeEventListener('sidebarClosing', startAutoClamping);
    window.removeEventListener('sidebarClosed', stopAutoClamping);
    window.removeEventListener('resize', autoClampScroll);
});

function dateToX(d: string | Date | undefined) {
    if (!d) return 0
    const dt = parseDate(d)
    return (dt.getTime() - startDate.value.getTime()) / msPerDay * dayWidth.value
}

function fallbackTimestamp(row: Row): string | Date | null {
    if (row.designation && [TaskDesignation.Requirement, TaskDesignation.Milestone].includes(row.designation)) {
        const ro = dragRowOverwrites.value.get(row.id)
        if (ro) return ro
    }
    if (row.designation == TaskDesignation.Requirement) {
        return row.earliestStart ?? null;
    }
    if (row.designation == TaskDesignation.Milestone) {
        return row.scheduleTarget ?? null;
    }
    return null;
}

function firstAllocStart(row: Row) {
    const alloc = row.allocations?.[0]
    if (alloc) {
        const over = dragOverwrites.value.get(alloc.dbId)
        return over ? over.start : parseDate(alloc.start)
    }
    return fallbackTimestamp(row)
}
function lastAllocEnd(row: Row) {
    const allocs: Allocation[] = row.allocations ?? []
    if (allocs.length > 0) {
        const last = allocs[allocs.length - 1]!
        const over = dragOverwrites.value.get(last.dbId)
        return over ? over.end : parseDate(last.end)
    }
    return fallbackTimestamp(row)
}

// helpers that return pixel positions for allocations, preferring drag overwrites when present
function allocStartX(alloc: Allocation) {
    const over = dragOverwrites.value.get(alloc.dbId)
    const d = over ? over.start : parseDate(alloc.start)
    return dateToX(d)
}
function allocEndX(alloc: Allocation) {
    const over = dragOverwrites.value.get(alloc.dbId)
    const d = over ? over.end : parseDate(alloc.end)
    return dateToX(d)
}
function rowTimestampX(row: Row) {
    // prefer row-level overwrite when dragging requirements/milestones
    return dateToX(fallbackTimestamp(row) ?? undefined)
}
function allocBeforeTarget(row: Row) {
    const first = row.allocations?.[0]?.start
    const schedule = fallbackTimestamp(row)
    if (!schedule || !first) return false
    return parseDate(first).getTime() <= parseDate(schedule).getTime()
}

// wheel zoom handler: zoom along x axis only
function onWheel(e: WheelEvent) {
    // only act when over timeline areas (handler attached there)
    e.preventDefault()
    const target = e.currentTarget as HTMLElement
    const rect = target.getBoundingClientRect()
    const offsetX = e.clientX - rect.left // mouse cursor offset to left border
    const oldZoom = zoomX.value
    const delta = e.deltaY
    // scale factor: exponential for smooth zoom
    const scaleFactor = Math.exp(-delta * 0.001)
    const newZoom = Math.max(0.1, Math.min(30, oldZoom * scaleFactor))
    if (newZoom === oldZoom) return
    const ratio = newZoom / oldZoom
    // keep the date under cursor fixed by adjusting scrollX
    const newScroll = (scrollX.value + offsetX) * ratio - offsetX
    zoomX.value = newZoom
    _assignClampedScrollX(newScroll)
}

function resetZoom() {
    const old = zoomX.value
    zoomX.value = 1
    // adjust scroll to keep center focused similarly
    if (scrollCell.value) {
        const rect = scrollCell.value.getBoundingClientRect()
        const center = rect.width / 2
        const newScroll = (scrollX.value + center) * (1 / old) - center
        _assignClampedScrollX(newScroll)
    } else {
        _assignClampedScrollX(scrollX.value)
    }
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

// TODO: used for multiple different cases (requirements, milestones, groups)
// -> split into different functions, find better name for general purpose event
function emitRowClick(id: number) {
    emit('row-click', id)
}

function prevAlloc(rw: RowWrapper, ai: number): Allocation | null {
    if (!rw.row.allocations) return null
    return rw.row.allocations[ai - 1] ?? null
}

function onAllocDragStart(rowId: number | null, allocId: number | null, edge: 'start' | 'end' | 'move', e: MouseEvent) {
    if (!rowId) return;
    // clear previous listeners/state
    if (draggingState.value != null) {
        document.removeEventListener('mousemove', draggingState.value.moveListener);
        document.removeEventListener('mouseup', draggingState.value.upListener);
        draggingState.value = null;
    }

    // locate original allocation bounds when available
    let origStart: Date | undefined = undefined;
    let origEnd: Date | undefined = undefined;
    if (allocId != null) {
        for (const rw of props.rows ?? []) {
            const found = (rw.allocations ?? []).find(a => a.dbId === allocId);
            if (found) {
                origStart = (found.start instanceof Date) ? found.start : new Date(found.start);
                origEnd = (found.end instanceof Date) ? found.end : new Date(found.end);
                break;
            }
        }
    }
    else {
        const row = props.rows.find(r => r.id == rowId);
        if (row?.designation === TaskDesignation.Requirement) {
            origStart = row.earliestStart ? new Date(row.earliestStart) : undefined
            origEnd = origStart
        }
        if (row?.designation === TaskDesignation.Milestone) {
            origStart = row.scheduleTarget ? new Date(row.scheduleTarget) : undefined
            origEnd = origStart
        }
    }

    const initDate = clientXToDate(e.clientX);

    const moveListener = (ev: MouseEvent) => {
        if (!draggingState.value) return;
        const date = clientXToDate(ev.clientX);
        const aId = draggingState.value.allocId;
        const s = draggingState.value.origStart ?? new Date();
        const t = draggingState.value.origEnd ?? new Date();
        let newS = s;
        let newE = t;
        if (aId == null) {
            // row-level single date (requirement/milestone)
            const rowId = draggingState.value.rowId;
            if (rowId != null) {
                dragRowOverwrites.value.set(rowId, roundToZoomRes(date));
            }
        } else {
            const offset = draggingState.value.grabOffsetMs ?? 0;
            if (draggingState.value.edge === 'start') {
                newS = roundToZoomRes(new Date(date.getTime() - offset))
            } else if (draggingState.value.edge === 'end') {
                newE = roundToZoomRes(new Date(date.getTime() - offset))
            } else {
                const duration = t.getTime() - s.getTime()
                newS = roundToZoomRes(new Date(date.getTime() - offset))
                newE = new Date(newS.getTime() + duration)
            }
            dragOverwrites.value.set(aId, { start: newS, end: newE });
        }
    };

    const upListener = () => {
        if (!draggingState.value) {
            document.removeEventListener('mousemove', moveListener);
            document.removeEventListener('mouseup', upListener);
            return;
        }
        const rowId = draggingState.value.rowId;
        const aId = draggingState.value.allocId;
        let finalStart: Date | undefined = undefined;
        let finalEnd: Date | undefined = undefined;
        if (aId != null) {
            const over = dragOverwrites.value.get(aId);
            if (over) {
                finalStart = over.start;
                finalEnd = over.end;
            }
        } else if (rowId != null) {
            const over = dragRowOverwrites.value.get(rowId);
            if (over) {
                finalStart = over;
                finalEnd = over;
            }
        }
        draggingState.value = null;
        document.removeEventListener('mousemove', moveListener);
        document.removeEventListener('mouseup', upListener);
        if (rowId != null && finalStart != null && finalEnd != null) {
            emit('alloc-drag-end', { rowId: rowId, allocId: aId, start: finalStart, end: finalEnd });
        }
    };

    const newDraggingState = { rowId, allocId, edge, origStart, origEnd, grabOffsetMs: 0, moveListener, upListener };
    if ((edge === 'move' || edge === 'start') && origStart) {
        const ds = newDraggingState;
        ds.grabOffsetMs = initDate.getTime() - origStart.getTime();
    }
    else if (edge === 'end' && origEnd) {
        const ds = newDraggingState;
        ds.grabOffsetMs = initDate.getTime() - origEnd.getTime();
    }

    if (allocId != null && origStart && origEnd) {
        dragOverwrites.value.set(allocId, { start: origStart, end: origEnd });
    }
    else if (rowId != null && origStart) {
        dragRowOverwrites.value.set(rowId, origStart);

    }

    draggingState.value = newDraggingState
    document.addEventListener('mousemove', moveListener);
    document.addEventListener('mouseup', upListener);
}

function onDeleteBooking(rowId: number | null, allocId: number | null) {
    emit('delete-booking', { rowId, allocId })
}

function onSplitBooking(rowId: number | null, allocId: number | null) {
    emit('split-booking', { rowId, allocId, zoom: zoomX.value })
}

function onJoinBookings(rowId: number | null, leftAllocId: number, rightAllocId: number) {
    emit('join-bookings', { rowId, leftAllocId, rightAllocId })
}

function toggleGroup(id: number) {
    const newSet = new Set(collapsedGroups.value)
    if (newSet.has(id)) newSet.delete(id)
    else newSet.add(id)
    collapsedGroups.value = newSet
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

.dragging>.gantt-descriptions-list,
.dragging>.gantt-header,
.dragging>.gantt-chart-scroll {
    cursor: grabbing;
}

.gantt-grid.dragging {
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

/* prevent text selection inside the gantt chart */
.svg-text-ellipsis,
.svg-text-month,
.svg-text-day,
.svg-text-bar,
.svg-text-alloc,
.row-name,
.row-symbol {
    user-select: none;
    -webkit-user-select: none;
    -ms-user-select: none;
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
    background: v-bind('props.hasAvailability ? "#f1f2f3" : "#fff"');
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
    border-top: 0.5px solid #f0f0f0;
    border-bottom: 0.5px solid #f0f0f0;
}

/* visual highlight for selected rows and allocations */
.selected-row {
    filter: drop-shadow(0 0 6px rgba(66, 165, 245, 0.7));
}

.gantt-row-description-highlight {
    background-color: #0074d330;
}

.gantt-row-description-selected {
    background-color: #ede7f6;
    /* light violet */
}

.gantt-row-description-not-selectable {
    color: #9e9e9e;
}

.selected-alloc {
    stroke-width: 2.5 !important;
    filter: drop-shadow(2px 2px 2px #555a)
}

.corner-buttons {
    display: flex;
    gap: 3px;
    justify-content: center;
    align-content: center;
    height: 100%;
}

.overlay-handles .drag-handle rect,
.overlay-handles .action-btn rect {
    transition: filter 120ms ease, opacity 120ms ease;
}

.overlay-handles .drag-handle:hover rect {
    filter: brightness(1.22);
}

.overlay-handles .action-btn:hover rect {
    filter: brightness(1.12);
}

.overlay-handles {
    pointer-events: auto;
}
</style>
