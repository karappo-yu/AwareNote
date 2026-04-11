import axios from 'axios'

const api = axios.create({ baseURL: '/api' })

// --- Types matching the real API ---
export interface Category {
  id: number
  name: string
  path: string
  book_count: number
  total_book_count: number
  sub_categories: Category[]
}

export interface Book {
  id: string
  title: string
  path: string
  kind?: string
  type: string  // 'pdf_book' | 'image_book'
  size: number
  mtime: string
  page_count: number
  is_favorite: boolean
  cover_path: string | null
  created_at: string
}

export interface BookDetail {
  id: string
  title: string
  path: string
  type: string
  page_count: number
  is_favorite: boolean
  description: string | null
  optimization_strategy: number | null
  avg_page_pixels: number | null
  is_oversized: boolean
}

export interface ConfigStats {
  total_books: number
  cache_size_mb: number
  server_status: string
  version: string
}

export interface ConfigSettings {
  app_name: string
  host: string
  port: number
  log_level: string
  database_url: string
  root_path: string
  scan_paths: string[]
  image_exts: string[]
  min_image_count: number
  cover_width: number
  image_page_preview_width: number
  oversized_image_avg_pixels: number
  pdf_svg_width: number
  max_render_jobs: number
  http_concurrency_limit: number
  database_max_connections: number
  database_min_connections: number
  file_io_concurrency: number
  [key: string]: unknown
}

export interface Config {
  stats: ConfigStats
  settings: ConfigSettings
}

export interface ClearCacheResponse {
  success: boolean
  space_freed_mb: number
  target: string
}

export interface ScanResponse {
  success: boolean
  message: string
  inserted_book_files: number
  updated_book_files: number
  deleted_book_files: number
  inserted_categories: number
  deleted_categories: number
  inserted_libraries: number
  deleted_libraries: number
}

export interface ScanStreamMessage {
  kind: 'log' | 'complete' | 'failed'
  message: string
  data: ScanResponse | null
}

// --- API functions ---
export const getBooks = (params?: Record<string, string>) =>
  api.get('/books', { params: { all: 'true', ...params } })
    .then(r => r.data.items as Book[])

export const getBook = (id: string) =>
  api.get(`/books/${id}`)
    .then(r => r.data as BookDetail)

export const getCategories = () =>
  api.get('/categories')
    .then(r => r.data as Category[])

export const getCategoryBooks = (categoryId: number) =>
  api.get(`/categories/${categoryId}/books`)
    .then(r => r.data.items as Book[])

export const getFavorites = () =>
  api.get('/books/favorite/list', { params: { all: 'true' } })
    .then(r => r.data.items as Book[])

export const addFavorite = (id: string) =>
  api.post(`/books/${id}/favorite`).then(r => r.data)

export const removeFavorite = (id: string) =>
  api.delete(`/books/${id}/favorite`).then(r => r.data)

export const toggleFavorite = async (id: string, current: boolean) =>
  current ? removeFavorite(id) : addFavorite(id)

export const revealInFinder = (id: string) =>
  api.post(`/books/${id}/reveal`).then(r => r.data)

// --- Config API ---
export const getConfig = () =>
  api.get('/config').then(r => r.data as Config)

export const updateConfig = (settings: Partial<ConfigSettings>) =>
  api.put('/config', settings).then(r => r.data as Config)

// --- Cache API ---
export const clearCache = (target?: string) =>
  api.delete(`/cache/clear${target ? `/${target}` : ''}`)
    .then(r => r.data as ClearCacheResponse)

// --- Scan API (SSE) ---
export function scanStream(
  onLog: (msg: string) => void,
  onComplete: (data: ScanResponse) => void,
  onFailed: (msg: string) => void,
): EventSource {
  const es = new EventSource('/scan/stream')

  es.addEventListener('log', (event) => {
    try {
      const payload: ScanStreamMessage = JSON.parse(event.data)
      onLog(payload.message)
    } catch {
      onLog('处理中...')
    }
  })

  es.addEventListener('complete', (event) => {
    try {
      const payload: ScanStreamMessage = JSON.parse(event.data)
      if (payload.data) onComplete(payload.data as ScanResponse)
      else onComplete({ success: true, message: payload.message } as ScanResponse)
    } catch {
      onComplete({ success: true, message: '扫描完成' } as ScanResponse)
    } finally {
      es.close()
    }
  })

  es.addEventListener('failed', (event) => {
    let message = '扫描失败'
    try {
      const payload: ScanStreamMessage = JSON.parse(event.data)
      message = payload.message || message
    } catch { /* ignore */ }
    onFailed(message)
    es.close()
  })

  es.onerror = () => {
    onFailed('扫描连接中断')
    es.close()
  }

  return es
}

// --- URL helpers ---
export const coverUrl = (id: string) => `/api/books/covers/${id}`
export const pdfPageSvgUrl = (id: string, page: number) => `/api/books/svg/${id}/${page}`
export const imagePageUrl = (id: string, page: number, realsize = false) =>
  `/api/books/${id}/${page}${realsize ? '?realsize=true' : ''}`

export const bookFormat = (type: string) =>
  type === 'pdf_book' ? 'pdf' : 'image'

// --- Image book pages by filename ---
export interface PageInfo {
  filename: string
  w: number | null
  h: number | null
}

export const getBookPages = (id: string) =>
  api.get(`/books/${id}/pages`)
    .then(r => r.data.pages as PageInfo[])

// --- Spread API ---
export interface SpreadInfo {
  book_id: string
  filename: string
  next_file: string
  created_at: number
}

export const getSpreads = (id: string) =>
  api.get(`/books/${id}/spreads`)
    .then(r => r.data.spreads as SpreadInfo[])

export const createSpread = (id: string, filename: string, next_file: string) =>
  api.post(`/books/${id}/spreads`, { filename, next_file })
    .then(r => r.data as { success: boolean; spread: SpreadInfo })

export const deleteSpread = (id: string, filename: string) =>
  api.delete(`/books/${id}/spreads/${encodeURIComponent(filename)}`)
    .then(r => r.data as { success: boolean })

export const imagePageByNameUrl = (id: string, filename: string, realsize = false) =>
  `/api/books/${id}/page/${encodeURIComponent(filename)}${realsize ? '?realsize=true' : ''}`

// --- User Data API ---
export interface UserDataItem {
  key: string
  value: string
}

export const getSettings = () =>
  api.get('/settings')
    .then(r => r.data.settings as UserDataItem[])

export const updateSettings = (settings: UserDataItem[]) =>
  api.put('/settings', { settings })
    .then(r => r.data as { success: boolean })

export const getBookSettings = (id: string) =>
  api.get(`/books/${id}/settings`)
    .then(r => r.data.settings as UserDataItem[])

export const updateBookSettings = (id: string, settings: UserDataItem[]) =>
  api.put(`/books/${id}/settings`, { settings })
    .then(r => r.data as { success: boolean })
