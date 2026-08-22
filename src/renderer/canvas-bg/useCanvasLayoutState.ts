import { useEffect, useRef, useState } from 'react'
import type { LayoutUpdateData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { runtimeStore } from '../shared/runtime-store'

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
    let raf = 0
    let rendered: LayoutUpdateData | null = null
    const flush = () => {
      raf = 0
      const next = runtimeStore.readLayoutData()
      if (next === rendered) return
      rendered = next
      setLayoutData(next)
      setLayoutTick((current) => current + 1)
    }
    const offSnapshot = api.onLayoutUpdate((data) => runtimeStore.applySnapshot(data))
    const offPatches = api.onRuntimePatch((batch) => runtimeStore.applyPatches(batch))
    const unsubscribe = runtimeStore.subscribe(() => {
      layoutRef.current = runtimeStore.readLayoutData()
      if (!raf) raf = requestAnimationFrame(flush)
    })
    return () => {
      offSnapshot()
      offPatches()
      unsubscribe()
      if (raf) cancelAnimationFrame(raf)
    }
  }, [api])

  return {
    layoutData,
    layoutRef,
    layoutTick,
  }
}
