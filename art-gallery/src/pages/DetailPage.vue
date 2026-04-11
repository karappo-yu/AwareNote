<template>
  <q-layout view="hHh lPr fFf">
    <!-- Top bar — auto-hide in masonry mode -->
    <q-header
      class="reader-bar"
      :class="{
        'header-hidden': thumbViewMode === 'masonry' && !headerHover,
        'masonry-header-float': thumbViewMode === 'masonry'
      }"
    >
      <q-toolbar style="height: 48px">
        <q-btn flat round dense icon="arrow_back" size="sm" @click="$router.back()" />
        <q-toolbar-title class="text-weight-medium ellipsis" style="font-size: 14px; color: var(--text-secondary)">
          {{ book?.title || '加载中...' }}
        </q-toolbar-title>

        <q-btn flat round dense icon="info_outline" size="sm" @click="infoDrawer = !infoDrawer">
          <q-tooltip>书籍信息</q-tooltip>
        </q-btn>
        <q-btn-toggle
          v-model="thumbViewMode"
          no-caps dense flat
          toggle-color="brand"
          :options="[
            { value: 'grid', slot: 'grid' },
            { value: 'masonry', slot: 'masonry' }
          ]"
        >
          <template v-slot:grid>
            <q-icon name="grid_view" size="18px" />
            <q-tooltip>网格</q-tooltip>
          </template>
          <template v-slot:masonry>
            <q-icon name="view_week" size="18px" />
            <q-tooltip>瀑布流</q-tooltip>
          </template>
        </q-btn-toggle>

        <!-- 视图控制按钮 -->
        <q-btn
          flat dense no-caps
          size="sm"
          color="brand"
          class="col-count-btn"
          @click.stop
        >
          {{ thumbViewMode === 'masonry' ? (masonryColCountCustom ?? masonryColCount) : thumbsPerPage }}
          <q-popup-proxy cover transition-show="jump-down" transition-hide="jump-up">
            <div class="q-pa-md col-popup">
              <q-slider
                v-if="thumbViewMode === 'masonry'"
                v-model="masonryColCountCustom"
                :min="1"
                :max="6"
                :step="1"
                label
                label-always
                color="brand"
                thumb-color="brand"
                class="col-slider"
              />
              <q-slider
                v-else
                v-model="thumbsPerPage"
                :min="12"
                :max="80"
                :step="4"
                label
                label-always
                color="brand"
                thumb-color="brand"
                class="col-slider"
              />
            </div>
          </q-popup-proxy>
        </q-btn>

        <q-btn flat round dense :icon="themeIcon" size="sm" @click="cycleTheme">
          <q-tooltip>{{ themeLabel }}</q-tooltip>
        </q-btn>
        <q-btn flat round dense icon="home" size="sm" @click="$router.push({ name: 'gallery' })">
          <q-tooltip>返回首页</q-tooltip>
        </q-btn>
      </q-toolbar>
    </q-header>

    <!-- Content -->
    <q-page-container>
      <q-page
        :style="thumbViewMode === 'masonry' ? 'margin-top: -48px; min-height: 100vh' : 'min-height: calc(100vh - 48px)'"
      >
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

        <q-inner-loading :showing="loading">
          <q-spinner-dots size="50px" color="brand" />
        </q-inner-loading>

        <template v-if="book">
          <!-- Thumbnails section — grid / masonry (full page) -->
          <div class="q-pa-md">

            <!-- Grid view — paginated, uniform ratio -->
            <template v-if="thumbViewMode === 'grid'">
              <div class="row q-col-gutter-sm" :style="{ direction: isRtl ? 'rtl' : 'ltr' }">
                <div
                  v-for="page in paginatedThumbs"
                  :key="page"
                  class="col-4 col-sm-3 col-md-2 col-lg-1"
                >
                  <q-card
                    flat
                    class="thumb-card cursor-pointer surface-card"
                    @click="openViewer(page)"
                  >
                    <q-img
                      :src="getPageUrl(page)"
                      :ratio="3/4"
                      fit="contain"
                    >
                      <template v-slot:loading>
                        <q-skeleton type="rect" style="height: 100%" />
                      </template>
                      <!-- Page favorite badge in grid view (not for virtual book) -->
                      <template v-if="!isVirtualBook">
                        <q-icon
                          v-if="isGridPageFavorited(page)"
                          name="favorite"
                          color="pink-4"
                          size="16px"
                          class="page-fav-badge"
                          @click.stop="toggleGridPageFavorite(page, $event)"
                        />
                        <q-icon
                          v-else
                          name="favorite_border"
                          size="16px"
                          class="page-fav-badge page-fav-badge-hover"
                          @click.stop="toggleGridPageFavorite(page, $event)"
                        />
                      </template>
                    </q-img>
                  </q-card>
                </div>
              </div>

              <!-- Grid pagination -->
              <div v-if="totalThumbPages > 1" class="row justify-center q-mt-lg">
                <q-pagination
                  v-model="thumbPage"
                  :max="totalThumbPages"
                  :max-pages="7"
                  direction-links
                  color="brand"
                  size="sm"
                />
              </div>
            </template>

            <!-- Masonry / Pinterest waterfall — infinite scroll, no pagination -->
            <template v-else>
              <div class="masonry-container" ref="masonryContainerRef" :style="masonryContainerStyle">
                <div
                  v-for="(item, idx) in masonryLayoutItems"
                  :key="'m-' + idx"
                  class="masonry-item cursor-pointer"
                  :class="{ 'masonry-spread-item': item.isSpread }"
                  :style="item.style"
                  @click="openViewer(item.isSpread ? item.pages![0] : item.page)"
                >
                  <q-card flat class="masonry-card surface-card">
                    <div
                      class="masonry-img-wrapper"
                      :style="item.wrapperStyle"
                    >
                      <q-skeleton
                        v-if="!item.loaded"
                        type="rect"
                        class="masonry-skeleton"
                        animation="fade"
                      />
                      <!-- Page favorite badge (not for virtual book) -->
                      <template v-if="!isVirtualBook">
                        <q-icon
                          v-if="isPageFavorited(item)"
                          name="favorite"
                          color="pink-4"
                          size="16px"
                          class="page-fav-badge"
                          @click.stop="togglePageFavoriteFromMasonry(item, $event)"
                        />
                        <q-icon
                          v-else
                          name="favorite_border"
                          size="16px"
                          class="page-fav-badge page-fav-badge-hover"
                          @click.stop="togglePageFavoriteFromMasonry(item, $event)"
                        />
                      </template>
                      <!-- Spread: two images side by side (RTL: reversed visual order) -->
                      <div v-if="item.isSpread" class="masonry-spread-container" :style="{ direction: item.direction === 'rtl' ? 'rtl' : 'ltr' }">
                        <img
                          :src="getPageUrl(item.pages![0])"
                          :alt="'第' + item.pages![0] + '页'"
                          class="masonry-img-native masonry-spread-half"
                          :class="{ 'img-loaded': item.loaded }"
                          loading="lazy"
                          @load="onMasonryImgLoad(item.pages![0])"
                        />
                        <img
                          :src="getPageUrl(item.pages![1])"
                          :alt="'第' + item.pages![1] + '页'"
                          class="masonry-img-native masonry-spread-half"
                          :class="{ 'img-loaded': item.loaded }"
                          loading="lazy"
                          @load="onMasonryImgLoad(item.pages![1])"
                        />
                      </div>
                      <!-- Normal single page -->
                      <img
                        v-else
                        :src="getPageUrl(item.page)"
                        :alt="'第' + item.page + '页'"
                        class="masonry-img-native"
                        :class="{ 'img-loaded': item.loaded }"
                        loading="lazy"
                        @load="onMasonryImgLoad(item.page)"
                        @click="openViewer(item.page)"
                      />
                    </div>
                    <!-- Virtual book: source book title (hidden per user request) -->
                  </q-card>
                </div>
              </div>

              <!-- Load more / sentinel -->
              <div ref="masonrySentinel" class="row justify-center q-mt-lg q-mb-md">
                <q-btn
                  v-if="masonryLoadedCount < virtualPageList.length"
                  flat no-caps
                  :loading="masonryLoading"
                  label="加载更多"
                  icon="expand_more"
                  color="brand"
                  class="load-more-btn"
                  @click="loadMoreMasonry"
                />
                <div v-else class="text-caption" style="color: var(--text-muted)">
                  全部 {{ virtualPageList.length }} 项已加载
                </div>
              </div>
            </template>
          </div>
        </template>
      </q-page>
    </q-page-container>

    <!-- Full-screen image viewer — X/Twitter style -->
    <q-dialog
      v-model="viewerOpen"
      full-width full-height
      transition-show="fade"
      transition-hide="fade"
      class="viewer-dialog"
      @keydown.escape="closeViewer()"
      @keydown.left="isRtl ? viewerNext() : viewerPrev()"
      @keydown.right="isRtl ? viewerPrev() : viewerNext()"
    >
      <div
        class="viewer-container column no-wrap"
        @click.self="closeViewer()"
      >
        <!-- Top bar — minimal overlay -->
        <div class="viewer-topbar row items-center q-px-md" @click.stop>
          <q-btn flat round dense icon="close" color="white" size="sm" class="viewer-close-btn" @click="closeViewer()" />
          <q-space />
          <span class="viewer-page-counter">{{ currentViewerSpread ? currentViewerSpread.pages[0] + '-' + currentViewerSpread.pages[1] : viewerPage }} / {{ book?.page_count }}</span>
          <q-space />
          <q-btn flat round dense icon="add" color="white" size="sm" class="viewer-tool-btn" @click="viewerZoom(0.3)" />
          <q-btn flat round dense icon="remove" color="white" size="sm" class="viewer-tool-btn" :class="{ 'opacity-disabled': viewerState.zoom <= 0.5 }" @click="viewerZoom(-0.3)" />
          <q-btn flat round dense icon="restart_alt" color="white" size="sm" class="viewer-tool-btn" @click="viewerReset" />
          <q-separator vertical color="white" class="q-mx-sm" style="opacity: 0.15; height: 20px" />
          <q-btn flat round dense icon="rotate_right" color="white" size="sm" class="viewer-tool-btn" @click="viewerRotate" />
        </div>

        <!-- Main area: image + nav arrows — click background to close -->
        <div
          class="col row relative-position"
          style="overflow: hidden"
          @click="closeViewer()"
          @wheel.prevent="handleWheel"
        >
          <!-- Prev arrow: LTR=左侧←, RTL=右侧→ -->
          <transition name="viewer-arrow-fade">
            <q-btn
              v-if="viewerPage > 1"
              class="viewer-nav-arrow"
              :class="[isRtl ? 'viewer-nav-right' : 'viewer-nav-left', { 'arrow-hidden': !arrowsVisible }]"
              round
              flat
              :icon="isRtl ? 'chevron_right' : 'chevron_left'"
              @click.stop="viewerPrev()"
              @mouseenter="arrowHoverIn"
              @mouseleave="arrowHoverOut"
            />
          </transition>

          <!-- Next arrow: LTR=右侧→, RTL=左侧← -->
          <transition name="viewer-arrow-fade">
            <q-btn
              v-if="book && viewerPage < book.page_count"
              class="viewer-nav-arrow"
              :class="[isRtl ? 'viewer-nav-left' : 'viewer-nav-right', { 'arrow-hidden': !arrowsVisible }]"
              round
              flat
              :icon="isRtl ? 'chevron_left' : 'chevron_right'"
              @click.stop="viewerNext()"
              @mouseenter="arrowHoverIn"
              @mouseleave="arrowHoverOut"
            />
          </transition>

          <!-- Image container — click background to close -->
          <div
            class="col row items-center justify-center relative-position"
            style="min-height: 0"
            @click="closeViewer()"
          >
            <!-- Spread: two images side by side (RTL: reversed order) -->
            <div v-if="currentViewerSpread" class="viewer-spread-container"
              :class="{ 'viewer-image-grabbing': isDragging, 'viewer-image-grab': !isDragging }"
              :style="{ ...viewerImgStyle, direction: currentViewerSpread.direction === 'rtl' ? 'rtl' : 'ltr' }"
              @click.stop
              @mousedown.stop="startDrag"
              @mousemove.stop="onDrag"
              @mouseup.stop="endDrag"
              @mouseleave="endDrag"
            >
              <img
                :src="getPageUrl(currentViewerSpread.pages[0], book?.optimization_strategy === 2)"
                class="viewer-spread-half"
                :class="{ 'viewer-image-svg': book && bookFormat(book.type) === 'pdf' }"
                draggable="false"
                @load="imgLoaded = true"
                @click.stop
                @mousedown.stop="startDrag"
                @mousemove.stop="onDrag"
                @mouseup.stop="endDrag"
                @mouseleave="endDrag"
              />
              <img
                :src="getPageUrl(currentViewerSpread.pages[1], book?.optimization_strategy === 2)"
                class="viewer-spread-half"
                :class="{ 'viewer-image-svg': book && bookFormat(book.type) === 'pdf' }"
                draggable="false"
                @load="imgLoaded = true"
                @click.stop
                @mousedown.stop="startDrag"
                @mousemove.stop="onDrag"
                @mouseup.stop="endDrag"
                @mouseleave="endDrag"
              />
            </div>
            <!-- Normal single page -->
            <img
              v-else
              :src="getPageUrl(viewerPage, book?.optimization_strategy === 2)"
              :style="viewerImgStyle"
              class="viewer-image"
              :class="{ 
                'viewer-image-grab': !isDragging, 
                'viewer-image-grabbing': isDragging,
                'viewer-image-svg': book && bookFormat(book.type) === 'pdf'
              }"
              draggable="false"
              @load="imgLoaded = true"
              @click.stop
              @mousedown.stop="startDrag"
              @mousemove.stop="onDrag"
              @mouseup.stop="endDrag"
              @mouseleave="endDrag"
            />
          </div>

          <q-inner-loading :showing="!imgLoaded" dark style="background: transparent">
            <q-spinner-dots size="48px" color="white" />
          </q-inner-loading>
        </div>

        <!-- Bottom bar — actions strip -->
        <div class="viewer-bottombar q-py-sm q-px-md" @click.stop>
          <div class="row items-center justify-between">
            <div class="row q-gutter-xs">
              <q-btn flat round dense color="white" size="sm" class="viewer-tool-btn" icon="flip" @click="viewerFlip" />
              <q-btn
                flat round dense
                :icon="isCurrentPageFavorited ? 'favorite' : 'favorite_border'"
                :color="isCurrentPageFavorited ? 'pink' : 'white'"
                size="sm"
                class="viewer-tool-btn"
                @click="togglePageFavorite"
              >
                <q-tooltip>{{ isCurrentPageFavorited ? '取消收藏此页' : '收藏此页' }}</q-tooltip>
              </q-btn>
              <!-- Spread buttons -->
              <template v-if="canCreateSpreadNext">
                <q-btn flat round dense color="white" size="sm" class="viewer-tool-btn" icon="join_inner" @click.stop="handleCreateSpreadNext">
                  <q-tooltip>拼下一页</q-tooltip>
                </q-btn>
              </template>
              <template v-if="currentViewerSpread">
                <q-btn flat round dense color="red-4" size="sm" class="viewer-tool-btn" icon="call_split" @click.stop="handleDeleteSpread">
                  <q-tooltip>取消拼接</q-tooltip>
                </q-btn>
              </template>
            </div>
            <span class="text-white text-caption ellipsis" style="opacity: 0.35; max-width: 300px">
              {{ book?.title || '' }}
            </span>
          </div>
        </div>
      </div>
    </q-dialog>
    <!-- Book info drawer — slides from right -->
    <q-drawer
      v-model="infoDrawer"
      side="right"
      :width="380"
      :breakpoint="600"
      overlay
      bordered
      class="info-drawer"
    >
      <div v-if="book" class="q-pa-lg">
        <!-- Close button -->
        <div class="row justify-end q-mb-md">
          <q-btn flat round dense icon="close" size="sm" @click="infoDrawer = false" />
        </div>

        <!-- Cover -->
        <div class="row justify-center q-mb-lg">
          <q-img
            :src="virtualBookCover || coverUrl(bookId)"
            :ratio="3/4"
            fit="contain"
            style="border-radius: 12px; box-shadow: 0 8px 32px rgba(0,0,0,0.3); max-width: 200px; width: 100%"
          >
            <template v-slot:loading>
              <q-skeleton type="rect" style="height: 100%; border-radius: 12px" />
            </template>
          </q-img>
        </div>

        <!-- Format + badges -->
        <div class="row items-center q-gutter-sm q-mb-sm">
          <q-badge
            :color="bookFormat(book.type) === 'pdf' ? 'red-10' : 'teal-8'"
            :label="bookFormat(book.type).toUpperCase()"
            class="format-badge"
          />
          <q-icon v-if="book.is_favorite" name="favorite" color="pink-4" size="16px" />
        </div>

        <!-- Title -->
        <div class="text-h6 text-weight-bold q-mb-md" style="line-height: 1.3; letter-spacing: -0.3px">
          {{ book.title }}
        </div>

        <!-- Meta chips -->
        <div class="row q-gutter-sm q-mb-md">
          <q-chip icon="description" outline color="brand" dense class="meta-chip">
            {{ book.page_count }} 页
          </q-chip>
          <q-chip v-if="book.optimization_strategy" icon="speed" outline color="brand" dense class="meta-chip">
            优化 Lv.{{ book.optimization_strategy }}
          </q-chip>
          <q-chip v-if="book.is_oversized" icon="warning" color="orange" dense class="meta-chip">
            超大文件
          </q-chip>
        </div>

        <!-- Path -->
        <div class="row items-center q-gutter-xs q-mb-lg" style="opacity: 0.4">
          <q-icon name="folder_open" size="14px" />
          <span style="font-size: 11px; word-break: break-all">{{ book.path }}</span>
        </div>

        <!-- Reading direction (non-virtual books only) -->
        <div v-if="!isVirtualBook" class="row items-center justify-between q-mb-lg">
          <span class="text-caption" style="opacity: 0.6">阅读方向</span>
          <q-btn
            flat no-caps dense
            :icon="isRtl ? 'arrow_back' : 'arrow_forward'"
            :label="isRtl ? 'RTL →' : 'LTR ←'"
            color="brand"
            class="reading-dir-btn"
            @click="toggleReadingDirection"
          >
            <q-tooltip>{{ isRtl ? '右翻书（日式漫画）' : '左翻书（默认）' }}</q-tooltip>
          </q-btn>
        </div>

        <q-separator class="q-mb-lg" style="opacity: 0.2" />

        <!-- Action buttons -->
        <q-btn
          unelevated no-caps
          icon="auto_stories"
          label="开始阅读"
          color="brand"
          size="md"
          class="action-btn-primary full-width q-mb-sm"
          @click="openViewer(1)"
        />

        <div v-if="!isVirtualBook" class="row q-gutter-sm">
          <q-btn
            flat no-caps
            :icon="book.is_favorite ? 'favorite' : 'favorite_border'"
            :color="book.is_favorite ? 'pink' : 'brand'"
            :label="book.is_favorite ? '已收藏' : '收藏'"
            class="col action-btn-outline"
            @click="handleFavorite"
          />
          <q-btn
            flat no-caps
            icon="folder_open"
            label="Finder"
            color="brand"
            class="col action-btn-outline"
            @click="handleReveal"
          />
        </div>
      </div>
    </q-drawer>
  </q-layout>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount, reactive, nextTick } from 'vue'
