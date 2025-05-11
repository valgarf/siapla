<template>
    <DialogLayout :dialogLayer="dialogLayer">
        <template #toolbar>
            <div class="col"></div>
            <q-btn flat @click="toggleEdit()" :loading="resourceStore.saving" color="primary"
                :disable="resourceStore.deleting" :icon="edit ? undefined : 'edit'" class="q-ma-xs">{{ edit ? "Save"
                    : null }}
            </q-btn>
            <q-btn flat @click="deleteResource()" :loading="resourceStore.deleting" color="negative" icon="delete"
                :disable="resourceStore.saving" class="q-ma-xs"></q-btn>
        </template>
        <q-card-section>
            <q-input v-if="edit" outlined placeholder="Name" class="text-h5" v-model="local_resource.name" />
            <div v-else class="text-h5">{{ local_resource.name }}</div>
        </q-card-section>

        <q-card-section>
            <DateTimeInput v-if="edit" label="Added" v-model="local_resource.added" />
            <div v-else class="row items-baseline">
                <div class="text-subtitle2 q-pr-md">Added:</div>
                <div>{{ format_datetime(local_resource.added) }}</div>
            </div>
        </q-card-section>
        <q-card-section>
            <DateTimeInput v-if="edit" label="Schedule" v-model="local_resource.removed" />
            <div v-else class="row items-baseline">
                <div class="text-subtitle2 q-pr-md">Removed:</div>
                <div>{{ format_datetime(local_resource.removed) }}</div>
            </div>
        </q-card-section>

    </DialogLayout>
</template>


<script setup lang="ts">
import { Dialog } from 'quasar'
import { ref, watchEffect } from 'vue';
import { type ResourceInput, useResourceStore } from 'src/stores/resource';
import DateTimeInput from './DateTimeInput.vue';
import { format_datetime } from 'src/common/datetime'
import { useDialogStore } from 'src/stores/dialog';
import DialogLayout from './DialogLayout.vue';

const resourceStore = useResourceStore();
const dialogStore = useDialogStore();

const local_resource_default = { name: "", timezone: Intl.DateTimeFormat().resolvedOptions().timeZone, added: new Date() };
const local_resource = ref<ResourceInput>(local_resource_default)
const edit = ref(local_resource.value.dbId == null)


interface Props {
    dialogLayer: number;
    resource: ResourceInput;
};

const props = defineProps<Props>();

watchEffect(() => {
    // resource changed
    local_resource.value = { ...local_resource_default, ...props.resource }
    edit.value = local_resource.value.dbId == null
})

// actions

async function toggleEdit() {
    if (edit.value) {
        await save()
        edit.value = false
    }
    else {
        edit.value = true
    }
}


async function save() {
    await resourceStore.saveResource(local_resource);
}

async function deleteResource() {
    const resourceId = local_resource.value.dbId
    if (resourceId == null) {
        dialogStore.popDialog()
        return
    }
    const dialogResolved = new Promise((resolve, reject) => {
        Dialog.create({
            title: 'Delete?',
            message: 'Would you really like to delete the resource?',
            cancel: true,
            persistent: true
        }).onOk(resolve).onCancel(reject).onDismiss(reject)
    })
    try {
        await dialogResolved
    } catch {
        return
    }
    await resourceStore.deleteResource(resourceId, true);
}

</script>