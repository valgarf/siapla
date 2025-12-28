<template>
    <SidebarLayout>
        <template #toolbar>
            <div class="col">
                <q-breadcrumbs>
                    <q-breadcrumbs-el disable label="Tasks" />
                    <q-breadcrumbs-el v-for="p in recursiveParents" :key="p.dbId" :label="p.title" :disable="edit"
                        class="clickable" @click="!edit && sidebarStore.pushSidebar(new TaskSidebarData(p.dbId))" />
                    <q-breadcrumbs-el disable label="" />
                </q-breadcrumbs>
            </div>
            <q-btn flat @click="toggleEdit()" :loading="taskStore.saving" color="primary" :disable="taskStore.deleting"
                :icon="edit ? 'save' : 'edit'" class="q-ma-xs" :class="{ shake: sidebarStore.shakeButtons }" />

            <q-btn v-if="edit && localTask.dbId != null" flat round icon="cancel" aria-label="Cancel" class="q-ma-xs"
                @click="cancelEdit" :class="{ shake: sidebarStore.shakeButtons }" />
            <q-btn flat @click="deleteTask()" :loading="taskStore.deleting" color="negative" icon="delete"
                :disable="taskStore.saving" class="q-ma-xs" :class="{ shake: sidebarStore.shakeButtons }"></q-btn>
        </template>
        <q-banner v-if="saveError" dense class="text-white bg-red">{{ saveError }}</q-banner>

        <div class="sidebar-content">

            <section class="sidebar-section">
                <q-btn-toggle v-if="edit" v-model="localTask.designation" rounded toggle-color="secondary"
                    class="q-mb-sm" text-color="secondary" color="white" :options="[
                        { label: 'Requirement', value: TaskDesignation.Requirement },
                        { label: 'Task', value: TaskDesignation.Task },
                        { label: 'Group', value: TaskDesignation.Group },
                        { label: 'Milestone', value: TaskDesignation.Milestone }
                    ]" />
                <q-input v-if="edit" outlined placeholder="Title" class="text-h5 responsive-field q-mb-sm"
                    v-model="localTask.title" />
                <div v-else class="text-h5 q-mb-sm">{{ localTask.title }}
                    <q-chip color="secondary" text-color="white" class="q-pa-md q-ml-sm">{{ localTask.designation
                    }}</q-chip>
                </div>
                <MarkdownEditor v-if="edit" placeholder="description" v-model="localTask.description" />
                <q-markdown v-else-if="localTask.description" :src="localTask.description" />
                <div v-else><i>No description</i></div>
            </section>

            <section v-if="localTask.designation == TaskDesignation.Task && taskIssues.length > 0"
                class="sidebar-section">
                <div class="issue-list">
                    <div class="issue-list-title">Issues</div>
                    <div v-for="(iss, idx) in taskIssues" :key="idx" class="issue-item">⚠ {{ iss.description }}</div>
                </div>
            </section>

            <section v-show="localTask.designation == TaskDesignation.Requirement"
                class="sidebar-section responsive-row">
                <DateTimeInput v-if="edit" label="Start" class="responsive-field" v-model="localTask.earliestStart" />
                <div v-else class="row items-baseline">
                    <div class="text-subtitle2 q-pr-md">Start:</div>
                    <div>{{ formatDatetime(localTask.earliestStart) }}</div>
                </div>
            </section>
            <section v-show="localTask.designation == TaskDesignation.Milestone" class="sidebar-section responsive-row">
                <DateTimeInput v-if="edit" label="Schedule" class="responsive-field"
                    v-model="localTask.scheduleTarget" />
                <q-input v-if="edit" label="Priority" type="number" step="0.01" min="0" class="responsive-field"
                    v-model.number="localTask.priority" />
                <div v-if="!edit" class="row items-baseline">
                    <div class="text-subtitle2 q-pr-md">Schedule:</div>
                    <div>{{ formatDatetime(localTask.scheduleTarget) }}</div>
                </div>
                <div v-if="!edit" class="row items-baseline">
                    <div class="text-subtitle2 q-pr-md">Priority:</div>
                    <div>{{ localTask.priority.toFixed(2) }}</div>
                </div>
            </section>

            <section v-show="[TaskDesignation.Task, TaskDesignation.Group].includes(localTask.designation)"
                class="sidebar-section">
                <div class="q-gutter-y-sm">
                    <div v-for="(option, idx) in resourceConstraints" :key="idx" class="row items-center q-gutter-sm">
                        <div class="col">
                            <EditableResourceList v-model="option.resources"
                                :name="`Resource Constraint ${idx + 1}${(localTask.resourceConstraints || []).length == 0 ? ' (inherited)' : ''}`"
                                :possible="allResources" :edit="edit" class="full-width" />
                            <div class="row q-gutter-sm items-center responsive-row">
                                <q-checkbox v-if="edit" v-model="option.optional" label="Optional" />
                                <div v-else class="q-ml-md text-subtitle2">{{ option.optional ? "Optional" : "Required"
                                }}
                                </div>
                                <q-input v-if="edit" v-model.number="option.speed" type="number" min="0" step="0.1"
                                    label="Speed" dense class="responsive-field" style="max-width: 160px;" />
                                <div v-else class="q-ml-lg text-subtitle2">Speed: {{ option.speed.toFixed(2) }}</div>
                            </div>
                        </div>
                        <q-btn flat round v-show="edit" icon="remove" color="negative"
                            @click="removeResourceSlot(idx)" />
                    </div>
                    <q-btn flat v-show="edit" icon="add" color="primary" label="Add Resource Constraint"
                        @click="addResourceSlot" />
                </div>
            </section>

            <section v-show="localTask.designation == TaskDesignation.Task" class="sidebar-section responsive-row">
                <q-input v-if="edit" label="effort (days)" stack-label type="number" class="responsive-field"
                    v-model.number="localTask.effort" />
                <div v-else class="row items-baseline">
                    <div class="text-subtitle2 q-pr-md">Effort:</div>
                    <div>{{ localTask.effort != null ? localTask.effort + " days" : "-" }}</div>
                </div>
            </section>
            <section class="sidebar-section column q-gutter-xs">
                <EditableTaskList v-model="localTask.predecessors" label="predecessors" :possible="possiblePredecessors"
                    v-show="localTask.designation != TaskDesignation.Requirement && ((localTask.predecessors?.length ?? 0) > 0 || edit)"
                    :edit="edit" />
                <EditableTaskList v-model="localTask.successors" label="successors" :possible="possibleSuccessors"
                    v-show="localTask.designation != TaskDesignation.Milestone && ((localTask.successors?.length ?? 0) > 0 || edit)"
                    :edit="edit" />
                <TaskSelect v-show="edit" v-model="localTask.parent" :possible="possibleParents" label="parent" />
                <EditableTaskList v-model="localTask.children" label="children" :possible="possibleChildren"
                    v-show="localTask.designation == TaskDesignation.Group && ((localTask.children?.length ?? 0) > 0 || edit)"
                    :edit="edit" />
                <div class="col" v-show="effectiveRequirements.length > 0">
                    <div class="text-subtitle2">Requirements</div>
                    <TaskChip v-for="task in effectiveRequirements" :clickable="!edit" :key="task.dbId" :task="task" />
                </div>
                <div class="col" v-show="effectiveMilestones.length > 0">
                    <div class="text-subtitle2">Milestones</div>
                    <TaskChip v-for="task in effectiveMilestones" :clickable="!edit" :key="task.dbId" :task="task" />
                </div>
            </section>
            <section v-show="localTask.designation == TaskDesignation.Task && !edit"
                class="sidebar-section bookings-section">
                <div class="col">
                    <div class="text-subtitle2">Bookings</div>
                    <div class="bookings-row">
                        <div v-for="(b, idx) in taskBookings()" :key="b.dbId || idx" class="booking-box">
                            <q-btn flat dense icon="delete" color="negative" @click="() => deleteBookingLocal(b)" />
                            <q-checkbox dense v-model="b.final" label="Final"
                                @update:modelValue="() => saveBookingLocal(b)" />
                            <div class="booking-resources">
                                <EditableResourceList :name="`Resources`" v-model="b.resources" :possible="allResources"
                                    :edit="true" @update:modelValue="() => saveBookingLocal(b)" />
                            </div>
                            <DateTimeInput :modelValue="b.start" label="Start" :maxWidth="218" class="responsive-field"
                                @update:modelValue="(start) => saveBookingLocal(b, start, null)" />
                            <DateTimeInput :modelValue="b.end" label="End" :maxWidth="218" class="responsive-field"
                                @update:modelValue="(end) => saveBookingLocal(b, null, end)" />
                        </div>
                    </div>
                    <div>
                        <q-btn flat icon="add" label="Add Booking" color="primary" @click="createBooking" />
                    </div>
                </div>
            </section>

        </div>
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
import DateTimeInput from '../forms/DateTimeInput.vue';
import SidebarLayout from './SidebarLayout.vue';
import EditableResourceList from '../forms/EditableResourceList.vue';
import EditableTaskList from '../forms/EditableTaskList.vue';
import MarkdownEditor from '../forms/MarkdownEditor.vue';
import TaskChip from '../common/TaskChip.vue';
import TaskSelect from '../forms/TaskSelect.vue';
import { usePlanStore, type Allocation } from 'src/stores/plan';

