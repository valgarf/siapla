<template>
  <div :style="`max-width: ${props.maxWidth}px`">
    <q-input :dense="props.dense" filled stack-label :label="props.label" mask="####-##-## ##:##" v-model="localInput"
      @blur="applyLocalInput()">
      <template v-slot:prepend>
        <q-icon name="event" class="cursor-pointer">
          <q-popup-proxy cover transition-show="scale" transition-hide="scale">
            <q-date v-model="datePickerModel" mask="YYYY-MM-DD HH:mm">
              <div class="row items-center justify-end">
                <q-btn v-close-popup label="Close" color="primary" flat />
              </div>
            </q-date>
          </q-popup-proxy>
        </q-icon>
      </template>

      <template v-slot:append>
        <q-icon name="access_time" class="cursor-pointer">
          <q-popup-proxy cover transition-show="scale" transition-hide="scale">
            <q-time v-model="datePickerModel" mask="YYYY-MM-DD HH:mm" format24h>
              <div class="row items-center justify-end">
                <q-btn v-close-popup label="Close" color="primary" flat />
              </div>
            </q-time>
          </q-popup-proxy>
        </q-icon>
      </template>
    </q-input>
  </div>
</template>

<script setup lang="ts">
import { date } from 'quasar';
import { type Ref } from 'vue'

interface Props {
  label: string
  maxWidth?: number
  dense?: boolean
}

const props = withDefaults(defineProps<Props>(), { maxWidth: 300, dense: false })

const model: Ref<Date | null> = defineModel({ required: true })

import { ref, watch } from 'vue'
import { debounce } from 'quasar'

// local string shown in input; updates to model are debounced
const localInput = ref('')
const datePickerModel = ref('')

function formatModel(d: Date | null) {
  return d == null ? '' : date.formatDate(d, 'YYYY-MM-DD HH:mm')
}

// initialize
localInput.value = formatModel(model.value)
datePickerModel.value = formatModel(model.value)

// when date picker (date/time popup) changes, apply immediately
watch(datePickerModel, (val) => {
  localInput.value = val
  applyToModel(val)
})

// when external model changes, refresh local strings
watch(model, (val) => {
  const f = formatModel(val)
  localInput.value = f
  datePickerModel.value = f
})

function applyToModel(val: string | null | undefined) {
  const parsed = val ? new Date(val) : null
  model.value = parsed
}

function applyLocalInput() {
  applyToModel(localInput.value)
}

// debounce applying input to the real model to wait until typing finishes
const applyLocalInputDebounced = debounce(() => {
  applyLocalInput()
}, 2000)

// debounce when typing
watch(localInput, () => {
  applyLocalInputDebounced()
})
</script>