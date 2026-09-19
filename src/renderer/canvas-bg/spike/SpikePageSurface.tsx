import { useCallback, useEffect, useLayoutEffect, useMemo, useRef } from 'react'
import type { PageFrameMeta } from '../../../shared/page-frames'
import type { PageSurfaceArm } from '../../../shared/page-surface-spike'
import { itemGeometry, type ItemGeometry } from '../../shared/chromeItemDraw'
import type { CanvasItemDraw } from '../canvasItemDrawOrder'
import { useFramePaintPacing } from '../useFramePaintPacing'
import { createRawWebGpuRenderer } from './rawWebGpuRenderer'
import type { SpikePageDraw, SpikePageRenderer, SpikePageRendererFactory } from './spikeRenderer'
import { createThreeRenderer } from './threeRenderer'
import { pruneVideoFrames, useVideoFrameStore } from './useVideoFrameStore'

/** Live counters the measurement harness reads off `window`, mutated in
 *  place rather than carried in React state — nothing here should trigger a
 *  render. */
interface PageSurfaceSpikeStats {
  arm: Exclude<PageSurfaceArm, '2d'>
  ready: boolean
  error: string | null
  framesArrived: number
  paints: number
  pagesDrawnLastPaint: number
  heldFrames: number
  lastPaintMs: number
}

declare global {
  interface Window {
    __pageSurfaceSpike?: PageSurfaceSpikeStats
  }
}

function rendererFactoryFor(arm: Exclude<PageSurfaceArm, '2d'>): SpikePageRendererFactory {
  switch (arm) {
    case 'webgpu-import':
      return createRawWebGpuRenderer('import')
    case 'webgpu-blit':
      return createRawWebGpuRenderer('blit')
    case 'three':
      return createThreeRenderer
  }
}

/**
 * Whether nothing a page's content can land inside a `width` × `height`
 * canvas. Unlike `itemOffCanvas` in `CanvasItemSurface`, this culls the
 * content rect only — the spike draws no chrome or bezel shadow, so it needs
 * no paint reach.
 */
function itemContentOffCanvas(g: ItemGeometry, width: number, height: number): boolean {
  return (
    g.contentX + g.contentW < 0 ||
    g.contentY + g.contentH < 0 ||
    g.contentX > width ||
    g.contentY > height
  )
}

/**
 * The WebGPU/three measurement spike's page surface (throwaway; see ADR
 * 0038's territory). Stacks above `CanvasItemSurface`, drawing each page's
 * content from its held VideoFrame through an arm-specific renderer, while
 * `CanvasItemSurface` keeps drawing shells and borders only (the preload
 * sends it no bitmaps once an arm is active). Paint scheduling mirrors
 * `CanvasItemSurface` exactly so the arms are comparable under identical
 * pacing.
 */
