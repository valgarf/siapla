import { graphql } from 'src/gql';
import { defineStore, acceptHMRUpdate } from 'pinia';
import { useMutation, useQuery } from '@vue/apollo-composable';
import type { TaskDesignation, TaskSaveInput, TasksQuery } from 'src/gql/graphql';
import { computed, type Ref } from 'vue';

export interface Task {
  dbId: number;
  title: string;
  description: string;
  parent: Task | null;
  children: Task[];
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

function convert_query_result(query: TasksQuery) {
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
  }
  return tasks;
}

function task_to_obj(task: Ref<TaskInput>): TaskSaveInput {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const { parent, children, ...fields } = task.value;
  const result: TaskSaveInput = { ...fields };
  if (parent != null) {
    result.parentId = parent.dbId;
  }
  return result;
}

// actual store
export const useTaskStore = defineStore('taskStore', () => {
  const query_get_all = useQuery(TASK_QUERY);
  const mut_save_task = useMutation(TASK_SAVE_MUTATION);
  const mut_delete_task = useMutation(TASK_DELETE_MUTATION);

  const apollo_objs = [query_get_all, mut_save_task, mut_delete_task];
  const task_map = computed(() => {
    if (query_get_all.result.value == null) {
      return null;
    } else {
      return convert_query_result(query_get_all.result.value);
    }
  });

  async function save_task(task: Ref<TaskInput>) {
    const resp = await mut_save_task.mutate({ task: task_to_obj(task) });
    const dbId = resp?.data?.taskSave.dbId;
    if (dbId != null) {
      task.value.dbId = dbId;
    }
    await query_get_all.refetch();
    // TODO: generic error handling?
  }

  async function delete_task(taskId: number) {
    const resp = await mut_delete_task.mutate({ taskId: taskId });
    const result = resp?.data?.taskDelete;
    if (result) {
      await query_get_all.refetch();
    }
    return result;
    // TODO: generic error handling?
  }

  // the store's state
  const tasks = computed(() => Array.from(task_map.value?.values() || []));
  return {
    gql: {
      query_get_all,
      mut_save_task,
    },
    loading: query_get_all.loading,
    saving: mut_save_task.loading,
    deleting: mut_delete_task.loading,
    tasks,
    top_level_tasks: computed(() => tasks.value.filter((v) => v.parent == null)),
    leaf_tasks: computed(() => tasks.value.filter((v) => v.children.length == 0)),
    // TODO: generic GQL error messages as notifications?
    apollo_errors: computed(() => apollo_objs.map((obj) => obj.error).filter((err) => err != null)),
    task: (dbId: number): Task | undefined => {
      return task_map.value?.get(dbId);
    },
    save_task,
    delete_task,
  };
});

if (import.meta.hot) {
  import.meta.hot.accept(acceptHMRUpdate(useTaskStore, import.meta.hot));
}
