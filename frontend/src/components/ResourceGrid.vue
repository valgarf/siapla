<template>
  <div v-if="!resourceStore.loading" style="
      display:inline-grid;
      grid-template-columns: repeat(auto-fill, 200px);
      grid-auto-columns: 200px;
      grid-auto-rows: 180px;
      gap: 10px 10px;
      place-items: stretch;
      width:90%;
    ">
    <q-card v-ripple class="cursor-pointer q-hoverable" style="align-content: center;" @click="showDetailsNew()">
      <div tabindex="-1" class="q-focus-helper"></div>
      <q-card-section style="text-align: center;">
        <q-icon name="add" size="lg" />
      </q-card-section>
    </q-card>
    <ResourceCard :resource="t" v-for="t in resources" :key="t.dbId" />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import ResourceCard from './ResourceCard.vue';
import { useResourceStore } from 'src/stores/resource';
import { NewResourceSidebarData, useSidebarStore } from 'src/stores/sidebar';


const resourceStore = useResourceStore();
const sidebarStore = useSidebarStore();

const resources = computed(() => {
  const result = [...resourceStore.resources];
  result.sort((t1, t2) => t1.name < t2.name ? 1 : -1)
  return result
})

interface Props {
  issuesOnly?: boolean;
};
withDefaults(defineProps<Props>(), {
  issuesOnly: true
});

function showDetailsNew() {
  sidebarStore.pushSidebar(new NewResourceSidebarData())
}

</script>
