<template>
  <canvas
    ref="canvasRef"
    class="doodle-canvas"
    :class="{ 'doodle-canvas--eraser': tool === 'eraser' }"
    :style="canvasCursor"
    @pointerdown="onPointerDown"
    @pointermove="onPointerMove"
    @pointerup="onPointerUp"
    @pointercancel="onPointerUp"
  />
</template>

<script setup lang="ts">
/**
 * DoodleCanvas — 涂鸦画布组件
 *
 * 叠加在查看器图片上的透明画布，支持：
 * - 画笔：平滑二次贝塞尔曲线笔迹
 * - 橡皮擦：真正的像素级擦除（destination-out 合成）。
 *   擦除动作作为 eraser 笔画与画笔统一按序存储，
 *   渲染时按顺序重放，撤销/重做/持久化逻辑完全一致
 * - 坐标归一化（0~1），窗口/图片尺寸变化时笔迹等比缩放
 * - devicePixelRatio 适配，高分屏不模糊
 *
 * 笔画状态由父组件持有（本组件只渲染 + 采集输入），
 * 父组件负责持久化（防抖保存到 per-book settings）。
 */
import { ref, watch, computed, onMounted, onBeforeUnmount } from 'vue'

export interface DoodlePoint {
  x: number // 归一化 0~1
  y: number // 归一化 0~1
}

export type DoodleTool = 'pen' | 'eraser'

export interface DoodleStroke {
  tool: DoodleTool
  /** 画笔颜色（eraser 忽略） */
  color?: string
  /** 归一化宽度（相对画布高度），渲染时随图片等比缩放 */
  width: number
  points: DoodlePoint[]
}

const props = defineProps<{
  strokes: DoodleStroke[]
  tool: DoodleTool
  color: string
  /** 当前工具粗细（当前锚点尺寸下的像素值） */
  width: number
  /**
   * 坐标锚点元素（通常为图片）：笔迹坐标归一化到它的矩形。
   * 画布可以比锚点大（涂出图片外），越界坐标 <0 或 >1 合法。
   */
  anchorEl?: HTMLElement | null
  disabled?: boolean
}>()

const emit = defineEmits<{
  (e: 'stroke-end', stroke: DoodleStroke): void
}>()

const canvasRef = ref<HTMLCanvasElement | null>(null)
let ctx: CanvasRenderingContext2D | null = null
let cssW = 1
let cssH = 1
let resizeObserver: ResizeObserver | null = null

// 正在绘制的笔迹
let drawing = false
let activePointerId: number | null = null
let livePoints: DoodlePoint[] = []

// 已提交笔画的渲染结果缓存，实时绘制时避免整帧重放所有历史笔画
let baseCanvas: HTMLCanvasElement | null = null

onMounted(() => {
  const cv = canvasRef.value
  if (!cv) return
  ctx = cv.getContext('2d')
  resizeObserver = new ResizeObserver(() => syncSize())
  resizeObserver.observe(cv)
  syncSize()
})

onBeforeUnmount(() => {
  resizeObserver?.disconnect()
  resizeObserver = null
})

// 笔画列表变化（新增/撤销/清空）→ 全量重放并刷新缓存
watch(
  () => props.strokes,
  () => redraw(),
  { deep: true }
)

/** 锚点（图片）相对画布的矩形度量 */
interface AnchorMetrics {
  ax: number // 锚点在画布内的偏移
  ay: number
  aw: number // 锚点尺寸
  ah: number
  cw: number // 画布 CSS 尺寸
  ch: number
}

function metrics(): AnchorMetrics {
  const cv = canvasRef.value!
  const cRect = cv.getBoundingClientRect()
  const cw = Math.max(1, cRect.width)
  const ch = Math.max(1, cRect.height)
  const a = props.anchorEl
  if (a) {
    const r = a.getBoundingClientRect()
    if (r.width > 1 && r.height > 1) {
      return {
        ax: r.left - cRect.left,
        ay: r.top - cRect.top,
        aw: r.width,
        ah: r.height,
        cw,
        ch,
      }
    }
  }
  // 无锚点时退化为整块画布
  return { ax: 0, ay: 0, aw: cw, ah: ch, cw, ch }
}

// 橡皮擦光标：圆圈直径 = 实际擦除直径（白圈+黑圈双色，深浅背景都可见）
const canvasCursor = computed(() => {
  if (props.tool !== 'eraser') return undefined
  const d = Math.max(6, Math.round(props.width))
  const r = d / 2
  const svg =
    `<svg xmlns='http://www.w3.org/2000/svg' width='${d}' height='${d}'>` +
    `<circle cx='${r}' cy='${r}' r='${Math.max(1, r - 1)}' fill='none' stroke='white' stroke-width='1.5'/>` +
    `<circle cx='${r}' cy='${r}' r='${Math.max(1, r - 1)}' fill='none' stroke='black' stroke-width='0.75'/>` +
    `</svg>`
  return { cursor: `url("data:image/svg+xml,${encodeURIComponent(svg)}") ${r} ${r}, auto` }
})

function syncSize() {
  const cv = canvasRef.value
  if (!cv || !ctx) return
  const rect = cv.getBoundingClientRect()
  cssW = Math.max(1, rect.width)
  cssH = Math.max(1, rect.height)
  const dpr = window.devicePixelRatio || 1
  cv.width = Math.max(1, Math.round(cssW * dpr))
  cv.height = Math.max(1, Math.round(cssH * dpr))
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  redraw()
}

