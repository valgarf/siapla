<template>
  <q-page>
    <PlanCalcBanner />
    <PlanGanttTasks />
  </q-page>
</template>

<script setup lang="ts">
import PlanGanttTasks from 'src/components/PlanGanttTasks.vue';
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