import { Dark, LocalStorage, useQuasar } from 'quasar'
import { getBook, getBookPages, getSpreads, createSpread, deleteSpread, toggleFavorite, revealInFinder, coverUrl as coverUrlFn, pdfPageSvgUrl, imagePageByNameUrl, bookFormat, type BookDetail, type PageInfo, type SpreadInfo, getPageFavorites, createPageFavorite, deletePageFavorite, type PageFavoriteInfo, getAllPageFavorites, type FavoritePageItem, getBookSettings, updateBookSettings } from '../api'
import { useSettings } from '../stores/settings'

const props = defineProps<{ id: string }>()
const bookId = computed(() => props.id)
const $q = useQuasar()
const settings = useSettings()

const book = ref<BookDetail | null>(null)
const pageNames = ref<PageInfo[]>([])  // image_book 页面文件名+尺寸列表
const spreads = ref<SpreadInfo[]>([])  // spread 标记列表
const pageFavorites = ref<PageFavoriteInfo[]>([])  // 页面收藏列表
const favoritePageSources = ref<FavoritePageItem[]>([])  // 虚拟书来源信息
const loading = ref(true)
const infoDrawer = ref(false)
const headerHover = ref(false)
const isVirtualBook = computed(() => bookId.value === '__page_favorites__')
const readingDirection = ref<'ltr' | 'rtl'>('ltr')
const isRtl = computed(() => readingDirection.value === 'rtl')
let headerHideTimer: ReturnType<typeof setTimeout> | null = null

