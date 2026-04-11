<template>
  <q-page class="q-pa-md q-pa-lg-md">
    <!-- Header -->
    <div class="row items-center justify-between q-mb-lg">
      <div>
        <div class="section-title">系统设置</div>
        <div class="section-subtitle">管理你的书库与运行时配置</div>
      </div>
      <q-btn
        unelevated no-caps
        icon="save"
        label="保存配置"
        color="brand"
        class="action-btn-primary"
        :loading="saving"
        @click="saveConfig"
      />
    </div>

    <q-inner-loading :showing="loading">
      <q-spinner-dots size="50px" color="brand" />
    </q-inner-loading>

    <template v-if="!loading && config">
      <!-- ═══════ 运行状态 ═══════ -->
      <div class="text-overline q-mb-sm q-mt-md" style="font-size: 10px; letter-spacing: 2px; color: var(--text-muted)">
        <q-icon name="activity" size="14px" class="q-mr-xs" />运行状态
      </div>
      <div class="row q-col-gutter-sm q-mb-lg">
        <div class="col-6 col-md-3">
          <q-card flat class="surface-card text-center" style="border-radius: 14px">
            <q-card-section>
              <div class="text-h5 text-weight-bold" style="color: var(--brand)">{{ config.stats.total_books }}</div>
              <div style="font-size: 11px; color: var(--text-muted)">总计书籍</div>
            </q-card-section>
          </q-card>
        </div>
        <div class="col-6 col-md-3">
          <q-card flat class="surface-card text-center" style="border-radius: 14px">
            <q-card-section>
              <div class="text-h5 text-weight-bold">{{ formatSize(config.stats.cache_size_mb) }}</div>
              <div style="font-size: 11px; color: var(--text-muted)">缓存占用</div>
            </q-card-section>
          </q-card>
        </div>
        <div class="col-6 col-md-3">
          <q-card flat class="surface-card text-center" style="border-radius: 14px">
            <q-card-section>
              <div class="text-h5 text-weight-bold" :style="{ color: config.stats.server_status === 'healthy' ? '#10b981' : '#f59e0b' }">
                {{ config.stats.server_status.toUpperCase() }}
              </div>
              <div style="font-size: 11px; color: var(--text-muted)">后端状态</div>
            </q-card-section>
          </q-card>
        </div>
        <div class="col-6 col-md-3">
          <q-card flat class="surface-card text-center" style="border-radius: 14px">
            <q-card-section>
              <div class="text-h5 text-weight-bold" style="color: var(--brand)">v{{ config.stats.version }}</div>
              <div style="font-size: 11px; color: var(--text-muted)">API 版本</div>
            </q-card-section>
          </q-card>
        </div>
      </div>

      <!-- ═══════ 基础文件设置 ═══════ -->
      <div class="text-overline q-mb-sm" style="font-size: 10px; letter-spacing: 2px; color: var(--text-muted)">
        <q-icon name="folder_cog" size="14px" class="q-mr-xs" />基础文件设置
      </div>
      <q-card flat class="surface-card q-mb-lg" style="border-radius: 14px">
        <q-card-section class="q-pa-md q-pa-lg-md">
          <!-- 扫描路径 -->
          <div class="q-mb-lg">
            <div class="text-caption text-weight-bold q-mb-sm" style="font-size: 11px; letter-spacing: 0.5px; color: var(--text-secondary)">
              扫描路径
            </div>
            <q-input
              v-model="form.scan_paths_text"
              type="textarea"
              filled
              autogrow
              placeholder="/path/to/library-1&#10;/path/to/library-2"
              class="settings-input"
            />
            <div class="text-caption q-mt-xs" style="color: var(--text-muted); font-size: 11px">每行一个目录，支持配置多个扫描路径</div>
          </div>

          <div class="row q-col-gutter-md">
            <!-- 图片格式 -->
            <div class="col-12 col-md-6">
              <div class="text-caption text-weight-bold q-mb-sm" style="font-size: 11px; letter-spacing: 0.5px; color: var(--text-secondary)">
                扫描的图片格式
              </div>
              <q-input
                v-model="form.image_exts_text"
                filled
                placeholder=".jpg, .png, .webp"
                class="settings-input"
              />
              <div class="text-caption q-mt-xs" style="color: var(--text-muted); font-size: 11px">使用逗号分隔不同后缀名</div>
            </div>
            <!-- 最少图片数 -->
            <div class="col-12 col-md-6">
              <div class="text-caption text-weight-bold q-mb-sm" style="font-size: 11px; letter-spacing: 0.5px; color: var(--text-secondary)">
                判定为书籍的最少图片数
              </div>
              <q-input
                v-model.number="form.min_image_count"
                type="number"
                filled
                :min="1"
                class="settings-input"
              />
              <div class="text-caption q-mt-xs" style="color: var(--text-muted); font-size: 11px">目录内图片数达到此值后才被识别为图片书</div>
            </div>
          </div>
        </q-card-section>
      </q-card>

      <!-- ═══════ 图像处理策略 ═══════ -->
      <div class="text-overline q-mb-sm" style="font-size: 10px; letter-spacing: 2px; color: var(--text-muted)">
        <q-icon name="image" size="14px" class="q-mr-xs" />图像处理策略
      </div>
      <q-card flat class="surface-card q-mb-lg" style="border-radius: 14px">
        <q-card-section class="q-pa-md q-pa-lg-md">
          <div class="row q-col-gutter-md">
            <div class="col-6 col-md-3">
              <div class="text-caption text-weight-bold q-mb-sm" style="font-size: 11px; letter-spacing: 0.5px; color: var(--text-secondary)">封面宽度 (px)</div>
              <q-input v-model.number="form.cover_width" type="number" filled :min="64" class="settings-input" />
            </div>
            <div class="col-6 col-md-3">
              <div class="text-caption text-weight-bold q-mb-sm" style="font-size: 11px; letter-spacing: 0.5px; color: var(--text-secondary)">超大图预览宽度 (px)</div>
              <q-input v-model.number="form.image_page_preview_width" type="number" filled :min="256" class="settings-input" />
            </div>
            <div class="col-6 col-md-3">
              <div class="text-caption text-weight-bold q-mb-sm" style="font-size: 11px; letter-spacing: 0.5px; color: var(--text-secondary)">超大图像素阈值</div>
              <q-input v-model.number="form.oversized_image_avg_pixels" type="number" filled :min="1000000" class="settings-input" />
            </div>
            <div class="col-6 col-md-3">
              <div class="text-caption text-weight-bold q-mb-sm" style="font-size: 11px; letter-spacing: 0.5px; color: var(--text-secondary)">PDF SVG 宽度 (px)</div>
              <q-input v-model.number="form.pdf_svg_width" type="number" filled :min="256" class="settings-input" />
            </div>
          </div>
        </q-card-section>
      </q-card>

      <!-- ═══════ 数据库与维护 ═══════ -->
      <div class="text-overline q-mb-sm" style="font-size: 10px; letter-spacing: 2px; color: var(--text-muted)">
        <q-icon name="database" size="14px" class="q-mr-xs" />数据库与维护
      </div>
      <q-card flat class="surface-card q-mb-lg" style="border-radius: 14px">
        <q-card-section class="q-pa-md q-pa-lg-md">
          <div class="row q-col-gutter-md">
            <!-- 刷新数据库 -->
            <div class="col-12 col-md-6">
              <q-card flat bordered class="q-pa-md" style="border-radius: 12px; border-color: var(--border-subtle)">
                <div class="row items-center justify-between no-wrap">
                  <div>
                    <div class="text-weight-bold" style="font-size: 14px">刷新数据库</div>
                    <div style="font-size: 11px; color: var(--text-muted)">触发后台文件系统扫描</div>
                  </div>
                  <q-btn
                    unelevated no-caps
                    icon="refresh"
                    label="立即扫描"
                    color="brand"
                    outline
                    class="action-btn-outline"
                    :loading="scanning"
                    @click="handleScan"
                  />
                </div>
              </q-card>
            </div>
            <!-- 清空缓存 -->
            <div class="col-12 col-md-6">
              <q-card flat bordered class="q-pa-md" style="border-radius: 12px; border-color: var(--border-subtle)">
                <div class="text-weight-bold q-mb-xs" style="font-size: 14px">清空系统缓存</div>
                <div style="font-size: 11px; color: var(--text-muted)" class="q-mb-sm">分别清理封面、缩略图和 SVG 缓存</div>
                <div class="row q-gutter-xs">
                  <q-btn
                    v-for="t in cacheTargets"
                    :key="t.value"
                    unelevated no-caps
                    :label="t.label"
                    :color="t.value === 'all' ? 'brand' : undefined"
                    :outline="t.value !== 'all'"
                    size="sm"
                    class="action-btn-outline"
                    :loading="clearingCache === t.value"
                    @click="handleClearCache(t.value)"
                  />
                </div>
              </q-card>
            </div>
          </div>

          <!-- 扫描日志 -->
          <q-slide-transition>
            <div v-if="logs.length > 0" class="q-mt-md">
              <q-card flat class="q-pa-none" style="border-radius: 12px; background: rgba(0,0,0,0.2); border: 1px solid var(--border-subtle)">
                <div class="row items-center justify-between q-px-md q-py-sm" style="border-bottom: 1px solid var(--border-subtle)">
                  <div class="row items-center q-gutter-xs">
                    <span class="log-dot" :class="{ 'log-dot-active': scanning }"></span>
                    <span style="font-size: 10px; font-weight: 700; color: var(--text-muted); letter-spacing: 1px">服务端扫描日志</span>
                  </div>
                  <q-btn flat dense no-caps label="清除" size="xs" style="color: var(--text-muted)" @click="logs = []" />
                </div>
                <div class="log-container q-pa-md" style="max-height: 200px; overflow-y: auto">
                  <div v-for="(log, idx) in logs" :key="idx" class="log-line q-mb-xs">
                    <span class="log-type" :class="'log-type-' + log.type">[{{ log.type }}]</span>
                    <span style="font-size: 12px; color: var(--text-secondary)">{{ log.message }}</span>
                  </div>
                </div>
              </q-card>
            </div>
          </q-slide-transition>
        </q-card-section>
      </q-card>
    </template>
  </q-page>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useQuasar } from 'quasar'
