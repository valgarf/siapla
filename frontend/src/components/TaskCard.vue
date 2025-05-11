<template>
    <q-card v-ripple class="cursor-pointer q-hoverable overflow-hidden" @click="showDetails()">
        <div tabindex="-1" class="q-focus-helper"></div>
        <q-card-section class="q-pb-xs">
            <q-breadcrumbs class="col" :class="{ invisible: parents.length == 0 }">
                <q-breadcrumbs-el v-for="p in parents" :key="p.dbId" :label="p.title" />
                <q-breadcrumbs-el label="" />
                <q-breadcrumbs-el label="" v-if="parents.length == 0" />
            </q-breadcrumbs>
            <div class="text-subtitle1">{{ task.title }}</div>
            <q-chip color="secondary" text-color="white" class="text-caption q-pa-sm q-ma-none">{{
                task.designation }}
            </q-chip>
        </q-card-section>
        <q-card-section class="q-pt-none q-pb-xs">
            <q-markdown :src="task.description" />
        </q-card-section>
    </q-card>
</template>

<script setup lang="ts">
import type { Task } from 'src/stores/task';
import { TaskDialogData, useDialogStore } from 'src/stores/dialog';
import { computed } from 'vue';

interface Props {
    task: Task;
};

const props = withDefaults(defineProps<Props>(), {});
const dialogStore = useDialogStore();


const parents = computed(() => {
    const parents = [];
    let parent = props.task.parent;
    while (parent != null) {
        parents.push(parent)
        parent = parent.parent
    }
    return parents.reverse()
})

function showDetails() {
    dialogStore.pushDialog(new TaskDialogData(props.task.dbId))
}

</script>