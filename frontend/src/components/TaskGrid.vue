<template>
  <div v-if="!taskStore.loading" style="
      display:inline-grid;
      grid-template-columns: repeat(auto-fill, 200px);
      grid-auto-columns: 200px;
      grid-auto-rows: 180px;
      gap: 10px 10px;
      place-items: stretch;
      width:90%;
    ">
    <q-card v-ripple class="cursor-pointer q-hoverable" style="align-content: center;" @click="showDetailsNew()">
      <div tabindex="-1" class="q-focus-helper"></div>
      <q-card-section style="text-align: center;">
        <q-icon name="add" size="lg" />
      </q-card-section>
    </q-card>
    <TaskCard :task="t" v-for="t in tasks" :key="t.dbId" />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import TaskCard from './TaskCard.vue';
import TaskDialog from './TaskDialog.vue';
import { Dialog } from 'quasar';
import { useTaskStore } from 'src/stores/task';


const taskStore = useTaskStore();

const tasks = computed(() => {
  const result = [...taskStore.tasks];
  result.sort((t1, t2) => t1.title < t2.title ? 1 : -1)
  return result
})

interface Props {
  issuesOnly?: boolean;
};
withDefaults(defineProps<Props>(), {
  issuesOnly: true
});

// details dialog
function showDetailsNew() {
  Dialog.create({
    component: TaskDialog,

    componentProps: {
      task: { title: "New Task" },
    }
  })
}

</script>
