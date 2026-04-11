import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { LocalStorage } from 'quasar'
import { getBooks, getCategories, getFavorites, type Book, type Category } from '../api'

const STORAGE_KEY = 'art-gallery-selected-category'

export const useLibrary = defineStore('library', () => {
  const books = ref<Book[]>([])
  const categories = ref<Category[]>([])
  const favorites = ref<Book[]>([])
  const loading = ref(false)
  const selectedCategory = ref<number | null>(LocalStorage.getItem(STORAGE_KEY) ?? null)
  const searchQuery = ref('')
  const sortBy = ref<'date' | 'title' | 'size' | 'pages'>('date')
  const sortDesc = ref(true)

  // Persist selected category to localStorage
  watch(selectedCategory, (val) => {
    if (val === null) {
      LocalStorage.remove(STORAGE_KEY)
    } else {
      LocalStorage.set(STORAGE_KEY, val)
    }
  })

  const filteredBooks = computed(() => {
    let result = [...books.value]

    if (searchQuery.value) {
      const q = searchQuery.value.toLowerCase()
      result = result.filter(b => b.title.toLowerCase().includes(q))
    }

    if (selectedCategory.value) {
      result = result.filter(b => {
        const cat = findCategory(categories.value, selectedCategory.value!)
        if (!cat) return false
        // 精确匹配路径前缀，避免 /books/art 误匹配 /books/art-history
        const catPath = cat.path.endsWith('/') ? cat.path : cat.path + '/'
        return b.path === cat.path || b.path.startsWith(catPath)
      })
    }

    const key: Record<string, string> = {
      date: 'created_at', title: 'title', size: 'size', pages: 'page_count'
    }
    const k = key[sortBy.value] as keyof Book
    result.sort((a, b) => {
      const va = a[k], vb = b[k]
      const cmp = typeof va === 'string' ? va.localeCompare(vb as string) : (va as number) - (vb as number)
      return sortDesc.value ? -cmp : cmp
    })

    return result
  })

  const categoryTree = computed(() => categories.value)

  function findCategory(cats: Category[], id: number): Category | null {
    for (const c of cats) {
      if (c.id === id) return c
      const found = findCategory(c.sub_categories ?? [], id)
      if (found) return found
    }
    return null
  }

  async function fetchBooks() {
    loading.value = true
    try {
      books.value = await getBooks()
    } finally {
      loading.value = false
    }
  }

  async function fetchCategories() {
    categories.value = await getCategories()
  }

  async function fetchFavorites() {
    favorites.value = await getFavorites()
  }

  return {
    books, categories, favorites, loading,
    selectedCategory, searchQuery, sortBy, sortDesc,
    filteredBooks, categoryTree,
    fetchBooks, fetchCategories, fetchFavorites
  }
})
