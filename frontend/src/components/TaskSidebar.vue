<template>
    <SidebarLayout>
        <template #toolbar>
            <q-breadcrumbs class="col">
                <q-breadcrumbs-el disable label="Task" />
                <q-breadcrumbs-el v-for="p in parents" :key="p.dbId" :label="p.title" :disable="edit"
                    @click="!edit && sidebarStore.pushSidebar(new TaskSidebarData(p.dbId))" />
                <q-breadcrumbs-el :label="local_task.title" />
            </q-breadcrumbs>
            <q-btn flat @click="toggleEdit()" :loading="taskStore.saving" color="primary" :disable="taskStore.deleting"
                :icon="edit ? 'save' : 'edit'" class="q-ma-xs">
            </q-btn>
            <q-btn v-if="edit" flat round icon="cancel" aria-label="Cancel" class="q-ma-xs" @click="cancelEdit" />
            <q-btn flat @click="deleteTask()" :loading="taskStore.deleting" color="negative" icon="delete"
                :disable="taskStore.saving" class="q-ma-xs"></q-btn>
        </template>
        <q-banner v-if="saveError" dense class="text-white bg-red">{{ saveError }}</q-banner>
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
                    { label: 'Group', value: TaskDesignation.Group },
                    { label: 'Milestone', value: TaskDesignation.Milestone }
                ]" />
            <q-chip v-else color="secondary" text-color="white" class="q-pa-md">{{
                local_task.designation }}</q-chip>
        </q-card-section>

        <q-card-section v-if="local_task.designation == TaskDesignation.Task && taskIssues.length > 0">
            <div class="issue-list">
                <div class="issue-list-title">Issues</div>
                <div v-for="(iss, idx) in taskIssues" :key="idx" class="issue-item">⚠ {{ iss.description }}</div>
            </div>
        </q-card-section>

        <q-card-section v-show="local_task.designation == TaskDesignation.Requirement">
            <DateTimeInput v-if="edit" label="Start" v-model="local_task.earliestStart" />
            <div v-else class="row items-baseline">
                <div class="text-subtitle2 q-pr-md">Start:</div>
                <div>{{ formatDatetime(local_task.earliestStart) }}</div>
            </div>
        </q-card-section>
        <q-card-section v-show="local_task.designation == TaskDesignation.Milestone">
            <DateTimeInput v-if="edit" label="Schedule" v-model="local_task.scheduleTarget" />
            <div v-else class="row items-baseline">
                <div class="text-subtitle2 q-pr-md">Schedule:</div>
                <div>{{ formatDatetime(local_task.scheduleTarget) }}</div>
            </div>
        </q-card-section>

        <q-card-section v-show="[TaskDesignation.Task, TaskDesignation.Group].includes(local_task.designation)">
            <div class="q-gutter-y-sm">
                <div v-for="(option, idx) in resourceConstraints" :key="idx" class="row items-center q-gutter-sm">
                    <div class="col">
                        <EditableResourceList v-model="option.resources"
                            :name="`Resource Constraint ${idx + 1}${(local_task.resourceConstraints || []).length == 0 ? ' (inherited)' : ''}`"
                            :possible="allResources" :edit="edit" class="full-width" />
                        <div class="row q-gutter-sm items-center">
                            <q-checkbox v-if="edit" v-model="option.optional" label="Optional" />
                            <div v-else class="q-ml-md text-subtitle2">
                                {{ option.optional ? "Optional" : "Required" }}
                            </div>
                            <q-input v-if="edit" v-model.number="option.speed" type="number" min="0" step="0.1"
                                label="Speed" dense style="max-width: 120px;" />
                            <div v-else class="q-ml-lg text-subtitle2">
                                Speed: {{ option.speed.toFixed(2) }}
                            </div>
                        </div>
                    </div>
                    <q-btn flat round v-show="edit" icon="remove" color="negative" @click="removeResourceSlot(idx)" />
                </div>
                <q-btn flat v-show="edit" icon="add" color="primary" label="Add Resource Constraint"
                    @click="addResourceSlot" />
            </div>
        </q-card-section>

        <q-card-section v-show="local_task.designation == TaskDesignation.Task">
            <q-input v-if="edit" label="effort (days)" stack-label type="number" v-model.number="local_task.effort" />
            <div v-else class="row items-baseline">
                <div class="text-subtitle2 q-pr-md">Effort:</div>
                <div>{{ local_task.effort != null ? local_task.effort + " days" : "-" }}</div>
            </div>
        </q-card-section>
        <q-card-section
            v-show="local_task.designation != TaskDesignation.Requirement && ((local_task.predecessors?.length ?? 0) > 0 || edit)">
            <EditableTaskList v-model="local_task.predecessors" name="predecessors" :possible="possiblePredecessors"
                :edit="edit" />
        </q-card-section>
        <q-card-section
            v-show="local_task.designation != TaskDesignation.Milestone && ((local_task.successors?.length ?? 0) > 0 || edit)">
            <EditableTaskList v-model="local_task.successors" name="successors" :possible="possibleSuccessors"
                :edit="edit" />
        </q-card-section>
        <q-card-section v-show="edit">
            <q-select filled v-model="parent" :options="possibleParents" use-chips stack-label label="parent" />
        </q-card-section>
        <q-card-section
            v-show="local_task.designation == TaskDesignation.Group && ((local_task.children?.length ?? 0) > 0 || edit)">
            <EditableTaskList v-model="local_task.children" name="children" :possible="possibleChildren" :edit="edit" />
        </q-card-section>
        <q-card-section v-show="effective_requirements.length > 0">
            <div class="col">
                <div class="text-subtitle2">Requirements</div>
                <TaskChip v-for="task in effective_requirements" :clickable="!edit" :key="task.dbId" :task="task" />
            </div>
        </q-card-section>
        <q-card-section v-show="effective_milestones.length > 0">
            <div class="col">
                <div class="text-subtitle2">Milestones</div>
                <TaskChip v-for="task in effective_milestones" :clickable="!edit" :key="task.dbId" :task="task" />
            </div>
        </q-card-section>
        <q-card-section v-show="local_task.designation == TaskDesignation.Task && !edit">
            <div class="col">
                <div class="text-subtitle2">Bookings</div>
                <div v-for="(b, idx) in taskBookings()" :key="b.dbId || idx" class="q-pa-sm"
                    style="border:1px solid #eee;border-radius:6px;margin-bottom:6px;">
                    <div class="row items-center q-gutter-sm" style="align-items: center;">
                        <q-btn flat dense icon="delete" color="negative" @click="() => deleteBookingLocal(b)" />
                        <q-checkbox dense v-model="b.final" label="Final"
                            @update:modelValue="() => saveBookingLocal(b)" />
                        <div class="col resource-list">
                            <EditableResourceList :name="`Resources`" v-model="b.resources" :possible="allResources"
                                :edit="true" @update:modelValue="() => saveBookingLocal(b)" />
                        </div>
                        <DateTimeInput v-model="b.start" label="Start" :maxWidth="218"
                            @update:modelValue="() => saveBookingLocal(b)" />
                        <DateTimeInput v-model="b.end" label="End" :maxWidth="218"
                            @update:modelValue="() => saveBookingLocal(b)" />
                    </div>
                </div>
                <div>
                    <q-btn flat icon="add" label="Add Booking" color="primary" @click="createBooking" />
                </div>
            </div>

        </q-card-section>


    </SidebarLayout>
