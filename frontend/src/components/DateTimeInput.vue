<template>
  <div class="" style="max-width: 300px">
    <q-input filled stack-label :label="label" v-model="input_model">
      <template v-slot:prepend>
        <q-icon name="event" class="cursor-pointer">
          <q-popup-proxy cover transition-show="scale" transition-hide="scale">
            <q-date v-model="input_model" mask="YYYY-MM-DD HH:mm">
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
            <q-time v-model="input_model" mask="YYYY-MM-DD HH:mm" format24h>
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
import { computed, type Ref } from 'vue'

interface Props {
  label: string
}

defineProps<Props>()

const model: Ref<Date | null> = defineModel({ required: true })

const input_model = computed({
  get(): string {
    if (model.value == null) {
      return ''
    }
    return date.formatDate(model.value, 'YYYY-MM-DD HH:mm')
  },
  set(value: string) {
    model.value = new Date(value)
  }
})
</script>