import { useEffect, useState } from 'react'
import type { ProjectedLayoutData } from '../../../shared/scene-projection'
import { runtimeStore, type RuntimeStoreHandle } from '../runtime-store'
import { projectLayoutData } from '../scene-projection'

/**
 * The payload with its scene projected from the camera this renderer holds.
 *
 * `zoom`, `pan` and `canvasOrigin` are the `camera` and `chrome` slices of the
 * same store commit the entities came from, so a camera patch on its own
 * re-projects every entity — which is the whole of what a pan or zoom does.
 *
 * A gesture delivers several camera patches per display frame, and every
 * layer downstream re-renders on a new payload. Rendering each patch as it
 * lands puts the tree several frames behind the native page views; so commits
 * collapse to one render per animation frame, and the render reads whatever
 * the store holds at that moment. Code that needs the store synchronously
 * (gesture logic on pointer events) subscribes to the store itself.
 *
 * Identity-stable per store commit (`projectLayoutData` caches on the payload),
 * so the memoized layers keep their bail-outs.
 */
export function useProjectedLayoutData(
  store: RuntimeStoreHandle = runtimeStore,
): ProjectedLayoutData {
  const [layoutData, setLayoutData] = useState(() => projectLayoutData(store.readLayoutData()))

  useEffect(() => {
    let raf = 0
    let rendered = projectLayoutData(store.readLayoutData())
    const flush = () => {
      raf = 0
      const next = projectLayoutData(store.readLayoutData())
      if (next === rendered) return
      rendered = next
      setLayoutData(next)
    }
    // A commit between the initial read and this subscription would otherwise
    // be missed until the next one.
    flush()
    const unsubscribe = store.subscribe(() => {
      if (!raf) raf = requestAnimationFrame(flush)
    })
    return () => {
      unsubscribe()
      if (raf) cancelAnimationFrame(raf)
    }
  }, [store])

  return layoutData
}
