<template>
    <q-card v-ripple :class="['cursor-pointer q-hoverable overflow-hidden', issueClass]" @click="showDetails()">
        <div tabindex="-1" class="q-focus-helper"></div>
        <q-card-section class="q-pb-xs">
            <q-breadcrumbs class="col" :class="{ invisible: parents.length == 0 }">
                <q-breadcrumbs-el v-for="p in parents" :key="p.dbId" :label="p.title" />
                <q-breadcrumbs-el label="" />
                <q-breadcrumbs-el label="" v-if="parents.length == 0" />
            </q-breadcrumbs>
            <div class="text-subtitle1">{{ task.title }}</div>
            <div v-if="taskIssues.length > 0" class="issue-banner">⚠ {{ taskIssues.length }} issue(s)</div>
            <q-chip color="secondary" text-color="white" class="text-caption q-pa-sm q-ma-none">{{
                task.designation }}
            </q-chip>
        </q-card-section>
        <q-card-section class="q-pt-none q-pb-xs">
            <q-markdown :src="task.description" />
        </q-card-section>
    </q-card>
</template>

<script setup lang="ts">
import type { Task } from 'src/stores/task';
import { TaskDialogData, useDialogStore } from 'src/stores/dialog';
import { computed } from 'vue';
import { type Issue, useIssueStore } from 'src/stores/issue';

interface Props {
    task: Task;
};

const props = withDefaults(defineProps<Props>(), {});
const dialogStore = useDialogStore();


const parents = computed(() => {
    const parents = [];
    let parent = props.task.parent;
    while (parent != null) {
        parents.push(parent)
        parent = parent.parent
    }
    return parents.reverse()
})

const issueStore = useIssueStore();
const taskIssues = computed(() => issueStore.issues.filter((i: Issue) => i.taskId === props.task.dbId));
const issueClass = computed(() => taskIssues.value.length > 0 ? 'task-has-issue' : '');

function showDetails() {
    dialogStore.pushDialog(new TaskDialogData(props.task.dbId))
}

</script>

<style scoped>
.task-has-issue {
    background: #fff3bf;
}

.issue-banner {
    background: #fff4b1;
    padding: 4px 8px;
    border-radius: 4px;
    margin-top: 6px;
    color: #6a4b00;
}
</style>