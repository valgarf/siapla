<template>
    <q-card v-ripple class="cursor-pointer q-hoverable" @click="showDetails()">
        <div tabindex="-1" class="q-focus-helper"></div>
        <q-card-section>
            <div class="text-subtitle1">{{ resource.name }}</div>
        </q-card-section>
    </q-card>
</template>

<script setup lang="ts">
import type { Resource } from 'src/stores/resource';
import { ResourceSidebarData, useSidebarStore } from 'src/stores/sidebar';

interface Props {
    resource: Resource;
};

const props = withDefaults(defineProps<Props>(), {});
const sidebarStore = useSidebarStore();
function showDetails() {
    sidebarStore.pushSidebar(new ResourceSidebarData(props.resource.dbId))
}

</script>