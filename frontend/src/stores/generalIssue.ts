import { acceptHMRUpdate, defineStore } from 'pinia';
import { computed } from 'vue';
import type { ApolloError } from '@apollo/client/core';
import { print, type DocumentNode } from 'graphql';
import { usePlanStore } from './plan';
import { useResourceStore } from './resource';
import { useTaskStore } from './task';
import { usePlanIssueStore } from './planIssue';

export type IssueSeverity = 'warning' | 'error';

export type DisplayIssue = {
    id: string;
    source: 'graphql';
    severity: IssueSeverity;
    title: string;
    summary: string;
    details: string;
    createdAt: string | null;
}

function stringifyMaybeJson(value: unknown): string {
    if (value == null) {
        return '';
    }
    if (typeof value === 'string') {
        return value;
    }
    try {
        return JSON.stringify(value, null, 2);
    } catch {
        return Object.prototype.toString.call(value);
    }
}

function extractApolloMessage(error: unknown): string {
    if (error == null) {
        return 'Unknown GraphQL error';
    }
    if (typeof error === 'string') {
        return error;
    }
    if (typeof error === 'object' && 'message' in error && typeof error.message === 'string') {
        return error.message;
    }
    return Object.prototype.toString.call(error);
}

function formatApolloDetails(error: unknown): string {
    const apolloError = error as ApolloError | null;
    if (apolloError == null || typeof apolloError !== 'object') {
        return stringifyMaybeJson(error);
    }

    const graphQLErrors = 'graphQLErrors' in apolloError ? stringifyMaybeJson(apolloError.graphQLErrors) : '';
    const networkError = 'networkError' in apolloError ? stringifyMaybeJson(apolloError.networkError) : '';
    const cause = 'cause' in apolloError ? stringifyMaybeJson(apolloError.cause) : '';

    const parts = [
        `message: ${extractApolloMessage(apolloError)}`,
        graphQLErrors ? `graphQLErrors:\n${graphQLErrors}` : '',
        networkError ? `networkError:\n${networkError}` : '',
        cause ? `cause:\n${cause}` : '',
    ].filter((p) => p.length > 0);

    return parts.join('\n\n');
}

function extractQueryText(queryDocument: unknown): string {
    if (queryDocument == null || typeof queryDocument !== 'object') {
        return '';
    }

    const loc = 'loc' in queryDocument ? queryDocument.loc : undefined;
    const source = loc != null && typeof loc === 'object' && 'source' in loc ? loc.source : undefined;
    const body = source != null && typeof source === 'object' && 'body' in source ? source.body : undefined;
    if (typeof body === 'string' && body.length > 0) {
        return body;
    }

    try {
        return print(queryDocument as DocumentNode);
    } catch {
        return stringifyMaybeJson(queryDocument);
    }
}

function formatGraphqlContext(queryDocument: unknown, variables: unknown): string {
    const queryText = extractQueryText(queryDocument);
    const variablesText = stringifyMaybeJson(variables);

    const parts = [
        queryText ? `query:\n${queryText}` : '',
        `variables:\n${variablesText || '{}'}`,
    ].filter((p) => p.length > 0);

    return parts.join('\n\n');
}

function toDisplayIssue(source: string, error: unknown, queryDocument: unknown, variables: unknown): DisplayIssue {
    const details = [
        formatApolloDetails(error),
        formatGraphqlContext(queryDocument, variables),
    ].filter((part) => part.length > 0).join('\n\n');

    return {
        id: `graphql-${source}`,
        source: 'graphql',
        severity: 'error',
        title: `GraphQL failure in ${source}`,
        summary: extractApolloMessage(error),
        details,
        createdAt: null,
    };
}

export const useGeneralIssueStore = defineStore('generalIssueStore', () => {
    const planStore = usePlanStore();
    const taskStore = useTaskStore();
    const resourceStore = useResourceStore();
    const planIssueStore = usePlanIssueStore();

    const graphqlIssues = computed<DisplayIssue[]>(() => {
        const entries: Array<{ source: string; error: unknown; queryDocument: unknown; variables: unknown }> = [];
        const planError = planStore.gql.queryGetAll.error;
        if (planError != null) {
            entries.push({
                source: 'plan.queryGetAll',
                error: planError,
                queryDocument: planStore.gql.queryGetAll.document,
                variables: planStore.gql.queryGetAll.variables,
            });
        }
        const taskError = taskStore.gql.queryGetAll.error;
        if (taskError != null) {
            entries.push({
                source: 'task.queryGetAll',
                error: taskError,
                queryDocument: taskStore.gql.queryGetAll.document,
                variables: taskStore.gql.queryGetAll.variables,
            });
        }
        const resourceError = resourceStore.gql.queryGetAll.error;
        if (resourceError != null) {
            entries.push({
                source: 'resource.queryGetAll',
                error: resourceError,
                queryDocument: resourceStore.gql.queryGetAll.document,
                variables: resourceStore.gql.queryGetAll.variables,
            });
        }
        const planIssueError = planIssueStore.gql.q.error;
        if (planIssueError != null) {
            entries.push({
                source: 'planIssue.queryGetAll',
                error: planIssueError,
                queryDocument: planIssueStore.gql.q.document,
                variables: planIssueStore.gql.q.variables,
            });
        }

        for (const entry of resourceStore.combinedAvailabilityQueryErrors) {
            entries.push({
                source: entry.source,
                error: entry.error,
                queryDocument: entry.queryDocument,
                variables: entry.variables,
            });
        }

        return entries.map((entry) => toDisplayIssue(entry.source, entry.error, entry.queryDocument, entry.variables));
    });

    const displayIssues = computed(() => graphqlIssues.value);
    const errorCount = computed(() => displayIssues.value.filter((iss) => iss.severity === 'error').length);
    const warningCount = computed(() => displayIssues.value.filter((iss) => iss.severity === 'warning').length);
    const toolbarSeverity = computed<IssueSeverity | null>(() => {
        if (errorCount.value > 0) {
            return 'error';
        }
        if (warningCount.value > 0) {
            return 'warning';
        }
        return null;
    });

    return {
        displayIssues,
        warningCount,
        errorCount,
        toolbarSeverity,
    }
});

if (import.meta.hot) {
    import.meta.hot.accept(acceptHMRUpdate(useGeneralIssueStore, import.meta.hot));
}
