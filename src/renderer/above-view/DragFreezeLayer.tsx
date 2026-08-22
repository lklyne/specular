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
import { pageDragDelta } from './optionDragCopy'

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
 * drag-start plus the chrome shared with canvas-bg's persistent pass,
 * offset by the pointer's live delta. Owns these pages entirely while
 * frozen — canvas-bg skips them (see `chromeCanvasDraw.ts`'s
 * `dragFrozenPageIds`) so the two renderers never draw the same page twice.
 *
 * The base geometry is a one-time snapshot taken when a page enters the
 * frozen set, then only ever offset by the total pointer delta — not
 * re-read from `layoutData` on every tick. `layoutData` for a drag-frozen
 * page keeps updating too (the drag's `dragPage` IPC round trip still
 * mutates the real entity), but that path is debounced and lags the
 * pointer by a tick; riding the drag-start snapshot + the synchronous local
 * delta (`pageDragDelta`, updated by `optionDragCopy.ts` on every
 * pointermove with no IPC) is what keeps the raster glued to the cursor at
 * rAF rate instead of trailing it.
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
  const baseGeometryRef = useRef<Map<string, ChromeCanvasItem>>(new Map())

  // Snapshot each frozen page's chrome geometry once, the instant it enters
  // the frozen set — not on every layoutData tick, or the raster would ride
  // the same debounced position it exists to avoid.
  useEffect(() => {
    if (!frozenState.active || frozenState.frames.length === 0) {
      baseGeometryRef.current = new Map()
      return
    }
    const pages = layoutRef.current.entities.filter(
      (e): e is CanvasScenePageEntity => e.kind === 'page',
    )
    const next = new Map<string, ChromeCanvasItem>()
    for (const frame of frozenState.frames) {
      const page = pages.find((p) => p.id === frame.pageId)
      if (page) next.set(page.id, toChromeItem(page))
    }
    baseGeometryRef.current = next
  }, [frozenState, layoutRef])

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
    if (baseGeometryRef.current.size === 0) return

    const { borderColor, bezelColor } = readChromeColors(canvas)
    const { totalDx, totalDy } = pageDragDelta

    for (const base of baseGeometryRef.current.values()) {
      const item: ChromeCanvasItem = {
        ...base,
        screenX: base.screenX + totalDx,
        screenY: base.screenY + totalDy,
        contentScreenX: (base.contentScreenX ?? base.screenX) + totalDx,
        contentScreenY: (base.contentScreenY ?? base.screenY) + totalDy,
      }
      const geometry = liveGeometry(item, transform)
      // A drag-frozen page draws its full chrome here regardless of its
      // normal SVG-vs-canvas shell toggle — above-view owns it entirely for
      // the duration of the freeze (canvas-bg excludes it from both its
      // chrome canvas and its SvgDeviceShellLayer), so there is no second
      // pass to defer the SVG shell to.
      drawItemBorders(ctx, geometry, borderColor, dpr, !!item.showDeviceFrame)
      if (item.showDeviceFrame) {
        drawItemShell(ctx, item, geometry, isDark, bezelColor, dpr)
      }
      const bitmap = bitmaps.get(item.id)
      if (bitmap && bitmap.width > 0) drawItemSnapshot(ctx, geometry, bitmap)
    }
  }, [transform, isDark, bitmaps])

  // rAF loop only while a freeze is active — `pageDragDelta` mutates outside
  // React, so this is the only way to redraw every frame without a
  // re-render per pointermove. Gated on `frozenState.active` (a real state
  // value) rather than the geometry ref's size: the geometry-snapshot
  // effect above runs in the same commit, before this one, so the ref is
  // already populated by the time this loop's first `draw()` fires.
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
