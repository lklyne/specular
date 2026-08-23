import { useEffect, useRef, useState } from 'react'
import type { ProjectedLayoutData } from '../../shared/scene-projection'
import type { LayoutUpdateData } from '../../shared/types'
import { runtimeStore } from '../shared/runtime-store'
import { projectLayoutData } from '../shared/scene-projection'

export function useCanvasLayoutState({
  initialLayoutData,
}: {
  initialLayoutData: LayoutUpdateData
}) {
  const initial = projectLayoutData(initialLayoutData)
  const layoutRef = useRef<ProjectedLayoutData>(initial)
  const [layoutData, setLayoutData] = useState<ProjectedLayoutData>(initial)
  const [layoutTick, setLayoutTick] = useState(0)

  useEffect(() => {
    // During a fast zoom the main process emits a payload per tick, faster than
    // one per frame. Keep the ref current synchronously (gesture logic reads it
    // on every pointer event) but coalesce the React render to one per animation
    // frame, so a burst of payloads collapses into a single re-render (#265).
    let raf = 0
    let rendered: ProjectedLayoutData | null = null
    const flush = () => {
      raf = 0
      const next = projectLayoutData(runtimeStore.readLayoutData())
      if (next === rendered) return
      rendered = next
      setLayoutData(next)
      setLayoutTick((current) => current + 1)
    }
    const unsubscribe = runtimeStore.subscribe(() => {
      layoutRef.current = projectLayoutData(runtimeStore.readLayoutData())
      if (!raf) raf = requestAnimationFrame(flush)
    })
    return () => {
      unsubscribe()
      if (raf) cancelAnimationFrame(raf)
    }
  }, [])

  return {
    layoutData,
    layoutRef,
    layoutTick,
  }
}
