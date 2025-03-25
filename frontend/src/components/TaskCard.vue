<template>
    <q-card v-ripple class="cursor-pointer q-hoverable">
        <div tabindex="-1" class="q-focus-helper"></div>
        <q-card-section>
            <div class="text-subtitle1">{{ task.title }}</div>
        </q-card-section>

        <q-card-section class="q-pt-none">
            {{ task.description }}
        </q-card-section>
    </q-card>
</template>

<script setup lang="ts">
import { watchEffect } from 'vue';
import { all_tasks, type Task } from 'src/model/tasks';


const { data, query: { error } } = all_tasks();

// const { result, loading, error } = useQuery(TASK_QUERY)

watchEffect(() => {
    console.log(error, data);
})


interface Props {
    task: Task;
};

withDefaults(defineProps<Props>(), {});

</script>