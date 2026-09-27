// One `brush` stroke painted by webgpu-brush into its own canvas.
//
// The canvas covers the stroke's canvas-space bounds and is painted once per
// (points, style, zoom) — panning only moves it. Zoom repaints after the zoom
// settles; in between, the last painting is stretched by CSS so a pinch never
// re-runs the brush every frame.

import { useEffect, useMemo, useRef, useState } from 'react'
import { createBrush } from 'webgpu-brush'
import type { AnnotationDrawingPoint, AnnotationDrawingStroke } from '../../shared/types'
import type { SceneView } from '../../shared/scene-projection'
import { canvasToScreenX, canvasToScreenY } from '../../shared/gesture-utils'
import {
  DEFAULT_NATURAL_BRUSH_PRESET,
  NATURAL_BRUSH_TIP_SIZE,
} from '../../shared/natural-brush'
import type { NaturalBrushGpu } from './naturalBrushDevice'

type Painting = ReturnType<typeof createBrush>

const ZOOM_SETTLE_MS = 120
// Keeps a long stroke at high zoom inside the device's texture limits.
const MAX_CANVAS_PX = 4096
// Canvas sizes round up to this step so a stroke growing under the pointer
// reuses its painting instead of rebuilding it on every move.
const CANVAS_SIZE_STEP_PX = 256
// Matches the pen's rendered diameter (perfect-freehand `size: width * 1.6`);
// webgpu-brush sizes its tip by radius.
const TIP_RADIUS_PER_WIDTH = 0.8
// Room for the brush's scatter past the stroke's centerline, in stroke widths.
const PAD_PER_WIDTH = 6
// Points closer than this many device pixels add no shape, only spline kinks.
const MIN_POINT_SPACING_PX = 1.5

export function NaturalBrushStroke({
  stroke,
  layout,
  inkColor,
  isDark,
  gpu,
}: {
  stroke: AnnotationDrawingStroke
  layout: SceneView
  inkColor: string
  isDark: boolean
  gpu: NaturalBrushGpu
}) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null)
  const paintingRef = useRef<{ api: Painting; gpu: NaturalBrushGpu; w: number; h: number } | null>(
    null,
  )
  const paintZoom = useSettledValue(layout.zoom, ZOOM_SETTLE_MS)
  const preset = stroke.brushPreset ?? DEFAULT_NATURAL_BRUSH_PRESET
  const bounds = useMemo(
    () => paddedBounds(stroke.points, stroke.width * PAD_PER_WIDTH + 4),
    [stroke.points, stroke.width],
  )
  const dpr = window.devicePixelRatio || 1
  const scale = Math.min(
    paintZoom * dpr,
    MAX_CANVAS_PX / Math.max(bounds.width, bounds.height),
  )
  const pxW = canvasSize(bounds.width * scale)
  const pxH = canvasSize(bounds.height * scale)

  useEffect(
    () => () => {
      paintingRef.current?.api.dispose()
      paintingRef.current = null
    },
    [],
  )

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    let painting = paintingRef.current
    if (!painting || painting.gpu !== gpu || painting.w !== pxW || painting.h !== pxH) {
      painting?.api.dispose()
      canvas.width = pxW
      canvas.height = pxH
      painting = {
        api: createBrush({ canvas, device: gpu.device, adapter: gpu.adapter }),
        gpu,
        w: pxW,
        h: pxH,
      }
      paintingRef.current = painting
    }
    const api = painting.api
    api.clear()
    api.seed(stroke.id)
    api.noiseSeed(stroke.id)
    api.push()
    // webgpu-brush puts the origin at the canvas center.
    api.translate(-pxW / 2, -pxH / 2)
    api.set(
      preset,
      cssColorToRgb(canvas, inkColor),
      (stroke.width * scale * TIP_RADIUS_PER_WIDTH) / NATURAL_BRUSH_TIP_SIZE[preset],
    )
    const pts = devicePoints(stroke.points, bounds.x, bounds.y, scale)
    if (pts.length === 1) {
      api.line(pts[0][0], pts[0][1], pts[0][0] + 0.5, pts[0][1])
    } else {
      api.spline(pts, 0.4)
    }
    api.pop()
    api.render()
  }, [gpu, pxW, pxH, scale, bounds, stroke.points, stroke.width, stroke.id, preset, inkColor, isDark])

  const left = canvasToScreenX(layout, bounds.x)
  const top = canvasToScreenY(layout, bounds.y) - layout.canvasOrigin.y
  return (
    <canvas
      ref={canvasRef}
      className="pointer-events-none absolute"
      style={{
        left,
        top,
        width: (pxW / scale) * layout.zoom,
        height: (pxH / scale) * layout.zoom,
      }}
      aria-hidden="true"
    />
  )
}

function canvasSize(px: number): number {
  const stepped = Math.ceil(px / CANVAS_SIZE_STEP_PX) * CANVAS_SIZE_STEP_PX
  return Math.min(Math.max(stepped, CANVAS_SIZE_STEP_PX), MAX_CANVAS_PX)
}

function useSettledValue(value: number, delayMs: number): number {
  const [settled, setSettled] = useState(value)
  useEffect(() => {
    if (value === settled) return
    const timer = setTimeout(() => setSettled(value), delayMs)
    return () => clearTimeout(timer)
  }, [value, settled, delayMs])
  return settled
}

function paddedBounds(points: AnnotationDrawingPoint[], pad: number) {
  let minX = Infinity
  let minY = Infinity
  let maxX = -Infinity
  let maxY = -Infinity
  for (const p of points) {
    minX = Math.min(minX, p.x)
    minY = Math.min(minY, p.y)
    maxX = Math.max(maxX, p.x)
    maxY = Math.max(maxY, p.y)
  }
  return {
    x: minX - pad,
    y: minY - pad,
    width: maxX - minX + pad * 2,
    height: maxY - minY + pad * 2,
  }
}

function devicePoints(
  points: AnnotationDrawingPoint[],
  originX: number,
  originY: number,
  scale: number,
): number[][] {
  const out: number[][] = []
  for (const p of points) {
    const x = (p.x - originX) * scale
    const y = (p.y - originY) * scale
    const prev = out[out.length - 1]
    if (prev && Math.hypot(x - prev[0], y - prev[1]) < MIN_POINT_SPACING_PX) continue
    out.push([x, y, 1])
  }
  return out
}

let colorProbe: CanvasRenderingContext2D | null = null

/**
 * Ink colors are CSS (theme variables, sometimes oklch); webgpu-brush parses
 * hex and rgb(). Resolve through the element's computed style, then through a
 * 2D canvas so any CSS color space comes out as sRGB bytes.
 */
function cssColorToRgb(el: HTMLElement, color: string): string {
  el.style.color = color
  const computed = getComputedStyle(el).color
  colorProbe ??= document.createElement('canvas').getContext('2d', { willReadFrequently: true })
  if (!colorProbe) return computed
  colorProbe.clearRect(0, 0, 1, 1)
  colorProbe.fillStyle = computed
  colorProbe.fillRect(0, 0, 1, 1)
  const [r, g, b] = colorProbe.getImageData(0, 0, 1, 1).data
  return `rgb(${r}, ${g}, ${b})`
}