import { getConfig, updateConfig, clearCache, scanStream, type Config, type ScanResponse } from '../api'

const $q = useQuasar()
const config = ref<Config | null>(null)
const loading = ref(true)
const saving = ref(false)
const scanning = ref(false)
const clearingCache = ref<string | null>(null)
const logs = ref<{ type: string; message: string }[]>([])

const cacheTargets = [
  { label: '全部', value: 'all' },
  { label: 'SVG', value: 'svg' },
  { label: '封面', value: 'covers' },
  { label: '缩略图', value: 'thumbnails' },
]

// 表单数据
const form = reactive({
  scan_paths_text: '',
  image_exts_text: '',
  min_image_count: 3,
  cover_width: 480,
  image_page_preview_width: 1600,
  oversized_image_avg_pixels: 10000000,
  pdf_svg_width: 1400,
})

function formatSize(mb: number): string {
  if (mb < 1) return `${(mb * 1024).toFixed(0)} KB`
  if (mb < 1024) return `${mb.toFixed(1)} MB`
  return `${(mb / 1024).toFixed(2)} GB`
}

function populateForm(settings: Config['settings']) {
  form.scan_paths_text = (settings.scan_paths || []).join('\n')
  form.image_exts_text = (settings.image_exts || []).join(', ')
  form.min_image_count = settings.min_image_count ?? 3
  form.cover_width = settings.cover_width ?? 480
  form.image_page_preview_width = settings.image_page_preview_width ?? 1600
  form.oversized_image_avg_pixels = settings.oversized_image_avg_pixels ?? 10000000
  form.pdf_svg_width = settings.pdf_svg_width ?? 1400
}

