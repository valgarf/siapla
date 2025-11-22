<template>
    <div>
        <q-toolbar>
            <q-btn :disable="dialogStore.atFirst()" flat round icon="arrow_back" @click="dialogStore.back" />
            <q-btn :disable="dialogStore.atLast()" flat round icon="arrow_forward" @click="dialogStore.next"
                class="q-mr-sm" />
            <slot name="toolbar"></slot>
            <q-space />
            <q-btn flat round icon="fullscreen" @click="dialogStore.toggleExpand" aria-label="Expand" />
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
import { useDialogStore } from 'src/stores/dialog';
const dialogStore = useDialogStore();

</script>