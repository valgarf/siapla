<template>
    <q-chip color="secondary" text-color="white" :clickable="clickable" @click="clickable && showDetails()">
        {{ resource.name }}
    </q-chip>
</template>

<script setup lang="ts">
import { ResourceDialogData, useDialogStore } from 'src/stores/dialog';
import type { Resource } from 'src/stores/resource';

interface Props {
    resource: Resource;
    clickable?: boolean;
}

const props = withDefaults(defineProps<Props>(), { clickable: true });
const dialogStore = useDialogStore();

function showDetails() {
    dialogStore.pushDialog(new ResourceDialogData(props.resource.dbId));
}
</script>