// ============== 虚拟页面列表 ==============
// 将 pages + spreads 合并为虚拟列表，spread 页合并为一个项
type VirtualPage = 
  | { type: 'single'; page: number }
  | { type: 'spread'; pages: [number, number]; direction: 'ltr' | 'rtl' }

const virtualPageList = computed<VirtualPage[]>(() => {
  if (!book.value) return []
  const totalPages = book.value.page_count
  // 构建 filename→page_index 映射 (仅 image_book)
  const filenameToPage = new Map<string, number>()
  pageNames.value.forEach((info, i) => {
    filenameToPage.set(info.filename, i + 1)  // 1-based
  })

  // 构建页码→spread 信息
  // spread.filename 是左页，spread.next_file 是右页
  const pageToSpread = new Map<number, { leftPage: number; rightPage: number; direction: 'ltr' | 'rtl' }>()
  for (const s of spreads.value) {
    const leftPage = filenameToPage.get(s.filename)
    const rightPage = filenameToPage.get(s.next_file)
    if (leftPage && rightPage) {
      pageToSpread.set(leftPage, { leftPage, rightPage, direction: (s.direction as 'ltr' | 'rtl') || 'ltr' })
    }
  }

  const result: VirtualPage[] = []
  const consumed = new Set<number>()

  for (let p = 1; p <= totalPages; p++) {
    if (consumed.has(p)) continue
    const spread = pageToSpread.get(p)
    if (spread) {
      result.push({ type: 'spread', pages: [spread.leftPage, spread.rightPage], direction: spread.direction })
      consumed.add(spread.leftPage)
      consumed.add(spread.rightPage)
    } else {
      result.push({ type: 'single', page: p })
      consumed.add(p)
    }
  }

  return result
})

