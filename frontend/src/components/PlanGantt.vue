<template>
  <div v-for="alloc in planStore.allocations" :key="alloc.dbId">
    {{alloc.task?.title}}:  {{alloc.start}} - {{alloc.end}}
  </div>
  <GGanttChart
    :chart-start="chartStart"
    :chart-end="chartEnd"
    precision="hour"
    bar-start="myBeginDate"
    bar-end="myEndDate"
    :date-format="false"
  >
    <GGanttRow label="My row 1" :bars="row1BarList" />
    <GGanttRow label="My row 2" :bars="row2BarList" />
</GGanttChart>
</template>

<script setup lang="ts">

import { GGanttChart, GGanttRow, type GanttBarObject } from '@infectoone/vue-ganttastic';
import { usePlanStore } from 'src/stores/plan';
import { type Ref, ref } from 'vue';
import dayjs from "dayjs"
import isSameOrAfter from "dayjs/plugin/isSameOrAfter.js"
import isSameOrBefore from "dayjs/plugin/isSameOrBefore.js"

dayjs.extend(isSameOrAfter)
dayjs.extend(isSameOrBefore)

const planStore = usePlanStore();
const chartStart = new Date("2021-07-12 12:00")
const chartEnd = new Date("2021-07-14 12:00")

interface Props {
  issuesOnly?: boolean;
};
withDefaults(defineProps<Props>(), {
  issuesOnly: true
});



const row1BarList: Ref<GanttBarObject[]> = ref([
  {
    myBeginDate: "2021-07-13 13:00",
    myEndDate: "2021-07-13 19:00",
    ganttBarConfig: {
      // each bar must have a nested ganttBarConfig object ...
      id: "unique-id-1", // ... and a unique "id" property
      label: "Lorem ipsum dolor"
    }
  }
])
const row2BarList: Ref<GanttBarObject[]> = ref([
  {
    myBeginDate: "2021-07-13 00:00",
    myEndDate: "2021-07-14 02:00",
    ganttBarConfig: {
      id: "another-unique-id-2",
      hasHandles: true,
      label: "Hey, look at me",
      style: {
        // arbitrary CSS styling for your bar
        background: "#e09b69",
        borderRadius: "20px",
        color: "black"
      }
    }
  }
])


</script>
