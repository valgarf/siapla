import { useQuery } from '@vue/apollo-composable';
import { acceptHMRUpdate, defineStore } from 'pinia';
import { graphql } from 'src/gql';
import { computed, type ComputedRef } from 'vue';
import type { Resource } from './resource';
import { useResourceStore } from './resource';
import { type Task, useTaskStore } from './task';
import { TaskDesignation, type PlanQuery } from 'src/gql/graphql';

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

  // raw allocations grouped by task id
  const allocations_by_task_raw = computed(() => {
    const m = new Map<number, Allocation[]>();
    for (const a of allocations.value) {
      const tid = a.task?.dbId;
      if (tid == null) continue;
      const arr = m.get(tid) ?? [];
      arr.push(a);
      m.set(tid, arr);
    }
    return m;
  });

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

  // include task-based dates (requirements / milestones) in the overall plan bounds
  const taskStore = useTaskStore();

  // allocation bounds map: computed per task id
  const allocBoundsMap = computed(() => {
    const map = new Map<number, ComputedRef<{ start: Date; end: Date } | null>>();

    function makeBounds(t: Task): ComputedRef<{ start: Date; end: Date } | null> {
      const existing = map.get(t.dbId);
      if (existing) return existing;

      const c = computed(() => {
        if (t.designation !== TaskDesignation.Group) {
          const allocs = allocations_by_task_raw.value.get(t.dbId) ?? [];
          if (allocs.length === 0) return null;
          const first = allocs[0]!;
          let minStart = first.start;
          let maxEnd = first.end;
          for (const a of allocs) {
            if (a.start < minStart) minStart = a.start;
            if (a.end > maxEnd) maxEnd = a.end;
          }
          return { start: minStart, end: maxEnd };
        }

        let minStart: Date | null = null;
        let maxEnd: Date | null = null;
        for (const child of t.children) {
          const childComp = makeBounds(child);
          const childBounds = childComp.value;
          if (!childBounds) continue;
          if (minStart == null || childBounds.start < minStart) minStart = childBounds.start;
          if (maxEnd == null || childBounds.end > maxEnd) maxEnd = childBounds.end;
        }
        if (minStart == null || maxEnd == null) return null;
        return { start: minStart, end: maxEnd };
      });

      map.set(t.dbId, c);
      return c;
    }

    for (const t of taskStore.tasks) {
      makeBounds(t);
    }
    return map;
  });

  const otherDates = computed(() => {
    const requirementTimes = (taskStore.tasks ?? []).filter((t: Task) => t.designation == TaskDesignation.Requirement && t.earliestStart != null).flatMap((t: Task) => { return t.earliestStart!.getTime() });
    const milestoneTargets = (taskStore.tasks ?? []).filter((t: Task) => t.designation == TaskDesignation.Milestone && t.scheduleTarget != null).flatMap((t: Task) => { return t.scheduleTarget!.getTime() });
    return [...requirementTimes, ...milestoneTargets];
  })
  const start = computed(() => {
    const allocStarts = allocations.value.map((a) => a.start.getTime());
    const all = [...allocStarts, ...otherDates.value];
    if (all.length === 0 || queryGetAll.loading.value || taskStore.loading) return new Date();
    return new Date(Math.min(...all));
  });

  const end = computed(() => {
    const allocEnds = allocations.value.map((a) => a.end.getTime());
    const all = [...allocEnds, ...otherDates.value];
    if (all.length === 0 || queryGetAll.loading.value || taskStore.loading) return new Date();
    return new Date(Math.max(...all));
  });
  const resource_ids = computed(() => { return Array.from(allocations_by_resource.value.keys()) })
  // combine raw allocations with synthetic bounds allocations per task
  const allocations_by_task = computed(() => {
    const base = new Map<number, Allocation[]>();
    for (const [tid, arr] of allocations_by_task_raw.value.entries()) {
      base.set(tid, arr.slice());
    }
    for (const [tid, comp] of allocBoundsMap.value.entries()) {
      const bounds = comp.value;
      if (!bounds) continue;
      const synthetic: Allocation = {
        dbId: -tid,
        start: bounds.start,
        end: bounds.end,
        task: taskStore.task(tid) ?? null,
        resources: [],
      };
      const arr = base.get(tid) ?? [];
      arr.push(synthetic);
      base.set(tid, arr);
    }
    return base;
  });

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
