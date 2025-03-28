<template>
    <q-dialog ref="dialogRef" @hide="onDialogHide">
        <q-card class="q-dialog-plugin card-size">
            <q-card-section>
                <div class="row items-center">
                    <q-input v-if="edit" outlined class="text-h5 col" v-model="local_task.title" />
                    <div v-else class="text-h5 col">{{ local_task.title }}</div>
                    <q-btn flat color="primary" :icon="edit ? undefined : 'edit'" class="q-ma-xs"
                        @click="toggleEdit()">{{ edit ? "Save" : null }}</q-btn>
                    <q-btn flat color="negative" icon="delete" class="q-ma-xs"></q-btn>
                    <q-btn flat @click="onDialogHide" icon="close"></q-btn>
                </div>
            </q-card-section>

            <q-card-section class="q-pt-none">
                <MarkdownEditor v-if="edit" v-model="local_task.description" />
                <q-markdown v-else :src="local_task.description" />
            </q-card-section>
        </q-card>
    </q-dialog>
</template>

<style lang="scss" scoped>
.card-size {
    max-width: max(600px, 80%);
    width: min(100vw, 960px);
}
</style>

<script setup lang="ts">
import { save_task, setup_save_mutations, type Task } from 'src/model/tasks';

import { useDialogPluginComponent } from 'quasar'
import MarkdownEditor from './MarkdownEditor.vue';
import { ref, watchEffect } from 'vue';

interface Props {
    task: Partial<Task>;
};
interface LocalTask extends Partial<Task> {
    title: string;
    description: string;
}


const props = withDefaults(defineProps<Props>(), { task: () => { return {} } });

const local_task_default = { title: "", description: "" };
const local_task = ref<LocalTask>(local_task_default)

watchEffect(() => {
    local_task.value = { ...local_task_default, ...props.task }
})

defineEmits([
    // required by dialog plugin
    ...useDialogPluginComponent.emits
])

const { dialogRef, onDialogHide } = useDialogPluginComponent()
// dialogRef      - Vue ref to be applied to QDialog
// onDialogHide   - Function to be used as handler for @hide on QDialog
// onDialogOK     - Function to call to settle dialog with "ok" outcome
//                    example: onDialogOK() - no payload
//                    example: onDialogOK({ /*...*/ }) - with payload
// onDialogCancel - Function to call to settle dialog with "cancel" outcome

const edit = ref(local_task.value.dbId == null)
async function toggleEdit() {
    if (edit.value) {
        await save()
    }
    edit.value = !edit.value
}


const mutations = setup_save_mutations();

const saving = ref(false)
async function save() {
    console.log("SAVE", { ...local_task.value });
    saving.value = true;
    try {
        await save_task(local_task, mutations);
    }
    finally {
        saving.value = false
    }

}

</script>