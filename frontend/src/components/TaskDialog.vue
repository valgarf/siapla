<template>
    <q-dialog ref="dialogRef" @hide="onDialogHide">
        <q-card class="q-dialog-plugin card-size">
            <q-card-section>
                <div class="row items-center">
                    <q-input v-if="edit" outlined class="text-h5 col" v-model="local_task.title" />
                    <div v-else class="text-h5 col">{{ local_task.title }}</div>
                    <q-btn flat @click="toggleEdit()" :loading="taskStore.saving" color="primary"
                        :disable="taskStore.deleting" :icon="edit ? undefined : 'edit'" class="q-ma-xs">{{ edit ? "Save"
                            : null }}
                    </q-btn>
                    <q-btn flat @click="deleteTask()" :loading="taskStore.deleting" color="negative" icon="delete"
                        :disable="taskStore.saving" class="q-ma-xs"></q-btn>
                    <q-btn flat @click="onDialogHide" icon="close"></q-btn>
                </div>
            </q-card-section>

            <q-card-section class="q-pt-none">
                <MarkdownEditor v-if="edit" v-model="local_task.description" />
                <q-markdown v-else :src="local_task.description" />
            </q-card-section>

            <q-card-section>
                <q-btn-toggle v-if="edit" v-model="local_task.designation" rounded toggle-color="primary"
                    text-color="primary" color="white" :options="[
                        { label: 'Requirement', value: TaskDesignation.Requirement },
                        { label: 'Task', value: TaskDesignation.Task },
                        { label: 'Milestone', value: TaskDesignation.Milestone }
                    ]" />
                <q-chip v-else color="primary" text-color="white" class="q-pa-md">{{
                    local_task.designation }}</q-chip>
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
import { Dialog, useDialogPluginComponent } from 'quasar'
import MarkdownEditor from './MarkdownEditor.vue';
import { ref, watchEffect } from 'vue';
import { type TaskInput, useTaskStore, type Task } from 'src/stores/task';
import { TaskDesignation } from 'src/gql/graphql';

interface Props {
    task: Partial<Task>;
};


const props = withDefaults(defineProps<Props>(), { task: () => { return {} } });

const local_task_default = { title: "", description: "", designation: TaskDesignation.Task };
const local_task = ref<TaskInput>(local_task_default)

watchEffect(() => {
    local_task.value = { ...local_task_default, ...props.task }
})

defineEmits([
    // required by dialog plugin
    ...useDialogPluginComponent.emits,
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

const taskStore = useTaskStore()


async function save() {
    console.log("SAVE", { ...local_task.value });
    await taskStore.save_task(local_task);
}

async function deleteTask() {
    const taskId = local_task.value.dbId
    if (taskId == null) {
        onDialogHide()
        return
    }
    const dialogResolved = new Promise((resolve, reject) => {
        Dialog.create({
            title: 'Delete?',
            message: 'Would you really like to delete the task?',
            cancel: true,
            persistent: true
        }).onOk(resolve).onCancel(reject).onDismiss(reject)
    })
    try {
        await dialogResolved
    } catch {
        return
    }
    await taskStore.delete_task(taskId);
    onDialogHide();
}

</script>