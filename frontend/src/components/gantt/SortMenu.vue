<template>
  <!-- activator slot: parent may provide a button or other control. -->
  <slot name="activator" :toggle="toggle" :open="open" :close="close" />

  <q-menu v-model="visible" self="bottom left" anchor="top left">
    <q-card style="min-width: 240px">
      <q-card-section>
        <div style="font-weight:600">Sort rows</div>
      </q-card-section>
      <q-separator />
      <Draggable v-model="modelOptions" item-key="key" animation="150" handle=".drag-handle" class="draggable-list">
        <template #item="{ element }">
          <div :key="element.key" class="sort-item" @click.stop="element.asc = !element.asc">
            <q-item class="drag-handle">
              <q-item-section side class="drag-indicator">
                <q-icon name="drag_indicator" />
              </q-item-section>
              <q-item-section>
                <div>{{ element.label }}</div>
              </q-item-section>
              <q-item-section side>
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

// visibility is controlled from an activator (e.g. a button) on the outside
const visible = ref(false)
function toggle() { visible.value = !visible.value }
function open() { visible.value = true }
function close() { visible.value = false }

// model is just passed through to 'Draggable'
const modelOptions = defineModel<SortOption[]>({ required: true })
</script>

<style scoped>
.sort-item {
  cursor: pointer
}

.sort-item:active {
  cursor: grabbing
}

.drag-indicator {
  cursor: grab
}
</style>