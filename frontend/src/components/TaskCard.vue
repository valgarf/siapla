<template>
    <q-card v-ripple class="cursor-pointer q-hoverable" @click="showDetails()">
        <div tabindex="-1" class="q-focus-helper"></div>
        <q-card-section>
            <div class="text-subtitle1">{{ task.title }}</div>
        </q-card-section>

        <q-card-section class="q-pt-none">
            {{ task.description }}
        </q-card-section>
    </q-card>
</template>

<script setup lang="ts">
import { useTaskStore, type Task } from 'src/stores/task';

interface Props {
    task: Task;
};

const props = withDefaults(defineProps<Props>(), {});
const taskStore = useTaskStore();
// details dialog
function showDetails() {
    taskStore.push_task_dialog(props.task.dbId)
}

</script>