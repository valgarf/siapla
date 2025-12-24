import { useMutation, useQuery } from '@vue/apollo-composable';
import { acceptHMRUpdate, defineStore } from 'pinia';
import { graphql } from 'src/gql';
import type { TaskDesignation, TaskSaveInput, TasksQuery } from 'src/gql/graphql';
import { computed, type Ref } from 'vue';
import { TaskSidebarData, useSidebarStore } from './sidebar';
import type { Resource } from './resource';
import { useResourceStore } from './resource';
import { ApolloError } from '@apollo/client/core';

export interface Task {
  dbId: number;
  title: string;
  description: string;
  parent: Task | null;
  children: Task[];
  predecessors: Task[];
  successors: Task[];
  earliestStart: Date | null;
  scheduleTarget: Date | null;
  effort: number | null;
  designation: TaskDesignation;
  resourceConstraints: ResourceConstraint[];
}

export interface ResourceConstraint {
  resources: Resource[];
  optional: boolean;
  speed: number;
}

export interface TaskInput extends Partial<Task> {
  title: string;
  description: string;
  designation: TaskDesignation;
}

const TASK_QUERY = graphql(`
  query tasks {
    tasks {
      dbId
      title
      description
      designation
      parent {
        dbId
      }
      predecessors {
        dbId
      }
      earliestStart
      scheduleTarget
      effort
      designation
      resourceConstraints {
        optional
        speed
        entries {
          resource {
            dbId
          }
        }
      }
    }
  }
`);

const TASK_SAVE_MUTATION = graphql(`
  mutation task_save($task: TaskSaveInput!) {
    taskSave(task: $task) {
      dbId
    }
  }
`);

const TASK_DELETE_MUTATION = graphql(`
  mutation task_delete($taskId: Int!) {
    taskDelete(taskId: $taskId)
  }
`);

function convertQueryResult(query: TasksQuery) {
  const resourceStore = useResourceStore();
  const tasks: Map<number, Task> = new Map(
    query.tasks.map((t) => {
      return [
        t.dbId,
        {
          dbId: t.dbId,
          title: t.title,
          description: t.description,
          designation: t.designation,
          parent: null,
          children: [],
          predecessors: [],
          successors: [],
          earliestStart: t.earliestStart == null ? null : new Date(t.earliestStart),
          scheduleTarget: t.scheduleTarget == null ? null : new Date(t.scheduleTarget),
          effort: t.effort ?? null,
          resourceConstraints: (t.resourceConstraints ?? []).map(rc => ({
            resources: (rc.entries ?? []).map(e => resourceStore.resource(e.resource.dbId)).filter(r => r != null),
            optional: rc.optional,
            speed: rc.speed,
          })),
        },
      ];
    }),
  );

  for (const t of query.tasks) {
    if (t.parent != null) {
      const task = tasks.get(t.dbId);
      const parent = tasks.get(t.parent.dbId);
      if (task != null && parent != null) {
        task.parent = parent;
        parent.children.push(task);
      }
    }
    for (const pre of t.predecessors) {
      const task = tasks.get(t.dbId);
      const pre_task = tasks.get(pre.dbId);
      if (task != null && pre_task != null) {
        task.predecessors.push(pre_task);
        pre_task.successors.push(task);
      }
    }
  }
  return tasks;
}

function taskToObj(task: Ref<TaskInput>): TaskSaveInput {
  const { parent, children, predecessors, successors, earliestStart, scheduleTarget, ...fields } =
    task.value;
  const predecessor_ids = predecessors?.map((t) => t.dbId) || [];
  const successor_ids = successors?.map((t) => t.dbId) || [];
  const children_ids = children?.map((t) => t.dbId) || [];
  const result: TaskSaveInput = {
    ...fields,
    predecessors: predecessor_ids,
    successors: successor_ids,
    children: children_ids,
    parentId: parent?.dbId || null,
    earliestStart: earliestStart == null ? null : earliestStart.toISOString(),
    scheduleTarget: scheduleTarget == null ? null : scheduleTarget.toISOString(),
    resourceConstraints: (task.value.resourceConstraints ?? []).map(opt => ({
      optional: opt.optional,
      speed: opt.speed,
      entries: (opt.resources ?? []).map(r => ({ resourceId: r.dbId })),
    })),
  };
  return result;
}

// actual store
export const useTaskStore = defineStore('taskStore', () => {
  const queryGetAll = useQuery(TASK_QUERY);
  const mutSaveTask = useMutation(TASK_SAVE_MUTATION);
  const mutDeleteTask = useMutation(TASK_DELETE_MUTATION);

  const apollo_objs = [queryGetAll, mutSaveTask, mutDeleteTask];
  const task_map = computed(() => {
    if (queryGetAll.result.value == null) {
      return null;
    } else {
      return convertQueryResult(queryGetAll.result.value);
    }
  });

  // Returns any mutation error as a string, or null on success
  async function saveTask(task: Ref<TaskInput>): Promise<string | null> {
    const sidebarStore = useSidebarStore();
    try {
      const resp = await mutSaveTask.mutate({ task: taskToObj(task) });
      // Apollo GraphQL errors (if any)
      const gqlErrors = resp?.errors ?? [];
      if (gqlErrors.length > 0) {
        return gqlErrors.map((e) => e.message).join('; ');
      }
      const dbId = resp?.data?.taskSave?.dbId;
      if (dbId != null) {
        if (task.value.dbId == null) {
          // a little hacky
          // TODO: necessary?
          task.value.dbId = dbId;
          await queryGetAll.refetch();
          // TODO: generic error handling?
          sidebarStore.replaceSidebar(new TaskSidebarData(dbId));
        } else {
          task.value.dbId = dbId;
          await queryGetAll.refetch();
          // TODO: generic error handling?
        }
      }
      return null;
    } catch (err: unknown) {
      return err instanceof ApolloError ? err.message : String(err)
    }
  }

  async function deleteTask(taskId: number, pop: boolean = true) {
    const sidebarStore = useSidebarStore();
    const resp = await mutDeleteTask.mutate({ taskId: taskId });
    const result = resp?.data?.taskDelete;
    if (result) {
      // TODO: a 'filter' that removes all corresponding sidebars would be better
      if (
        pop &&
        sidebarStore.activeSidebar instanceof TaskSidebarData &&
        sidebarStore.activeSidebar.taskId == taskId
      ) {
        sidebarStore.popSidebar();
      }
      await queryGetAll.refetch();
    }
    return result;
    // TODO: generic error handling?
  }
  const tasks = computed(() => Array.from(task_map.value?.values() || []));

  return {
    gql: {
      queryGetAll,
      mutSaveTask,
    },
    loading: queryGetAll.loading,
    saving: mutSaveTask.loading,
    deleting: mutDeleteTask.loading,
    tasks,
    topLevelTasks: computed(() => tasks.value.filter((v) => v.parent == null)),
    leafTasks: computed(() => tasks.value.filter((v) => v.children.length == 0)),
    // TODO: generic GQL error messages as notifications?
    apolloErrors: computed(() => apollo_objs.map((obj) => obj.error).filter((err) => err != null)),
    task: (dbId: number): Task | undefined => {
      return task_map.value?.get(dbId);
    },
    saveTask,
    deleteTask,
  };
});

if (import.meta.hot) {
  import.meta.hot.accept(acceptHMRUpdate(useTaskStore, import.meta.hot));
}
