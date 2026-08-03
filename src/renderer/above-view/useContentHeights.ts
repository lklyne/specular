/**
 * Measured heights of content-sized entities (stickies), keyed by id. Written
 * by the body layer as the text lays out, read by `contentHeightLayout`, and
 * forwarded to main for persistence and hit-testing.
 *
 * The local map updates immediately — rendering must not wait a frame — but
 * the report to main is debounced: during a side-handle reflow drag or while
 * typing, the raw stream is one measurement per frame, and each would be its
 * own IPC message and Y.Doc transaction (`captureTimeout: 0` means one undo
 * step each). Main only needs the settled value, so it gets the trailing one.
 */

import { useCallback, useEffect, useRef, useState } from 'react'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import type { LayoutUpdateData } from '../../shared/types'

const REPORT_DELAY_MS = 300

export function useContentHeights(
  api: Pick<CanvasBgElectronAPI, 'updateEntity'>,
  layoutData: LayoutUpdateData,
): {
  contentHeights: Map<string, number>
  reportContentHeight: (id: string, height: number) => void
} {
  const [contentHeights, setContentHeights] = useState<Map<string, number>>(new Map())

  const pendingRef = useRef(new Map<string, number>())
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const flush = useCallback(() => {
    timerRef.current = null
    for (const [id, height] of pendingRef.current) {
      api.updateEntity('text', id, { height })
    }
    pendingRef.current.clear()
  }, [api])

  const reportContentHeight = useCallback(
    (id: string, height: number) => {
      setContentHeights((prev) => {
        if (prev.get(id) === height) return prev
        const next = new Map(prev)
        next.set(id, height)
        return next
      })
      pendingRef.current.set(id, height)
      if (timerRef.current) clearTimeout(timerRef.current)
      timerRef.current = setTimeout(flush, REPORT_DELAY_MS)
    },
    [flush],
  )

  // A pending report runs on unmount so the settled height isn't lost to a
  // tab switch.
  useEffect(() => {
    return () => {
      if (!timerRef.current) return
      clearTimeout(timerRef.current)
      flush()
    }
  }, [flush])

  // Entries for deleted entities would otherwise pin the map (and defeat its
  // empty fast path in `contentHeightLayout`) forever. Entries for live
  // entities stay even once main agrees — a measured height keeps winning
  // over broadcast values (e.g. an undo reverting the height).
  useEffect(() => {
    const live = new Set(layoutData.entities.map((entity) => entity.id))
    for (const id of pendingRef.current.keys()) {
      if (!live.has(id)) pendingRef.current.delete(id)
    }
    setContentHeights((prev) => {
      if (prev.size === 0) return prev
      if ([...prev.keys()].every((id) => live.has(id))) return prev
      return new Map([...prev].filter(([id]) => live.has(id)))
    })
  }, [layoutData.entities])

  return { contentHeights, reportContentHeight }
}