// 检查瀑布流项是否已收藏
function isPageFavorited(item: { isSpread: boolean; page?: number; pages?: [number, number] | null; filename: string }) {
  if (item.isSpread && item.pages) {
    const leftFilename = pageNames.value[item.pages[0] - 1]?.filename
    const rightFilename = pageNames.value[item.pages[1] - 1]?.filename
    if (!leftFilename || !rightFilename) return false
    return pageFavorites.value.some(f => f.filename === leftFilename) &&
           pageFavorites.value.some(f => f.filename === rightFilename)
  }
  return pageFavorites.value.some(f => f.filename === item.filename)
}

// 网格视图：检查单页是否已收藏
function isGridPageFavorited(page: number) {
  const filename = pageNames.value[page - 1]?.filename
  if (!filename) return false
  return pageFavorites.value.some(f => f.filename === filename)
}

// 瀑布流：hover 心形点击收藏/取消
async function togglePageFavoriteFromMasonry(
  item: { isSpread: boolean; page?: number; pages?: [number, number] | null; filename: string },
  event?: Event
) {
  event?.stopPropagation()
  if (!book.value) return

  const favorited = isPageFavorited(item)

  if (item.isSpread && item.pages) {
    const leftFilename = pageNames.value[item.pages[0] - 1]?.filename
    const rightFilename = pageNames.value[item.pages[1] - 1]?.filename
    if (!leftFilename || !rightFilename) return
    if (favorited) {
      await deletePageFavorite(bookId.value, leftFilename)
      await deletePageFavorite(bookId.value, rightFilename)
      $q.notify({ type: 'info', message: '已取消收藏' })
    } else {
      await createPageFavorite(bookId.value, leftFilename)
      await createPageFavorite(bookId.value, rightFilename)
      $q.notify({ type: 'positive', message: '已收藏拼接页' })
    }
  } else {
    const filename = pageNames.value[(item.page || 1) - 1]?.filename
    if (!filename) return
    if (favorited) {
      await deletePageFavorite(bookId.value, filename)
      $q.notify({ type: 'info', message: '已取消收藏' })
    } else {
      await createPageFavorite(bookId.value, filename)
      $q.notify({ type: 'positive', message: '已收藏' })
    }
  }
  pageFavorites.value = await getPageFavorites(bookId.value)
}

// 网格视图：hover 心形点击收藏/取消
async function toggleGridPageFavorite(page: number, event?: Event) {
  event?.stopPropagation()
  if (!book.value) return
  const filename = pageNames.value[page - 1]?.filename
  if (!filename) return
  if (isGridPageFavorited(page)) {
    await deletePageFavorite(bookId.value, filename)
    $q.notify({ type: 'info', message: '已取消收藏' })
  } else {
    await createPageFavorite(bookId.value, filename)
    $q.notify({ type: 'positive', message: '已收藏' })
  }
  pageFavorites.value = await getPageFavorites(bookId.value)
}

// 虚拟书：获取收藏页的来源书名

// 返回顶部
const showBackToTop = ref(false)
function scrollToTop() {
  window.scrollTo({ top: 0, behavior: 'smooth' })
}
function onMouseMove(e: MouseEvent) {
  // 页面有滚动 且 鼠标接近屏幕底部时显示
  showBackToTop.value = window.scrollY > 300 && e.clientY > window.innerHeight - 150
}

// Reactive window width for responsive masonry columns
const windowWidth = ref(window.innerWidth)
function onResize() { windowWidth.value = window.innerWidth }

// Grid view state
const thumbsPerPage = ref(40)
const thumbPage = ref(1)

// View mode
const thumbViewMode = ref<'grid' | 'masonry'>('masonry')

// 切换视图模式时立即显示顶栏
watch(thumbViewMode, (val) => {
  if (val === 'masonry') {
    headerHover.value = true
    if (headerHideTimer) clearTimeout(headerHideTimer)
    headerHideTimer = setTimeout(() => { headerHover.value = false }, 3000)
  }
})

// 网格模式下调整每页张数时重置到第一页
watch(thumbsPerPage, () => {
  if (thumbViewMode.value === 'grid') {
    thumbPage.value = 1
  }
})

// Masonry infinite scroll state
const MASONRY_BATCH = 30
const masonryLoaded = ref(MASONRY_BATCH)
const masonryLoading = ref(false)
const masonrySentinel = ref<HTMLElement | null>(null)
let scrollObserver: IntersectionObserver | null = null

// 瀑布流列数控制 — 跟随 settings store，刷新后也能拿到正确值
const masonryColCountCustom = ref(settings.getNumber('masonry_cols', 3))
watch(() => settings.loaded, (ready) => {
  if (ready) masonryColCountCustom.value = settings.getNumber('masonry_cols', 3)
})

// 列数变化时保存到后端设置
watch(masonryColCountCustom, (val) => {
  settings.set('masonry_cols', String(val))
  nextTick(() => {
    setupScrollObserver()
  })
})

