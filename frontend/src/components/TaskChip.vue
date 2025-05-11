<template>
    <q-chip color="primary" text-color="white" :clickable="clickable" @click="clickable && showDetails()">{{ task.title
        }}</q-chip>
</template>

<script setup lang="ts">
import type { Task } from 'src/stores/task';
import { TaskDialogData, useDialogStore } from 'src/stores/dialog';

interface Props {
    task: Task;
    clickable?: boolean
};

const props = withDefaults(defineProps<Props>(), { clickable: true });
const dialogStore = useDialogStore();
// details dialog
function showDetails() {
    dialogStore.pushDialog(new TaskDialogData(props.task.dbId))
}

</script>