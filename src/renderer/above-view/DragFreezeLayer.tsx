import { useCallback, useEffect, useRef, useState } from 'react'
import type { CanvasScenePageEntity, FrozenPagesState, LayoutUpdateData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import type { SceneCameraTransform } from '../../shared/scene-camera-transform'
import {
  drawItemBorders,
  drawItemShell,
  drawItemSnapshot,
  liveGeometry,
  readChromeColors,
  type ChromeCanvasItem,
} from '../shared/chromeItemDraw'
import { useFrozenPageBitmaps } from '../shared/useFrozenPageBitmaps'

const EMPTY_FROZEN_STATE: FrozenPagesState = { revision: 0, target: 'above', active: false, frames: [] }

function toChromeItem(page: CanvasScenePageEntity): ChromeCanvasItem {
  return {
    id: page.id,
    screenX: page.screenX,
    screenY: page.screenY,
    screenWidth: page.screenWidth,
    screenHeight: page.screenHeight,
    contentScreenX: page.contentScreenX,
    contentScreenY: page.contentScreenY,
    contentScreenWidth: page.contentScreenWidth,
    contentScreenHeight: page.contentScreenHeight,
    deviceId: page.deviceId,
    deviceOrientation: page.deviceOrientation,
    showDeviceFrame: page.showDeviceFrame,
    useSvgDeviceShell: page.useSvgDeviceShell,
    width: page.width,
  }
}

/**
 * Draws the drag-frozen page(s): the bitmap `capturePageFrame` took at
 * drag-start plus the chrome shared with canvas-bg's persistent pass, at
 * the page's current layout position. Owns these pages entirely while
 * frozen — canvas-bg skips them (see `chromeCanvasDraw.ts`'s
 * `dragFrozenPageIds`) so the two renderers never draw the same page twice.
 */
export function DragFreezeLayer({
  api,
  layoutRef,
  transform,
  isDark,
}: {
  api: CanvasBgElectronAPI
  layoutRef: React.MutableRefObject<LayoutUpdateData>
  transform: SceneCameraTransform
  isDark: boolean
}) {
  const [frozenState, setFrozenState] = useState<FrozenPagesState>(EMPTY_FROZEN_STATE)
  useEffect(
    () =>
      api.onFrozenPagesState((data) => {
        if (data.target === 'above') setFrozenState(data)
      }),
    [api],
  )
  const bitmaps = useFrozenPageBitmaps(frozenState, (revision) => api.frozenPagesReady('above', revision))

  const canvasRef = useRef<HTMLCanvasElement>(null)

  const draw = useCallback(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const dpr = Math.max(window.devicePixelRatio || 1, 1)
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
    if (!frozenState.active || bitmaps.size === 0) return

    const { borderColor, bezelColor } = readChromeColors(canvas)
    // layoutData is the authority on where a dragged page is: grid and
    // alignment snapping resolve in main, and the selection handles follow
    // the same data, so the raster lands exactly where they do. Its screen
    // coords are window-relative; aboveView sits canvasOrigin.y down the
    // window with a camera built at y: 0, so y is rebased here (as
    // PageFocusRingLayer does).
    const layout = layoutRef.current
    const originY = layout.canvasOrigin.y
    for (const entity of layout.entities) {
      if (entity.kind !== 'page') continue
      const bitmap = bitmaps.get(entity.id)
      if (!bitmap || bitmap.width === 0) continue
      const item = toChromeItem(entity)
      item.screenY -= originY
      if (item.contentScreenY !== undefined) item.contentScreenY -= originY
      const geometry = liveGeometry(item, transform)
      // above-view owns the whole page while it is frozen (canvas-bg skips
      // its chrome canvas and SVG shell for it), so the shell is drawn here
      // regardless of the page's usual SVG-vs-canvas toggle.
      drawItemBorders(ctx, geometry, borderColor, dpr, !!item.showDeviceFrame)
      if (item.showDeviceFrame) {
        drawItemShell(ctx, item, geometry, isDark, bezelColor, dpr)
      }
      drawItemSnapshot(ctx, geometry, bitmap)
    }
  }, [frozenState.active, transform, isDark, bitmaps, layoutRef])

  // rAF loop only while a freeze is active: layoutRef mutates outside React,
  // so this is how the raster tracks each layout update without a
  // re-render per tick.
  useEffect(() => {
    if (!frozenState.active) {
      draw()
      return
    }
    let raf = 0
    const tick = () => {
      draw()
      raf = requestAnimationFrame(tick)
    }
    raf = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(raf)
  }, [frozenState.active, draw])

  useEffect(() => {
    window.addEventListener('resize', draw)
    return () => window.removeEventListener('resize', draw)
  }, [draw])

  return (
    <canvas
      ref={canvasRef}
      aria-hidden="true"
      data-drag-freeze-canvas
      className="pointer-events-none absolute inset-0 h-full w-full"
    />
  )
}
