<template>
  <q-page class="row items-center justify-evenly">
    <TaskGrid :issues-only="true" />
  </q-page>
</template>

<script setup lang="ts">
import TaskGrid from 'src/components/TaskGrid.vue';


</script>
