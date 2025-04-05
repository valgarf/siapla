import { graphql } from 'src/gql';
import { defineStore, acceptHMRUpdate } from 'pinia';
import { useMutation, useQuery } from '@vue/apollo-composable';
import type { TaskDesignation, TaskSaveInput, TasksQuery } from 'src/gql/graphql';
import { computed, ref, type Ref } from 'vue';

export interface Task {
  dbId: number;
  title: string;
  description: string;
  parent: Task | null;
  children: Task[];
  predecessors: Task[];
  successors: Task[];
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
          predecessors: [],
          successors: [],
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

function task_to_obj(task: Ref<TaskInput>): TaskSaveInput {
  const { parent, children, predecessors, successors, ...fields } = task.value;
  const predecessor_ids = predecessors?.map((t) => t.dbId) || [];
  const successor_ids = successors?.map((t) => t.dbId) || [];
  const children_ids = children?.map((t) => t.dbId) || [];
  const result: TaskSaveInput = {
    ...fields,
    predecessors: predecessor_ids,
    successors: successor_ids,
    children: children_ids,
    parentId: parent?.dbId || null,
  };
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
      if (task.value.dbId == null) {
        pop_task_dialog(); // a little hacky
      }
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
  const tasks = computed(() => Array.from(task_map.value?.values() || []));

  // the store's state
  const task_dialog_history: Ref<Task[]> = ref([]);
  const new_task: Ref<boolean> = ref(false);

  function push_task_dialog(taskId: number) {
    const task = task_map.value?.get(taskId);
    if (task != null) {
      task_dialog_history.value.push(task);
    }
  }
  function push_new_task_dialog() {
    new_task.value = true;
  }
  function pop_task_dialog() {
    if (new_task.value) {
      new_task.value = false;
    } else {
      task_dialog_history.value.pop();
    }
  }
  function reset_task_dialog(taskId?: number) {
    new_task.value = false;
    task_dialog_history.value = [];
    if (taskId != null) {
      push_task_dialog(taskId);
    }
  }
  const active_task_dialog = computed(() => {
    if (new_task.value) {
      console.log('New Task Dialog');
      return {};
    }
    if (task_dialog_history.value.length > 0) {
      console.log('Existing task: ');
      return task_dialog_history.value[task_dialog_history.value.length - 1];
    }
    return null;
  });

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
    push_task_dialog,
    push_new_task_dialog,
    pop_task_dialog,
    reset_task_dialog,
    active_task_dialog,
  };
});

if (import.meta.hot) {
  import.meta.hot.accept(acceptHMRUpdate(useTaskStore, import.meta.hot));
}
