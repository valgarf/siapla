<template>
    <q-select v-if="edit" filled v-model="select_model" multiple :options="select_possible" use-chips stack-label
        :label="name" />
    <div v-else-if="model.length" class="col">
        <div class="text-subtitle2">{{ name }}</div>
        <TaskChip v-for="task in model" :key="task.dbId" :task="task" />
    </div>
</template>

<script setup lang="ts">
import { useTaskStore, type Task } from 'src/stores/task';
import { computed, type Ref } from 'vue';
import TaskChip from './TaskChip.vue';

interface Props {
    name: string;
    edit: boolean;
    possible: Task[];
};
const props = withDefaults(defineProps<Props>(), {});
const model: Ref<Task[]> = defineModel({ required: true })

const taskStore = useTaskStore();

// This is a not so nice workaround to get select to work. 
// If we use actual tasks in the model, we get recursion errors, so we only provide the ids.
interface SelectOpt {
    label: string,
    value: number,
}

function to_select_opt(t: Task): SelectOpt {
    return { label: t.title, value: t.dbId }
}
function from_select_opt(t: SelectOpt): Task | undefined {
    return taskStore.task(t.value)
}

const select_model = computed({
    get() {
        return model.value.map(to_select_opt) || []
    },
    set(value: SelectOpt[]) {
        model.value = value.map(from_select_opt).filter((v: Task | undefined) => v != undefined)
    }
})
const select_possible = computed(() => { return props.possible.map(to_select_opt) })



</script>