async function fetchConfig() {
  loading.value = true
  try {
    config.value = await getConfig()
    populateForm(config.value.settings)
  } finally {
    loading.value = false
  }
}

async function saveConfig() {
  saving.value = true
  try {
    const payload: Record<string, unknown> = {
      scan_paths: form.scan_paths_text.split('\n').map(s => s.trim()).filter(Boolean),
      image_exts: form.image_exts_text.split(',').map(s => s.trim()).filter(Boolean),
      min_image_count: form.min_image_count,
      cover_width: form.cover_width,
      image_page_preview_width: form.image_page_preview_width,
      oversized_image_avg_pixels: form.oversized_image_avg_pixels,
      pdf_svg_width: form.pdf_svg_width,
    }
    const result = await updateConfig(payload)
    config.value = result
    populateForm(result.settings)
    $q.notify({ type: 'positive', message: '设置已保存并生效' })
  } catch {
    $q.notify({ type: 'negative', message: '保存设置失败' })
  } finally {
    saving.value = false
  }
}

function handleScan() {
  scanning.value = true
  logs.value = []
  addLog('INFO', '正在启动扫描...')

  scanStream(
    (msg) => { addLog('INFO', msg) },
    (data: ScanResponse) => {
      addLog('SUCCESS', `新增 ${data.inserted_book_files} 本，更新 ${data.updated_book_files} 本，删除 ${data.deleted_book_files} 本`)
      scanning.value = false
      $q.notify({ type: 'positive', message: data.message || '扫描完成' })
      fetchConfig()
    },
    (msg) => {
      addLog('ERROR', msg)
      scanning.value = false
      $q.notify({ type: 'negative', message: msg })
    },
  )
}

