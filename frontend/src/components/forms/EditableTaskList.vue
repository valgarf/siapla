<template>
    <q-select v-if="edit" ref="select" v-model="selectModel" :options="selectPossible" :label="label" filled
        :multiple="!single" use-chips hide-dropdown-icon use-input stack-label input-debounce="0"
        @focus="beginSelection" @blur="endSelection" @filter="filterFn" @popup-show="popupVisible = true"
        @popup-hide="popupVisible = false">
        <template v-slot:append>
            <q-btn class="popup-button" flat :ripple="false" icon="arrow_drop_down"
                :class="{ 'rotate-180': popupVisible }" @click="popupButton" />
        </template>
    </q-select>
    <div v-else-if="(single && model != null) || (!single && (model as Task[]).length) || createButton" class="col">
        <div class="text-subtitle2">{{ label }}</div>
        <TaskChip v-for="task in (model == null ? [] : single ? [model as Task] : model as Task[])" :key="task.dbId"
            :task="task" />
        <q-chip clickable v-if="!edit && createButton" outline icon="add" color="primary" label="Create"
            @click="emit('create')" />
    </div>
</template>

<script setup lang="ts">
import { onUnmounted, watch, computed, ref, useTemplateRef } from 'vue';
import { QSelect } from 'quasar';
import { useTaskStore, type Task } from 'src/stores/task';
import { useSelectionStore } from 'src/stores/selection'
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
const model = defineModel<Task[] | Task | undefined | null>({ required: true })
const popupVisible = ref(false)
const taskStore = useTaskStore();
const select = useTemplateRef<QSelect>('select')

// This is a not so nice workaround to get select to work. 
// If we use actual tasks in the model, we get recursion errors, so we only provide the ids.
interface SelectOpt {
    label: string,
    value: number,
}

function popupButton(evt: Event) {
    if (select.value == null) {
        return
    }
    if (popupVisible.value) {
        select.value.hidePopup()
        evt.stopPropagation()
    }
    else {
        // trick: we use 'null' to indicate the filter value was set by the button
        select.value.filter(null as unknown as string)
        select.value.showPopup()
        evt.stopPropagation()
    }
}
function toSelectOpt(t: Task): SelectOpt {
    return { label: t.title, value: t.dbId }
}
function fromSelectOpt(t: SelectOpt): Task | undefined {
    return taskStore.task(t.value)
}

const selectModel = computed({
    get() {
        if (props.single) {
            const value = model.value as Task | null;
            return value != null ? toSelectOpt(value) : null;
        }
        else {
            const value = model.value as Task[] | null;
            return (value ?? []).map(toSelectOpt) || []
        }
    },
    set(value: SelectOpt[] | SelectOpt | null) {
        if (props.single) {
            const singleValue = Array.isArray(value) ? value[0] : value
            if (singleValue == null) {
                model.value = null
            }
            else {
                model.value = fromSelectOpt(singleValue)
            }
        }
        else {
            const arrayValue = value == null ? [] : (Array.isArray(value) ? value : [value])
            model.value = arrayValue.map(fromSelectOpt).filter((v: Task | undefined) => v != undefined)
        }
    }
})
const filterValue = ref<string>('')
function filterFn(val: string | null, update: (fn: () => void) => void, abort: () => void) {
    if (val == null) {
        // trick: we use 'null' to indicate the filter value was set by the button
        update(() => { filterValue.value = '' })
        return
    }
    if (val.length < 1) {
        abort();
        return
    }
    update(() => { filterValue.value = val.toLowerCase() ?? '' })
}
const selectPossible = computed(() => {
    const result = props.possible.map(toSelectOpt)
    if (!filterValue.value.length) {
        return result
    }
    return result.filter((opt) => opt.label.toLowerCase().indexOf(filterValue.value) > -1)
})

// selection store sync
const selection = useSelectionStore()

function possibleIds(arr: Task[]) { return arr.map(r => r.dbId) }

let selStop: (() => void) | null = null
let possibleStop: (() => void) | null = null
let modelStop: (() => void) | null = null
let focused = false

const modelValueIds = computed(() => {
    let selected: number[] = []
    if (model.value && Array.isArray(model.value)) {
        selected = model.value.map(r => r.dbId)
    }
    else if (model.value) {
        selected = [model.value.dbId]
    }
    return selected
})

function beginSelection() {
    if (focused) return
    focused = true
    selection.setMode('TASK')
    selection.setKey(props.selectKey)
    selection.setPossible(possibleIds(props.possible))
    selection.setSingle(!!props.single)
    selection.setSelected(modelValueIds.value)

    possibleStop = watch(() => props.possible, (n) => selection.setPossible(possibleIds(n)), { deep: true })
    modelStop = watch(modelValueIds, (nv) => selection.setSelected(nv), { deep: true })
    selStop = watch(() => selection.selected, (ids) => {
        if (selection.mode !== 'TASK') return
        if (selection.key() !== props.selectKey) return
        if (modelValueIds.value.length == ids.length && modelValueIds.value?.every((v, i) => v == ids[i])) return
        const mappedOpts = ids.map(id => fromSelectOpt({ label: '', value: id })).filter((v) => v != undefined)
        if (props.single) {
            model.value = mappedOpts[0] ?? undefined
        }
        else {
            model.value = mappedOpts
        }

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

<style lang="scss" scoped>
.popup-button {
    height: 100%;
    width: 10px;
    transition: transform .28s;
}
</style>
<style lang="scss">
.popup-button>.q-focus-helper {
    display: none;
}
</style>