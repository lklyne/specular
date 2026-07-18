/**
 * Element-attachment subscription source (ADR 0032).
 *
 * For each live page, the set of distinct DOM selectors that anchored items
 * reference is *main-side truth* — derived from workspace state, not from
 * renderer popover state. An item contributes its selector when its
 * `pageAnchor.element.selector` names that page; items sharing a selector share
 * one subscription (keyed dedupe).
 *
 * `refreshAttachmentSubscriptions()` recomputes every page's set and pushes it
 * to the page preload over `element-attachment-subscriptions`, but only when
 * that page's set actually changed — a cheap per-page set-equality check makes
 * the call a no-op almost always, so it can ride the single mutation seam
 * (`mutateWorkspace`) plus the async stamp and page-load events without
 * chasing every mutation site.
 *
 * `requestAttachmentSubscriptionRefresh()` coalesces a burst of mutations in
 * one tick into a single recompute via a microtask.
 */

import { ipcChannels } from '../../shared/ipc-contract'
import { pages } from './runtime-context'
import { anchorableEntities } from './page-anchor-state'
import { workspaceAnnotations } from './workspace-model'
import { safeSend } from './safe-send'

// Last selector set *sent* to each page, joined into a stable key, so a
// recompute that produces the same set sends nothing. Pages absent here are
// treated as last-sent-empty, so a page with no attachments never gets a push.
const lastSentByPage = new Map<string, string>()

function selectorsForPage(pageId: string): string[] {
  const selectors = new Set<string>()
  for (const entity of anchorableEntities()) {
    const element = entity.pageAnchor?.element
    if (element && entity.pageAnchor?.pageId === pageId) selectors.add(element.selector)
  }
  for (const annotation of workspaceAnnotations) {
    const element = annotation.pageAnchor?.element
    if (element && annotation.pageAnchor?.pageId === pageId) selectors.add(element.selector)
  }
  // Sorted so the equality key is order-independent.
  return [...selectors].sort()
}

/** Recompute every live page's selector set and push the ones that changed. */
export function refreshAttachmentSubscriptions(): void {
  const seen = new Set<string>()
  for (const page of pages) {
    seen.add(page.id)
    const selectors = selectorsForPage(page.id)
    const key = selectors.join('\n')
    if ((lastSentByPage.get(page.id) ?? '') === key) continue
    lastSentByPage.set(page.id, key)
    if (page.pageView.webContents.isDestroyed()) continue
    safeSend(page.pageView.webContents, ipcChannels.elementAttachmentSubscriptions, {
      selectors,
    })
  }
  // Forget pages that are gone so a recreated id starts from empty.
  for (const pageId of [...lastSentByPage.keys()]) {
    if (!seen.has(pageId)) lastSentByPage.delete(pageId)
  }
}

let pending = false

/** Coalesced recompute-and-push: safe to call from any mutation seam. */
export function requestAttachmentSubscriptionRefresh(): void {
  if (pending) return
  pending = true
  queueMicrotask(() => {
    pending = false
    refreshAttachmentSubscriptions()
  })
}

/**
 * Forget a page's last-sent set so the next refresh re-pushes it even if
 * unchanged. A (re)loaded page's preload starts with no subscriptions, so main
 * must re-declare them; the dedupe cache would otherwise suppress the resend.
 */
export function resetAttachmentSubscriptionsForPage(pageId: string): void {
  lastSentByPage.delete(pageId)
  requestAttachmentSubscriptionRefresh()
}
