import { useEffect, useState } from 'react'
import type { ProjectedLayoutData } from '../../shared/scene-projection'
import type { LayoutUpdateData } from '../../shared/types'
import { useProjectedLayoutRef } from '../shared/hooks/useProjectedLayoutRef'
import { runtimeStore } from '../shared/runtime-store'
import { projectLayoutData } from '../shared/scene-projection'

export function useCanvasLayoutState({
  initialLayoutData,
}: {
  initialLayoutData: LayoutUpdateData
}) {
  const layoutRef = useProjectedLayoutRef()
  const [layoutData, setLayoutData] = useState<ProjectedLayoutData>(() =>
    projectLayoutData(initialLayoutData),
  )
  const [layoutTick, setLayoutTick] = useState(0)

  useEffect(() => {
    // During a fast zoom the main process emits a payload per tick, faster than
    // one per frame, so the React render coalesces to one per animation frame
    // and a burst of payloads collapses into a single re-render (#265).
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
