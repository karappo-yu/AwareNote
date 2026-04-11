<template>
  <q-layout view="hHh LpR fFf">
    <!-- Header — glass effect -->
    <q-header class="gallery-header">
      <q-toolbar class="q-px-md" style="height: 56px">
        <q-btn flat round dense icon="menu" @click="drawer = !drawer" size="sm" />

        <q-toolbar-title shrink class="text-weight-bold q-ml-xs" style="letter-spacing: 0.5px; font-size: 18px">
          <q-icon name="auto_stories" class="q-mr-xs" color="brand" />
          Art Gallery
        </q-toolbar-title>

        <q-input
          v-model="library.searchQuery"
          dense outlined
          placeholder="搜索书名..."
          class="q-ml-lg"
          style="min-width: 280px; max-width: 400px"
          bg-color="transparent"
          color="brand"
        >
          <template v-slot:prepend><q-icon name="search" size="18px" /></template>
          <template v-slot:append>
            <q-icon
              v-if="library.searchQuery"
              name="close"
              class="cursor-pointer"
              size="16px"
              @click="library.searchQuery = ''"
            />
          </template>
        </q-input>

        <q-space />

        <q-btn flat round :icon="$q.dark.isActive ? 'light_mode' : 'dark_mode'" size="sm" @click="toggleDark">
          <q-tooltip>{{ $q.dark.isActive ? '亮色模式' : '暗色模式' }}</q-tooltip>
        </q-btn>

        <q-btn flat round icon="favorite_border" size="sm" @click="$router.push({ name: 'favorites' })">
          <q-tooltip>我的收藏</q-tooltip>
        </q-btn>
        <q-btn flat round icon="settings" size="sm" @click="$router.push({ name: 'settings' })">
          <q-tooltip>系统设置</q-tooltip>
        </q-btn>
      </q-toolbar>
    </q-header>

    <!-- Sidebar — clean categories, auto-fit width -->
    <q-drawer
      v-model="drawer"
      show-if-above
      :width="drawerWidth"
      :breakpoint="768"
      class="gallery-drawer"
    >
      <q-scroll-area class="fit drawer-scroll">
        <div class="q-pa-md" style="width: 320px; max-width: 320px; box-sizing: border-box">
          <div class="text-overline q-mb-sm" style="font-size: 10px; letter-spacing: 2px; color: var(--text-muted)">
            分类
          </div>

          <q-item
            dense
            clickable
            :active="library.selectedCategory === null && $route.name === 'gallery'"
            active-class="glow-active"
            class="q-mb-xs"
            style="border-radius: 8px"
            @click="selectCategory(null)"
          >
            <q-item-section>
              <q-item-label style="font-size: 13px; font-weight: 600">全部书籍</q-item-label>
            </q-item-section>
            <q-item-section side>
              <span style="font-size: 11px; color: var(--text-muted)">{{ library.books.length }}</span>
            </q-item-section>
          </q-item>

          <category-tree
            :categories="library.categoryTree"
            :selected="library.selectedCategory"
            @select="selectCategory"
          />
        </div>
      </q-scroll-area>

      <!-- Resize handle removed — width auto-fits content -->
    </q-drawer>

    <!-- Main content -->
    <q-page-container>
      <router-view />
    </q-page-container>
  </q-layout>
</template>

<script setup lang="ts">
import { ref, onMounted, onActivated, nextTick } from 'vue'
import { Dark, LocalStorage, useQuasar } from 'quasar'
import { useRoute, useRouter, onBeforeRouteLeave } from 'vue-router'
import { useLibrary } from '../stores/library'
import CategoryTree from '../components/CategoryTree.vue'

defineOptions({ name: 'GalleryLayout' })

const drawer = ref(false)
const library = useLibrary()
const $q = useQuasar()
const route = useRoute()
const router = useRouter()

// Fixed drawer width — same as original app (w-80 = 320px)
const drawerWidth = 320

function selectCategory(catId: number | null) {
  library.selectedCategory = catId
  // Navigate back to gallery if currently on another page (favorites, settings)
  if (route.name !== 'gallery') {
    router.push({ name: 'gallery' })
  }
}

function toggleDark() {
  Dark.toggle()
  LocalStorage.set('darkMode', Dark.isActive)
}

// 滚动位置保存与恢复（keep-alive）
// 关键：在 onDeactivated 时 DOM 已切换，scrollY 已被重置
// 必须在 onBeforeRouteLeave 时捕获真实滚动位置
const savedScrollTop = ref(0)

onBeforeRouteLeave((_to, from) => {
  if (from.name === 'gallery' || from.name === 'favorites' || from.name === 'settings') {
    savedScrollTop.value = window.scrollY
  }
})

onActivated(() => {
  if (savedScrollTop.value > 0) {
    nextTick(() => {
      requestAnimationFrame(() => {
        window.scrollTo(0, savedScrollTop.value)
      })
    })
  }
})

onMounted(async () => {
  await Promise.all([library.fetchBooks(), library.fetchCategories()])
})
</script>

<style lang="sass" scoped>
.drawer-scroll
  :deep(.q-scrollarea__content)
    position: relative !important
    width: 100% !important

  // Ultra-thin scrollbar that auto-hides
  :deep(.q-scrollarea__container)
    scrollbar-width: none  // Firefox: hide by default

  :deep(.q-scrollarea__thumb)
    background: transparent !important
    width: 3px !important
    border-radius: 3px
    transition: background 0.3s ease

  :deep(.q-scrollarea__bar)
    background: transparent !important
    width: 3px !important
    opacity: 0
    transition: opacity 0.3s ease

  // Show scrollbar only on hover
  &:hover
    :deep(.q-scrollarea__thumb)
      background: var(--border-light) !important

    :deep(.q-scrollarea__bar)
      opacity: 1

    :deep(.q-scrollarea__container)
      scrollbar-width: thin
      scrollbar-color: var(--border-light) transparent

  // Webkit: hide by default, show on hover
  :deep(.q-scrollarea__container)::-webkit-scrollbar
    width: 3px

  :deep(.q-scrollarea__container)::-webkit-scrollbar-track
    background: transparent

  :deep(.q-scrollarea__container)::-webkit-scrollbar-thumb
    background: transparent
    border-radius: 3px
    transition: background 0.3s

  &:hover :deep(.q-scrollarea__container)::-webkit-scrollbar-thumb
    background: var(--border-light)
</style>
