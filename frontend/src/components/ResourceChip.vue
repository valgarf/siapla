<template>
    <q-chip color="secondary" text-color="white" :clickable="clickable" @click="clickable && showDetails()">
        {{ resource.name }}
    </q-chip>
</template>

<script setup lang="ts">
import { ResourceSidebarData, useSidebarStore } from 'src/stores/sidebar';
import type { Resource } from 'src/stores/resource';

interface Props {
    resource: Resource;
    clickable?: boolean;
}

const props = withDefaults(defineProps<Props>(), { clickable: true });
const sidebarStore = useSidebarStore();

function showDetails() {
    sidebarStore.pushSidebar(new ResourceSidebarData(props.resource.dbId));
}
</script>
