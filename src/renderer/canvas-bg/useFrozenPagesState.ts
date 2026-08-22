import { useEffect, useState } from 'react'
import type { FrozenPagesState } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'

const NO_FROZEN_PAGES: FrozenPagesState = {
  revision: 0,
  target: 'bg',
  active: false,
  frames: [],
}

/**
 * Tracks the frozen-pages broadcast from the canvas-bg side: the `bg` target
 * is this view's own raster set, while the `above` target only contributes
 * page ids — above-view draws chrome and raster for drag-frozen pages, so
 * this view has to skip them.
 */
export function useFrozenPagesState(api: CanvasBgElectronAPI): {
  frozenPages: FrozenPagesState
  dragFrozenPageIds: ReadonlySet<string>
} {
  const [frozenPages, setFrozenPages] = useState<FrozenPagesState>(NO_FROZEN_PAGES)
  const [dragFrozenPageIds, setDragFrozenPageIds] = useState<ReadonlySet<string>>(new Set())
  useEffect(
    () =>
      api.onFrozenPagesState((data) => {
        if (data.target === 'bg') {
          setFrozenPages(data)
        } else if (data.target === 'above') {
          setDragFrozenPageIds(
            data.active ? new Set(data.frames.map((frame) => frame.pageId)) : new Set(),
          )
        }
      }),
    [api],
  )
  return { frozenPages, dragFrozenPageIds }
}