</template>


<script setup lang="ts">
import { Dialog } from 'quasar';
import { formatDatetime } from 'src/common/datetime';
import { TaskDesignation } from 'src/gql/graphql';
import { TaskSidebarData, useSidebarStore } from 'src/stores/sidebar';
import { useResourceStore } from 'src/stores/resource';
import { useTaskStore, type Task, type TaskInput } from 'src/stores/task';
import { computed, ref, watchEffect } from 'vue';
import { type Issue, useIssueStore } from 'src/stores/issue';
import DateTimeInput from './DateTimeInput.vue';
import SidebarLayout from './SidebarLayout.vue';
import EditableResourceList from './EditableResourceList.vue';
import EditableTaskList from './EditableTaskList.vue';
import MarkdownEditor from './MarkdownEditor.vue';
import TaskChip from './TaskChip.vue';
import { usePlanStore, type Allocation } from 'src/stores/plan';

const taskStore = useTaskStore();
const sidebarStore = useSidebarStore();
const resourceStore = useResourceStore();
const planStore = usePlanStore();

const local_task_default = { title: "", description: "", designation: TaskDesignation.Task, predecessors: [], successors: [], children: [], parent: null, resourceConstraints: [] };
const local_task = ref<TaskInput>(local_task_default)
const edit = ref(local_task.value.dbId == null)


interface Props {
    task: TaskInput;
};

const props = defineProps<Props>();

watchEffect(() => {
    // task changed
    local_task.value = { ...local_task_default, ...props.task }
    edit.value = local_task.value.dbId == null
})



const parents = computed(() => {
    const parents = [];
    let parent = local_task.value.parent;
    while (parent != null && parent.dbId != local_task.value.dbId) {
        parents.push(parent)
        parent = parent.parent
    }
    return parents.reverse()
})

const possiblePredecessors = computed(() => {
    return taskStore.tasks.filter((t) => t.dbId != local_task.value.dbId && t.designation != TaskDesignation.Milestone)
})
const possibleSuccessors = computed(() => {
    return taskStore.tasks.filter((t) => t.dbId != local_task.value.dbId && t.designation != TaskDesignation.Requirement)
})
const possibleChildren = computed(() => {
    return taskStore.tasks.filter((t) => {
        const parent_ids = parents.value.map((p) => p.dbId);
        return !parent_ids.includes(t.dbId) && local_task.value.dbId != t.dbId
    })
})

