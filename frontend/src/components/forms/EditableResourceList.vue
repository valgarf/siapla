<template>
    <q-select v-if="edit" filled v-model="select_model" multiple :options="selectPossible" use-chips use-input
        stack-label :label="name" @focus="beginSelection" @blur="endSelection" @filter="filterFn" />
    <div v-else-if="model.length" class="col">
        <div class="text-subtitle2">{{ name }}</div>
        <ResourceChip v-for="resource in model" :key="resource.dbId" :resource="resource" />
    </div>
</template>

<script setup lang="ts">
import { useResourceStore, type Resource } from 'src/stores/resource';
import { computed, ref, type Ref } from 'vue';
import ResourceChip from '../common/ResourceChip.vue';

interface Props {
    name: string;
    edit: boolean;
    possible: Resource[];
    single?: boolean;
    selectKey?: string;
};
const props = withDefaults(defineProps<Props>(), { single: false, selectKey: '' });
const model: Ref<Resource[]> = defineModel({ required: true })

const resourceStore = useResourceStore();

// Workaround for q-select: use ids instead of objects
interface SelectOpt {
    label: string,
    value: number,
}

function toSelectOpt(r: Resource): SelectOpt {
    return { label: r.name, value: r.dbId }
}
function fromSelectOpt(r: SelectOpt): Resource | undefined {
    return resourceStore.resource(r.value)
}

const select_model = computed({
    get() {
        return model.value.map(toSelectOpt) || []
    },
    set(value: SelectOpt[]) {
        model.value = value.map(fromSelectOpt).filter((v: Resource | undefined) => v != undefined)
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

function possibleIds(arr: Resource[]) { return arr.map(r => r.dbId) }

let selStop: (() => void) | null = null
let possibleStop: (() => void) | null = null
let modelStop: (() => void) | null = null
let focused = false

function beginSelection() {
    if (focused) return
    focused = true
    selection.setMode('RESOURCE')
    selection.setKey(props.selectKey)
    selection.setPossible(possibleIds(props.possible))
    selection.setSingle(!!props.single)
    selection.setSelected(model.value.map(r => r.dbId))

    possibleStop = watch(() => props.possible, (n) => selection.setPossible(possibleIds(n)), { deep: true })
    modelStop = watch(model, (nv) => selection.setSelected(nv.map(r => r.dbId)), { deep: true })
    selStop = watch(() => selection.selected, (ids) => {
        if (selection.mode !== 'RESOURCE') return
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
    if (selection.mode === 'RESOURCE' && selection.key() === props.selectKey) selection.clear()

}

onUnmounted(() => {
    endSelection()
})
</script>
