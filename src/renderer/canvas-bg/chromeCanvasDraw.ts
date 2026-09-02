/**
 * canvas-bg's persistent chrome pass: every page/file item's borders,
 * device shell, and frozen-page raster, drawn in one full-window canvas.
 * Per-item geometry and draw calls live in `../shared/chromeItemDraw`,
 * shared with above-view's drag-freeze layer.
 */
import {
  drawItemChrome,
  drawItemSnapshot,
  itemGeometry,
  readChromeColors,
  type ChromeCanvasItem,
} from '../shared/chromeItemDraw'
import { prepareScreenCanvas } from '../shared/screenCanvas'

export type { ChromeCanvasItem } from '../shared/chromeItemDraw'

export function drawChromeCanvas({
  canvas,
  pages,
  fileEntities,
  snapshots,
  isDark,
  devicePixelRatio,
  dragFrozenPageIds,
}: {
  canvas: HTMLCanvasElement
  pages: ChromeCanvasItem[]
  fileEntities: ChromeCanvasItem[]
  /** Frozen-page rasters by page id; a page with no entry draws no raster. */
  snapshots: ReadonlyMap<string, ImageBitmap>
  isDark: boolean
  devicePixelRatio: number
  /** Pages drag-frozen on above-view, which draws their chrome and raster
   *  for the duration of the drag; this pass must not draw them underneath. */
  dragFrozenPageIds?: ReadonlySet<string>
}): void {
  const prepared = prepareScreenCanvas(canvas, devicePixelRatio)
  if (!prepared) return
  const { ctx, dpr } = prepared

  const { borderColor, bezelColor } = readChromeColors(canvas)

  const visiblePages =
    dragFrozenPageIds && dragFrozenPageIds.size > 0
      ? pages.filter((p) => !dragFrozenPageIds.has(p.id))
      : pages

  const borderItems = visiblePages.filter((p) => !p.showDeviceFrame)
  const shellItems = [
    // The SVG shell layer owns its pages entirely (borders included).
    ...visiblePages.filter((p) => p.showDeviceFrame && !p.useSvgDeviceShell),
    ...fileEntities.filter((f) => f.showDeviceFrame),
  ]

  // Chrome in native stacking order — plain-page borders, then shells — with
  // the page rasters standing in for the live views on top.
  for (const item of [...borderItems, ...shellItems]) {
    drawItemChrome(ctx, item, itemGeometry(item), isDark, bezelColor, borderColor, dpr)
  }
  if (snapshots.size > 0) {
    for (const page of visiblePages) {
      const bitmap = snapshots.get(page.id)
      // A closed bitmap reports zero size; drawing it throws.
      if (bitmap && bitmap.width > 0) drawItemSnapshot(ctx, itemGeometry(page), bitmap)
    }
  }
}
