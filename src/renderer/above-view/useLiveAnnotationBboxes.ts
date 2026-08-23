/**
 * Live-bbox round-trip for element-anchored annotation popovers (ADR 0006).
 *
 * The hook tracks which element-anchored popovers are currently visible (open
 * thread + pending composer), groups their `(annotationId, selector)` pairs by
 * `pageId`, and pushes the per-page set to main whenever it changes. Pages
 * resolve the selectors against their live DOM and report back; main folds the
 * answers into the `annotationBboxes` slice, which is what this reads. Main
 * also holds a stale anchor's last-known bbox, so a popover whose element
 * disappeared stays put instead of jumping to (0,0).
 */

import { useEffect, useMemo, useRef } from 'react'
import type { AnnotationBboxSubscription, DevtoolsPanelDomRect } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { useSlice } from '../shared/hooks/useRuntimeStore'
import type { RuntimeStore } from '../../shared/runtime-store'

type SubscriptionApi = Pick<CanvasBgElectronAPI, 'setAnnotationBboxSubscriptions'>

export interface AnnotationBboxLookup {
  /** Live (or last-known live) bbox for an annotation, or undefined when no
   *  live update has arrived yet — caller should fall back to the stored
   *  anchor.boundingBox. */
  get: (annotationId: string) => DevtoolsPanelDomRect | undefined
  /** True when the page reported the selector no longer resolves. The popover
   *  should keep its last-known position and surface a "stale anchor" hint. */
  isStale: (annotationId: string) => boolean
}

const selectBboxes = (store: RuntimeStore) => store.slices.annotationBboxes

export function useLiveAnnotationBboxes({
  api,
  subscriptions,
}: {
  api: SubscriptionApi
  subscriptions: Array<{ pageId: string; annotationId: string; selector: string }>
}): AnnotationBboxLookup {
  const bboxes = useSlice(selectBboxes)
  const lastSubKeyByPageRef = useRef<Map<string, string>>(new Map())

  // Group subscriptions by page and push the per-page set whenever it changes.
  // We unsubscribe (empty array) for any page that previously had subs but
  // doesn't now, so pages can stop their scroll-tracking work.
  const subsByPage = useMemo(() => {
    const grouped = new Map<string, AnnotationBboxSubscription[]>()
    for (const sub of subscriptions) {
      const list = grouped.get(sub.pageId) ?? []
      list.push({ annotationId: sub.annotationId, selector: sub.selector })
      grouped.set(sub.pageId, list)
    }
    return grouped
  }, [subscriptions])

  useEffect(() => {
    const seenPages = new Set<string>()
    for (const [pageId, subs] of subsByPage) {
      seenPages.add(pageId)
      const sortedKey = subs
        .map((s) => `${s.annotationId}:${s.selector}`)
        .sort()
        .join('|')
      if (lastSubKeyByPageRef.current.get(pageId) === sortedKey) continue
      lastSubKeyByPageRef.current.set(pageId, sortedKey)
      api.setAnnotationBboxSubscriptions(pageId, subs)
    }
    // Empty out pages that fell off.
    for (const pageId of [...lastSubKeyByPageRef.current.keys()]) {
      if (seenPages.has(pageId)) continue
      lastSubKeyByPageRef.current.delete(pageId)
      api.setAnnotationBboxSubscriptions(pageId, [])
    }
  }, [api, subsByPage])

  return useMemo(
    () => ({
      get: (annotationId: string) => bboxes?.[annotationId]?.boundingBox ?? undefined,
      isStale: (annotationId: string) => bboxes?.[annotationId]?.stale === true,
    }),
    [bboxes],
  )
}
