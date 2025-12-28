<template>
    <q-select v-model="selectedOpt" :options="options" :label="label" filled use-chips stack-label map-options />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useTaskStore, type Task } from 'src/stores/task'

interface SelectOpt {
    label: string
    value: number
}

const props = defineProps<{
    possible: Task[]
    label?: string
}>()


const taskStore = useTaskStore();
const model = defineModel<Task | null | undefined>({ required: true })

function toSelectOpt(t: Task | null | undefined): SelectOpt | null {
    if (t == null) {
        return null
    }
    return { label: t.title, value: t.dbId }
}

function fromSelectOpt(t: SelectOpt | null): Task | null {
    if (t == null) {
        return null
    }
    return taskStore.task(t.value) ?? null
}


const options = computed(() => props.possible.map(toSelectOpt))

const selectedOpt = computed<SelectOpt | null>({
    get() {
        return toSelectOpt(model.value)
    },
    set(value) {
        model.value = fromSelectOpt(value)
    }
})
</script>
