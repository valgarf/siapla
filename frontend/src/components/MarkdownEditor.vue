<template>
    <q-card flat bordered v-if="preview">
        <q-tabs v-model="mode" dense class="text-grey" active-color="primary" indicator-color="primary" align="justify"
            narrow-indicator>
            <q-tab name="markdown" label="Markdown" />
            <q-tab name="preview" label="Preview" />
        </q-tabs>

        <q-separator />

        <q-tab-panels v-model="mode" animated>
            <q-tab-panel name="markdown" class="q-pa-s q-pt-none">
                <q-input borderless :label="label" :placeholder="placeholder" v-model="markdown" autogrow
                    class="q-pa-none" />
            </q-tab-panel>
            <q-tab-panel name="preview" class="q-pa-s">
                <q-markdown :src="markdown" />
            </q-tab-panel>
        </q-tab-panels>
    </q-card>
    <q-input v-else outlined :label="label" :placeholder="placeholder" v-model="markdown" autogrow class="q-pa-none" />
</template>

<script setup lang="ts">
import { ref } from 'vue';

const markdown = defineModel<string>({ default: '', required: true })
interface Props {
    preview?: boolean
    label?: string
    placeholder?: string
}
defineProps<Props>()

const mode = ref("markdown")

</script>