<template>
    <div>
        <div v-if="currentComp" class="q-pa-md">
            <component :is="currentComp[1]" v-bind="currentComp[2]" />
        </div>
        <div v-else class="q-pa-md">No selection</div>
    </div>
</template>

<script setup lang="ts">
import { type SidebarData, NewResourceSidebarData, NewTaskSidebarData, ResourceSidebarData, TaskSidebarData, useSidebarStore } from 'src/stores/sidebar';
import { type Component, computed } from 'vue';
import TaskSidebar from './TaskSidebar.vue';
import ResourceSidebar from './ResourceSidebar.vue';
import { type TaskInput, useTaskStore } from 'src/stores/task';
import { useResourceStore } from 'src/stores/resource';
// no external assert import: throw errors directly

const sidebarStore = useSidebarStore();
const taskStore = useTaskStore();
const resourceStore = useResourceStore();

function mapSidebarDataToComponent(cd: SidebarData): [number, Component, object] | null {
    if (cd == null) return null;
    if (cd instanceof TaskSidebarData) {
        return [cd.taskId, TaskSidebar, { task: taskStore.task(cd.taskId) }];
    }
    if (cd instanceof NewTaskSidebarData) {
        // build initial task object from defaults (resolve ids to task objects)
        const t: Partial<TaskInput> = {};
        if (cd.parentId !== undefined && cd.parentId !== null) {
            t.parent = taskStore.task(cd.parentId) ?? null;
        }
        if (cd.predecessorIds !== undefined && cd.predecessorIds.length > 0) {
            t.predecessors = cd.predecessorIds.map((id) => taskStore.task(id)).filter((x) => x != null);
        }
        if (cd.successorIds !== undefined && cd.successorIds.length > 0) {
            t.successors = cd.successorIds.map((id) => taskStore.task(id)).filter((x) => x != null);
        }
        if (cd.resourceConstraints !== undefined && cd.resourceConstraints.length > 0) {
            t.resourceConstraints = cd.resourceConstraints.map(obj => ({ ...obj }))
        }
        return [0, TaskSidebar, { task: t }];
    }
    if (cd instanceof ResourceSidebarData) {
        return [cd.resourceId, ResourceSidebar, { resource: resourceStore.resource(cd.resourceId) }];
    }
    if (cd instanceof NewResourceSidebarData) {
        return [0, ResourceSidebar, { resource: {} }];
    }
    throw new Error('Unexpected sidebar type');
}

const currentComp = computed(() => {
    // sidebarStore.activeSidebar might be a computed ref; attempt to read .value if present
    if (sidebarStore.activeSidebar == null) {
        return null
    }
    return mapSidebarDataToComponent(sidebarStore.activeSidebar);
});
</script>

<style scoped>
.sidebar-expanded {
    width: 100% !important;
}
</style>