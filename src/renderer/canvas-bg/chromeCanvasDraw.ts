/**
 * canvas-bg's persistent chrome pass: every page/file item's borders and
 * device shell, drawn in one full-window canvas. Per-item geometry and draw
 * calls live in `../shared/chromeItemDraw`.
 */
import {
  drawItemChrome,
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
  isDark,
  devicePixelRatio,
}: {
  canvas: HTMLCanvasElement
  pages: ChromeCanvasItem[]
  fileEntities: ChromeCanvasItem[]
  isDark: boolean
  devicePixelRatio: number
}): void {
  const prepared = prepareScreenCanvas(canvas, devicePixelRatio)
  if (!prepared) return
  const { ctx, dpr } = prepared

  const { borderColor, bezelColor } = readChromeColors(canvas)

  const borderItems = pages.filter((p) => !p.showDeviceFrame)
  const shellItems = [
    // The SVG shell layer owns its pages entirely (borders included).
    ...pages.filter((p) => p.showDeviceFrame && !p.useSvgDeviceShell),
    ...fileEntities.filter((f) => f.showDeviceFrame),
  ]

  // Chrome in native stacking order — plain-page borders, then shells.
  // `PageTextureSurface` draws each page's texture on top of this.
  for (const item of [...borderItems, ...shellItems]) {
    drawItemChrome(ctx, item, itemGeometry(item), isDark, bezelColor, borderColor, dpr)
  }
}
