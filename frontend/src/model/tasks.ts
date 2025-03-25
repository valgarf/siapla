import { useQuery, type UseQueryReturn } from '@vue/apollo-composable';
import { computed, type ComputedRef } from 'vue';
import { graphql } from 'src/gql';
import { type TasksQuery } from 'src/gql/graphql';

export interface Task {
  dbId: number;
  title: string;
  description: string;
}

export class TaskData {
  tasks: Task[];
  public constructor(query: TasksQuery) {
    const tasks: Task[] = query.tasks.map((t) => {
      return {
        dbId: t.dbId,
        title: t.title,
        description: t.description,
      };
    });
    this.tasks = tasks;
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