const resourceConstraints = computed(() => {
    {
        let result = local_task.value.resourceConstraints ?? []
        if (!edit.value) {
            let parent = local_task.value.parent;
            while (result.length < 1 && parent != null) {
                result = parent.resourceConstraints;
                parent = parent.parent;
            }
        }
        return result;
    }
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
    return taskStore.tasks.filter((t) => t.dbId != local_task.value.dbId && t.designation == TaskDesignation.Group).map(to_select_opt)
})
const parent = computed({
    get() {
        return local_task.value.parent != null ? to_select_opt(local_task.value.parent) : null
    },
    set(value) {
        local_task.value.parent = value != null ? from_select_opt(value) ?? null : null
    }
})

function _get_milestones(task: Partial<Task>, seen: Set<number>): Set<Task> {
    let result: Set<Task> = new Set([])
    if (task.dbId) {
        if (seen.has(task.dbId)) {
            return result;
        }
        seen.add(task.dbId)
    }
    if (task.designation == TaskDesignation.Milestone && task.dbId != null) {
        const store_task = taskStore.task(task.dbId)
        if (store_task != null) { result.add(store_task) }
    }
    if (task.parent != null) {
        result = result.union(_get_milestones(task.parent, seen))
    }
    for (const suc of task.successors ?? []) {
        result = result.union(_get_milestones(suc, seen))
    }
    return result
}

const effective_milestones = computed(() => {
    const result = Array.from(_get_milestones(local_task.value, new Set())).filter((t) => t.dbId != local_task.value.dbId);
    result.sort((lhs, rhs) => lhs.title < rhs.title ? -1 : lhs.title > rhs.title ? 1 : 0)
    return result
})

function _get_requirements(task: Partial<Task>, seen: Set<number>): Set<Task> {
    let result: Set<Task> = new Set([])
    if (task.dbId) {
        if (seen.has(task.dbId)) {
            return result;
        }
        seen.add(task.dbId)
    }
    if (task.designation == TaskDesignation.Requirement && task.dbId != null) {
        const store_task = taskStore.task(task.dbId)
        if (store_task != null) { result.add(store_task) }
    }
    if (task.parent != null) {
        result = result.union(_get_requirements(task.parent, seen))
    }
    for (const pre of task.predecessors ?? []) {
        result = result.union(_get_requirements(pre, seen))
    }
    return result
}

const effective_requirements = computed(() => {
    const result = Array.from(_get_requirements(local_task.value, new Set())).filter((t) => t.dbId != local_task.value.dbId);
    result.sort((lhs, rhs) => lhs.title < rhs.title ? -1 : lhs.title > rhs.title ? 1 : 0)
    return result
})

// actions

const saveError = ref<string | null>(null)

async function toggleEdit() {
    if (edit.value) {
        const err = await save()
        saveError.value = err
        if (!err) edit.value = false
    }
    else {
        saveError.value = null
        edit.value = true
    }
}

function cancelEdit() {
    // reset local values from props
    local_task.value = { ...local_task_default, ...props.task };
    saveError.value = null;
    edit.value = false;
}


async function save(): Promise<string | null> {
    // reset error before saving
    saveError.value = null
    const err = await taskStore.saveTask(local_task)
    return err
}

async function deleteTask() {
    const taskId = local_task.value.dbId
    if (taskId == null) {
        sidebarStore.popSidebar()
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
    await taskStore.deleteTask(taskId, true);
}

const allResources = computed(() => resourceStore.resources);

const issueStore = useIssueStore();
const taskIssues = computed(() => {
    const tid = local_task.value.dbId;
    if (tid == null) return [] as Issue[];
    return issueStore.issues.filter((i) => i.taskId === tid);
});


function taskBookings(): Allocation[] {
    const tid = local_task.value.dbId;
    if (tid == null) return [];
    return planStore.bookingsByTask(tid);
}

function addResourceSlot() {
    if (!local_task.value.resourceConstraints) local_task.value.resourceConstraints = [];
    local_task.value.resourceConstraints.push({ resources: [], optional: false, speed: 1 });
}
function removeResourceSlot(idx: number) {
    if (!local_task.value.resourceConstraints) return;
    local_task.value.resourceConstraints.splice(idx, 1);
}

async function saveBookingLocal(b: Allocation) {
    // delegate to plan store
    await planStore.saveBooking(b);
}

async function deleteBookingLocal(b: Allocation) {
    await planStore.deleteBooking(b.dbId);
}

function createBooking() {
    void planStore.createBookingFromPlan(local_task.value.dbId ?? null);
}

// ...existing code...
</script>

<style scoped>
.issue-list {
    background: #fff4b1;
    padding: 8px;
    border-radius: 6px;
}

.issue-list-title {
    font-weight: bold;
    margin-bottom: 6px;
}

.issue-item {
    padding: 4px 0;
}

.resource-list {
    min-width: 200px;
}
</style>