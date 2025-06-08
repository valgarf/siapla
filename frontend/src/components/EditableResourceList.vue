<template>
    <q-select v-if="edit" filled v-model="select_model" multiple :options="select_possible" use-chips stack-label
        :label="name" />
    <div v-else-if="model.length" class="col">
        <div class="text-subtitle2">{{ name }}</div>
        <ResourceChip v-for="resource in model" :key="resource.dbId" :resource="resource" />
    </div>
</template>

<script setup lang="ts">
import { useResourceStore, type Resource } from 'src/stores/resource';
import { computed, type Ref } from 'vue';
import ResourceChip from './ResourceChip.vue';

interface Props {
    name: string;
    edit: boolean;
    possible: Resource[];
};
const props = withDefaults(defineProps<Props>(), {});
const model: Ref<Resource[]> = defineModel({ required: true })

const resourceStore = useResourceStore();

// Workaround for q-select: use ids instead of objects
interface SelectOpt {
    label: string,
    value: number,
}

function to_select_opt(r: Resource): SelectOpt {
    return { label: r.name, value: r.dbId }
}
function from_select_opt(r: SelectOpt): Resource | undefined {
    return resourceStore.resource(r.value)
}

const select_model = computed({
    get() {
        return model.value.map(to_select_opt) || []
    },
    set(value: SelectOpt[]) {
        model.value = value.map(from_select_opt).filter((v: Resource | undefined) => v != undefined)
    }
})
const select_possible = computed(() => { return props.possible.map(to_select_opt) })
</script>
