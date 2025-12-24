import { type Ref, ref } from 'vue'

// Shared gantt scrolling/panning state across multiple charts
export const scrollX = ref(0)
export const scrollYMap: Ref<{ [name: string]: number }> = ref({})
export const panInitialized = ref<boolean>(false)
