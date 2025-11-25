<template>
  <q-layout view="hHh Lpr lFf">
    <q-header elevated>
      <q-toolbar>
        <q-btn flat dense round icon="menu" aria-label="Menu" @click="toggleLeftDrawer" />

        <q-toolbar-title>
          SIAPLA
        </q-toolbar-title>

        <!-- :icon="sidebarStore.isOpen ? 'expand_circle_up' : 'expand_circle_down'"  (expand_circle_up does not work)-->
        <q-btn flat dense round :icon="sidebarStore.isOpen ? 'expand_less' : 'expand_more'" aria-label="Toggle sidebar"
          @click="toggleSidebar" />
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
    <q-drawer side="right" :model-value="sidebarStore.isOpen" bordered :elevated="false"
      :width="sidebarStore.isExpanded ? windowSize.width - 57 : DEFAULT_SIDEBAR_WIDTH" @before-hide="sidebarStartHiding"
      @hide="sidebarHidden">
      <div class="q-pa-md sidebar-content">
        <SidebarComponentSelector />
      </div>
    </q-drawer>
  </q-layout>
</template>

<script setup lang="ts">
import { ref } from 'vue';
// import EssentialLink, { type EssentialLinkProps } from 'components/EssentialLink.vue';
import PageLink, { type PageLinkProps } from 'components/PageLink.vue';
import SidebarComponentSelector from 'components/SidebarComponentSelector.vue';
import { useSidebarStore } from 'src/stores/sidebar';

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

const sidebarStore = useSidebarStore();

import { onMounted, onUnmounted } from 'vue';

const DEFAULT_SIDEBAR_WIDTH = 560;

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

function sidebarStartHiding() {
  window.dispatchEvent(new CustomEvent('sidebarClosing'));
}

function sidebarHidden() {
  window.dispatchEvent(new CustomEvent('sidebarClosed'));
}

function toggleSidebar() {
  sidebarStore.toggleOpen();
}

function toggleLeftDrawer() {
  leftDrawerOpen.value = !leftDrawerOpen.value;
}
</script>

<style scoped>
.sidebar-expanded {
  width: 100% !important;
}

/* ensure drawer inner content is scrollable and can reach last element */
.sidebar-content {
  max-height: calc(100vh - 56px);
  /* account for header */
  overflow: auto;
}
</style>
