<template>
    <q-select v-if="edit" filled v-model="selectModel" multiple :options="selectPossible" use-chips stack-label
        :label="label" />
    <div v-else-if="model?.length || createButton" class="col">
        <div class="text-subtitle2">{{ label }}</div>
        <TaskChip v-for="task in model" :key="task.dbId" :task="task" />
    </div>
</template>

<script setup lang="ts">
import { useTaskStore, type Task } from 'src/stores/task';
import { computed } from 'vue';
import TaskChip from '../common/TaskChip.vue';

interface Props {
    label?: string;
    edit: boolean;
    createButton?: boolean;
    possible: Task[];
};
const props = withDefaults(defineProps<Props>(), { createButton: true });
const model = defineModel<Task[] | undefined>({ required: true })

const taskStore = useTaskStore();

// This is a not so nice workaround to get select to work. 
// If we use actual tasks in the model, we get recursion errors, so we only provide the ids.
interface SelectOpt {
    label: string,
    value: number,
}

function toSelectOpt(t: Task): SelectOpt {
    return { label: t.title, value: t.dbId }
}
function fromSelectOpt(t: SelectOpt): Task | undefined {
    return taskStore.task(t.value)
}

const selectModel = computed({
    get() {
        return (model.value ?? []).map(toSelectOpt) || []
    },
    set(value: SelectOpt[]) {
        model.value = value.map(fromSelectOpt).filter((v: Task | undefined) => v != undefined)
    }
})
const selectPossible = computed(() => { return props.possible.map(toSelectOpt) })



</script>