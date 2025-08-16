
<template>
  <div class="gantt-grid">
    <!-- Top left: empty -->
    <div class="gantt-corner"></div>
    <!-- Top right: header -->
    <div class="gantt-header">
      <div class="gantt-header-scroll" :style="{ width: timelineWidth + 'px', left: '0px', transform: `translate(${-scrollX}px, 0)` }">
        <svg :width="timelineWidth" :height="headerHeight">
          <!-- Year/Month row -->
          <g>
            <template v-for="(month, i) in months" :key="i">
              <rect :x="month.x" y="0" :width="month.width" :height="monthRowHeight" fill="#f5f5f5" stroke="#ccc" stroke-width="1" />
              <text :x="month.x + 4" :y="monthRowHeight - 6" font-size="12" fill="#333">{{ month.label }}</text>
              <!-- Draw vertical bar at the end of each month except the last -->
              <!-- <line v-if="i < months.length - 1" :x1="month.x + month.width" y1="0" :x2="month.x + month.width" :y2="monthRowHeight" stroke="#ccc" stroke-width="1" /> -->
            </template>
          </g>
          <!-- Day row -->
          <g>
            <template v-for="(day, i) in days" :key="i">
              <rect :x="day.x" :y="monthRowHeight" :width="dayWidth" :height="dayRowHeight" fill="#fafafa" stroke="#ccc" stroke-width="1" />
              <text :x="day.x + 2" :y="monthRowHeight + dayRowHeight - 6" font-size="10" fill="#666">{{ day.label }}</text>
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
    <div class="gantt-resources" :style="{ height: chartHeight + 'px', width: resourceColWidth + 'px', position: 'relative', overflow: 'hidden' }">
      <div :style="{ position: 'absolute', top: -scrollY + 'px', left: 0, width: '100%' }">
        <div v-for="(rid, i) in Array.from(planStore.resource_ids)" :key="i" class="gantt-resource-row" :style="{ height: rowHeight + 'px' }">
          <span>{{ resourceStore.resource(rid)?.name ?? '<UNNAMED>' }}</span>
        </div>
      </div>
    </div>
    <!-- Bottom right: scrollable chart -->
  <div class="gantt-chart-scroll" ref="scrollCell" @mousedown="onPanStart" @mousemove="onPanMove" @mouseup="onPanEnd" @mouseleave="onPanEnd">
      <svg :width="timelineWidth" :height="chartHeight" :style="{ transform: `translate(${-scrollX}px, ${-scrollY}px)` }">
        <!-- Horizontal lines between resources -->
        <g>
          <template v-for="(rid, i) in Array.from(planStore.resource_ids)" :key="i">
            <line :x1="0" :y1="i * rowHeight" :x2="timelineWidth" :y2="i * rowHeight" stroke="#ddd" stroke-width="1" />
          </template>
          <line :x1="0" :y1="planStore.resource_ids.length * rowHeight" :x2="timelineWidth" :y2="planStore.resource_ids.length * rowHeight" stroke="#ddd" stroke-width="1" />
        </g>
        <!-- Vertical lines between days -->
        <g>
          <template v-for="(day, i) in days" :key="i">
            <line :x1="day.x" :y1="0" :x2="day.x" :y2="chartHeight" stroke="#eee" stroke-width="1" />
          </template>
        </g>
        <!-- Allocation bars -->
        <g>
          <template v-for="(rid, i) in Array.from(planStore.resource_ids)" :key="rid">
            <template v-for="alloc in planStore.by_resource(rid)" :key="rid+'-'+alloc.dbId">
              <rect
                :x="dateToX(alloc.start)"
                :y="i * rowHeight + barPadding"
                :width="dateToX(alloc.end) - dateToX(alloc.start)"
                :height="barHeight"
                fill="#42a5f5"
                rx="3"
              />
              <text
                :x="dateToX(alloc.start) + 4"
                :y="i * rowHeight + barPadding + barHeight/2 + 4"
                font-size="11"
                fill="#fff"
              >{{ alloc.task?.title ?? '' }}</text>
            </template>
          </template>
        </g>
      </svg>
    </div>
  </div>
</template>

<style>
html, body, #app {
  height: 100vh;
  margin: 0;
  padding: 0;
  overflow: hidden;
}
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
}
.gantt-corner {
  grid-column: 1;
  grid-row: 1;
  background: #fff;
  border-bottom: 1px solid #ddd;
  border-right: 1px solid #ddd;
  width: var(--resource-col-width, 160px);
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
  background: #fafbfc;
  position: relative;
}
.gantt-chart-scroll:active {
  cursor: grabbing;
}

</style>
<script setup lang="ts">
import { usePlanStore } from 'src/stores/plan';
import { useResourceStore } from 'src/stores/resource';
import { ref, computed } from 'vue';

const planStore = usePlanStore();
const resourceStore = useResourceStore();

const resourceColWidth = 160;
const rowHeight = 40;
const barPadding = 6;
const barHeight = rowHeight - barPadding * 2;
const dayWidth = 32;
const monthRowHeight = 28;
const dayRowHeight = 22;
const headerHeight = monthRowHeight + dayRowHeight;

const startDay = computed(() => {
  const d = planStore.start;
  return new Date(d.getFullYear(), d.getMonth(), d.getDate());
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
        label: `${curYear}-${String(curMonth+1).padStart(2,'0')}`,
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
    label: `${curYear}-${String(curMonth+1).padStart(2,'0')}`,
    x: startIdx * dayWidth,
    width: (i - startIdx) * dayWidth,
  });
  return arr;
});

const timelineWidth = computed(() => days.value.length * dayWidth);
const chartHeight = computed(() => Array.from(planStore.resource_ids).length * rowHeight);

function dateToX(date: string | Date): number {
  const d = parseDate(date);
  return (d.getTime() - startDay.value.getTime()) / (1000*60*60*24) * dayWidth;
}

const scrollX = ref(0);
const scrollY = ref(0);
const isPanning = ref(false);
const panStartX = ref(0);
const panStartY = ref(0);
const panOrigX = ref(0);
const panOrigY = ref(0);
const scrollCell = ref<HTMLElement|null>(null);

function onPanStart(e: MouseEvent) {
  isPanning.value = true;
  panStartX.value = e.clientX;
  panStartY.value = e.clientY;
  panOrigX.value = scrollX.value;
  panOrigY.value = scrollY.value;
}
function onPanMove(e: MouseEvent) {
  if (!isPanning.value) return;
  const dx = e.clientX - panStartX.value;
  const dy = e.clientY - panStartY.value;
  let newX = panOrigX.value - dx;
  let newY = panOrigY.value - dy;
  if (scrollCell.value) {
    const rect = scrollCell.value.getBoundingClientRect();
    let visibleHeight = rect.height;
    const visibleWidth = rect.width;
    const viewportHeight = window.innerHeight;
    if (rect.bottom > viewportHeight) {
      visibleHeight -= (rect.bottom - viewportHeight);
    }
    newX = Math.max(0, Math.min(newX, timelineWidth.value - visibleWidth));
    newY = Math.max(0, Math.min(newY, chartHeight.value - visibleHeight));
    console.log(scrollCell.value, scrollCell.value.clientWidth, scrollCell.value.clientHeight, visibleHeight)
  }
  scrollX.value = newX;
  scrollY.value = newY;
}
function onPanEnd() {
  isPanning.value = false;
}
</script>