const masonryLoadedCount = computed(() => Math.min(masonryLoaded.value, virtualPageList.value.length))

// 横向瀑布流：已加载图片的跟踪
const masonryLoadedSet = ref<Set<number>>(new Set())
function onMasonryImgLoad(page: number) {
  masonryLoadedSet.value.add(page)
}

// 横向瀑布流布局计算
const masonryContainerRef = ref<HTMLElement | null>(null)
const MASONRY_GAP = 10

const masonryLayoutItems = computed(() => {
  const vpl = virtualPageList.value
  const loadedCount = masonryLoadedCount.value
  const items = vpl.slice(0, loadedCount)
  const colCount = masonryColCount.value
  const gap = MASONRY_GAP
  const containerWidth = masonryContainerRef.value?.clientWidth || 900

  // 计算每列宽度（百分比）
  const gapPct = gap * 100 / containerWidth
  const colWidthPct = (100 - (colCount - 1) * gapPct) / colCount

  // 列高度跟踪
  const colHeights = new Array(colCount).fill(0)

  return items.map((item) => {
    const isSpread = item.type === 'spread'
    const spreadPages = isSpread ? (item as { type: 'spread'; pages: [number, number] }).pages : null
    const page = !isSpread ? (item as { type: 'single'; page: number }).page : 0

    // 计算宽高比
    const isPdfBook = book.value && bookFormat(book.value.type) === 'pdf'
    const DEFAULT_ASPECT = isPdfBook ? 0.7071 : 1

    let aspectRatio: number
    let singleAspect: number  // 单页宽高比，用于计算等高
    let widthCols: number  // 占几列

    if (isSpread && spreadPages) {
      // spread: 两张图横向拼合，合并宽度=2W, 合并高度=H → aspect = 2W/H
      const leftInfo = pageNames.value[spreadPages[0] - 1]
      const rightInfo = pageNames.value[spreadPages[1] - 1]
      const leftAspect = (leftInfo?.w && leftInfo?.h) ? leftInfo.w / leftInfo.h : DEFAULT_ASPECT
      const rightAspect = (rightInfo?.w && rightInfo?.h) ? rightInfo.w / rightInfo.h : DEFAULT_ASPECT
      // 两张图的 height 应该一致（等尺寸书籍），取平均高度
      // 合并比例 ≈ leftAspect + rightAspect (因为高度相同)
      aspectRatio = leftAspect + rightAspect
      singleAspect = leftAspect  // 用左页的单页比例算高度
      widthCols = 2
    } else {
      const info = pageNames.value[page - 1]
      aspectRatio = (info?.w && info?.h) ? info.w / info.h : DEFAULT_ASPECT
      singleAspect = aspectRatio
      widthCols = 1
    }

    // 找放置位置
    let minCol: number
    let left: number
    let itemWidthPct: number

    if (widthCols === 2) {
      // spread 占两列：找相邻两列高度和最小的位置
      if (colCount < 2) {
        // 只有一列时无法放 spread，当单列处理
        minCol = 0
        left = 0
        itemWidthPct = colWidthPct
        widthCols = 1
      } else {
        let bestPair = 0
        let bestHeight = Infinity
        for (let c = 0; c <= colCount - 2; c++) {
          const pairHeight = Math.max(colHeights[c], colHeights[c + 1])
          if (pairHeight < bestHeight) {
            bestHeight = pairHeight
            bestPair = c
          }
        }
        minCol = bestPair
        left = minCol * (colWidthPct + gapPct)
        itemWidthPct = 2 * colWidthPct + gapPct  // 两列宽度 + 中间 gap
      }
    } else {
      minCol = colHeights.indexOf(Math.min(...colHeights))
      left = minCol * (colWidthPct + gapPct)
      itemWidthPct = colWidthPct
    }

    // 卡片高度：spread 和单页等高（锁定行对齐）
    // spread 容器宽度虽然包含中间 gap，但高度应等于同列宽下单页的高度
    const colWidthPx = containerWidth * colWidthPct / 100
    const itemHeight = colWidthPx / singleAspect
    // 虚拟书：底部有来源书名文字行，需额外高度
    const virtualSourceHeight = isVirtualBook.value ? 28 : 0

    const top = colHeights[minCol]
    // 更新涉及的列高度
    if (widthCols === 2) {
      const newHeight = top + itemHeight + virtualSourceHeight + gap
      colHeights[minCol] = newHeight
      colHeights[minCol + 1] = newHeight
    } else {
      colHeights[minCol] = top + itemHeight + virtualSourceHeight + gap
    }

    // wrapperStyle 的 aspectRatio
    let wrapperStyle: Record<string, string | undefined>
    if (isSpread && spreadPages) {
      const leftInfo = pageNames.value[spreadPages[0] - 1]
      const rightInfo = pageNames.value[spreadPages[1] - 1]
      const leftW = leftInfo?.w || 1
      const leftH = leftInfo?.h || 1
      const rightW = rightInfo?.w || 1
      const rightH = rightInfo?.h || 1
      const totalW = leftW + rightW
      const avgH = (leftH + rightH) / 2
      wrapperStyle = { aspectRatio: `${totalW}/${avgH}` }
    } else {
      const info = pageNames.value[page - 1]
      wrapperStyle = {
        aspectRatio: (info?.w && info?.h) ? `${info.w}/${info.h}` : isPdfBook ? '1/1.414' : undefined,
      }
    }

    // RTL: 镜像水平位置
    const finalLeft = isRtl.value ? 100 - left - itemWidthPct : left

    return {
      page: isSpread ? spreadPages![0] : page,
      pages: isSpread ? spreadPages : null,
      isSpread,
      direction: isSpread ? (item as { type: 'spread'; pages: [number, number]; direction: 'ltr' | 'rtl' }).direction : 'ltr',
      filename: isSpread
        ? (pageNames.value[spreadPages![0] - 1]?.filename ?? '')
        : (pageNames.value[page - 1]?.filename ?? ''),
      loaded: isSpread
        ? (masonryLoadedSet.value.has(spreadPages![0]) && masonryLoadedSet.value.has(spreadPages![1]))
        : masonryLoadedSet.value.has(page),
      style: {
        position: 'absolute' as const,
        left: `${finalLeft}%`,
        top: `${top}px`,
        width: `${itemWidthPct}%`,
      },
      wrapperStyle,
    }
  })
})

