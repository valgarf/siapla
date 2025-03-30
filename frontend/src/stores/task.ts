import { graphql } from 'src/gql';
import { defineStore, acceptHMRUpdate } from 'pinia';
import { useMutation, useQuery } from '@vue/apollo-composable';
import type { TaskSaveInput, TasksQuery } from 'src/gql/graphql';
import { computed, type Ref } from 'vue';

export interface Task {
  dbId: number;
  title: string;
  description: string;
  parent: Task | null;
  children: Task[];
}

interface TaskInput extends Partial<Task> {
  title: string;
  description: string;
}

const TASK_QUERY = graphql(`
  query tasks {
    tasks {
      dbId
      title
      description
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

function convert_query_result(query: TasksQuery) {
  const tasks: Map<number, Task> = new Map(
    query.tasks.map((t) => {
      return [
        t.dbId,
        {
          dbId: t.dbId,
          title: t.title,
          description: t.description,
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

export const useTaskStore = defineStore('taskStore', () => {
  const query_get_all = useQuery(TASK_QUERY);
  const mut_save_task = useMutation(TASK_SAVE_MUTATION);

  const task_map = computed(() => {
    if (query_get_all.result.value == null) {
      return null;
    } else {
      return convert_query_result(query_get_all.result.value);
    }
    // TODO: generic error handling?
  });

  async function save_task(task: Ref<TaskInput>) {
    const resp = await mut_save_task.mutate({ task: task_to_obj(task) });
    const dbId = resp?.data?.taskSave.dbId;
    if (dbId != null) {
      task.value.dbId = dbId;
    }
    await query_get_all.refetch();
    // TODO: generic GQL error messages as notifications?
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
    tasks,
    top_level_tasks: computed(() => tasks.value.filter((v) => v.parent == null)),
    leaf_tasks: computed(() => tasks.value.filter((v) => v.children.length == 0)),
    apollo_errors: computed(() => {
      const result = [];
      if (query_get_all.error) {
        result.push(query_get_all.error);
      }
      if (mut_save_task.error) {
        result.push(mut_save_task.error);
      }
      return result;
    }),
    task: (dbId: number): Task | undefined => {
      return task_map.value?.get(dbId);
    },
    save_task,
  };
});

if (import.meta.hot) {
  import.meta.hot.accept(acceptHMRUpdate(useTaskStore, import.meta.hot));
}
