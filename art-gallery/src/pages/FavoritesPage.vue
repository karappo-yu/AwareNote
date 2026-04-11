<template>
  <q-page class="q-pa-lg">
    <div class="q-mb-lg">
      <div class="section-title">我的收藏</div>
      <div class="section-subtitle">{{ totalDisplay }} 项</div>
    </div>

    <q-inner-loading :showing="loading">
      <q-spinner-dots size="50px" color="brand" />
    </q-inner-loading>

    <!-- Empty -->
    <div v-if="!loading && favorites.length === 0 && pageFavoriteCount === 0" class="column items-center q-mt-xl">
      <q-icon name="favorite_border" size="80px" style="opacity: 0.12" />
      <div class="text-h6 q-mt-md" style="opacity: 0.3">还没有收藏</div>
    </div>

    <!-- Favorite books grid -->
    <div v-if="!loading && (favorites.length > 0 || pageFavoriteCount > 0)" class="row q-col-gutter-md">
      <!-- Virtual book: collected pages -->
      <div v-if="pageFavoriteCount > 0" class="col-6 col-sm-4 col-md-3 col-lg-2">
        <q-card
          flat
          class="book-card cursor-pointer virtual-book-card"
          @click="openPageFavorites"
        >
          <q-img
            v-if="virtualBookCoverUrl"
            :src="virtualBookCoverUrl"
            :ratio="3/4"
            fit="cover"
          >
            <template v-slot:loading>
              <div class="full-width full-height flex flex-center" style="background: var(--bg-surface)">
                <q-skeleton type="rect" style="height: 100%; width: 100%" />
              </div>
            </template>
            <q-badge color="purple" floating class="format-badge q-mt-sm q-mr-sm">FAV</q-badge>
          </q-img>
          <div v-else class="virtual-book-cover-fallback">
            <q-icon name="auto_stories" size="48px" color="brand" />
            <div class="virtual-book-count">{{ pageFavoriteCount }}</div>
          </div>
          <div class="book-card-info">
            <div class="book-card-title">收藏的页面</div>
            <div class="book-card-meta">{{ pageFavoriteCount }} 页</div>
          </div>
        </q-card>
      </div>

      <div
        v-for="book in favorites"
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
          </q-img>

          <div class="book-card-info">
            <div class="book-card-title">{{ book.title }}</div>
            <div class="book-card-meta">
              {{ book.page_count }} 页
            </div>
          </div>
        </q-card>
      </div>
    </div>
  </q-page>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { getFavorites, coverUrl as coverUrlFn, bookFormat, getAllPageFavorites, imagePageByNameUrl, pdfPageSvgUrl, type Book, type FavoritePageItem } from '../api'

const router = useRouter()
const favorites = ref<Book[]>([])
const pageFavoriteCount = ref(0)
const pageFavorites = ref<FavoritePageItem[]>([])
const loading = ref(true)
const coverUrl = coverUrlFn

// 虚拟书封面：用第一张收藏页的图片
const virtualBookCoverUrl = computed(() => {
  const first = pageFavorites.value[0]
  if (!first) return ''
  if (first.book_type === 'pdf') {
    const pageNum = parseInt(first.filename, 10)
    return pdfPageSvgUrl(first.book_id, pageNum)
  }
  return imagePageByNameUrl(first.book_id, first.filename)
})

const totalDisplay = computed(() => {
  const bookCount = favorites.value.length
  const pageCount = pageFavoriteCount.value
  if (pageCount > 0) return bookCount + 1 + ' 本/集'
  return bookCount + ' 本'
})

function openPageFavorites() {
  router.push({ name: 'detail', params: { id: '__page_favorites__' } })
}

onMounted(async () => {
  loading.value = true
  try {
    const [books, pages] = await Promise.all([
      getFavorites(),
      getAllPageFavorites(),
    ])
    favorites.value = books
    pageFavorites.value = pages
    pageFavoriteCount.value = pages.length
  } finally { loading.value = false }
})
</script>

<style scoped lang="sass">
.virtual-book-cover-fallback
  aspect-ratio: 3/4
  display: flex
  flex-direction: column
  align-items: center
  justify-content: center
  background: var(--bg-surface)
  border-radius: 8px 8px 0 0
  position: relative

.virtual-book-count
  position: absolute
  bottom: 8px
  right: 8px
  background: rgba(0,0,0,0.5)
  color: white
  border-radius: 12px
  padding: 2px 8px
  font-size: 12px
</style>
