import type { LayoutSnapshotRef } from '../shared/hooks/useProjectedLayoutRef'
import type { ProjectedLayoutData, ProjectedPageEntity } from '../../shared/scene-projection'
import { memo, useCallback, useEffect, useRef, useState } from 'react'
import type { FrozenPagesState } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import {
  drawItemBorders,
  drawItemShell,
  drawItemSnapshot,
  itemGeometry,
  readChromeColors,
  type ChromeCanvasItem,
} from '../shared/chromeItemDraw'
import { useFrozenPageBitmaps } from '../shared/useFrozenPageBitmaps'
import { prepareScreenCanvas } from '../shared/screenCanvas'

const EMPTY_FROZEN_STATE: FrozenPagesState = { revision: 0, target: 'above', active: false, frames: [] }

function toChromeItem(page: ProjectedPageEntity): ChromeCanvasItem {
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
 * Draws each drag-frozen page at its current layout position: the bitmap
 * `capturePageFrame` took at drag start plus the chrome canvas-bg would
 * otherwise draw. canvas-bg skips these pages (`dragFrozenPageIds` in
 * `chromeCanvasDraw.ts`) so no page is drawn twice.
 */
export const DragFreezeLayer = memo(function DragFreezeLayer({
  api,
  layoutRef,
  isDark,
}: {
  api: CanvasBgElectronAPI
  layoutRef: LayoutSnapshotRef
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
    const prepared = prepareScreenCanvas(canvas, window.devicePixelRatio)
    if (!prepared) return
    const { ctx, dpr } = prepared
    if (!frozenState.active || bitmaps.size === 0) return

    const { borderColor, bezelColor } = readChromeColors(canvas)
    // Grid and alignment snapping resolve in main, so layoutData is the
    // only position that carries them; the selection handles read the same
    // data. Its screen coords are window-relative, and aboveView sits
    // canvasOrigin.y down the window with a camera built at y: 0, so y is
    // rebased here as PageFocusRingLayer does.
    const layout = layoutRef.current
    const originY = layout.canvasOrigin.y
    for (const entity of layout.entities) {
      if (entity.kind !== 'page') continue
      const bitmap = bitmaps.get(entity.id)
      if (!bitmap || bitmap.width === 0) continue
      const item = toChromeItem(entity)
      item.screenY -= originY
      if (item.contentScreenY !== undefined) item.contentScreenY -= originY
      const geometry = itemGeometry(item)
      // canvas-bg skips both its chrome canvas and SVG shell for a frozen
      // page, so the shell is drawn here whatever the page's usual toggle.
      if (item.showDeviceFrame) {
        drawItemShell(ctx, item, geometry, isDark, bezelColor, borderColor, dpr)
      } else {
        drawItemBorders(ctx, geometry, borderColor, dpr)
      }
      drawItemSnapshot(ctx, geometry, bitmap)
    }
  }, [frozenState.active, isDark, bitmaps, layoutRef])

  // layoutRef mutates outside React, so a rAF loop (only while frozen) is
  // how the raster tracks each layout update without a re-render per tick.
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
})
