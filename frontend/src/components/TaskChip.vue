<template>
    <q-chip color="primary" text-color="white" :clickable="clickable" @click="clickable && showDetails()">{{ task.title
    }}</q-chip>
</template>

<script setup lang="ts">
import type { Task } from 'src/stores/task';
import { TaskSidebarData, useSidebarStore } from 'src/stores/sidebar';

interface Props {
    task: Task;
    clickable?: boolean
};

const props = withDefaults(defineProps<Props>(), { clickable: true });
const sidebarStore = useSidebarStore();
function showDetails() {
    sidebarStore.pushSidebar(new TaskSidebarData(props.task.dbId))
}

</script>