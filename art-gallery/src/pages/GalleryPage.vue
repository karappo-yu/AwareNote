<template>
  <q-page class="q-pa-lg" style="position: relative; min-height: 100vh">
    <!-- 返回顶部按钮 -->
    <q-page-sticky position="bottom-right" :offset="[18, 18]" style="z-index: 9999">
      <q-btn
        class="back-to-top-btn"
        :class="{ visible: showBackToTop }"
        round
        color="brand"
        icon="keyboard_arrow_up"
        size="md"
        @click="scrollToTop"
      >
        <q-tooltip>返回顶部</q-tooltip>
      </q-btn>
    </q-page-sticky>

    <!-- Toolbar — clean & compact -->
    <div class="row items-center q-mb-lg" style="position: relative; z-index: 1">
      <div>
        <div class="section-title">
          {{ library.selectedCategory ? currentCategoryName : '全部书籍' }}
        </div>
        <div class="section-subtitle">
          {{ library.filteredBooks.length }} 本
        </div>
      </div>

      <q-space />

      <q-btn-toggle
        v-model="library.sortBy"
        no-caps flat dense
        toggle-color="brand"
        class="q-mr-sm"
        :options="[
          { label: '日期', value: 'date' },
          { label: '标题', value: 'title' },
          { label: '大小', value: 'size' },
          { label: '页数', value: 'pages' }
        ]"
      />

      <q-btn
        flat round dense
        :icon="library.sortDesc ? 'arrow_downward' : 'arrow_upward'"
        color="brand"
        size="sm"
        @click="library.sortDesc = !library.sortDesc"
      >
        <q-tooltip>{{ library.sortDesc ? '降序' : '升序' }}</q-tooltip>
      </q-btn>

      <q-separator vertical inset class="q-mx-sm" />

      <q-btn-toggle
        v-model="viewMode"
        no-caps flat dense
        toggle-color="brand"
        :options="[
          { icon: 'grid_view', value: 'grid' },
          { icon: 'view_list', value: 'list' },
          { icon: 'view_week', value: 'masonry' }
        ]"
      />
    </div>

    <!-- Loading -->
    <q-inner-loading :showing="library.loading">
      <q-spinner-dots size="50px" color="brand" />
    </q-inner-loading>

    <!-- Empty state -->
    <div v-if="!library.loading && library.filteredBooks.length === 0" class="column items-center q-mt-xl">
      <q-icon name="menu_book" size="80px" style="opacity: 0.15" />
      <div class="text-h6 q-mt-md" style="opacity: 0.3">没有找到书籍</div>
    </div>

    <!-- Grid View — Kavita/Plex inspired -->
    <div v-if="viewMode === 'grid' && !library.loading" class="row q-col-gutter-md">
      <div
        v-for="book in paginatedBooks"
        :key="book.id"
        class="col-6 col-sm-4 col-md-3 col-lg-2"
      >
        <q-card
          flat
          class="book-card cursor-pointer"
          @click="$router.push({ name: 'detail', params: { id: book.id } })"
        >
          <!-- Cover image -->
          <q-img
            :src="coverUrl(book.id)"
            :ratio="3/4"
            fit="cover"
          >
            <template v-slot:loading>
              <div class="full-width full-height flex flex-center" style="background: var(--bg-surface)">
                <q-skeleton type="rect" style="height: 100%; width: 100%" />
              </div>
            </template>

            <!-- Format badge -->
            <q-badge
              :color="bookFormat(book.type) === 'pdf' ? 'red-10' : 'teal-8'"
              floating
              class="format-badge q-mt-sm q-mr-sm"
            >
              {{ bookFormat(book.type).toUpperCase() }}
            </q-badge>

            <!-- Favorite icon -->
            <q-icon
              v-if="book.is_favorite"
              name="favorite"
              color="pink-4"
              class="absolute-top-left q-ma-sm"
              size="18px"
            />
          </q-img>

          <!-- Card info — clean Kavita style -->
          <div class="book-card-info">
            <div class="book-card-title">{{ book.title }}</div>
            <div class="book-card-meta">
              {{ book.page_count }} 页 · {{ formatSize(book.size) }}
            </div>
          </div>
        </q-card>
      </div>
    </div>

    <!-- Masonry View — infinite scroll, no pagination -->
    <div v-if="viewMode === 'masonry' && !library.loading" class="row q-col-gutter-md">
      <div
        v-for="book in library.filteredBooks"
        :key="book.id"
        class="col-6 col-sm-4 col-md-3 col-lg-2"
      >
        <q-card
          flat
          class="book-card cursor-pointer"
          @click="$router.push({ name: 'detail', params: { id: book.id } })"
        >
          <q-img
            :src="coverUrl(book.id)"
            :ratio="3/4"
            fit="cover"
            loading="lazy"
          >
            <template v-slot:loading>
              <div class="full-width full-height flex flex-center" style="background: var(--bg-surface)">
                <q-skeleton type="rect" style="height: 100%; width: 100%" />
              </div>
            </template>

            <q-badge
              :color="bookFormat(book.type) === 'pdf' ? 'red-10' : 'teal-8'"
              floating
              class="format-badge q-mt-sm q-mr-sm"
            >
              {{ bookFormat(book.type).toUpperCase() }}
            </q-badge>

            <q-icon
              v-if="book.is_favorite"
              name="favorite"
              color="pink-4"
              class="absolute-top-left q-ma-sm"
              size="18px"
            />
          </q-img>

          <div class="book-card-info">
            <div class="book-card-title">{{ book.title }}</div>
            <div class="book-card-meta">
              {{ book.page_count }} 页 · {{ formatSize(book.size) }}
            </div>
          </div>
        </q-card>
      </div>
    </div>

    <!-- List View — refined -->
    <q-list v-if="viewMode === 'list' && !library.loading" class="rounded-borders surface-card" style="border-radius: 14px; overflow: hidden">
      <q-item
        v-for="book in paginatedBooks"
        :key="book.id"
        clickable v-ripple
        class="q-py-sm"
        @click="$router.push({ name: 'detail', params: { id: book.id } })"
      >
        <q-item-section avatar>
          <q-avatar rounded size="56px" style="border-radius: 8px">
            <q-img :src="coverUrl(book.id)" fit="cover" style="border-radius: 8px" />
          </q-avatar>
        </q-item-section>

        <q-item-section>
          <q-item-label class="text-weight-medium" style="font-size: 14px">{{ book.title }}</q-item-label>
          <q-item-label caption style="font-size: 12px">
            {{ book.page_count }} 页 · {{ formatSize(book.size) }}
          </q-item-label>
        </q-item-section>

        <q-item-section side>
          <div class="row items-center q-gutter-xs">
            <q-badge
              :color="bookFormat(book.type) === 'pdf' ? 'red-10' : 'teal-8'"
              outline
              class="format-badge"
            >
              {{ bookFormat(book.type).toUpperCase() }}
            </q-badge>
            <q-icon v-if="book.is_favorite" name="favorite" color="pink-4" size="16px" />
          </div>
        </q-item-section>
      </q-item>
    </q-list>

    <!-- Pagination (grid/list only, masonry shows all) -->
    <div v-if="viewMode !== 'masonry' && totalPages > 1" class="row justify-center q-mt-lg">
      <q-pagination
        v-model="currentPage"
        :max="totalPages"
        :max-pages="9"
        direction-links boundary-links
        color="brand"
        size="md"
      />
    </div>
  </q-page>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useLibrary } from '../stores/library'
