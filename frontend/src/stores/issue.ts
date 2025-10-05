import { useQuery } from '@vue/apollo-composable';
import { graphql } from 'src/gql';
import { acceptHMRUpdate, defineStore } from 'pinia';
import { computed, type ComputedRef } from 'vue';
import type { IssueCode } from 'src/gql/graphql';

const ISSUES_QUERY = graphql(`
  query issues {
    issues {
      dbId
      code
      description
      type
      task {
        dbId
      }
    }
  }
`);

export type Issue = {
    dbId: number;
    code: IssueCode;
    description: string;
    type: string;
    taskId: number | null;
}

export const useIssueStore = defineStore('issueStore', () => {
    const q = useQuery(ISSUES_QUERY);
    const issues: ComputedRef<Issue[]> = computed(() => q.result.value?.issues.map((iss) => {
        return {
            dbId: iss.dbId,
            code: iss.code,
            description: iss.description,
            type: iss.type,
            taskId: iss.task?.dbId ?? null,
        }
    }) ?? []);
    const issueTaskMap = computed(() => {
        const result: Map<number, Issue[]> = new Map();
        for (const iss of issues.value) {
            if (iss.taskId != null) {
                const arr: Issue[] = result.get(iss.taskId) ?? []
                arr.push(iss)
                result.set(iss.taskId, arr);
            }
        }
        return result;

    });

    return {
        gql: { q },
        issues,
        issueTaskMap,
        refetch: async () => { await q.refetch(); }
    }
});

if (import.meta.hot) {
    import.meta.hot.accept(acceptHMRUpdate(useIssueStore, import.meta.hot));
}