const taskStore = useTaskStore();
const sidebarStore = useSidebarStore();
const resourceStore = useResourceStore();
const planStore = usePlanStore();

const localTaskDefault = { title: "", description: "", designation: TaskDesignation.Task, predecessors: [], successors: [], children: [], parent: null, resourceConstraints: [], priority: 1.0 };
const localTask = ref<TaskInput>(localTaskDefault)
const edit = ref(localTask.value.dbId == null)


interface Props {
    task: TaskInput;
};

const props = defineProps<Props>();

watchEffect(() => {
    // task changed
    localTask.value = { ...localTaskDefault, ...props.task }
    edit.value = localTask.value.dbId == null
})



const recursiveParents = computed(() => {
    const parents = [];
    let parent = localTask.value.parent;
    while (parent != null && parent.dbId != localTask.value.dbId) {
        parents.push(parent)
        parent = parent.parent
    }
    return parents.reverse()
})

function _sortByTitle(t1: Task, t2: Task): number {
    return t1.title.localeCompare(t2.title, undefined, { sensitivity: "accent" })
}

const possiblePredecessors = computed(() => {
    const excludeIds = new Set(recursiveSuccessors.value.map((t) => t.dbId))
    return taskStore.tasks.filter((t) => t.dbId != localTask.value.dbId && t.designation != TaskDesignation.Milestone && !excludeIds.has(t.dbId)).sort(_sortByTitle)
})
const possibleSuccessors = computed(() => {
    const excludeIds = new Set(recursivePredecessors.value.map((t) => t.dbId))
    return taskStore.tasks.filter((t) => t.dbId != localTask.value.dbId && t.designation != TaskDesignation.Requirement && !excludeIds.has(t.dbId)).sort(_sortByTitle)
})
const possibleParents = computed(() => {
    const excludeIds = new Set(recursiveChildren.value.map((t) => t.dbId))
    return taskStore.tasks.filter((t) => t.dbId != localTask.value.dbId && t.designation == TaskDesignation.Group && !excludeIds.has(t.dbId)).sort(_sortByTitle)
})
const possibleChildren = computed(() => {
    const excludeIds = recursiveParents.value.map((p) => p.dbId);
    return taskStore.tasks.filter((t) => localTask.value.dbId != t.dbId && !excludeIds.includes(t.dbId)).sort(_sortByTitle)
})

