import type { ProjectedLayoutData } from '../../shared/scene-projection'
import { useCallback, useEffect, useMemo, useState } from 'react'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'

/**
 * Focused-thread state for the canvas. The conversation itself lives in the
 * right panel; the canvas keeps only a highlight ring on the focused thread's
 * anchor. Main owns which thread is focused (`focusedAnnotationId` in
 * ui-state) and echoes changes here via `annotationThreadOpen`; clicking a
 * badge or region overlay reports the focus intent back to main.
 */
export function useAnnotationThreadState({
  api,
  layoutData,
}: {
  api: CanvasBgElectronAPI
  layoutData: ProjectedLayoutData
}) {
  const [focusedThreadId, setFocusedThreadId] = useState<string | null>(null)

  const focusedThread = useMemo(
    () =>
      focusedThreadId
        ? (layoutData.annotations ?? []).find((annotation) => annotation.id === focusedThreadId) ??
          null
        : null,
    [layoutData.annotations, focusedThreadId],
  )

  useEffect(() => {
    const cleanup = api.onAnnotationThreadOpen(({ annotationId }) => {
      setFocusedThreadId(annotationId ?? null)
    })
    return cleanup
  }, [api])

  // The focused thread vanished from the payload — deleted, resolved, or its
  // page navigated off the annotation's document — so drop the ring.
  useEffect(() => {
    if (!focusedThreadId) return
    if (focusedThread) return
    setFocusedThreadId(null)
  }, [focusedThread, focusedThreadId])

  // Badge / region-overlay click: the anchor is already in view, so skip the
  // camera reveal; main opens the panel thread and echoes the focus back.
  const focusThread = useCallback(
    (annotationId: string) => {
      setFocusedThreadId(annotationId)
      api.openAnnotationThread(annotationId, { reveal: false })
    },
    [api],
  )

  const closeThread = useCallback(() => {
    setFocusedThreadId(null)
    api.openAnnotationThread(null)
  }, [api])

  return {
    closeThread,
    focusThread,
    focusedThread,
    focusedThreadId,
  }
}
