<template>
  <q-menu v-model="visible" self="bottom left" anchor="top left">
    <q-card style="min-width: 240px">
      <q-card-section>
        <div style="font-weight:600">Sort rows</div>
      </q-card-section>
      <q-separator />
      <q-list dense>
        <div class="sort-list">
          <div v-for="(opt, i) in localOptions" :key="opt.key" class="sort-item" :draggable="true"
            @dragstart="onDragStart($event, i)" @dragover="onDragOver($event)" @drop="onDrop($event, i)">
            <q-item>
              <q-item-section avatar>
                <q-icon name="drag_indicator" />
              </q-item-section>
              <q-item-section>
                <div @click.stop="toggleDir(i)">{{ opt.label }}</div>
              </q-item-section>
              <q-item-section side top>
                <q-icon :name="opt.asc ? 'arrow_upward' : 'arrow_downward'" size="16px" />
              </q-item-section>
            </q-item>
          </div>
          <div class="sort-item drop-end" @dragover.prevent @drop="onDropEnd($event)" />
        </div>
      </q-list>
    </q-card>
  </q-menu>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'

interface SortOption { key: string; label: string; asc: boolean }

const props = defineProps<{ modelValue: boolean; options: SortOption[] }>()
const emit = defineEmits<{
  (e: 'update:modelValue', v: boolean): void
  (e: 'update:options', v: SortOption[]): void
}>()

const visible = computed({
  get: () => props.modelValue,
  set: (v: boolean) => emit('update:modelValue', v)
})

// local copy to allow immediate visual reordering
const localOptions = ref<SortOption[]>(props.options.map(o => ({ ...o })))
watch(() => props.options, (nv) => { localOptions.value = nv.map(o => ({ ...o })) }, { deep: true })

const draggingIndex = ref<number | null>(null)

function onDragStart(e: DragEvent, idx: number) {
  draggingIndex.value = idx
  try { e.dataTransfer?.setData('text/plain', String(idx)) } catch { /* ignore */ }
  e.dataTransfer!.effectAllowed = 'move'
}

function onDragOver(e: DragEvent) {
  e.preventDefault()
}

function applyReorder(newArr: SortOption[]) {
  localOptions.value = newArr
  emit('update:options', newArr.map(o => ({ ...o })))
}

function onDrop(e: DragEvent, idx: number) {
  e.preventDefault()
  const from = draggingIndex.value
  if (from == null) return
  if (from === idx) return
  const arr = localOptions.value.slice()
  const item = arr.splice(from, 1)[0]
  if (!item) return
  arr.splice(idx, 0, item)
  applyReorder(arr)
  draggingIndex.value = null
}

function onDropEnd(e: DragEvent) {
  e.preventDefault()
  const from = draggingIndex.value
  if (from == null) return
  const arr = localOptions.value.slice()
  const item = arr.splice(from, 1)[0]
  if (!item) return
  arr.push(item)
  applyReorder(arr)
  draggingIndex.value = null
}

function toggleDir(i: number) {
  const arr = localOptions.value.slice()
  const existing = arr[i]
  if (!existing) return
  arr[i] = { ...existing, asc: !existing.asc }
  applyReorder(arr)
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