const resourceConstraints = computed(() => {
    {
        let result = localTask.value.resourceConstraints ?? []
        if (!edit.value) {
            let parent = localTask.value.parent;
            while (result.length < 1 && parent != null) {
                result = parent.resourceConstraints;
                parent = parent.parent;
            }
        }
        return result;
    }
})

const recursiveChildren = computed(() => {
    const result = Array.from(_getChildren(localTask.value, new Set())).filter((t) => t.dbId != localTask.value.dbId)
    return result
})

const recursivePredecessors = computed(() => {
    const result = Array.from(_getRecursivePredecessors(localTask.value, new Set())).filter((t) => t.dbId != localTask.value.dbId)
    return result
})

const recursiveSuccessors = computed(() => {
    const result = Array.from(_getRecursiveSuccessors(localTask.value, new Set())).filter((t) => t.dbId != localTask.value.dbId)
    return result
})

function _getMilestones(task: Partial<Task>, seen: Set<number>): Set<Task> {
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
        result = result.union(_getMilestones(task.parent, seen))
    }
    for (const suc of task.successors ?? []) {
        result = result.union(_getMilestones(suc, seen))
    }
    return result
}

function _getChildren(task: Partial<Task>, seen: Set<number>): Set<Task> {
    let result: Set<Task> = new Set([])
    if (task.dbId) {
        if (seen.has(task.dbId)) {
            return result;
        }
        seen.add(task.dbId)
    }
    for (const ch of task.children ?? []) {
        result = result.union(_getChildren(ch, seen))
        if (ch.dbId != null) {
            const store_task = taskStore.task(ch.dbId)
            if (store_task != null) { result.add(store_task) }
        }
    }
    return result
}