// 容器高度 = 最大列高度
const masonryContainerStyle = computed(() => {
  const items = masonryLayoutItems.value
  if (items.length === 0) return { position: 'relative' as const }
  const isPdf = book.value && bookFormat(book.value.type) === 'pdf'
  const DEFAULT_ASPECT = isPdf ? 0.7071 : 1
  const containerWidth = masonryContainerRef.value?.clientWidth || 900
  const maxTop = Math.max(...items.map(item => {
    const top = parseFloat(item.style.top)
    const itemWidthPct = parseFloat(item.style.width)
    const itemWidthPx = containerWidth * itemWidthPct / 100
    // 从 wrapperStyle 的 aspectRatio 反推高度
    const arStr = item.wrapperStyle.aspectRatio
    let aspectRatio = DEFAULT_ASPECT
    if (arStr) {
      const parts = arStr.split('/')
      if (parts.length === 2) {
        aspectRatio = parseFloat(parts[0]) / parseFloat(parts[1])
      }
    }
    const itemHeight = itemWidthPx / aspectRatio
    return top + itemHeight
  }))
  return {
    position: 'relative' as const,
    height: `${maxTop + MASONRY_GAP}px`,
  }
})

// Distribute pages across columns left-to-right (round-robin)
const masonryColCount = computed(() => {
  // 优先使用用户自定义值
  if (masonryColCountCustom.value !== null) {
    return masonryColCountCustom.value
  }
  const w = windowWidth.value
  if (w <= 480) return 2
  if (w <= 768) return 2
  if (w <= 1200) return 3
  return 4
})

function loadMoreMasonry() {
  if (!book.value) return
  const total = virtualPageList.value.length
  if (masonryLoaded.value >= total) return
  masonryLoading.value = true
  // Simulate slight delay for UX feedback
  setTimeout(() => {
    masonryLoaded.value = Math.min(masonryLoaded.value + MASONRY_BATCH, total)
    masonryLoading.value = false
  }, 150)
}

function setupScrollObserver() {
  if (scrollObserver) scrollObserver.disconnect()
  scrollObserver = new IntersectionObserver((entries) => {
    if (entries[0]?.isIntersecting && !masonryLoading.value) {
      loadMoreMasonry()
    }
  }, { rootMargin: '400px' })

  nextTick(() => {
    if (masonrySentinel.value) {
      scrollObserver?.observe(masonrySentinel.value)
    }
  })
}

// ============== Viewer state ==============
const viewerOpen = ref(false)
const viewerPage = ref(1)
const viewerState = reactive({ zoom: 1, rotation: 0, flipH: false })
const imgLoaded = ref(true) // template ref

// 当前查看器页面是否是 spread 的一部分
const currentViewerSpread = computed<{ pages: [number, number]; direction: 'ltr' | 'rtl' } | null>(() => {
  const p = viewerPage.value
  const filename = pageNames.value[p - 1]?.filename
  if (!filename) return null
  // 检查当前页是否是某 spread 的左页
  const asLeft = spreads.value.find(s => s.filename === filename)
  if (asLeft) {
    const rightPage = pageNames.value.findIndex(info => info.filename === asLeft.next_file)
    if (rightPage !== -1) return { pages: [p, rightPage + 1], direction: (asLeft.direction as 'ltr' | 'rtl') || 'ltr' }
  }
  // 检查当前页是否是某 spread 的右页
  const asRight = spreads.value.find(s => s.next_file === filename)
  if (asRight) {
    const leftPage = pageNames.value.findIndex(info => info.filename === asRight.filename)
    if (leftPage !== -1) return { pages: [leftPage + 1, p], direction: (asRight.direction as 'ltr' | 'rtl') || 'ltr' }
  }
  return null
})

// 当前查看器页面是否已收藏
const isCurrentPageFavorited = computed(() => {
  // 虚拟书：所有页面都是已收藏的
  if (isVirtualBook.value) return true
  const p = viewerPage.value
  const filename = pageNames.value[p - 1]?.filename
  if (!filename) return false
  // spread 时，左右两页都收藏了才算已收藏
  if (currentViewerSpread.value) {
    const leftFilename = pageNames.value[currentViewerSpread.value.pages[0] - 1]?.filename
    const rightFilename = pageNames.value[currentViewerSpread.value.pages[1] - 1]?.filename
    if (!leftFilename || !rightFilename) return false
    return pageFavorites.value.some(f => f.filename === leftFilename) &&
           pageFavorites.value.some(f => f.filename === rightFilename)
  }
  return pageFavorites.value.some(f => f.filename === filename)
})

// 收藏/取消收藏当前页
async function togglePageFavorite() {
  if (!book.value) return
  const p = viewerPage.value

  // 虚拟书：取消收藏（从来源书中删除）
  if (isVirtualBook.value) {
    if (currentViewerSpread.value) {
      const leftInfo = favoritePageSources.value.find(s =>
        pageNames.value[currentViewerSpread.value!.pages[0] - 1]?.filename === `fav://${s.book_id}/${s.filename}`
      )
      const rightInfo = favoritePageSources.value.find(s =>
        pageNames.value[currentViewerSpread.value!.pages[1] - 1]?.filename === `fav://${s.book_id}/${s.filename}`
      )
      if (leftInfo) await deletePageFavorite(leftInfo.book_id, leftInfo.filename)
      if (rightInfo) await deletePageFavorite(rightInfo.book_id, rightInfo.filename)
      $q.notify({ type: 'info', message: '已取消收藏拼接页' })
    } else {
      const filename = pageNames.value[p - 1]?.filename
      if (!filename) return
      const source = favoritePageSources.value.find(s => filename === `fav://${s.book_id}/${s.filename}`)
      if (!source) return
      await deletePageFavorite(source.book_id, source.filename)
      $q.notify({ type: 'info', message: '已取消收藏' })
    }
    // 重新加载虚拟书
    await loadBook()
    return
  }

  // 普通书籍
  if (currentViewerSpread.value) {
    const leftFilename = pageNames.value[currentViewerSpread.value.pages[0] - 1]?.filename
    const rightFilename = pageNames.value[currentViewerSpread.value.pages[1] - 1]?.filename
    if (!leftFilename || !rightFilename) return
    if (isCurrentPageFavorited.value) {
      await deletePageFavorite(bookId.value, leftFilename)
      await deletePageFavorite(bookId.value, rightFilename)
      $q.notify({ type: 'info', message: '已取消收藏' })
    } else {
      await createPageFavorite(bookId.value, leftFilename)
      await createPageFavorite(bookId.value, rightFilename)
      $q.notify({ type: 'positive', message: '已收藏拼接页' })
    }
  } else {
    const filename = pageNames.value[p - 1]?.filename
    if (!filename) return
    if (isCurrentPageFavorited.value) {
      await deletePageFavorite(bookId.value, filename)
      $q.notify({ type: 'info', message: '已取消收藏' })
    } else {
      await createPageFavorite(bookId.value, filename)
      $q.notify({ type: 'positive', message: '已收藏' })
    }
  }
  // 重新加载收藏列表
  pageFavorites.value = await getPageFavorites(bookId.value)
}

// 当前查看器页面能否创建 spread
const canCreateSpreadNext = computed(() => {
  if (!book.value) return false
  const p = viewerPage.value
  if (p >= book.value.page_count) return false
  // 当前页和下一页都未在 spread 中
  const filename = pageNames.value[p - 1]?.filename
  const nextFilename = pageNames.value[p]?.filename
  if (!filename || !nextFilename) return false
  // 检查不在已有 spread 中
  return !spreads.value.some(s => s.filename === filename || s.next_file === filename || s.filename === nextFilename || s.next_file === nextFilename)
})

