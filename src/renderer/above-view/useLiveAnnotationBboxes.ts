/**
 * Live-bbox round-trip for element-anchored annotation popovers (ADR 0006).
 *
 * Maintains a `liveBboxes` map keyed by annotation id. The hook tracks which
 * element-anchored popovers are currently visible (open thread + pending
 * composer), groups their `(annotationId, selector)` pairs by `pageId`, and
 * pushes the per-page set to main whenever it changes. Pages resolve the
 * selectors against their live DOM and stream bbox updates back; we merge
 * them into the map. Stale anchors (selector returned `null`) hold their
 * last-known bbox so the popover doesn't jump to (0,0).
 */

import { useEffect, useMemo, useRef, useState } from 'react'
import type { AnnotationBboxSubscription, DevtoolsPanelDomRect } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'

type SubscriptionApi = Pick<
  CanvasBgElectronAPI,
  'setAnnotationBboxSubscriptions' | 'onAnnotationLiveBbox'
>

export interface AnnotationBboxLookup {
  /** Live (or last-known live) bbox for an annotation, or undefined when no
   *  live update has arrived yet — caller should fall back to the stored
   *  anchor.boundingBox. */
  get: (annotationId: string) => DevtoolsPanelDomRect | undefined
  /** True when the page reported the selector no longer resolves. The popover
   *  should keep its last-known position and surface a "stale anchor" hint. */
  isStale: (annotationId: string) => boolean
}

export function useLiveAnnotationBboxes({
  api,
  subscriptions,
}: {
  api: SubscriptionApi
  subscriptions: Array<{ pageId: string; annotationId: string; selector: string }>
}): AnnotationBboxLookup {
  const [bboxes, setBboxes] = useState<Map<string, DevtoolsPanelDomRect>>(() => new Map())
  const [staleIds, setStaleIds] = useState<Set<string>>(() => new Set())
  const lastSubKeyByPageRef = useRef<Map<string, string>>(new Map())

  // Subscribe to bbox updates streamed back from pages.
  useEffect(() => {
    const cleanup = api.onAnnotationLiveBbox((update) => {
      if (!update?.annotationId) return
      if (update.boundingBox) {
        setBboxes((prev) => {
          const next = new Map(prev)
          next.set(update.annotationId, update.boundingBox as DevtoolsPanelDomRect)
          return next
        })
        setStaleIds((prev) => {
          if (!prev.has(update.annotationId)) return prev
          const next = new Set(prev)
          next.delete(update.annotationId)
          return next
        })
      } else {
        // Selector no longer resolves: hold last-known bbox, mark stale.
        setStaleIds((prev) => {
          if (prev.has(update.annotationId)) return prev
          const next = new Set(prev)
          next.add(update.annotationId)
          return next
        })
      }
    })
    return cleanup
  }, [api])

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

  // Forget bbox / stale entries that are no longer subscribed.
  useEffect(() => {
    const subscribedIds = new Set(subscriptions.map((s) => s.annotationId))
    setBboxes((prev) => {
      let changed = false
      const next = new Map(prev)
      for (const id of next.keys()) {
        if (!subscribedIds.has(id)) {
          next.delete(id)
          changed = true
        }
      }
      return changed ? next : prev
    })
    setStaleIds((prev) => {
      let changed = false
      const next = new Set(prev)
      for (const id of next) {
        if (!subscribedIds.has(id)) {
          next.delete(id)
          changed = true
        }
      }
      return changed ? next : prev
    })
  }, [subscriptions])

  return useMemo(
    () => ({
      get: (annotationId: string) => bboxes.get(annotationId),
      isStale: (annotationId: string) => staleIds.has(annotationId),
    }),
    [bboxes, staleIds],
  )
}
