import { useMemo } from 'react'
import type { ProjectedLayoutData } from '../../../shared/scene-projection'
import { runtimeStore, type RuntimeStoreHandle } from '../runtime-store'
import { projectLayoutData } from '../scene-projection'

/** The projected payload, read at the moment a gesture asks for it. */
export interface LayoutSnapshotRef {
  readonly current: ProjectedLayoutData
}

/**
 * What gesture code reads on a pointer event: the store's current commit,
 * projected.
 *
 * Handlers need the layout synchronously, which is why this is a ref rather
 * than the rendered value — the render is one per animation frame and lands
 * after the event. Projecting eagerly on every commit would do the same work
 * several times a frame during a gesture and throw all but the last result
 * away, so it happens on read instead; both `readLayoutData` and
 * `projectLayoutData` cache against the commit they came from, so repeated
 * reads within a frame cost a map lookup.
 */
export function useProjectedLayoutRef(
  store: RuntimeStoreHandle = runtimeStore,
): LayoutSnapshotRef {
  return useMemo(
    () => ({
      get current(): ProjectedLayoutData {
        return projectLayoutData(store.readLayoutData())
      },
    }),
    [store],
  )
}
