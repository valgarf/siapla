<template>
    <q-chip color="primary" text-color="white" clickable @click="showDetails">{{ task.title }}</q-chip>
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