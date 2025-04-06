<template>
    <q-dialog ref="dialogRef" @hide="taskStore.reset_task_dialog()"
        :model-value="taskStore.active_task_dialog !== null">
        <q-card class="q-dialog-plugin card-size">
            <q-card-section>
                <div class="row items-center">
                    <q-btn v-if="taskStore.prev_task_dialog != null" flat round icon="arrow_back"
                        @click="taskStore.pop_task_dialog()" />
                    <q-breadcrumbs class="col">
                        <q-breadcrumbs-el disable label="Task" />
                        <q-breadcrumbs-el v-for="p in parents" :key="p.dbId" :label="p.title" :disable="edit"
                            @click="!edit && taskStore.push_task_dialog(p.dbId)" />
                        <q-breadcrumbs-el :label="local_task.title" />
                    </q-breadcrumbs>
                    <q-btn flat @click="toggleEdit()" :loading="taskStore.saving" color="primary"
                        :disable="taskStore.deleting" :icon="edit ? undefined : 'edit'" class="q-ma-xs">{{ edit ? "Save"
                            : null }}
                    </q-btn>
                    <q-btn flat @click="deleteTask()" :loading="taskStore.deleting" color="negative" icon="delete"
                        :disable="taskStore.saving" class="q-ma-xs"></q-btn>
                    <q-btn flat @click="taskStore.reset_task_dialog()" icon="close"></q-btn>
                </div>
            </q-card-section>
            <q-card-section>
                <q-input v-if="edit" outlined placeholder="Title" class="text-h5" v-model="local_task.title" />
                <div v-else class="text-h5">{{ local_task.title }}</div>

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
                <EditableTaskList v-model="local_task.predecessors" name="predecessors" :possible="possiblePredecessors"
                    :edit="edit" />
            </q-card-section>
            <q-card-section>
                <EditableTaskList v-model="local_task.successors" name="successors" :possible="possibleSuccessors"
                    :edit="edit" />
            </q-card-section>
            <q-card-section v-if="edit">
                <q-select filled v-model="parent" :options="possibleParents" use-chips stack-label label="parent" />
            </q-card-section>
            <q-card-section>
                <EditableTaskList v-model="local_task.children" name="children" :possible="possibleChildren"
                    :edit="edit" />
            </q-card-section>
            <q-card-section>
                <DateTimeInput v-if="edit" label="Start" v-model="local_task.earliestStart" />
                <div v-else class="row items-baseline">
                    <div class="text-subtitle2 q-pr-md">Start:</div>
                    <div>{{ format_datetime(local_task.earliestStart) }}</div>
                </div>
            </q-card-section>
            <q-card-section>
                <DateTimeInput v-if="edit" label="Schedule" v-model="local_task.scheduleTarget" />
                <div v-else class="row items-baseline">
                    <div class="text-subtitle2 q-pr-md">Schedule:</div>
                    <div>{{ format_datetime(local_task.scheduleTarget) }}</div>
                </div>
            </q-card-section>
            <q-card-section>
                <q-input v-if="edit" label="effort (days)" stack-label type="number"
                    v-model.number="local_task.effort" />
                <div v-else class="row items-baseline">
                    <div class="text-subtitle2 q-pr-md">Effort:</div>
                    <div>{{ local_task.effort != null ? local_task.effort + " days" : "-" }}</div>
                </div>
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
import { Dialog } from 'quasar'
import MarkdownEditor from './MarkdownEditor.vue';
import { computed, ref, watchEffect } from 'vue';
import { type TaskInput, useTaskStore, type Task } from 'src/stores/task';
import { TaskDesignation } from 'src/gql/graphql';
import EditableTaskList from './EditableTaskList.vue';
import DateTimeInput from './DateTimeInput.vue';
import { format_datetime } from 'src/common/datetime'

const taskStore = useTaskStore();


const local_task_default = { title: "", description: "", designation: TaskDesignation.Task, predecessors: [], successors: [], children: [], parent: null };
const local_task = ref<TaskInput>(local_task_default)
const edit = ref(local_task.value.dbId == null)

watchEffect(() => {
    // task changed
    local_task.value = { ...local_task_default, ...taskStore.active_task_dialog }
    edit.value = local_task.value.dbId == null
})



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
    return taskStore.tasks.filter((t) => t.dbId != local_task.value.dbId)
})
const possibleSuccessors = computed(() => {
    return taskStore.tasks.filter((t) => t.dbId != local_task.value.dbId)
})
const possibleChildren = computed(() => {
    return taskStore.tasks.filter((t) => {
        const parent_ids = parents.value.map((p) => p.dbId);
        return !parent_ids.includes(t.dbId) && local_task.value.dbId != t.dbId
    })
})

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

const possibleParents = computed(() => {
    return taskStore.tasks.filter((t) => t.dbId != local_task.value.dbId).map(to_select_opt)
})
const parent = computed({
    get() {
        return local_task.value.parent != null ? to_select_opt(local_task.value.parent) : null
    },
    set(value) {
        local_task.value.parent = value != null ? from_select_opt(value) ?? null : null
    }
})

// actions

async function toggleEdit() {
    if (edit.value) {
        await save()
    }
    edit.value = !edit.value
}


async function save() {
    await taskStore.save_task(local_task);
}

async function deleteTask() {
    const taskId = local_task.value.dbId
    if (taskId == null) {
        taskStore.pop_task_dialog()
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
    taskStore.pop_task_dialog();
}

</script>