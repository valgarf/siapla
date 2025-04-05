<template>
    <q-dialog ref="dialogRef" @hide="onDialogHide">
        <q-card class="q-dialog-plugin card-size">
            <q-card-section><q-breadcrumbs>
                    <q-breadcrumbs-el label="Taks" />
                    <q-breadcrumbs-el v-for="p in parents" :key="p.dbId" :label="p.title" />
                    <q-breadcrumbs-el :label="local_task.title" />
                </q-breadcrumbs>
            </q-card-section>
            <q-card-section>
                <div class="row items-center">
                    <q-input v-if="edit" outlined placeholder="Title" class="text-h5 col" v-model="local_task.title" />
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
                <MarkdownEditor v-if="edit" placeholder="description" v-model="local_task.description" />
                <q-markdown v-else :src="local_task.description" />
            </q-card-section>

            <q-card-section>
                <q-btn-toggle v-if="edit" v-model="local_task.designation" rounded toggle-color="secondary"
                    text-color="secondary" color="white" :options="[
                        { label: 'Requirement', value: TaskDesignation.Requirement },
                        { label: 'Task', value: TaskDesignation.Task },
                        { label: 'Milestone', value: TaskDesignation.Milestone }
                    ]" />
                <q-chip v-else color="secondary" text-color="white" class="q-pa-md">{{
                    local_task.designation }}</q-chip>
            </q-card-section>
            <q-card-section>
                <q-select v-if="edit" filled v-model="predecessors" multiple :options="possiblePredecessors" use-chips
                    stack-label label="predecessors" />
                <div v-else class="row items-center">
                    <div>predecessors</div>
                    <TaskChip v-for="task in local_task.predecessors" :key="task.dbId" :task="task" />
                </div>
            </q-card-section>
            <q-card-section>
                <q-select v-if="edit" filled v-model="successors" multiple :options="possibleSuccessors" use-chips
                    stack-label label="successors" />
                <div v-else class="row items-center">
                    <div>successors</div>
                    <TaskChip v-for="task in local_task.successors" :key="task.dbId" :task="task" />
                </div>
            </q-card-section>
            <q-card-section>
                <q-select v-if="edit" filled v-model="children" multiple :options="possibleChildren" use-chips
                    stack-label label="children" />
                <div v-else class="row items-center">
                    <div>children</div>
                    <TaskChip v-for="task in local_task.children" :key="task.dbId" :task="task" />
                </div>
                <q-select v-if="edit" filled v-model="parent" :options="possibleParents" use-chips stack-label
                    label="parent" />
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
import { computed, ref, watchEffect } from 'vue';
import { type TaskInput, useTaskStore, type Task } from 'src/stores/task';
import { TaskDesignation } from 'src/gql/graphql';
import TaskChip from './TaskChip.vue';

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

// This is a not so nice workaround to get select to work. 
// If we use actual tasks in the model, we get recursion errors, so we only provide the ids.
interface SelectOpt {
    label: string,
    value: number,
}
function to_select_opt(t: Task): SelectOpt {
    return { label: t.title, value: t.dbId }
}
function from_select_opt(t: SelectOpt): Task | undefined {
    return taskStore.task(t.value)
}

const parents = computed(() => {
    const parents = [];
    let parent = local_task.value.parent;
    while (parent != null) {
        parents.push(parent)
        parent = parent.parent
    }
    return parents.reverse()
})

const possiblePredecessors = computed(() => {
    return taskStore.tasks.filter((t) => t.dbId != local_task.value.dbId).map(to_select_opt)
})
const possibleSuccessors = computed(() => {
    return taskStore.tasks.filter((t) => t.dbId != local_task.value.dbId).map(to_select_opt)
})
const possibleChildren = computed(() => {
    return taskStore.tasks.filter((t) => {
        const parent_ids = parents.value.map((p) => p.dbId);
        return !parent_ids.includes(t.dbId) && local_task.value.dbId != t.dbId
    }).map(to_select_opt)
})
const possibleParents = computed(() => {
    return taskStore.tasks.filter((t) => t.dbId != local_task.value.dbId).map(to_select_opt)
})
const predecessors = computed({
    get() {
        return local_task.value.predecessors?.map(to_select_opt) || []
    },
    set(value) {
        local_task.value.predecessors = value.map(from_select_opt).filter((v) => v != undefined)
    }
})
const successors = computed({
    get() {
        return local_task.value.successors?.map(to_select_opt) || []
    },
    set(value) {
        local_task.value.successors = value.map(from_select_opt).filter((v) => v != undefined)
    }
})
const children = computed({
    get() {
        return local_task.value.children?.map(to_select_opt) || []
    },
    set(value) {
        local_task.value.children = value.map(from_select_opt).filter((v) => v != undefined)
    }
})
const parent = computed({
    get() {
        return local_task.value.parent != null ? to_select_opt(local_task.value.parent) : null
    },
    set(value) {
        local_task.value.parent = value != null ? from_select_opt(value) ?? null : null
    }
})


async function save() {
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