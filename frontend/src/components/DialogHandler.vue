<template>
    <div>
        <div v-if="currentComp" class="q-pa-md">
            <component :is="currentComp[1]" v-bind="currentComp[2]" />
        </div>
        <div v-else class="q-pa-md">No selection</div>
    </div>
</template>

<script setup lang="ts">
import { type DialogData, NewResourceDialogData, NewTaskDialogData, ResourceDialogData, TaskDialogData, useDialogStore } from 'src/stores/dialog';
import { type Component, computed } from 'vue';
import TaskDialog from './TaskDialog.vue';
import ResourceDialog from './ResourceDialog.vue';
import { useTaskStore } from 'src/stores/task';
import { useResourceStore } from 'src/stores/resource';
// no external assert import: throw errors directly

const dialogStore = useDialogStore();
const taskStore = useTaskStore();
const resourceStore = useResourceStore();

function mapDialogDataToComponent(cd: DialogData): [number, Component, object] | null {
    if (cd == null) return null;
    if (cd instanceof TaskDialogData) {
        return [cd.taskId, TaskDialog, { task: taskStore.task(cd.taskId) }];
    }
    if (cd instanceof NewTaskDialogData) {
        return [0, TaskDialog, { task: {} }];
    }
    if (cd instanceof ResourceDialogData) {
        return [cd.resourceId, ResourceDialog, { resource: resourceStore.resource(cd.resourceId) }];
    }
    if (cd instanceof NewResourceDialogData) {
        return [0, ResourceDialog, { resource: {} }];
    }
    throw new Error('Unexpected dialog type');
}

const currentComp = computed(() => {
    // dialogStore.activeDialog might be a computed ref; attempt to read .value if present
    const maybe = dialogStore.activeDialog as unknown;
    const cd = (maybe && typeof (maybe as { value?: unknown }).value !== 'undefined') ? (maybe as { value: unknown }).value : maybe;
    if (!cd) return null;
    return mapDialogDataToComponent(cd as DialogData);
});
</script>

<style scoped>
.sidebar-expanded {
    width: 100% !important;
}
</style>