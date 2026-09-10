import { useEffect, useRef } from 'react'
import type { MutableRefObject } from 'react'
import type { OsrLabElectronAPI } from '../../shared/electron-api/osr-lab'
import type { LabCamera, LabRect, OsrLabConfig } from '../../shared/osr-lab'
import {
  hitTestPages,
  panCamera,
  projectRect,
  visiblePageIds,
  zoomCameraAt,
} from '../../shared/osr-lab'
import type { LabFrameStore } from './useLabFrames'

export interface LabPageLayout {
  id: string
  rect: LabRect
}

export interface LabCanvasHandle {
  cameraRef: MutableRefObject<LabCamera>
  setCamera: (camera: LabCamera) => void
  requestDraw: () => void
  viewportSize: () => { width: number; height: number }
  /** Mean draw time over the last second, ms; the HUD reads it. */
  drawStats: { drawMsMean: number | null; drawsPerSecond: number }
}

interface LabCanvasProps {
  api: OsrLabElectronAPI
  config: OsrLabConfig
  pages: LabPageLayout[]
  frames: LabFrameStore
  enteredPageId: string | null
  onEnterPage: (pageId: string | null) => void
  cursor: string
  handleRef: MutableRefObject<LabCanvasHandle | null>
}

function pageLocal(camera: LabCamera, rect: LabRect, sx: number, sy: number): { x: number; y: number } {
  const projected = projectRect(camera, rect)
  return { x: (sx - projected.x) / camera.zoom, y: (sy - projected.y) / camera.zoom }
}

function buttonName(button: number): 'left' | 'middle' | 'right' {
  return button === 1 ? 'middle' : button === 2 ? 'right' : 'left'
}

/**
 * The lab's canvas: every page is its latest frame drawn at the camera's
 * projection of the page rect. Pointer input over the entered page is
 * forwarded in page-local CSS coordinates; everywhere else it drives the
 * camera. Keyboard reaches the entered page through the hidden sink input.
 */