async function handleCreateSpreadNext() {
  if (!book.value) return
  const p = viewerPage.value
  const filename = pageNames.value[p - 1]?.filename
  const nextFilename = pageNames.value[p]?.filename
  if (!filename || !nextFilename) return
  try {
    await createSpread(bookId.value, filename, nextFilename, readingDirection.value)
    // 重新加载 spreads
    spreads.value = await getSpreads(bookId.value)
    $q.notify({ type: 'positive', message: '已拼接' })
  } catch { $q.notify({ type: 'negative', message: '拼接失败' }) }
}

async function handleDeleteSpread() {
  const spreadInfo = currentViewerSpread.value
  if (!spreadInfo) return
  const filename = pageNames.value[spreadInfo.pages[0] - 1]?.filename
  if (!filename) return
  try {
    await deleteSpread(bookId.value, filename)
    spreads.value = await getSpreads(bookId.value)
    $q.notify({ type: 'positive', message: '已取消拼接' })
  } catch { $q.notify({ type: 'negative', message: '取消拼接失败' }) }
}

// Arrow auto-hide
const arrowsVisible = ref(false)
const arrowHovered = ref(false)
let hideTimer: ReturnType<typeof setTimeout> | null = null

function arrowHoverIn() {
  arrowHovered.value = true
  if (hideTimer) { clearTimeout(hideTimer); hideTimer = null }
}

function arrowHoverOut() {
  arrowHovered.value = false
  resetArrowTimer()
}

function onViewerMouseMove(e: MouseEvent) {
  if (!viewerOpen.value) return
  const nearEdge = e.clientX < 120 || e.clientX > window.innerWidth - 120
  if (nearEdge) {
    resetArrowTimer()
  } else if (arrowsVisible.value) {
    arrowsVisible.value = false
    if (hideTimer) { clearTimeout(hideTimer); hideTimer = null }
  }
}

// Drag / pan
const isDragging = ref(false)
const dragStart = reactive({ x: 0, y: 0 })
const panOffset = reactive({ x: 0, y: 0 })

const coverUrl = coverUrlFn

// 虚拟书封面：用第一张收藏页的图片
const virtualBookCover = computed(() => {
  if (!isVirtualBook.value || favoritePageSources.value.length === 0) return ''
  return getPageUrl(1)
})

function getPageUrl(page: number, realsize = false) {
  if (!book.value) return ''
  // 虚拟书：从来源信息中构建真实 URL
  if (isVirtualBook.value) {
    const source = favoritePageSources.value[page - 1]
    if (!source) return ''
    if (source.book_type === 'pdf') {
      const pageNum = parseInt(source.filename, 10)
      return pdfPageSvgUrl(source.book_id, pageNum)
    }
    return imagePageByNameUrl(source.book_id, source.filename, realsize)
  }
  if (bookFormat(book.value.type) === 'pdf') {
    return pdfPageSvgUrl(bookId.value, page)
  }
  // image_book：优先用文件名 URL
  const info = pageNames.value[page - 1]
  if (info) {
    return imagePageByNameUrl(bookId.value, info.filename, realsize)
  }
  // 兜底：pages 列表尚未加载时用序号
  return `/api/books/${bookId.value}/${page}${realsize ? '?realsize=true' : ''}`
}

// Grid pagination
const totalThumbPages = computed(() => Math.ceil((book.value?.page_count ?? 0) / thumbsPerPage.value))
const paginatedThumbs = computed(() => {
  if (!book.value) return []
  const start = (thumbPage.value - 1) * thumbsPerPage.value
  return Array.from(
    { length: Math.min(thumbsPerPage.value, book.value.page_count - start) },
    (_, i) => start + i + 1
  )
})

const viewerImgStyle = computed(() => {
  return {
    transform: `translate(${panOffset.x}px, ${panOffset.y}px) scale(${viewerState.zoom}) rotate(${viewerState.rotation}deg) scaleX(${viewerState.flipH ? -1 : 1})`,
    transition: isDragging.value ? 'none' : 'transform 0.2s ease'
  }
})

function openViewer(page: number) {
  viewerPage.value = page
  viewerState.zoom = 1
  viewerState.rotation = 0
  viewerState.flipH = false
  panOffset.x = 0
  panOffset.y = 0
  imgLoaded.value = false
  viewerOpen.value = true
  arrowsVisible.value = false
}

function closeViewer() {
  viewerOpen.value = false
}

function viewerZoom(d: number) {
  viewerState.zoom = Math.max(0.2, Math.min(5, viewerState.zoom + d))
}
function viewerReset() { viewerState.zoom = 1; viewerState.rotation = 0; viewerState.flipH = false; panOffset.x = 0; panOffset.y = 0 }
function viewerRotate() { viewerState.rotation += 90 }
function viewerFlip() { viewerState.flipH = !viewerState.flipH }

function resetArrowTimer() {
  arrowsVisible.value = true
  if (hideTimer) clearTimeout(hideTimer)
  hideTimer = setTimeout(() => {
    if (!arrowHovered.value) {
      arrowsVisible.value = false
    }
  }, 3000)
}

function viewerPrev() {
  resetArrowTimer()
  if (viewerPage.value <= 1) return
  // 先退一页
  let targetPage = viewerPage.value - 1
  // 如果退到的页是某 spread 的右页，再退一页到左页
  const targetFilename = pageNames.value[targetPage - 1]?.filename
  if (targetFilename) {
    const isSpreadRight = spreads.value.some(s => s.next_file === targetFilename)
    if (isSpreadRight && targetPage > 1) {
      targetPage--
    }
  }
  viewerPage.value = targetPage
  imgLoaded.value = false
  panOffset.x = 0
  panOffset.y = 0
}

function viewerNext() {
  resetArrowTimer()
  if (!book.value || viewerPage.value >= book.value.page_count) return
  // 如果当前页是 spread 的左页，跳过右页
  const currentFilename = pageNames.value[viewerPage.value - 1]?.filename
  if (currentFilename) {
    const spread = spreads.value.find(s => s.filename === currentFilename)
    if (spread && viewerPage.value + 1 <= book.value.page_count) {
      viewerPage.value = viewerPage.value + 2
      imgLoaded.value = false
      panOffset.x = 0
      panOffset.y = 0
      return
    }
  }
  // 如果下一页是某 spread 的右页（孤立的右页，不应该出现但防御性处理），跳过
  const nextFilename = pageNames.value[viewerPage.value]?.filename
  if (nextFilename) {
    const isSpreadRight = spreads.value.some(s => s.next_file === nextFilename)
    if (isSpreadRight) {
      viewerPage.value = viewerPage.value + 1  // 跳到 spread 左页
      imgLoaded.value = false
      panOffset.x = 0
      panOffset.y = 0
      return
    }
  }
  viewerPage.value++
  imgLoaded.value = false
  panOffset.x = 0
  panOffset.y = 0
}

