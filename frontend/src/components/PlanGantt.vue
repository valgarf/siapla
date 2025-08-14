<template>
  <div v-for="alloc in planStore.allocations" :key="alloc.dbId">
    {{alloc.task?.title}}:  {{alloc.start}} - {{alloc.end}}
  </div>
  <div>{{ planStore.start }}</div>
  <div>{{ planStore.end }}</div>
  <div>{{ startDay }}</div>
  <div>{{ endDay }}</div>
  <svg>
    <g v-for="(rid, ridx) in planStore.resource_ids" :key="rid"  :transform="row_y(ridx)">
      <text x="0" y="0">{{row_title(rid)}}</text>
    </g>
  </svg>
</template>

<script setup lang="ts">

import { usePlanStore } from 'src/stores/plan';
import { useResourceStore } from 'src/stores/resource';
import { computed } from 'vue';

const DAY_IN_MS = 1000 * 3600 * 24
const planStore = usePlanStore();
const resourceStore = useResourceStore();

const startDay = computed(() => {
  const d = planStore.start;
  return new Date(d.getFullYear(), d.getMonth(), d.getDate());
})
const endDay = computed(() => {
  const d = planStore.end;
  return new Date(d.getFullYear(), d.getMonth(), d.getDate() + 1);
})


function row_y(idx: number): string {
  return `translate(0 ${idx * 35 + 50})`
}

function row_title(rid: number): string {
  return resourceStore.resource(rid)?.name ?? "<UNNAMED>"
}


interface Props {
  issuesOnly?: boolean;
};
withDefaults(defineProps<Props>(), {
  issuesOnly: true
});





</script>
