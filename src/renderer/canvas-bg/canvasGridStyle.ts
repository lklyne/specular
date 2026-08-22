import { GRID_SIZE } from '../../shared/constants'
import { prepareScreenCanvas } from '../shared/screenCanvas'

const MIN_GRID_SPACING_PX = 8
const FULL_OPACITY_SPACING_PX = 18
const MAX_GRID_STEP_MULTIPLIER = 64

function devicePixelRatioOrOne(devicePixelRatio: number) {
  return Math.max(devicePixelRatio, 1)
}

function snapToDevicePixel(value: number, devicePixelRatio: number) {
  const dpr = devicePixelRatioOrOne(devicePixelRatio)
  return Math.round(value * dpr) / dpr
}

function gridStepMultiplierForZoom(zoom: number) {
  let multiplier = 1
  while (
    GRID_SIZE * zoom * multiplier < MIN_GRID_SPACING_PX &&
    multiplier < MAX_GRID_STEP_MULTIPLIER
  ) {
    multiplier *= 2
  }
  return multiplier
}

function gridAlpha(spacing: number, isDark: boolean) {
  const minAlpha = isDark ? 0.56 : 0.52
  const alpha = spacing / FULL_OPACITY_SPACING_PX
  return Math.max(minAlpha, Math.min(1, alpha))
}

function buildCanvasGridMetrics({
  canvasOrigin,
  pan,
  zoom,
  isDark,
  devicePixelRatio,
}: {
  canvasOrigin: { x: number; y: number }
  pan: { x: number; y: number }
  zoom: number
  isDark: boolean
  devicePixelRatio: number
}) {
  const stepMultiplier = gridStepMultiplierForZoom(zoom)
  const spacing = GRID_SIZE * zoom * stepMultiplier
  const originX = canvasOrigin.x + pan.x
  const originY = canvasOrigin.y + pan.y

  return {
    originX,
    originY,
    spacing,
    stepMultiplier,
    dotRadius: Math.max(
      0.6,
      Math.round(0.7 * devicePixelRatioOrOne(devicePixelRatio)) /
        devicePixelRatioOrOne(devicePixelRatio),
    ),
    alpha: gridAlpha(spacing, isDark),
  }
}

export function buildCanvasGridStyle() {
  return {
    backgroundColor: 'var(--surface-canvas)',
  }
}

export function drawCanvasGrid({
  canvas,
  color,
  canvasOrigin,
  pan,
  zoom,
  isDark,
  devicePixelRatio,
}: {
  canvas: HTMLCanvasElement
  color: string
  canvasOrigin: { x: number; y: number }
  pan: { x: number; y: number }
  zoom: number
  isDark: boolean
  devicePixelRatio: number
}) {
  const prepared = prepareScreenCanvas(canvas, devicePixelRatioOrOne(devicePixelRatio))
  if (!prepared) return
  const { ctx, width, height, dpr } = prepared

  const metrics = buildCanvasGridMetrics({
    canvasOrigin,
    pan,
    zoom,
    isDark,
    devicePixelRatio: dpr,
  })

  const { spacing, originX, originY, dotRadius, alpha } = metrics
  if (!Number.isFinite(spacing) || spacing <= 0) return

  const startX = originX + Math.ceil((0 - originX) / spacing) * spacing
  const startY = originY + Math.ceil((0 - originY) / spacing) * spacing

  ctx.fillStyle = color
  ctx.globalAlpha = alpha

  for (let y = startY; y <= height; y += spacing) {
    const snappedY = snapToDevicePixel(y, dpr)
    for (let x = startX; x <= width; x += spacing) {
      const snappedX = snapToDevicePixel(x, dpr)
      ctx.beginPath()
      ctx.arc(snappedX, snappedY, dotRadius, 0, Math.PI * 2)
      ctx.fill()
    }
  }

  ctx.globalAlpha = 1
}
