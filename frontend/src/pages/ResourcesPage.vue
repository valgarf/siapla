<template>
  <q-page>
    <PlanCalcBanner />
    <ResourcesGanttChart />
  </q-page>
</template>

<script setup lang="ts">
import ResourcesGanttChart from 'src/components/gantt/ResourcesGanttChart.vue';
import PlanCalcBanner from 'src/components/PlanCalcBanner.vue';
</script>

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
