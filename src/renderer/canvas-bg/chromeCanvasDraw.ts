/**
 * canvas-bg's persistent chrome pass: every page/file item's borders,
 * device shell, and frozen-page raster, drawn in one full-window canvas.
 * Per-item geometry and draw calls live in `../shared/chromeItemDraw`,
 * shared with above-view's drag-freeze layer.
 */
import {
  drawItemBorders,
  drawItemShell,
  drawItemSnapshot,
  liveGeometry,
  readChromeColors,
  type ChromeCanvasItem,
} from '../shared/chromeItemDraw'
import type { SceneCameraTransform } from '../../shared/scene-camera-transform'

export type { ChromeCanvasItem } from '../shared/chromeItemDraw'

export function drawChromeCanvas({
  canvas,
  pages,
  fileEntities,
  snapshots,
  transform,
  isDark,
  devicePixelRatio,
  dragFrozenPageIds,
}: {
  canvas: HTMLCanvasElement
  pages: ChromeCanvasItem[]
  fileEntities: ChromeCanvasItem[]
  /** Frozen-page rasters by page id; a page with no entry draws no raster. */
  snapshots: ReadonlyMap<string, ImageBitmap>
  transform: SceneCameraTransform
  isDark: boolean
  devicePixelRatio: number
  /** Pages drag-frozen on above-view, which draws their chrome and raster
   *  for the duration of the drag; this pass must not draw them underneath. */
  dragFrozenPageIds?: ReadonlySet<string>
}): void {
  const dpr = Math.max(devicePixelRatio, 1)
  const width = canvas.clientWidth
  const height = canvas.clientHeight
  const targetWidth = Math.max(1, Math.ceil(width * dpr))
  const targetHeight = Math.max(1, Math.ceil(height * dpr))
  if (canvas.width !== targetWidth) canvas.width = targetWidth
  if (canvas.height !== targetHeight) canvas.height = targetHeight

  const ctx = canvas.getContext('2d')
  if (!ctx) return
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  ctx.clearRect(0, 0, width, height)

  const { borderColor, bezelColor } = readChromeColors(canvas)

  const visiblePages =
    dragFrozenPageIds && dragFrozenPageIds.size > 0
      ? pages.filter((p) => !dragFrozenPageIds.has(p.id))
      : pages

  const framedFiles = fileEntities.filter((f) => f.showDeviceFrame)
  // The SVG shell layer owns its pages entirely (borders included).
  const borderItems = [
    ...visiblePages.filter((p) => !(p.showDeviceFrame && p.useSvgDeviceShell)),
    ...framedFiles,
  ]
  const shellItems = [
    ...visiblePages.filter((p) => p.showDeviceFrame && !p.useSvgDeviceShell),
    ...framedFiles,
  ]

  // Three passes matching the native stacking order: all borders, then all
  // shells (the bezel fill covers the inner border ring on shell pages), then
  // the page rasters standing in for the live views on top.
  for (const item of borderItems) {
    drawItemBorders(ctx, liveGeometry(item, transform), borderColor, dpr, !!item.showDeviceFrame)
  }
  for (const item of shellItems) {
    drawItemShell(ctx, item, liveGeometry(item, transform), isDark, bezelColor, dpr)
  }
  if (snapshots.size > 0) {
    for (const page of visiblePages) {
      const bitmap = snapshots.get(page.id)
      // A closed bitmap reports zero size; drawing it throws.
      if (bitmap && bitmap.width > 0) drawItemSnapshot(ctx, liveGeometry(page, transform), bitmap)
    }
  }
}
