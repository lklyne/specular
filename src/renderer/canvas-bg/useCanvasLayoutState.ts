import { useEffect, useRef, useState } from 'react'
import type { LayoutUpdateData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'

export function useCanvasLayoutState({
  api,
  initialLayoutData,
}: {
  api: CanvasBgElectronAPI
  initialLayoutData: LayoutUpdateData
}) {
  const layoutRef = useRef<LayoutUpdateData>(initialLayoutData)
  const [layoutData, setLayoutData] = useState<LayoutUpdateData>(initialLayoutData)
  const [layoutTick, setLayoutTick] = useState(0)

  useEffect(() => {
    // During a fast zoom the main process emits a payload per tick, faster than
    // one per frame. Keep the ref current synchronously (gesture logic reads it
    // on every pointer event) but coalesce the React render to one per animation
    // frame, so a burst of payloads collapses into a single re-render (#265).
    let pending: LayoutUpdateData | null = null
    let raf = 0
    const flush = () => {
      raf = 0
      if (!pending) return
      setLayoutData(pending)
      setLayoutTick((current) => current + 1)
      pending = null
    }
    const cleanup = api.onLayoutUpdate((data) => {
      layoutRef.current = data
      pending = data
      if (!raf) raf = requestAnimationFrame(flush)
    })
    return () => {
      cleanup()
      if (raf) cancelAnimationFrame(raf)
    }
  }, [api])

  return {
    layoutData,
    layoutRef,
    layoutTick,
  }
}
