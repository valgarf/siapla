<template>
  <q-page>
    <PlanCalcBanner />
    <PlanGanttResources />
  </q-page>
</template>

<script setup lang="ts">
import PlanGanttResources from 'src/components/PlanGanttResources.vue';
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
