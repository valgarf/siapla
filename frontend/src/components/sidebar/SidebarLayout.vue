<template>
    <div class="margins">
        <q-toolbar>
            <q-btn v-if="!sidebarStore.currentEditing" :disable="sidebarStore.atFirst()" flat round icon="arrow_back"
                @click="sidebarStore.back" />
            <q-btn v-if="!sidebarStore.currentEditing" :disable="sidebarStore.atLast()" flat round icon="arrow_forward"
                @click="sidebarStore.next" class="q-mr-sm" />
            <slot name="toolbar"></slot>
            <q-space />
            <q-btn flat round icon="fullscreen" @click="sidebarStore.toggleExpand" aria-label="Expand" />
        </q-toolbar>

        <div>
            <slot></slot>
        </div>
    </div>
</template>

<style scoped>
.q-btn.disabled {
    opacity: 0.3 !important;
}

.margins {
    margin-left: auto;
    margin-right: auto;
    max-width: 1000px;
}
</style>

<script setup lang="ts">
import { useSidebarStore } from 'src/stores/sidebar';
const sidebarStore = useSidebarStore();

</script>