// Wheel zoom — centered on pointer position
function handleWheel(e: WheelEvent) {
  const delta = e.deltaY > 0 ? -0.12 : 0.12
  const oldZoom = viewerState.zoom
  viewerState.zoom = Math.max(0.2, Math.min(5, oldZoom + delta))
}

// Drag to pan
function startDrag(e: MouseEvent) {
  if (e.button !== 0 || viewerState.zoom <= 1) return
  isDragging.value = true
  dragStart.x = e.clientX - panOffset.x
  dragStart.y = e.clientY - panOffset.y
}

function onDrag(e: MouseEvent) {
  if (!isDragging.value) return
  panOffset.x = e.clientX - dragStart.x
  panOffset.y = e.clientY - dragStart.y
}

function endDrag() {
  isDragging.value = false
}

// Auto-hide header in masonry mode — listen on document so scroll position doesn't matter
function onDocMouseMove(e: MouseEvent) {
  if (thumbViewMode.value !== 'masonry') {
    headerHover.value = false
    return
  }
  // 如果鼠标在 header 区域内（包括 popup），不隐藏
  const header = document.querySelector('.reader-bar')
  if (header?.contains(e.target as Node)) return

  // Show header when mouse is within 60px of viewport top
  if (e.clientY < 60) {
    headerHover.value = true
    if (headerHideTimer) clearTimeout(headerHideTimer)
  } else if (headerHover.value) {
    if (headerHideTimer) clearTimeout(headerHideTimer)
    headerHideTimer = setTimeout(() => { headerHover.value = false }, 800)
  }
}

// 三态主题：auto → light → dark → auto
type ThemeMode = 'auto' | 'light' | 'dark'
const themeMode = ref<ThemeMode>((LocalStorage.getItem<ThemeMode>('themeMode')) || 'auto')
const prefersDark = computed(() => window.matchMedia('(prefers-color-scheme: dark)').matches)

function applyTheme(mode: ThemeMode) {
  themeMode.value = mode
  LocalStorage.set('themeMode', mode)
  if (mode === 'auto') Dark.set(prefersDark.value)
  else Dark.set(mode === 'dark')
}

const themeIcon = computed(() => {
  if (themeMode.value === 'auto') return 'brightness_auto'
  return $q.dark.isActive ? 'light_mode' : 'dark_mode'
})
const themeLabel = computed(() => {
  if (themeMode.value === 'auto') return '跟随系统'
  return $q.dark.isActive ? '亮色模式' : '暗色模式'
})

function cycleTheme() {
  const next: ThemeMode = themeMode.value === 'auto' ? 'light' : themeMode.value === 'light' ? 'dark' : 'auto'
  applyTheme(next)
}

// 切换阅读方向
async function toggleReadingDirection() {
  const next = readingDirection.value === 'ltr' ? 'rtl' : 'ltr'
  readingDirection.value = next
  try {
    await updateBookSettings(bookId.value, [{ key: 'reading_direction', value: next }])
  } catch {
    console.warn('Failed to persist reading_direction')
  }
}

async function handleFavorite() {
  if (!book.value || isVirtualBook.value) return
  try {
    await toggleFavorite(bookId.value, book.value.is_favorite)
    book.value.is_favorite = !book.value.is_favorite
    $q.notify({ type: 'positive', message: book.value.is_favorite ? '已添加收藏' : '已取消收藏' })
  } catch { $q.notify({ type: 'negative', message: '操作失败' }) }
}

async function handleReveal() {
  if (isVirtualBook.value) return
  try {
    await revealInFinder(bookId.value)
    $q.notify({ type: 'info', message: '已在 Finder 中打开' })
  } catch { $q.notify({ type: 'negative', message: '操作失败' }) }
}

async function loadBook() {
  loading.value = true
  pageNames.value = []
  spreads.value = []
  pageFavorites.value = []
  readingDirection.value = 'ltr'
  try {
    // 虚拟书：收藏的页面
    if (isVirtualBook.value) {
      const pages = await getAllPageFavorites()
      book.value = {
        id: '__page_favorites__',
        title: '收藏的页面',
        path: '',
        type: 'image_book',
        page_count: pages.length,
        is_favorite: true,
        description: null,
        optimization_strategy: 0,
        avg_page_pixels: null,
        is_oversized: false,
      }
      masonryLoaded.value = MASONRY_BATCH
      // 构建虚拟页面列表
      pageNames.value = pages.map(p => ({
        filename: `fav://${p.book_id}/${p.filename}`,
        w: p.w ?? 800,
        h: p.h ?? 1200,
      }))
      // 构建 spread 信息：收藏页中 next_file 非空的表示 spread 左页
      // next_file 是原书中的真实文件名，需要找到对应的虚拟书页码
      spreads.value = pages
        .filter(p => p.next_file !== null)
        .map(p => ({
          book_id: '__page_favorites__',
          filename: `fav://${p.book_id}/${p.filename}`,
          next_file: `fav://${p.book_id}/${p.next_file}`,
          direction: p.direction || 'ltr',
          created_at: 0,
        }))
      // 存储来源信息供详情页和查看器使用
      favoritePageSources.value = pages
      setupScrollObserver()
      return
    }
    book.value = await getBook(bookId.value)
    masonryLoaded.value = MASONRY_BATCH
    // 所有类型都加载页面列表和尺寸信息（用于瀑布流布局）
    if (book.value) {
      try {
        const [pages, spreadData, favData, bookSettings] = await Promise.all([
          getBookPages(bookId.value),
          getSpreads(bookId.value),
          getPageFavorites(bookId.value),
          getBookSettings(bookId.value),
        ])
        pageNames.value = pages
        spreads.value = spreadData
        pageFavorites.value = favData
        // 读取阅读方向
        const dirSetting = bookSettings.find(s => s.key === 'reading_direction')
        readingDirection.value = dirSetting?.value === 'rtl' ? 'rtl' : 'ltr'
      } catch {
        // 加载失败时回退到序号请求
        console.warn('Failed to load page info, falling back to index-based URLs')
      }
    }
    setupScrollObserver()
  }
  finally { loading.value = false }
}

onMounted(() => {
  loadBook()
  document.addEventListener('mousemove', onDocMouseMove)
  document.addEventListener('mousemove', onMouseMove)
  document.addEventListener('mousemove', onViewerMouseMove, true) // capture 阶段，避免被图片 stop 阻断
  window.addEventListener('resize', onResize)
})

watch(bookId, () => { loadBook() })

onBeforeUnmount(() => {
  scrollObserver?.disconnect()
  document.removeEventListener('mousemove', onDocMouseMove)
  document.removeEventListener('mousemove', onMouseMove)
  document.removeEventListener('mousemove', onViewerMouseMove, true)
  window.removeEventListener('resize', onResize)
  if (headerHideTimer) clearTimeout(headerHideTimer)
})
</script>