export function SpikePageSurface({
  arm,
  draws,
}: {
  arm: Exclude<PageSurfaceArm, '2d'>
  draws: CanvasItemDraw[]
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const rendererRef = useRef<SpikePageRenderer | null>(null)
  const scheduledPaint = useRef(0)
  const paintRef = useRef(() => {})
  const inputsRef = useRef({ draws })
  inputsRef.current = { draws }
  const statsRef = useRef<PageSurfaceSpikeStats>({
    arm,
    ready: false,
    error: null,
    framesArrived: 0,
    paints: 0,
    pagesDrawnLastPaint: 0,
    heldFrames: 0,
    lastPaintMs: 0,
  })

  useEffect(() => {
    window.__pageSurfaceSpike = statsRef.current
    return () => {
      if (window.__pageSurfaceSpike === statsRef.current) delete window.__pageSurfaceSpike
    }
  }, [])

  const requestPaint = useCallback(() => {
    if (scheduledPaint.current) return
    scheduledPaint.current = requestAnimationFrame(() => {
      scheduledPaint.current = 0
      paintRef.current()
    })
  }, [])

  const { requestFramePaint, notePaint } = useFramePaintPacing(requestPaint)

  const handleFrame = useCallback(
    (pageId: string, frame: VideoFrame, meta: PageFrameMeta) => {
      statsRef.current.framesArrived += 1
      rendererRef.current?.frameArrived(pageId, frame, meta)
      requestFramePaint(meta.frameRate)
    },
    [requestFramePaint],
  )

  const store = useVideoFrameStore(handleFrame)

  const paint = useCallback(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    // Resizing a WebGPU canvas is not free, so the backing store is only
    // touched when its CSS box actually changed.
    const dpr = window.devicePixelRatio || 1
    const clientWidth = canvas.clientWidth
    const clientHeight = canvas.clientHeight
    const targetWidth = Math.round(clientWidth * dpr)
    const targetHeight = Math.round(clientHeight * dpr)
    if (canvas.width !== targetWidth) canvas.width = targetWidth
    if (canvas.height !== targetHeight) canvas.height = targetHeight
    notePaint()

    const stats = statsRef.current
    stats.paints += 1
    stats.heldFrames = store.size

    const renderer = rendererRef.current
    if (!renderer || targetWidth === 0 || targetHeight === 0) {
      stats.pagesDrawnLastPaint = 0
      return
    }

    const list: SpikePageDraw[] = []
    for (const draw of inputsRef.current.draws) {
      if (!draw.pageId) continue
      const entry = store.get(draw.pageId)
      if (!entry) continue
      const g = itemGeometry(draw.item)
      if (itemContentOffCanvas(g, clientWidth, clientHeight)) continue
      list.push({
        pageId: draw.pageId,
        frame: entry.frame,
        meta: entry.meta,
        x: g.contentX * dpr,
        y: g.contentY * dpr,
        width: g.contentW * dpr,
        height: g.contentH * dpr,
        radius: g.innerRadius * dpr,
      })
    }

    const start = performance.now()
    renderer.draw(list, targetWidth, targetHeight)
    stats.lastPaintMs = performance.now() - start
    stats.pagesDrawnLastPaint = list.length
  }, [notePaint, store])
  paintRef.current = paint

  // Create the arm's renderer once per mount and dispose it on unmount. The
  // factory is async (WebGPU adapter/device acquisition), so a surface that
  // unmounts before it resolves must still dispose the renderer it hands back.
  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    let disposed = false
    rendererFactoryFor(arm)(canvas)
      .then((renderer) => {
        if (disposed) {
          renderer.dispose()
          return
        }
        rendererRef.current = renderer
        statsRef.current.ready = true
        requestPaint()
      })
      .catch((error: unknown) => {
        const message = error instanceof Error ? error.message : String(error)
        statsRef.current.error = message
        console.error(`[canvas-bg] spike renderer (${arm}) failed to initialize`, error)
      })
    return () => {
      disposed = true
      rendererRef.current?.dispose()
      rendererRef.current = null
    }
  }, [arm, requestPaint])

  // A pan moves every item but changes no page's membership, so the prune
  // below keys on the ids rather than on each new draw list.
  const pageIdsKey = useMemo(
    () => draws.flatMap((draw) => (draw.pageId ? [draw.pageId] : [])).join('\n'),
    [draws],
  )

  // `CanvasItemSurface` still mounts and drives `requestPageFrames`; this
  // surface only needs to release textures for pages that left the scene.
  useEffect(() => {
    const pageIds = new Set(pageIdsKey ? pageIdsKey.split('\n') : [])
    pruneVideoFrames(store, pageIds, (pageId) => rendererRef.current?.pageRemoved(pageId))
  }, [store, pageIdsKey])

  useEffect(() => () => {
    pruneVideoFrames(store, new Set(), (pageId) => rendererRef.current?.pageRemoved(pageId))
  }, [store])

  useLayoutEffect(() => {
    // This paint already shows every frame that has arrived.
    cancelAnimationFrame(scheduledPaint.current)
    scheduledPaint.current = 0
    paint()
  }, [draws, paint])

  useEffect(() => {
    window.addEventListener('resize', requestPaint)
    return () => {
      window.removeEventListener('resize', requestPaint)
      cancelAnimationFrame(scheduledPaint.current)
      scheduledPaint.current = 0
    }
  }, [requestPaint])

  return (
    <canvas
      ref={canvasRef}
      aria-hidden
      data-spike-page-surface
      className="pointer-events-none absolute inset-0 h-full w-full"
    />
  )
}
