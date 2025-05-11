<template>
    <q-chip color="primary" text-color="white" :clickable="clickable" @click="clickable && showDetails()">{{ task.title
    }}</q-chip>
</template>

<script setup lang="ts">
import type { Task } from 'src/stores/task';
import { useTaskDialogStore } from 'src/stores/task_dialog';

interface Props {
    task: Task;
    clickable?: boolean
};

const props = withDefaults(defineProps<Props>(), { clickable: true });
const taskDialogStore = useTaskDialogStore();
// details dialog
function showDetails() {
    taskDialogStore.push_task_dialog(props.task.dbId)
}

</script>