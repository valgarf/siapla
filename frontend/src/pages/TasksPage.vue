<template>
  <q-page>
    <PlanCalcBanner />
    <TasksGanttChart />
  </q-page>
</template>

<script setup lang="ts">
import TasksGanttChart from 'src/components/gantt/TasksGanttChart.vue';
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
