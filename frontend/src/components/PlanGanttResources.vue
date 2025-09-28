<template>

  <div class="gantt-grid">
    <!-- Top left: new task / resource buttons -->
    <div class="gantt-corner" id="corner-resources">
      <div class="corner-buttons">
        <q-btn aria-label="New task" flat @click.stop="onNewTask" icon="add_task">
        </q-btn>
        <q-btn aria-label="New resource" flat @click.stop="onNewResource" icon="person_add">
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
              <rect :x="month.x" y="0" :width="month.width" :height="monthRowHeight" fill="#fff" stroke="#ccc"
                stroke-width="1" />
              <text :x="month.x + 4" :y="monthRowHeight - 6" font-size="12" fill="#333">
                {{ month.label }}
              </text>
            </template>
          </g>
          <!-- Day row with weekend highlight -->
          <g>
            <template v-for="(day, i) in days" :key="i">
              <rect v-if="day.date.getDay() === 0 || day.date.getDay() === 6" :x="day.x" :y="monthRowHeight"
                :width="dayWidth" :height="dayRowHeight" fill="#fffbe6" stroke="#ccc" stroke-width="1" />
              <rect v-else :x="day.x" :y="monthRowHeight" :width="dayWidth" :height="dayRowHeight" fill="#fff"
                stroke="#ccc" stroke-width="1" />
              <text :x="day.x + 2" :y="monthRowHeight + dayRowHeight - 6" font-size="10" fill="#666">
                {{ day.label }}
              </text>
            </template>
          </g>
          <!-- Vertical lines between days -->
          <!-- <g>
            <template v-for="(day, i) in days" :key="i">
              <line :x1="day.x" :y1="monthRowHeight" :x2="day.x" :y2="headerHeight" stroke="#ccc" stroke-width="1"  />
            </template>
          </g> -->
        </svg>
      </div>
    </div>
    <!-- Bottom left: resources -->
    <div class="gantt-resources"
      :style="{ height: chartHeight + 'px', width: resourceColWidth + 'px', position: 'relative', overflow: 'hidden' }"
      @mousedown="onPanStart" @mousemove="onPanMoveY" @mouseup="onPanEnd" @mouseleave="onPanEnd">
      <div :style="{ position: 'absolute', top: -scrollY + 'px', left: 0, width: '100%' }">
        <div v-for="(res, i) in Array.from(resourceStore.resources)" :key="i" class="gantt-resource-row"
          :style="{ height: rowHeight + 'px' }" @click.stop="() => onResourceClick(res.dbId)">
          <span>{{ resourceStore.resource(res.dbId)?.name ?? '<UNNAMED>' }}</span>
        </div>
      </div>
    </div>
    <!-- Bottom right: scrollable chart -->
    <div class="gantt-chart-scroll" ref="scrollCell" @mousedown="onPanStart" @mousemove="onPanMove" @mouseup="onPanEnd"
      @mouseleave="onPanEnd">
      <svg :width="timelineWidth" :height="chartHeight"
        :style="{ transform: `translate(${-scrollX}px, ${-scrollY}px)` }">
        <!-- Weekend highlight in chart -->
        <g>
          <template v-for="(day, i) in days" :key="'w'+i">
            <rect v-if="day.date.getDay() === 0 || day.date.getDay() === 6" :x="day.x" y="0" :width="dayWidth"
              :height="chartHeight" fill="#fffbe6" opacity="1" stroke="none" />
          </template>
        </g>
        <!-- Resource availability bars for all non-vacation segments in the visible timeframe -->
        <g>
          <template v-for="(res, i) in resourceStore.resources" :key="'avail'+res.dbId">
            <template v-for="seg in getCombinedAvailability(res.dbId)" :key="'seg'+seg.start+seg.end">
              <rect :x="dateToX(seg.start)" :y="i * rowHeight" :width="dateToX(seg.end) - dateToX(seg.start)"
                :height="rowHeight" fill="#fff" opacity="0.7" stroke="none" />
            </template>
          </template>
        </g>
        <!-- Horizontal lines between resources -->
        <g>
          <template v-for="(res, i) in resourceStore.resources" :key="i">
            <line :x1="0" :y1="i * rowHeight" :x2="timelineWidth" :y2="i * rowHeight" stroke="#ddd" stroke-width="1" />
          </template>
          <line :x1="0" :y1="planStore.resource_ids.length * rowHeight" :x2="timelineWidth"
            :y2="planStore.resource_ids.length * rowHeight" stroke="#ddd" stroke-width="1" />
        </g>
        <!-- Vertical lines between days -->
        <g>
          <template v-for="(day, i) in days" :key="i">
            <line :x1="day.x" :y1="0" :x2="day.x" :y2="chartHeight" stroke="#ddd" stroke-width="1" />
          </template>
        </g>
        <!-- Vacation bars per resource -->
        <!-- <g>
          <template v-for="(res, i) in resourceStore.resources" :key="'vac'+rid">
            <template v-for="vac in resourceStore.resource(rid)?.vacations ?? []" :key="'vac'+rid+vac.from+vac.until">
              <rect :x="dateToX(vac.from)" :y="i * rowHeight" :width="dateToX(vac.until) - dateToX(vac.from)"
                :height="rowHeight" fill="#f3f4f5" opacity="1" stroke="none" />
            </template>
          </template>
        </g> -->


        <!-- Allocation bars -->
        <g>
          <template v-for="(rid, i) in Array.from(planStore.resource_ids)" :key="rid">
            <template v-for="alloc in planStore.by_resource(rid)" :key="rid+'-'+alloc.dbId">
              <rect :x="dateToX(alloc.start)" :y="i * rowHeight + barPadding"
                :width="dateToX(alloc.end) - dateToX(alloc.start)" :height="barHeight" fill="#42a5f5" rx="3"
                @click.stop="() => onAllocClick(alloc.task?.dbId)" />
              <text :x="dateToX(alloc.start) + 4" :y="i * rowHeight + barPadding + barHeight / 2 + 4" font-size="11"
                fill="#fff">{{ alloc.task?.title ?? '' }}</text>
            </template>
          </template>
        </g>
      </svg>
    </div>
  </div>
