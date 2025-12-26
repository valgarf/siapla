<template>
  <!-- activator slot: parent may provide a button or other control. -->
  <slot name="activator" :toggle="toggle" :open="open" />

  <q-menu v-model="visible" self="bottom left" anchor="top left">
    <q-card style="min-width: 240px">
      <q-card-section>
        <div style="font-weight:600">Sort rows</div>
      </q-card-section>
      <q-separator />
      <Draggable v-model="modelOptions" item-key="key" animation="150" handle=".drag-handle" class="draggable-list">
        <template #item="{ element, index }">
          <div :key="element.key" class="sort-item">
            <q-item class="drag-handle">
              <q-item-section avatar>
                <q-icon name="drag_indicator" />
              </q-item-section>
              <q-item-section>
                <div @click.stop="toggleDir(index)">{{ element.label }}</div>
              </q-item-section>
              <q-item-section side top>
                <q-icon :name="element.asc ? 'arrow_upward' : 'arrow_downward'" size="16px" />
              </q-item-section>
            </q-item>
          </div>
        </template>
      </Draggable>
    </q-card>
  </q-menu>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import Draggable from 'vuedraggable'

interface SortOption { key: string; label: string; asc: boolean }

// internal visibility for the menu; parent does not need to control it
const visible = ref(false)
function toggle() { visible.value = !visible.value }
function open() { visible.value = true }

// proxy the external modelValue through a computed so Draggable mutates it
const modelOptions = defineModel<SortOption[]>({ required: true })

function toggleDir(i: number) {
  const arr = modelOptions.value?.slice()
  const existing = arr[i]
  if (!existing) return
  arr[i] = { ...existing, asc: !existing.asc }
  // assign back to trigger setter and emit
  modelOptions.value = arr
}
</script>

<style scoped>
.sort-list {
  display: flex;
  flex-direction: column
}

.sort-item {
  cursor: grab
}

.sort-item:active {
  cursor: grabbing
}

.drop-end {
  height: 8px
}
</style>