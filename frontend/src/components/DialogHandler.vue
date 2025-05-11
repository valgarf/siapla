<template>
    <div>
        <component :is="comp[1]" v-bind="comp[2]" v-for="comp in components" :key="'dialog-' + comp[0]" />
    </div>
</template>

<style lang="scss" scoped>
.card-size {
    max-width: max(600px, 80%);
    width: min(100vw, 960px);
}
</style>

<script setup lang="ts">
import { type DialogData, NewResourceDialogData, NewTaskDialogData, ResourceDialogData, TaskDialogData, useDialogStore } from 'src/stores/dialog';
import { type Component, computed } from 'vue';
import TaskDialog from './TaskDialog.vue';
import ResourceDialog from './ResourceDialog.vue';
import { useTaskStore } from 'src/stores/task';
import assert from 'assert';
import { useResourceStore } from 'src/stores/resource';

const dialogStore = useDialogStore();
const taskStore = useTaskStore();
const resourceStore = useResourceStore();

function mapDialogDataToComponent(cd: DialogData, idx: number): [number, Component, object] {
    if (cd instanceof TaskDialogData) {
        return [idx, TaskDialog, { dialogLayer: idx, task: taskStore.task(cd.taskId) }];
    }
    if (cd instanceof NewTaskDialogData) {
        return [idx, TaskDialog, { dialogLayer: idx, task: {} }];
    }
    if (cd instanceof ResourceDialogData) {
        return [idx, ResourceDialog, { dialogLayer: idx, resource: resourceStore.resource(cd.resourceId) }];
    }
    if (cd instanceof NewResourceDialogData) {
        return [idx, ResourceDialog, { dialogLayer: idx, resource: {} }];
    }
    assert(false, "Unexpected dialog type");
}

const components = computed(() =>
    dialogStore.activeDialogs.map(mapDialogDataToComponent)
)

</script>