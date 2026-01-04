<template>
    <q-select v-if="edit" filled v-model="selectModel" multiple :options="selectPossible" use-chips use-input
        stack-label :label="label" @focus="beginSelection" @blur="endSelection" @filter="filterFn" />
    <div v-else-if="model?.length || createButton" class="col">
        <div class="text-subtitle2">{{ label }}</div>
        <TaskChip v-for="task in model" :key="task.dbId" :task="task" />
        <q-chip clickable v-if="!edit && createButton" outline icon="add" color="primary" label="Create"
            @click="emit('create')" />
    </div>
</template>

<script setup lang="ts">
import { useTaskStore, type Task } from 'src/stores/task';
import { computed, ref } from 'vue';
import TaskChip from '../common/TaskChip.vue';

interface Props {
    label?: string;
    edit: boolean;
    createButton?: boolean;
    possible: Task[];
    single?: boolean;
    selectKey?: string;
};
const props = withDefaults(defineProps<Props>(), { createButton: true, single: false, selectKey: '' });
const emit = defineEmits<{
    (e: 'create'): void
}>()
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
const filterValue = ref<string>('')
function filterFn(val: string | null, update: (fn: () => void) => void) {
    update(() => { filterValue.value = val?.toLowerCase() ?? '' })
}
const selectPossible = computed(() => {
    const result = props.possible.map(toSelectOpt)
    if (!filterValue.value.length) {
        return result
    }
    return result.filter((opt) => opt.label.toLowerCase().indexOf(filterValue.value) > -1)
})

// selection store sync
import { onUnmounted, watch } from 'vue'
import { useSelectionStore } from 'src/stores/selection'
const selection = useSelectionStore()

function possibleIds(arr: Task[]) { return arr.map(r => r.dbId) }

let selStop: (() => void) | null = null
let possibleStop: (() => void) | null = null
let modelStop: (() => void) | null = null
let focused = false

function beginSelection() {
    if (focused) return
    focused = true
    selection.setMode('TASK')
    selection.setKey(props.selectKey)
    selection.setPossible(possibleIds(props.possible))
    selection.setSingle(!!props.single)
    selection.setSelected((model.value ?? []).map(r => r.dbId))

    possibleStop = watch(() => props.possible, (n) => selection.setPossible(possibleIds(n)), { deep: true })
    modelStop = watch(model, (nv) => selection.setSelected((nv ?? []).map(r => r.dbId)), { deep: true })
    selStop = watch(() => selection.selected, (ids) => {
        if (selection.mode !== 'TASK') return
        if (selection.key() !== props.selectKey) return
        if (model.value?.length == ids.length && model.value?.every((v, i) => v.dbId == ids[i])) return
        model.value = ids.map(id => fromSelectOpt({ label: '', value: id })).filter((v) => v != undefined)
    }, { deep: true })
}

function endSelection() {
    if (!focused) return
    focused = false
    if (possibleStop) { possibleStop(); possibleStop = null }
    if (modelStop) { modelStop(); modelStop = null }
    if (selStop) { selStop(); selStop = null }
    if (selection.mode === 'TASK' && selection.key() === props.selectKey) selection.clear()
}

onUnmounted(() => {
    endSelection()
})



</script>