import { useQuery } from '@vue/apollo-composable';
import { acceptHMRUpdate, defineStore } from 'pinia';
import { graphql } from 'src/gql';
import { computed } from 'vue';
import type { Resource } from './resource';
import { useResourceStore } from './resource';
import { type Task, useTaskStore } from './task';
import { type PlanQuery } from 'src/gql/graphql';

export interface Allocation {
  dbId: number;
  start: Date;
  end: Date;
  task: Task | null;
  resources: Resource[];
}

const PLAN_QUERY = graphql(`
  query plan {
    currentPlan {
      allocations {
        dbId
        start
        end
        task {
          dbId
        }
        resources {
          dbId
        }
      }
  }
}
`);

function convertQueryResult(query: PlanQuery): Allocation[] {
  const resourceStore = useResourceStore();
  const taskStore = useTaskStore();
  const allocations: Allocation[] = query.currentPlan.allocations.map(a => { return { dbId: a.dbId, start: new Date(a.start), end: new Date(a.end), task: taskStore.task(a.task.dbId) ?? null, resources: a.resources.map(r => { return resourceStore.resource(r.dbId) ?? null }).filter(r => r != null) } });
  return allocations;
}

// actual store
export const usePlanStore = defineStore('planStore', () => {
  const queryGetAll = useQuery(PLAN_QUERY);
  const apollo_objs = [queryGetAll];
  const allocations = computed(() => {
    if (queryGetAll.result.value == null) {
      return [];
    } else {
      return convertQueryResult(queryGetAll.result.value);
    }
  });
  const allocations_by_task = computed(() => {
    return Map.groupBy(allocations.value, a => a.task?.dbId)
  })
  const allocations_by_resource = computed(() => {
    return Map.groupBy(allocations.value.flatMap((a) => { return a.resources.map(r => { return { resId: r.dbId, alloc: a } }) }), a => a.resId)
  })
  const allocations_map = computed(() => new Map(
    allocations.value.map((a) => {
      return [
        a.dbId,
        a]
    })
  ))

  const start = computed(() => { return new Date(Math.min(...allocations.value.map((a) => a.start.getTime()))) })
  const end = computed(() => { return new Date(Math.max(...allocations.value.map((a) => a.end.getTime()))) })
  const resource_ids = computed(() => { return allocations_by_resource.value.keys() })
  return {
    gql: {
      queryGetAll,
    },
    loading: queryGetAll.loading,
    allocations,
    start,
    end,
    resource_ids,
    // TODO: generic GQL error messages as notifications?
    apolloErrors: computed(() => apollo_objs.map((obj) => obj.error).filter((err) => err != null)),
    allocation: (dbId: number): Allocation | undefined => {
      return allocations_map.value.get(dbId);
    },
    by_resource: (dbId: number): Allocation[] => {
      return allocations_by_resource.value.get(dbId)?.map((abyr) => abyr.alloc) ?? [];
    },
    by_task: (dbId: number): Allocation[] => {
      return allocations_by_task.value.get(dbId) ?? [];
    },
  };
});

if (import.meta.hot) {
  import.meta.hot.accept(acceptHMRUpdate(usePlanStore, import.meta.hot));
}
