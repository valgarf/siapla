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
import { ResourceDialogData, useDialogStore } from 'src/stores/dialog';

interface Props {
    resource: Resource;
};

const props = withDefaults(defineProps<Props>(), {});
const dialogStore = useDialogStore();
// details dialog
function showDetails() {
    dialogStore.pushDialog(new ResourceDialogData(props.resource.dbId))
}

</script>