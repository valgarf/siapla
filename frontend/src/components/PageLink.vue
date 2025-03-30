<template>
  <q-item clickable tag="a" :target="newPage ? '_blank' : undefined" :to="link">
    <q-item-section v-if="icon" avatar>
      <q-icon :name="icon" />
    </q-item-section>

    <q-item-section>
      <q-item-label>{{ title }}</q-item-label>
      <q-item-label caption>{{ caption }}</q-item-label>
    </q-item-section>
  </q-item>
</template>

<script setup lang="ts">
export interface PageLinkProps {
  title: string;
  caption?: string;
  link?: string;
  icon?: string;
  newPage?: boolean;
};

withDefaults(defineProps<PageLinkProps>(), {
  caption: '',
  link: '#',
  icon: '',
  newPage: false,
});
</script>