function addLog(type: string, message: string) {
  logs.value.push({ type, message })
}

async function handleClearCache(target: string) {
  const label = cacheTargets.find(t => t.value === target)?.label || target
  $q.dialog({
    title: '确认清空缓存',
    message: `确定要清空「${label}」缓存吗？此操作不可撤销。`,
    cancel: { label: '取消', flat: true, noCaps: true },
    ok: { label: '确认清空', color: 'negative', noCaps: true, unelevated: true },
    persistent: true,
  }).onOk(async () => {
    clearingCache.value = target
    try {
      const result = await clearCache(target)
      $q.notify({ type: 'positive', message: `${label}缓存清理成功：释放了 ${result.space_freed_mb.toFixed(1)} MB 空间` })
      fetchConfig()
    } catch {
      $q.notify({ type: 'negative', message: '缓存清理失败' })
    } finally {
      clearingCache.value = null
    }
  })
}

onMounted(() => { fetchConfig() })
</script>

<style scoped>
.settings-input :deep(.q-field__control) {
  border-radius: 10px;
  font-size: 13px;
}

.settings-input :deep(.q-field__control) {
  background: var(--bg-surface-elevated) !important;
}

.log-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--text-muted);
  display: inline-block;
}

.log-dot-active {
  background: #10b981;
  animation: pulse-dot 1.5s infinite;
}

@keyframes pulse-dot {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.log-type {
  font-size: 11px;
  font-weight: 700;
  margin-right: 4px;
}

.log-type-INFO { color: var(--text-muted); }
.log-type-SUCCESS { color: #10b981; }
.log_type-ERROR { color: #ef4444; }
.log-type-ERROR { color: #ef4444; }

.log-container::-webkit-scrollbar {
  width: 4px;
}
.log-container::-webkit-scrollbar-thumb {
  background: rgba(128, 128, 128, 0.2);
  border-radius: 4px;
}
</style>
