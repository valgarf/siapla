<template>
    <q-chip color="primary" text-color="white" clickable @click="showDetails">{{ task.title }}</q-chip>
</template>

<script setup lang="ts">
import { Dialog } from 'quasar';
import TaskDialog from './TaskDialog.vue';
import { type Task } from 'src/stores/task';

interface Props {
    task: Task;
};

const props = withDefaults(defineProps<Props>(), {});

// details dialog
function showDetails() {
    Dialog.create({
        component: TaskDialog,

        componentProps: {
            task: props.task,
        }
    })
}

</script>