</template>

<style>
html,
body,
#app {
  height: 100vh;
  margin: 0;
  padding: 0;
  overflow: hidden;
}
</style>
<style scoped>
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
  width: var(--resource-col-width, 200px);
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
  font-size: 12px;
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
<script setup lang="ts">

import { usePlanStore } from 'src/stores/plan';
import { useResourceStore } from 'src/stores/resource';
import { useDialogStore, ResourceDialogData, TaskDialogData, NewTaskDialogData, NewResourceDialogData } from 'src/stores/dialog';
import { ref, computed } from 'vue';

const planStore = usePlanStore();
const resourceStore = useResourceStore();
const dialogStore = useDialogStore();

const resourceColWidth = 200;
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

function parseDate(d: string | Date): Date {
  return typeof d === 'string' ? new Date(d) : d;
}

const days = computed(() => {
  const arr: { date: Date; label: number; x: number }[] = [];
  const d = new Date(startDay.value);
  const end = new Date(endDay.value);
  let i = 0;
  while (d <= end) {
    arr.push({
      date: new Date(d),
      label: d.getDate(),
      x: i * dayWidth,
    });
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
      arr.push({
        label: `${curYear}-${String(curMonth + 1).padStart(2, '0')}`,
        x: startIdx * dayWidth,
        width: (i - startIdx) * dayWidth,
      });
      curMonth = d.getMonth();
      curYear = d.getFullYear();
      startIdx = i;
    }
    d.setDate(d.getDate() + 1);
    i++;
  }
  arr.push({
    label: `${curYear}-${String(curMonth + 1).padStart(2, '0')}`,
    x: startIdx * dayWidth,
    width: (i - startIdx) * dayWidth,
  });
  return arr;
});

const timelineWidth = computed(() => days.value.length * dayWidth);
const chartHeight = computed(() => resourceStore.resources.length * rowHeight);

function dateToX(date: string | Date): number {
  const d = parseDate(date);
  return (d.getTime() - startDay.value.getTime()) / (1000 * 60 * 60 * 24) * dayWidth;
}

const scrollX = ref(0);
const scrollY = ref(0);
const isPanning = ref(false);
const panStartX = ref(0);
const panStartY = ref(0);
const panOrigX = ref(0);
const panOrigY = ref(0);
const scrollCell = ref<HTMLElement | null>(null);

function onPanStart(e: MouseEvent) {
  isPanning.value = true;
  panStartX.value = e.clientX;
  panStartY.value = e.clientY;
  panOrigX.value = scrollX.value;
  panOrigY.value = scrollY.value;
}
function onPanMove(e: MouseEvent) {
  onPanMoveX(e)
  onPanMoveY(e)
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

const combinedAvailabiltyQuery = resourceStore.fetchCombinedAvailability(startDay, endDay);
// Return the unwrapped array from the store's computed Ref for template iteration.
function getCombinedAvailability(resourceId: number): { start: Date, end: Date }[] {
  const q = combinedAvailabiltyQuery;
  if (!q || !q.result || q.result.value == null) return [] as Array<{ start: Date; end: Date }>;
  const data = q.result.value;
  const r = data.resources.find((rr) => rr.dbId === resourceId);
  if (!r) return [] as Array<{ start: Date; end: Date }>;
  return r.combinedAvailability.map((it) => ({ start: new Date(it.start), end: new Date(it.end) }));

}

function onResourceClick(rid: number) {
  // open resource dialog for the clicked resource
  dialogStore.pushDialog(new ResourceDialogData(rid));
}

function onAllocClick(taskId: number | undefined | null) {
  // try to extract a task dbId from the allocation and open its dialog
  if (taskId != null) {
    dialogStore.pushDialog(new TaskDialogData(taskId));
  }
}

function onNewTask() {
  dialogStore.pushDialog(new NewTaskDialogData());
}

function onNewResource() {
  dialogStore.pushDialog(new NewResourceDialogData());
}

</script>