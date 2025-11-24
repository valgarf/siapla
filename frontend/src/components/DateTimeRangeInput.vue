<template>
    <div class="row items-center" :style="`max-width: ${props.maxWidth}px`">
        <q-input dense filled stack-label :label="props.label" v-model="localRange" @blur="onInputBlur"
            mask="####-##-## ##:## — ####-##-## ##:##">
            <template v-slot:append>
                <q-btn dense flat round icon="access_time" @click.stop="proxyOpen = true"
                    aria-label="Open range picker" />
            </template>
        </q-input>

        <q-popup-proxy v-model="proxyOpen" cover transition-show="scale" transition-hide="scale">
            <div class="q-pa-sm date-range-popup">
                <div class="row q-gutter-sm items-center">
                    <div class="col">
                        <div class="text-subtitle2">Start</div>
                        <q-date v-model="startDate" mask="YYYY-MM-DD">
                            <div class="row items-center justify-end">
                                <q-btn v-close-popup label="Close" color="primary" flat
                                    @click.stop="proxyOpen = false" />
                            </div>
                        </q-date>
                    </div>
                    <div class="col">
                        <div class="text-subtitle2">Time</div>
                        <q-time v-model="startTime" format24h />
                    </div>
                </div>
                <div class="q-mt-sm">to</div>
                <div class="row q-gutter-sm items-center q-mt-sm">
                    <div class="col">
                        <div class="text-subtitle2">End</div>
                        <q-date v-model="endDate" mask="YYYY-MM-DD">
                            <div class="row items-center justify-end">
                                <q-btn v-close-popup label="Close" color="primary" flat
                                    @click.stop="proxyOpen = false" />
                            </div>
                        </q-date>
                    </div>
                    <div class="col">
                        <div class="text-subtitle2">Time</div>
                        <q-time v-model="endTime" format24h />
                    </div>
                </div>
                <div class="row justify-end q-mt-sm">
                    <q-btn flat label="Apply" color="primary" @click="() => { applyRange(); proxyOpen = false }" />
                </div>
            </div>
        </q-popup-proxy>
    </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { date, debounce } from 'quasar'

interface Props {
    label?: string
    maxWidth?: number
}
const props = withDefaults(defineProps<Props>(), { label: 'Range', maxWidth: 440 })

// accept an object-like v-model with start/end fields
const model = defineModel<Record<string, unknown> | null>({ required: true })

const localRange = ref('')
const proxyOpen = ref(false)

function fmt(d: Date | null | undefined) {
    return d == null ? '' : date.formatDate(d, 'YYYY-MM-DD HH:mm')
}

// date/time parts
const startDate = ref('')
const startTime = ref('00:00')
const endDate = ref('')
const endTime = ref('00:00')

// init from model when mounted / changes
watch(model, (v) => {
    const obj = v ?? null
    if (!obj) {
        localRange.value = ''
        startDate.value = ''
        startTime.value = '00:00'
        endDate.value = ''
        endTime.value = '00:00'
        return
    }
    // model expected to be an object like { start: Date|string, end: Date|string }
    const sRaw = obj['start'] ?? null
    const eRaw = obj['end'] ?? null
    if (sRaw && eRaw) {
        let s: Date | null = null
        let e: Date | null = null
        if (typeof sRaw === 'string' || typeof sRaw === 'number') s = new Date(String(sRaw))
        else if (sRaw instanceof Date) s = sRaw
        if (typeof eRaw === 'string' || typeof eRaw === 'number') e = new Date(String(eRaw))
        else if (eRaw instanceof Date) e = eRaw
        if (s && e) {
            startDate.value = date.formatDate(s, 'YYYY-MM-DD')
            startTime.value = date.formatDate(s, 'HH:mm')
            endDate.value = date.formatDate(e, 'YYYY-MM-DD')
            endTime.value = date.formatDate(e, 'HH:mm')
            localRange.value = `${fmt(s)} — ${fmt(e)}`
            return
        }
    }
    localRange.value = ''
}, { immediate: true })

function parseParts(): { start: Date | null; end: Date | null } {
    if (!startDate.value || !endDate.value) return { start: null, end: null }
    const s = new Date(startDate.value + 'T' + (startTime.value || '00:00'))
    const e = new Date(endDate.value + 'T' + (endTime.value || '00:00'))
    return { start: s, end: e }
}

function applyRange() {
    const { start, end } = parseParts()
    if (!start || !end) return
    // enforce max 1 year
    const oneYear = new Date(start)
    oneYear.setFullYear(oneYear.getFullYear() + 1)
    if (end > oneYear) {
        // clamp end
        end.setFullYear(start.getFullYear() + 1)
    }
    // set the v-model value to an object with ISO strings
    try {
        model.value = { start: start.toISOString(), end: end.toISOString() }
    } catch {
        // ignore
    }
    localRange.value = `${fmt(start)} — ${fmt(end)}`
}

// debounce typed input by 2 seconds; apply immediately on blur
function tryParseAndApply(val: string) {
    const parts = val.split(/\s+[-—–]+\s+/)
    if (parts.length !== 2) return
    const sRawPart = parts[0]
    const eRawPart = parts[1]
    if (!sRawPart || !eRawPart) return
    const sStr = String(sRawPart).trim()
    const eStr = String(eRawPart).trim()
    const s = new Date(sStr)
    const e = new Date(eStr)
    if (isNaN(s.getTime()) || isNaN(e.getTime())) return
    // clamp end to one year
    const oneYear = new Date(s)
    oneYear.setFullYear(oneYear.getFullYear() + 1)
    if (e > oneYear) e.setFullYear(s.getFullYear() + 1)
    model.value = { start: s.toISOString(), end: e.toISOString() }
    localRange.value = `${fmt(s)} — ${fmt(e)}`
}

const applyDebounced = debounce((val: string) => {
    tryParseAndApply(val)
}, 2000)

function onInputBlur() {
    applyDebounced.cancel()
    tryParseAndApply(localRange.value)
}

watch(localRange, (val) => {
    applyDebounced(val)
})
</script>
