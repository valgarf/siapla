<template>
    <q-dialog @hide="dialogStore.closeLayer(dialogLayer)" :model-value="true">
        <q-card class="q-dialog-plugin card-size">
            <q-card-section>
                <div class="row items-center">
                    <q-btn :disable="dialogStore.atFirst(dialogLayer)" flat round icon="arrow_back"
                        @click="dialogStore.back(dialogLayer)" />
                    <q-btn :disable="dialogStore.atLast(dialogLayer)" flat round icon="arrow_forward"
                        @click="dialogStore.next(dialogLayer)" class="q-mr-sm" />
                    <slot name="toolbar"></slot>
                    <q-btn flat @click="dialogStore.closeLayer(dialogLayer)" icon="close"></q-btn>
                </div>
            </q-card-section>
            <slot></slot>
        </q-card>
    </q-dialog>
</template>

<style lang="scss" scoped>
.card-size {
    max-width: max(600px, 80%);
    width: min(100vw, 960px);
}

.q-btn.disabled {
    opacity: 0.3 !important;
}
</style>

<script setup lang="ts">

import { useDialogStore } from 'src/stores/dialog';
const dialogStore = useDialogStore();

interface Props {
    dialogLayer: number;
};

defineProps<Props>();

</script>