<template>
  <div>
    <p>{{ title }}</p>
    <ul>
      <li v-for="todo in todos" :key="todo.id" @click="increment">
        {{ todo.id }} - {{ todo.content }}
      </li>
    </ul>
    <p v-if="loading">Loading GQL...</p>
    <p v-else-if="error">Error: {{ error }}</p>
    <div v-else>
      <ul>
        <li v-for="t in result.tasks" :key="t.dbId">
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
import { computed, ref, watchEffect } from 'vue';
import type { Todo, Meta } from './models';
import { useQuery } from '@vue/apollo-composable'
import { gql } from '@apollo/client/core'

// error is a possible return value...
const { result, loading, error } = useQuery(gql` 
  query{
    tasks {
      dbId
      title
      description
    }
  }
`)

watchEffect(() => {
  console.log(error, result);
})


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