import { useSettings } from '../stores/settings'
import { coverUrl as coverUrlFn, bookFormat } from '../api'

const library = useLibrary()
const settings = useSettings()
const viewMode = ref<'grid' | 'list' | 'masonry'>((settings.get('gallery_view_mode') as 'grid' | 'list' | 'masonry') || 'masonry')
// settings 从后端加载完毕后，更新为持久化的视图模式
watch(() => settings.loaded, (ready) => {
  if (ready) viewMode.value = (settings.get('gallery_view_mode') as 'grid' | 'list' | 'masonry') || 'masonry'
})
const currentPage = ref(1)
const perPage = 30
const showBackToTop = ref(false)

// 持久化 viewMode 到设置
watch(viewMode, (val) => {
  settings.set('gallery_view_mode', val)
  currentPage.value = 1
})

// 切换目录时重置页码
watch(() => library.selectedCategory, () => {
  currentPage.value = 1
})

// 返回顶部
function scrollToTop() {
  window.scrollTo({ top: 0, behavior: 'smooth' })
}

// 页面有滚动 且 鼠标接近屏幕底部时显示
function onMouseMove(e: MouseEvent) {
  showBackToTop.value = window.scrollY > 300 && e.clientY > window.innerHeight - 150
}

onMounted(() => {
  document.addEventListener('mousemove', onMouseMove)
})
onUnmounted(() => {
  document.removeEventListener('mousemove', onMouseMove)
})

const coverUrl = coverUrlFn

const currentCategoryName = computed(() => {
  const find = (cats: typeof library.categories): string => {
    for (const c of cats) {
      if (c.id === library.selectedCategory) return c.name
      if (c.sub_categories?.length) {
        const r = find(c.sub_categories)
        if (r) return r
      }
    }
    return ''
  }
  return find(library.categories)
})

const totalPages = computed(() => Math.ceil(library.filteredBooks.length / perPage))
const paginatedBooks = computed(() => {
  const start = (currentPage.value - 1) * perPage
  return library.filteredBooks.slice(start, start + perPage)
})

function formatSize(bytes: number): string {
  if (!bytes) return '0 B'
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}
</script>
