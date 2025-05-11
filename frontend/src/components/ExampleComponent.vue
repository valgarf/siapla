<template>
  <div>
    <p>{{ title }}</p>
    <ul>
      <li v-for="todo in todos" :key="todo.id" @click="increment">
        {{ todo.id }} - {{ todo.content }}
      </li>
    </ul>
    <p v-if="taskStore.loading">Loading GQL...</p>
    <p v-else-if="taskStore.apolloErrors">Error: {{ taskStore.apolloErrors }}</p>
    <div v-else>
      <ul>
        <li v-for="t in taskStore.tasks" :key="t.dbId">
          {{ t.title }} | {{ t.description }}
        </li>
      </ul>
    </div>
    <p>Count: {{ todoCount }} / {{ meta.totalCount }}</p>
    <p>Active: {{ active ? 'yes' : 'no' }}</p>
    <p>Clicks on todos: {{ clickCount }}</p>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import type { Todo, Meta } from './models';
import { useTaskStore } from 'src/stores/task';

const taskStore = useTaskStore();


interface Props {
  title: string;
  todos?: Todo[];
  meta: Meta;
  active: boolean;
};

const props = withDefaults(defineProps<Props>(), {
  todos: () => []
});

const clickCount = ref(0);
function increment() {
  clickCount.value += 1;
  return clickCount.value;
}

const todoCount = computed(() => props.todos.length);
</script>
