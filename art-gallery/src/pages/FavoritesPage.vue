<template>
  <q-page class="q-pa-lg">
    <div class="q-mb-lg">
      <div class="section-title">我的收藏</div>
      <div class="section-subtitle">{{ favorites.length }} 本</div>
    </div>

    <q-inner-loading :showing="loading">
      <q-spinner-dots size="50px" color="brand" />
    </q-inner-loading>

    <!-- Empty -->
    <div v-if="!loading && favorites.length === 0" class="column items-center q-mt-xl">
      <q-icon name="favorite_border" size="80px" style="opacity: 0.12" />
      <div class="text-h6 q-mt-md" style="opacity: 0.3">还没有收藏</div>
    </div>

    <!-- Grid -->
    <div v-if="!loading && favorites.length > 0" class="row q-col-gutter-md">
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
import { ref, onMounted } from 'vue'
import { getFavorites, coverUrl as coverUrlFn, bookFormat, type Book } from '../api'

const favorites = ref<Book[]>([])
const loading = ref(true)
const coverUrl = coverUrlFn

onMounted(async () => {
  loading.value = true
  try { favorites.value = await getFavorites() }
  finally { loading.value = false }
})
</script>
