<template>
  <q-layout view="hHh Lpr lFf">
    <q-header elevated>
      <q-toolbar>
        <q-btn flat dense round icon="menu" aria-label="Menu" @click="toggleLeftDrawer" />

        <q-toolbar-title>
          SIAPLA
        </q-toolbar-title>

        <!-- :icon="dialogStore.isOpen ? 'expand_circle_up' : 'expand_circle_down'"  (expand_circle_up does not work)-->
        <q-btn flat dense round icon="info" aria-label="Toggle sidebar" @click="toggleSidebar" />
      </q-toolbar>
    </q-header>

    <q-drawer v-model="leftDrawerOpen" show-if-above mini-to-overlay bordered :width="200" :mini="leftDrawerMini"
      @mouseenter="leftDrawerMini = false" @mouseleave="leftDrawerMini = true" elevated>
      <q-list>
        <PageLink v-for="page in pages" :key="page.title" v-bind="page" />
      </q-list>
      <!-- <q-list>
        <q-item-label header>
          Essential Links
        </q-item-label>

        <EssentialLink v-for="link in linksList" :key="link.title" v-bind="link" />
      </q-list> -->
    </q-drawer>

    <q-page-container>
      <router-view />
    </q-page-container>
    <q-drawer side="right" :model-value="dialogStore.isOpen" bordered :elevated="false"
      :width="dialogStore.isExpanded ? windowSize.width - 57 : 560">
      <div class="q-pa-md">
        <DialogHandler />
      </div>
    </q-drawer>
  </q-layout>
</template>

<script setup lang="ts">
import { ref } from 'vue';
// import EssentialLink, { type EssentialLinkProps } from 'components/EssentialLink.vue';
import PageLink, { type PageLinkProps } from 'components/PageLink.vue';
import DialogHandler from 'components/DialogHandler.vue';
import { useDialogStore } from 'src/stores/dialog';

// const linksList: EssentialLinkProps[] = [
//   {
//     title: 'Docs',
//     caption: 'quasar.dev',
//     icon: 'school',
//     link: 'https://quasar.dev'
//   },
//   {
//     title: 'Github',
//     caption: 'github.com/quasarframework',
//     icon: 'code',
//     link: 'https://github.com/quasarframework'
//   },
//   {
//     title: 'Discord Chat Channel',
//     caption: 'chat.quasar.dev',
//     icon: 'chat',
//     link: 'https://chat.quasar.dev'
//   },
//   {
//     title: 'Forum',
//     caption: 'forum.quasar.dev',
//     icon: 'record_voice_over',
//     link: 'https://forum.quasar.dev'
//   },
//   {
//     title: 'Twitter',
//     caption: '@quasarframework',
//     icon: 'rss_feed',
//     link: 'https://twitter.quasar.dev'
//   },
//   {
//     title: 'Facebook',
//     caption: '@QuasarFramework',
//     icon: 'public',
//     link: 'https://facebook.quasar.dev'
//   },
//   {
//     title: 'Quasar Awesome',
//     caption: 'Community Quasar projects',
//     icon: 'favorite',
//     link: 'https://awesome.quasar.dev'
//   }
// ];

const pages: PageLinkProps[] = [
  {
    icon: 'task_alt',
    title: "Tasks",
    link: "/"
  },
  {
    icon: 'person',
    title: "Resources",
    link: "/resources"
  }
  // {
  //   title: "Tasks",
  //   link: "/tasks"
  // },
  // {
  //   title: "Resources",
  //   link: "/resources"
  // },


]

const leftDrawerOpen = ref(false);
const leftDrawerMini = ref(true);

const dialogStore = useDialogStore();

import { onMounted, onUnmounted } from 'vue';

const windowSize = ref({ width: typeof window !== 'undefined' ? window.innerWidth : 1024, height: typeof window !== 'undefined' ? window.innerHeight : 768 });

function updateWindowSize() {
  if (typeof window === 'undefined') return;
  windowSize.value.width = window.innerWidth;
  windowSize.value.height = window.innerHeight;
}

onMounted(() => {
  if (typeof window !== 'undefined') {
    window.addEventListener('resize', updateWindowSize);
  }
});
onUnmounted(() => {
  if (typeof window !== 'undefined') {
    window.removeEventListener('resize', updateWindowSize);
  }
});

function toggleSidebar() {
  dialogStore.toggleOpen();
}

function toggleLeftDrawer() {
  leftDrawerOpen.value = !leftDrawerOpen.value;
}
</script>

<style scoped>
.sidebar-expanded {
  width: 100% !important;
}
</style>
