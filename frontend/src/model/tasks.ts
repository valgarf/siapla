import { useQuery, type UseQueryReturn } from '@vue/apollo-composable';
import { computed, type ComputedRef } from 'vue';
import { graphql } from 'src/gql';
import { type TasksQuery } from 'src/gql/graphql';

export interface Task {
  dbId: number;
  title: string;
  description: string;
  parent: Task | null;
  children: Task[];
}

export class TaskData {
  _tasks: Map<number, Task>;

  public constructor(query: TasksQuery) {
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
    this._tasks = tasks;
  }

  public get tasks() {
    return this._tasks.values();
  }

  public get top_level_tasks() {
    return Array.from(this._tasks.values()).filter((value) => value.parent == null);
  }

  public get leaf_tasks() {
    return Array.from(this._tasks.values()).filter((value) => value.children.length == 0);
  }

  public task(dbId: number): Task | undefined {
    return this._tasks.get(dbId);
  }
}

export interface TaskReturn {
  query: UseQueryReturn<TasksQuery, Record<string, never>>;
  data: ComputedRef<TaskData | null>;
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

export function all_tasks(): TaskReturn {
  const query = useQuery(TASK_QUERY);

  const data: ComputedRef<TaskData | null> = computed(() => {
    if (query.result.value == null) {
      return null;
    } else {
      return new TaskData(query.result.value);
    }
  });

  return { query: query, data: data };
}