export function LabCanvas({
  api,
  config,
  pages,
  frames,
  enteredPageId,
  onEnterPage,
  cursor,
  handleRef,
}: LabCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null)
  const sinkRef = useRef<HTMLInputElement | null>(null)
  const cameraRef = useRef<LabCamera>({ x: 60, y: 60, zoom: 0.25 })
  const dirtyRef = useRef(true)
  const pagesRef = useRef(pages)
  pagesRef.current = pages
  const enteredRef = useRef(enteredPageId)
  enteredRef.current = enteredPageId
  const configRef = useRef(config)
  configRef.current = config
  const drawStats = useRef({ drawMsMean: null as number | null, drawsPerSecond: 0 })
  const visibleKeyRef = useRef('')

  const requestDraw = () => {
    dirtyRef.current = true
  }

  const publishVisible = () => {
    const canvas = canvasRef.current
    if (!canvas) return
    const ids = visiblePageIds(cameraRef.current, pagesRef.current, {
      width: canvas.clientWidth,
      height: canvas.clientHeight,
    })
    const key = ids.join(',')
    if (key === visibleKeyRef.current) return
    visibleKeyRef.current = key
    api.setVisiblePages(ids)
  }

  const setCamera = (camera: LabCamera) => {
    cameraRef.current = camera
    dirtyRef.current = true
  }

  handleRef.current = {
    cameraRef,
    setCamera,
    requestDraw,
    viewportSize: () => ({
      width: canvasRef.current?.clientWidth ?? 0,
      height: canvasRef.current?.clientHeight ?? 0,
    }),
    drawStats: drawStats.current,
  }

  // Draw loop. Draws only when something changed; the cadence is rAF.
  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    let raf = 0
    let windowDraws = 0
    let windowDrawMs = 0
    let lastVisiblePublish = 0
    const tick = setInterval(() => {
      drawStats.current.drawsPerSecond = windowDraws
      drawStats.current.drawMsMean = windowDraws > 0 ? windowDrawMs / windowDraws : null
      windowDraws = 0
      windowDrawMs = 0
    }, 1000)

    const draw = (now: number) => {
      raf = requestAnimationFrame(draw)
      if (!dirtyRef.current) return
      dirtyRef.current = false
      const started = performance.now()
      const dpr = window.devicePixelRatio || 1
      const width = canvas.clientWidth
      const height = canvas.clientHeight
      if (canvas.width !== Math.round(width * dpr) || canvas.height !== Math.round(height * dpr)) {
        canvas.width = Math.round(width * dpr)
        canvas.height = Math.round(height * dpr)
      }
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
      ctx.fillStyle = '#111114'
      ctx.fillRect(0, 0, width, height)
      const camera = cameraRef.current
      ctx.imageSmoothingEnabled = true
      ctx.imageSmoothingQuality = 'medium'
      const dsf = configRef.current.deviceScaleFactor || 1
      for (const page of pagesRef.current) {
        const rect = projectRect(camera, page.rect)
        if (rect.x + rect.width < 0 || rect.y + rect.height < 0 || rect.x > width || rect.y > height) continue
        const frame = frames.frames.get(page.id)
        if (frame) {
          ctx.drawImage(frame.bitmap, rect.x, rect.y, rect.width, rect.height)
        } else {
          ctx.fillStyle = '#27272a'
          ctx.fillRect(rect.x, rect.y, rect.width, rect.height)
          ctx.fillStyle = '#71717a'
          ctx.font = '12px system-ui'
          ctx.fillText('waiting for first frame…', rect.x + 8, rect.y + 18)
        }
        const popup = frames.popups.get(page.id)
        if (popup) {
          // Electron reports no position for popup widgets; draw at the page
          // origin so the test shows whether the texture arrives at all.
          const w = (popup.bitmap.width / dsf) * camera.zoom
          const h = (popup.bitmap.height / dsf) * camera.zoom
          ctx.drawImage(popup.bitmap, rect.x, rect.y, w, h)
          ctx.strokeStyle = '#f87171'
          ctx.lineWidth = 2
          ctx.strokeRect(rect.x, rect.y, w, h)
        }
        ctx.strokeStyle = page.id === enteredRef.current ? '#3b82f6' : '#3f3f46'
        ctx.lineWidth = page.id === enteredRef.current ? 3 : 1
        ctx.strokeRect(rect.x - 0.5, rect.y - 0.5, rect.width + 1, rect.height + 1)
      }
      const drawMs = performance.now() - started
      windowDraws++
      windowDrawMs += drawMs
      if (now - lastVisiblePublish > 100) {
        lastVisiblePublish = now
        publishVisible()
      }
    }
    raf = requestAnimationFrame(draw)
    const resize = new ResizeObserver(() => {
      dirtyRef.current = true
    })
    resize.observe(canvas)
    return () => {
      cancelAnimationFrame(raf)
      clearInterval(tick)
      resize.disconnect()
    }
  }, [frames])

  useEffect(() => {
    dirtyRef.current = true
  }, [pages, enteredPageId, config])

  // Pointer, wheel and keyboard.
  useEffect(() => {
    const canvas = canvasRef.current
    const sink = sinkRef.current
    if (!canvas || !sink) return
    let panDrag: { x: number; y: number; pointerId: number } | null = null
    let forwarding: { pageId: string; pointerId: number } | null = null

    const entered = () => {
      const id = enteredRef.current
      return id ? (pagesRef.current.find((page) => page.id === id) ?? null) : null
    }

    const forwardPointer = (event: PointerEvent, kind: 'down' | 'up' | 'move', page: LabPageLayout) => {
      const local = pageLocal(cameraRef.current, page.rect, event.clientX, event.clientY)
      api.forwardPointer({
        pageId: page.id,
        kind,
        x: local.x,
        y: local.y,
        button: buttonName(event.button < 0 ? 0 : event.button),
        buttons: event.buttons,
        clickCount: kind === 'move' ? 0 : Math.max(1, event.detail),
        shiftKey: event.shiftKey,
        ctrlKey: event.ctrlKey,
        altKey: event.altKey,
        metaKey: event.metaKey,
      })
    }

    const onPointerDown = (event: PointerEvent) => {
      const camera = cameraRef.current
      const hit = hitTestPages(camera, pagesRef.current.map((page) => page.rect), event.clientX, event.clientY)
      const page = hit >= 0 ? pagesRef.current[hit] : null
      if (event.button === 1 || (event.button === 0 && event.altKey) || !page) {
        if (page === null && event.button === 0 && enteredRef.current) onEnterPage(null)
        panDrag = { x: event.clientX, y: event.clientY, pointerId: event.pointerId }
        canvas.setPointerCapture(event.pointerId)
        event.preventDefault()
        return
      }
      if (page.id !== enteredRef.current) {
        onEnterPage(page.id)
        sink.focus()
        event.preventDefault()
        return
      }
      forwarding = { pageId: page.id, pointerId: event.pointerId }
      canvas.setPointerCapture(event.pointerId)
      sink.focus()
      forwardPointer(event, 'down', page)
      event.preventDefault()
    }

    const onPointerMove = (event: PointerEvent) => {
      if (panDrag && event.pointerId === panDrag.pointerId) {
        setCamera(panCamera(cameraRef.current, event.clientX - panDrag.x, event.clientY - panDrag.y))
        panDrag = { ...panDrag, x: event.clientX, y: event.clientY }
        return
      }
      const page = entered()
      if (!page) return
      if (forwarding && event.pointerId !== forwarding.pointerId) return
      const hit = hitTestPages(cameraRef.current, [page.rect], event.clientX, event.clientY)
      if (hit < 0 && !forwarding) return
      forwardPointer(event, 'move', page)
    }

    const onPointerUp = (event: PointerEvent) => {
      if (panDrag && event.pointerId === panDrag.pointerId) {
        panDrag = null
        canvas.releasePointerCapture(event.pointerId)
        return
      }
      if (forwarding && event.pointerId === forwarding.pointerId) {
        const page = entered()
        if (page) forwardPointer(event, 'up', page)
        forwarding = null
        canvas.releasePointerCapture(event.pointerId)
      }
    }

    const onWheel = (event: WheelEvent) => {
      event.preventDefault()
      const camera = cameraRef.current
      if (event.metaKey || event.ctrlKey) {
        setCamera(zoomCameraAt(camera, event.deltaY, event.clientX, event.clientY))
        return
      }
      const page = entered()
      if (page && hitTestPages(camera, [page.rect], event.clientX, event.clientY) >= 0) {
        const local = pageLocal(camera, page.rect, event.clientX, event.clientY)
        api.forwardWheel({
          pageId: page.id,
          x: local.x,
          y: local.y,
          deltaX: event.deltaX,
          deltaY: event.deltaY,
          hasPreciseScrollingDeltas: event.deltaMode === WheelEvent.DOM_DELTA_PIXEL,
          shiftKey: event.shiftKey,
          ctrlKey: event.ctrlKey,
          altKey: event.altKey,
          metaKey: event.metaKey,
        })
        return
      }
      setCamera(panCamera(camera, -event.deltaX, -event.deltaY))
    }

    const onContextMenu = (event: Event) => event.preventDefault()

    const keyPayload = (event: KeyboardEvent, kind: 'down' | 'up', pageId: string) => ({
      pageId,
      kind,
      key: event.key,
      code: event.code,
      text:
        kind === 'down' && !event.ctrlKey && !event.metaKey && event.key.length === 1
          ? event.key
          : kind === 'down' && event.key === 'Enter'
            ? '\r'
            : null,
      repeat: event.repeat,
      shiftKey: event.shiftKey,
      ctrlKey: event.ctrlKey,
      altKey: event.altKey,
      metaKey: event.metaKey,
    })

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onEnterPage(null)
        sink.blur()
        return
      }
      if (event.isComposing || event.keyCode === 229) return
      const page = entered()
      if (!page) return
      event.preventDefault()
      api.forwardKey(keyPayload(event, 'down', page.id))
    }
    const onKeyUp = (event: KeyboardEvent) => {
      if (event.isComposing) return
      const page = entered()
      if (!page) return
      event.preventDefault()
      api.forwardKey(keyPayload(event, 'up', page.id))
    }
    const onCompositionEnd = (event: CompositionEvent) => {
      const page = entered()
      if (page && event.data) api.insertText(page.id, event.data)
      sink.value = ''
    }

    canvas.addEventListener('pointerdown', onPointerDown)
    canvas.addEventListener('pointermove', onPointerMove)
    canvas.addEventListener('pointerup', onPointerUp)
    canvas.addEventListener('pointercancel', onPointerUp)
    canvas.addEventListener('wheel', onWheel, { passive: false })
    canvas.addEventListener('contextmenu', onContextMenu)
    sink.addEventListener('keydown', onKeyDown)
    sink.addEventListener('keyup', onKeyUp)
    sink.addEventListener('compositionend', onCompositionEnd)
    return () => {
      canvas.removeEventListener('pointerdown', onPointerDown)
      canvas.removeEventListener('pointermove', onPointerMove)
      canvas.removeEventListener('pointerup', onPointerUp)
      canvas.removeEventListener('pointercancel', onPointerUp)
      canvas.removeEventListener('wheel', onWheel)
      canvas.removeEventListener('contextmenu', onContextMenu)
      sink.removeEventListener('keydown', onKeyDown)
      sink.removeEventListener('keyup', onKeyUp)
      sink.removeEventListener('compositionend', onCompositionEnd)
    }
  }, [api, onEnterPage])

  return (
    <>
      <canvas ref={canvasRef} style={{ cursor: enteredPageId ? cursor : 'default' }} />
      <input
        ref={sinkRef}
        className="lab-keyboard-sink"
        aria-label="Keyboard input for the entered page"
        autoComplete="off"
      />
    </>
  )
}
