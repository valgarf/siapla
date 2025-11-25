<template>
    <div>
        <q-toolbar>
            <q-btn :disable="sidebarStore.atFirst()" flat round icon="arrow_back" @click="sidebarStore.back" />
            <q-btn :disable="sidebarStore.atLast()" flat round icon="arrow_forward" @click="sidebarStore.next"
                class="q-mr-sm" />
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
</style>

<script setup lang="ts">
import { useSidebarStore } from 'src/stores/sidebar';
const sidebarStore = useSidebarStore();

</script>