function _getRecursivePredecessors(task: Partial<Task>, seen: Set<number>): Set<Task> {
    let result: Set<Task> = new Set([])
    if (task.dbId) {
        if (seen.has(task.dbId)) {
            return result;
        }
        seen.add(task.dbId)
    }
    for (const pre of task.predecessors ?? []) {
        // Recurse into the predecessor itself
        result = result.union(_getRecursivePredecessors(pre, seen))
        // If the predecessor is a group, include all its children and their recursive predecessors
        if (pre.designation == TaskDesignation.Group) {
            for (const ch of pre.children ?? []) {
                result.add(ch)
                result = result.union(_getRecursivePredecessors(ch, seen))
            }
        }
        // Finally add the predecessor itself
        if (pre.dbId != null) {
            const store_task = taskStore.task(pre.dbId)
            if (store_task != null) { result.add(store_task) }
        }
    }
    return result
}

function _getRecursiveSuccessors(task: Partial<Task>, seen: Set<number>): Set<Task> {
    let result: Set<Task> = new Set([])
    if (task.dbId) {
        if (seen.has(task.dbId)) {
            return result;
        }
        seen.add(task.dbId)
    }
    for (const suc of task.successors ?? []) {
        // Recurse into the successor itself
        result = result.union(_getRecursiveSuccessors(suc, seen))
        // If the successor is a group, include all its children and their recursive successors
        if (suc.designation == TaskDesignation.Group) {
            for (const ch of suc.children ?? []) {
                result = result.union(_getRecursiveSuccessors(ch, seen))
                result.add(ch)
            }
        }
        // Finally add the successor itself
        if (suc.dbId != null) {
            const store_task = taskStore.task(suc.dbId)
            if (store_task != null) { result.add(store_task) }
        }
    }
    return result
}

const effectiveMilestones = computed(() => {
    const result = Array.from(_getMilestones(localTask.value, new Set())).filter((t) => t.dbId != localTask.value.dbId);
    result.sort((lhs, rhs) => lhs.title < rhs.title ? -1 : lhs.title > rhs.title ? 1 : 0)
    return result
})

function _getRequirements(task: Partial<Task>, seen: Set<number>): Set<Task> {
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
        result = result.union(_getRequirements(task.parent, seen))
    }
    for (const pre of task.predecessors ?? []) {
        result = result.union(_getRequirements(pre, seen))
    }
    return result
}

