<template>
  <q-layout view="hHh lPr fFf">
    <!-- Top bar — minimal, translucent -->
    <q-header class="reader-bar" style="border-bottom: 1px solid var(--border-subtle)">
      <q-toolbar style="height: 44px">
        <q-btn flat round dense icon="arrow_back" size="sm" @click="$router.back()" />
        <q-toolbar-title class="text-weight-medium ellipsis" style="font-size: 13px; color: var(--text-secondary)">
          {{ book?.title || '加载中...' }}
        </q-toolbar-title>

        <q-btn flat round dense :icon="$q.dark.isActive ? 'light_mode' : 'dark_mode'" size="sm" @click="toggleDark">
          <q-tooltip>{{ $q.dark.isActive ? '亮色模式' : '暗色模式' }}</q-tooltip>
        </q-btn>
        <q-btn flat round dense icon="home" size="sm" @click="$router.push({ name: 'gallery' })">
          <q-tooltip>返回首页</q-tooltip>
        </q-btn>
      </q-toolbar>
    </q-header>

    <!-- Bottom bar — reading controls -->
    <q-footer style="background: var(--bg-surface); border-top: 1px solid var(--border-subtle)">
      <q-toolbar style="height: 52px" class="q-px-md">
        <q-btn flat round dense icon="navigate_before" :disable="currentPage <= 1" size="sm" @click="currentPage--">
          <q-tooltip>上一页</q-tooltip>
        </q-btn>

        <q-slider
          v-model="currentPage"
          :min="1"
          :max="book?.page_count ?? 1"
          color="brand"
          track-size="3px"
          thumb-size="14px"
          class="q-mx-md col"
        />

        <q-btn flat round dense icon="navigate_next" :disable="currentPage >= (book?.page_count ?? 1)" size="sm" @click="currentPage++">
          <q-tooltip>下一页</q-tooltip>
        </q-btn>

        <q-separator vertical inset class="q-mx-sm" style="opacity: 0.2" />

        <q-chip dense square style="font-size: 11px; min-width: 60px; justify-content: center">
          {{ currentPage }} / {{ book?.page_count ?? '-' }}
        </q-chip>

        <q-separator vertical inset class="q-mx-sm" style="opacity: 0.2" />

        <!-- Fit mode buttons -->
        <q-btn-toggle
          v-model="fitMode"
          no-caps flat dense
          toggle-color="brand"
          size="sm"
          :options="[
            { value: 'width', slot: 'width' },
            { value: 'screen', slot: 'screen' },
            { value: 'original', slot: 'original' }
          ]"
        >
          <template v-slot:width>
            <q-icon name="fit_screen" size="18px" />
            <q-tooltip>适应宽度</q-tooltip>
          </template>
          <template v-slot:screen>
            <q-icon name="fullscreen" size="18px" />
            <q-tooltip>适应屏幕</q-tooltip>
          </template>
          <template v-slot:original>
            <q-icon name="zoom_out_map" size="18px" />
            <q-tooltip>原始大小</q-tooltip>
          </template>
        </q-btn-toggle>
      </q-toolbar>
    </q-footer>

    <!-- Content -->
    <q-page-container>
      <q-page class="flex flex-center" style="overflow: auto; background: var(--bg-base)">
        <q-inner-loading :showing="loading">
          <q-spinner-dots size="50px" color="brand" />
        </q-inner-loading>

        <img
          v-if="!loading && book"
          :src="getPageUrl(currentPage, book?.optimization_strategy === 2)"
          :style="imgStyle"
          style="transition: max-width 0.3s ease, max-height 0.3s ease"
        />
      </q-page>
    </q-page-container>
  </q-layout>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { Dark, LocalStorage, useQuasar } from 'quasar'
import { useRoute } from 'vue-router'
import { getBook, getBookPages, pdfPageSvgUrl, imagePageByNameUrl, bookFormat, type BookDetail, type PageInfo } from '../api'

const route = useRoute()
const $q = useQuasar()
const props = defineProps<{ id: string }>()
const bookId = computed(() => props.id)

const book = ref<BookDetail | null>(null)
const pageNames = ref<PageInfo[]>([])  // image_book 页面文件名+尺寸列表
const loading = ref(true)
const currentPage = ref(Number(route.query.p) || 1)
const fitMode = ref<'width' | 'screen' | 'original'>('width')

function getPageUrl(page: number, realsize = false) {
  if (!book.value) return ''
  if (bookFormat(book.value.type) === 'pdf') {
    return pdfPageSvgUrl(bookId.value, page)
  }
  // image_book：优先用文件名 URL
  const info = pageNames.value[page - 1]
  if (info) {
    return imagePageByNameUrl(bookId.value, info.filename, realsize)
  }
  // 兜底
  return `/api/books/${bookId.value}/${page}${realsize ? '?realsize=true' : ''}`
}

const imgStyle = computed(() => {
  switch (fitMode.value) {
    case 'width':
      return { maxWidth: '100%', maxHeight: 'none', objectFit: 'contain' }
    case 'screen':
      return { maxWidth: '100%', maxHeight: 'calc(100vh - 96px)', objectFit: 'contain' }
    case 'original':
      return { maxWidth: 'none', maxHeight: 'none', objectFit: 'none' }
    default:
      return {}
  }
})

function toggleDark() {
  Dark.toggle()
  LocalStorage.set('darkMode', Dark.isActive)
}

function handleKey(e: KeyboardEvent) {
  if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
    if (currentPage.value > 1) currentPage.value--
  } else if (e.key === 'ArrowRight' || e.key === 'ArrowDown' || e.key === ' ') {
    if (book.value && currentPage.value < book.value.page_count) currentPage.value++
  }
}

async function loadBook() {
  loading.value = true
  pageNames.value = []
  currentPage.value = Number(route.query.p) || 1
  try {
    book.value = await getBook(bookId.value)
    // 所有类型都加载页面列表和尺寸信息
    if (book.value) {
      try {
        pageNames.value = await getBookPages(bookId.value)
      } catch {
        console.warn('Failed to load page info, falling back to index-based URLs')
      }
    }
  }
  finally { loading.value = false }
}

onMounted(() => {
  loadBook()
  window.addEventListener('keydown', handleKey)
})

watch(bookId, () => { loadBook() })

onUnmounted(() => {
  window.removeEventListener('keydown', handleKey)
})
</script>
