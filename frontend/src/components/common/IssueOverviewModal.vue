<template>
    <q-dialog v-model="dialogModel">
        <q-card style="min-width: 720px; max-width: 90vw; width: 960px;">
            <q-card-section class="row items-center q-pb-none">
                <div class="text-h6">Warnings and errors</div>
                <q-space />
                <q-btn icon="close" flat round dense v-close-popup />
            </q-card-section>

            <q-card-section>
                <div v-if="displayIssues.length === 0" class="text-body2 text-grey-7">
                    No current warnings or errors.
                </div>
                <q-list v-else bordered separator>
                    <q-expansion-item v-for="issue in displayIssues" :key="issue.id" :icon="issueIcon(issue.severity)"
                        :label="issue.summary" :caption="issueCaption(issue)"
                        :header-class="issueHeaderClass(issue.severity)">
                        <q-card flat>
                            <q-card-section class="q-pt-sm q-pb-sm">
                                <div class="text-subtitle2 q-mb-xs">{{ issue.title }}</div>
                                <div class="text-caption text-grey-7 q-mb-sm">{{ issueSource(issue) }}</div>
                                <pre class="issue-details">{{ issue.details }}</pre>
                            </q-card-section>
                        </q-card>
                    </q-expansion-item>
                </q-list>
            </q-card-section>
        </q-card>
    </q-dialog>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useGeneralIssueStore, type DisplayIssue as GeneralDisplayIssue, type IssueSeverity } from 'src/stores/generalIssue';

const props = defineProps<{
    modelValue: boolean;
}>();

const emit = defineEmits<{
    (event: 'update:modelValue', value: boolean): void;
}>();

type DisplayIssue = GeneralDisplayIssue | {
    id: string;
    source: 'planning';
    severity: 'warning';
    title: string;
    summary: string;
    details: string;
    taskId: number | null;
    createdAt: null;
};

const generalIssueStore = useGeneralIssueStore();

const dialogModel = computed({
    get: () => props.modelValue,
    set: (value: boolean) => emit('update:modelValue', value),
});

const displayIssues = computed<DisplayIssue[]>(() => {
    return generalIssueStore.displayIssues
});

function issueIcon(severity: IssueSeverity): string {
    return severity === 'error' ? 'error' : 'warning';
}

function issueHeaderClass(severity: IssueSeverity): string {
    return severity === 'error' ? 'text-negative' : 'text-warning';
}

function issueSource(issue: DisplayIssue): string {
    if (issue.source === 'graphql') {
        return issue.createdAt == null ? 'Frontend GraphQL' : `Frontend GraphQL • ${new Date(issue.createdAt).toLocaleString()}`;
    }
    return issue.taskId == null ? 'Planning • General' : `Planning • Task ${issue.taskId}`;
}

function issueCaption(issue: DisplayIssue): string {
    return issue.severity === 'error' ? 'Error' : 'Warning';
}
</script>

<style scoped>
.issue-details {
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 40vh;
    overflow: auto;
    margin: 0;
}
</style>