const effectiveRequirements = computed(() => {
    const result = Array.from(_getRequirements(localTask.value, new Set())).filter((t) => t.dbId != localTask.value.dbId);
    result.sort((lhs, rhs) => lhs.title < rhs.title ? -1 : lhs.title > rhs.title ? 1 : 0)
    return result
})

// actions

const saveError = ref<string | null>(null)

async function toggleEdit() {
    if (edit.value) {
        const err = await save()
        saveError.value = err
        if (!err) {
            edit.value = false
            sidebarStore.setEditing(false)
        }
    }
    else {
        saveError.value = null
        edit.value = true
        sidebarStore.setEditing(true)
    }
}

function cancelEdit() {
    // reset local values from props
    localTask.value = { ...localTaskDefault, ...props.task };
    saveError.value = null;
    edit.value = false;
    sidebarStore.cancelEdit();
    sidebarStore.setEditing(false)
}


async function save(): Promise<string | null> {
    // reset error before saving
    saveError.value = null
    const err = await taskStore.saveTask(localTask)
    return err
}

async function deleteTask() {
    const taskId = localTask.value.dbId
    if (taskId == null) {
        cancelEdit()
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
    const tid = localTask.value.dbId;
    if (tid == null) return [] as Issue[];
    return issueStore.issues.filter((i) => i.taskId === tid);
});


function taskBookings(): Allocation[] {
    const tid = localTask.value.dbId;
    if (tid == null) return [];
    return planStore.bookingsByTask(tid);
}

function addResourceSlot() {
    if (!localTask.value.resourceConstraints) localTask.value.resourceConstraints = [];
    localTask.value.resourceConstraints.push({ resources: [], optional: false, speed: 1 });
}
function removeResourceSlot(idx: number) {
    if (!localTask.value.resourceConstraints) return;
    localTask.value.resourceConstraints.splice(idx, 1);
}

async function saveBookingLocal(b: Allocation, overwriteStart: Date | null = null, overwriteEnd: Date | null = null) {
    // delegate to plan store
    if (overwriteStart != null || overwriteEnd != null) {
        const s = overwriteStart || b.start
        const e = overwriteEnd || b.end
        if (Math.abs(e.getTime() - s.getTime()) <= 365 * 24 * 3600 * 1000) {
            b.start = s
            b.end = e
        }
    }
    await planStore.saveBooking(b);
}

async function deleteBookingLocal(b: Allocation) {
    await planStore.deleteBooking(b.dbId);
}

function createBooking() {
    void planStore.createBookingFromPlan(localTask.value.dbId ?? null);
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

/* Layout helpers for expanded sidebar - flex based, no breakpoint required */
.sidebar-content {
    display: flex;
    flex-flow: column nowrap;
    gap: 0px;
    align-items: stretch;
}

.sidebar-section {
    padding: 12px 12px;
    border-bottom: 1px solid #f0f0f0;
    /* flex: 1 1 320px; */
    /* min-width: 220px; */
    /* max-width: 720px; */
}

.responsive-field {
    width: 100%;
    max-width: 520px;
    box-sizing: border-box;
}

.responsive-row {
    display: flex;
    gap: 12px;
    align-items: center;
    flex-wrap: wrap;
}

.bookings-row {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
    align-items: flex-start;
}

.booking-box {
    display: flex;
    gap: 8px;
    flex-flow: row wrap;
    align-items: center;
    padding: 10px;
    border: 1px solid #eee;
    border-radius: 6px;
}

.booking-resources {
    min-width: 160px;
    max-width: 360px;
}

.clickable {
    cursor: pointer;
}

/* shake animation for blocked save/cancel */
@keyframes shake {

    10%,
    90% {
        transform: translate3d(-1px, 0, 0);
    }

    20%,
    80% {
        transform: translate3d(2px, 0, 0);
    }

    30%,
    50%,
    70% {
        transform: translate3d(-4px, 0, 0);
    }

    40%,
    60% {
        transform: translate3d(4px, 0, 0);
    }
}

.shake {
    animation: shake 0.6s;
}
</style>