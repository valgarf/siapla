import { useMutation, useQuery, useSubscription } from '@vue/apollo-composable';
import { acceptHMRUpdate, defineStore } from 'pinia';
import { graphql } from 'src/gql';
import { computed, type ComputedRef, watch } from 'vue'; // type Ref, ref,
import type { Resource } from './resource';
import { useResourceStore } from './resource';
import { type Task, useTaskStore } from './task';
import { TaskDesignation, type PlanQuery, CalculationState, type Exact, type MutationBookingSaveArgs, AllocationType } from 'src/gql/graphql';
import { useIssueStore } from './issue';

export interface Allocation {
  dbId: number;
  start: Date;
  end: Date;
  task: Task | null;
  resources: Resource[];
  allocationType: AllocationType | null;
  final?: boolean;
}

const PLAN_QUERY = graphql(`
  query plan {
    currentPlan {
      allocations {
        dbId
        start
        end
        allocationType
        final
        task { dbId }
        resources { dbId }
      }
    }
  }
`);

const CALC_SUB = graphql(`
  subscription calcUpdate {
    calculationUpdate {
      state
    }
  }
`);

const RECALC_MUT = graphql(`
  mutation recalculate { recalculateNow }
`);

const BOOKING_SAVE = graphql(`
  mutation bookingSave($dbId: Int, $taskId: Int!, $start: DateTime!, $end: DateTime!, $resources: [Int!]!, $final: Boolean!) {
    bookingSave(dbId: $dbId, taskId: $taskId, start: $start, end: $end, resources: $resources, final: $final) { dbId }
  }
`);
const BOOKING_DELETE = graphql(`
  mutation bookingDelete($dbId: Int!) { bookingDelete(dbId: $dbId) }
`);

function convertQueryResult(query: PlanQuery): Allocation[] {
  const resourceStore = useResourceStore();
  const taskStore = useTaskStore();
  const allocations: Allocation[] = query.currentPlan.allocations.map(a => {
    const resources = a.resources.map(r => resourceStore.resource(r.dbId)).filter(r => r != null);
    return { dbId: a.dbId, start: new Date(a.start), end: new Date(a.end), allocationType: a.allocationType, final: a.final, task: taskStore.task(a.task.dbId) ?? null, resources: resources }
  });
  // Not sure if this is necessary. Mucks up the arrows
  // allocations.sort((lhs, rhs) => {
  //   if (lhs.allocationType != rhs.allocationType) {
  //     if (lhs.allocationType == AllocationType.Booking) {
  //       return 1
  //     }
  //     else {
  //       return -1
  //     }
  //   }
  //   return (lhs.start.getTime() - rhs.start.getTime())
  // })
  return allocations;
}

// actual store
export const usePlanStore = defineStore('planStore', () => {
  const issueStore = useIssueStore();
  const queryGetAll = useQuery(PLAN_QUERY);
  const calcSub = useSubscription(CALC_SUB);
  const mutRecalculate = useMutation(RECALC_MUT);
  // const calculationState: Ref<CalculationState> = ref(CalculationState.Calculating);
  // calcSub.start()
  const calculationState = computed(() => { return calcSub.result.value?.calculationUpdate?.state ?? CalculationState.Modified });
  watch(
    () => calcSub.result.value,
    (v) => {
      const state = v?.calculationUpdate?.state;
      // if (state != null) {
      //   calculationState.value = state;
      // }
      if (state === CalculationState.Finished) {
        // react to finished events: refetch plan
        queryGetAll.refetch()?.catch((reason) => { console.warn("Failed to load plan: ", reason) });
        issueStore.refetch()?.catch((reason) => { console.warn("Failed to load issues: ", reason) });
      }
    }
  );
  const apollo_objs = [queryGetAll];
  const allocations = computed(() => {
    if (queryGetAll.result.value == null) {
      return [];
    } else {
      return convertQueryResult(queryGetAll.result.value);
    }
  });

  // booking mutations
  const mutBookingSave = useMutation(BOOKING_SAVE);
  const mutBookingDelete = useMutation(BOOKING_DELETE);

  async function createBookingFromPlan(taskId?: number | null) {
    if (taskId == null) return;
    // find earliest PLAN allocation for this task
    const matching = allocations.value.filter(a => a.task?.dbId === taskId && (a.allocationType ?? 'PLAN') === 'PLAN');
    matching.sort((l, r) => new Date(l.start).getTime() - new Date(r.start).getTime());
    const source = matching[0];
    const start = source ? source.start : new Date(Date.now() - 7 * 24 * 60 * 60 * 1000);
    const end = source ? source.end : new Date(Date.now());
    const resources = source ? source.resources.map(r => r.dbId) : [];
    try {
      const vars: Exact<MutationBookingSaveArgs> = { dbId: null, taskId, start: start.toISOString(), end: end.toISOString(), resources, final: false };
      await mutBookingSave.mutate(vars);
      await queryGetAll.refetch?.();
    } catch (err) {
      console.warn('Failed to create booking', err);
    }
  }

  async function saveBooking(b: Allocation) {
    try {
      const taskId = b.task?.dbId;
      if (!taskId) {
        console.warn('Cannot save booking without a task id', b);
        return;
      }
      const vars: Exact<MutationBookingSaveArgs> = { dbId: b.dbId > 0 ? b.dbId : null, taskId, start: (b.start instanceof Date) ? b.start.toISOString() : String(b.start), end: (b.end instanceof Date) ? b.end.toISOString() : String(b.end), resources: (b.resources || []).map(r => r.dbId), final: !!b.final };
      await mutBookingSave.mutate(vars);
      await queryGetAll.refetch?.();
    } catch (err) {
      console.warn('Failed to save booking', err);
    }
  }

  async function deleteBooking(dbId?: number | null) {
    if (!dbId) return;
    try {
      await mutBookingDelete.mutate({ dbId });
      await queryGetAll.refetch?.();
    } catch (err) {
      console.warn('Failed to delete booking', err);
    }
  }

  function bookingsByTask(taskId?: number | null) {
    if (taskId == null) return [] as Allocation[];
    return allocations.value.filter(a => a.task?.dbId === taskId && a.allocationType === AllocationType.Booking);
  }

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
    if (all.length === 0) return new Date();
    return new Date(Math.min(...all));
  });

  const end = computed(() => {
    const allocEnds = allocations.value.map((a) => a.end.getTime());
    const all = [...allocEnds, ...otherDates.value];
    if (all.length === 0) return new Date();
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
      if (base.has(tid)) {
        // only add bounds if the task has no own allocations (i.e. groups)
        continue;
      }
      const bounds = comp.value;
      if (!bounds) continue;
      const synthetic: Allocation = {
        dbId: -tid,
        start: bounds.start,
        end: bounds.end,
        task: taskStore.task(tid) ?? null,
        resources: [],
        allocationType: null
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
      mutRecalculate,
    },
    calculationState,
    loading: computed(() => queryGetAll.loading.value || calcSub.loading.value),
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
    // booking helpers
    bookingsByTask: (dbId: number) => bookingsByTask(dbId),
    createBookingFromPlan: (taskId?: number | null) => createBookingFromPlan(taskId),
    saveBooking: (b: Allocation) => saveBooking(b),
    deleteBooking: (dbId?: number | null) => deleteBooking(dbId),
    recalculate: () => {
      return mutRecalculate.mutate()
    }
  };
});

if (import.meta.hot) {
  import.meta.hot.accept(acceptHMRUpdate(usePlanStore, import.meta.hot));
}
