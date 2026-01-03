import { acceptHMRUpdate, defineStore } from 'pinia'
import { ref } from 'vue'

export type SelectionMode = 'TASK' | 'RESOURCE' | null

export const useSelectionStore = defineStore('selection', () => {
    const mode = ref<SelectionMode>(null)
    const possible = ref<number[]>([])
    const selected = ref<number[]>([])
    const single = ref(false)
    const ownerKey = ref<string | null>(null)

    function setMode(m: SelectionMode) {
        mode.value = m
    }
    function setKey(k: string | null) { ownerKey.value = k }
    function setPossible(ids: number[]) {
        possible.value = ids.slice()
        // prune selected to possible
        selected.value = selected.value.filter((id) => possible.value.includes(id))
    }
    function setSingle(v: boolean) { single.value = v }
    function clear() {
        mode.value = null
        possible.value = []
        selected.value = []
        single.value = false
    }
    function setSelected(ids: number[]) {
        if (single.value && ids.length > 1) ids = ids.slice(0, 1)
        selected.value = ids.filter((id) => possible.value.length === 0 || possible.value.includes(id))
    }
    function toggle(id: number) {
        if (possible.value.length > 0 && !possible.value.includes(id)) return
        const idx = selected.value.indexOf(id)
        if (idx >= 0) {
            selected.value.splice(idx, 1)
        } else {
            if (single.value) selected.value = [id]
            else selected.value.push(id)
        }
    }

    const isSelectable = (id: number) => (possible.value.length === 0 || possible.value.includes(id))
    const isSelected = (id: number) => selected.value.includes(id)

    const key = () => ownerKey.value

    return {
        mode,
        possible,
        selected,
        single,
        ownerKey,
        setMode,
        setKey,
        setPossible,
        setSingle,
        clear,
        setSelected,
        toggle,
        isSelectable,
        isSelected,
        key,
    }
})

if (import.meta.hot) {
    import.meta.hot.accept(acceptHMRUpdate(useSelectionStore, import.meta.hot));
}