/** 全量重放已提交笔画，并更新缓存位图 */
function redraw() {
  const cv = canvasRef.value
  if (!cv || !ctx) return
  const m = metrics()
  ctx.clearRect(0, 0, m.cw, m.ch)
  for (const s of props.strokes) drawStroke(ctx, s, m)

  if (!baseCanvas) baseCanvas = document.createElement('canvas')
  baseCanvas.width = cv.width
  baseCanvas.height = cv.height
  const bctx = baseCanvas.getContext('2d')
  if (bctx) {
    const dpr = window.devicePixelRatio || 1
    bctx.setTransform(dpr, 0, 0, dpr, 0, 0)
    bctx.clearRect(0, 0, m.cw, m.ch)
    for (const s of props.strokes) drawStroke(bctx, s, m)
  }
}

/** 实时绘制：缓存位图 + 当前笔画（橡皮擦同样以 destination-out 作用在缓存上） */
function renderLive() {
  if (!ctx) return
  const m = metrics()
  ctx.clearRect(0, 0, m.cw, m.ch)
  if (baseCanvas) ctx.drawImage(baseCanvas, 0, 0, m.cw, m.ch)
  drawStroke(
    ctx,
    {
      tool: props.tool,
      color: props.color,
      width: props.width / m.ah,
      points: livePoints,
    },
    m
  )
}

function drawStroke(
  target: CanvasRenderingContext2D,
  s: DoodleStroke,
  m: AnchorMetrics
) {
  if (s.points.length === 0) return
  const lw = Math.max(1, s.width * m.ah)
  const isEraser = s.tool === 'eraser'
  target.save()
  // 橡皮擦：destination-out 真正擦除像素（对缓存/重放均生效）
  target.globalCompositeOperation = isEraser ? 'destination-out' : 'source-over'
  target.strokeStyle = isEraser ? '#000' : (s.color ?? '#000')
  target.fillStyle = target.strokeStyle
  target.lineWidth = lw
  target.lineCap = 'round'
  target.lineJoin = 'round'

  // 归一化坐标 → 锚点像素坐标（允许越界：x<0 / x>1 落在锚点外）
  const pts = s.points.map(p => ({ x: p.x * m.aw + m.ax, y: p.y * m.ah + m.ay }))
  if (pts.length === 1) {
    // 单点 → 圆点
    target.beginPath()
    target.arc(pts[0].x, pts[0].y, lw / 2, 0, Math.PI * 2)
    target.fill()
    target.restore()
    return
  }
  target.beginPath()
  target.moveTo(pts[0].x, pts[0].y)
  // 相邻中点 + 二次贝塞尔平滑
  for (let i = 1; i < pts.length - 1; i++) {
    const mx = (pts[i].x + pts[i + 1].x) / 2
    const my = (pts[i].y + pts[i + 1].y) / 2
    target.quadraticCurveTo(pts[i].x, pts[i].y, mx, my)
  }
  const last = pts[pts.length - 1]
  target.lineTo(last.x, last.y)
  target.stroke()
  target.restore()
}

// ============== 指针输入 ==============

function normPoint(e: PointerEvent): DoodlePoint {
  const cv = canvasRef.value!
  const a = props.anchorEl
  // 图片尚未加载（0×0）时退化为画布矩形，避免除零
  let r = a?.getBoundingClientRect()
  if (!r || r.width < 1 || r.height < 1) r = cv.getBoundingClientRect()
  // 不做 0~1 裁剪：允许涂出锚点（图片）范围外
  return {
    x: (e.clientX - r.left) / r.width,
    y: (e.clientY - r.top) / r.height,
  }
}

function onPointerDown(e: PointerEvent) {
  if (props.disabled) return
  // 多指触控：忽略非首发指针，避免笔迹跳变
  if (drawing && activePointerId !== e.pointerId) return
  e.preventDefault()
  e.stopPropagation()
  try {
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
  } catch { /* ignore */ }
  drawing = true
  activePointerId = e.pointerId
  livePoints = [normPoint(e)]
  if (props.tool === 'eraser') renderLive()
}

function onPointerMove(e: PointerEvent) {
  if (!drawing || props.disabled || e.pointerId !== activePointerId) return
  e.preventDefault()
  const p = normPoint(e)
  const last = livePoints[livePoints.length - 1]
  // 过滤微小抖动，减少点位数量
  if (last && Math.hypot((p.x - last.x) * cssW, (p.y - last.y) * cssH) < 1.5) return
  livePoints.push(p)
  renderLive()
}

function onPointerUp(e: PointerEvent) {
  if (!drawing || e.pointerId !== activePointerId) return
  drawing = false
  activePointerId = null
  try {
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId)
  } catch { /* ignore */ }

  let pts = livePoints
  if (pts.length === 1) {
    // 单击 → 补一个点使其成为圆点（画笔画点 / 橡皮点擦）
    pts = [pts[0], { x: pts[0].x + 0.0008, y: pts[0].y + 0.0008 }]
  }
  livePoints = []
  // 笔迹宽度归一化到锚点（图片）高度，与历史数据语义一致
  const m = metrics()
  emit('stroke-end', {
    tool: props.tool,
    color: props.tool === 'pen' ? props.color : undefined,
    width: props.width / m.ah,
    points: pts,
  })
}
</script>
