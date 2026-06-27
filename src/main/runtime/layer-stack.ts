/**
 * LAYER_STACK — the declarative z-order for singleton overlay WCVs.
 *
 * `applyStack()` is an idempotent full child-list reconcile: it computes
 * the desired ordered child list — `bgView` → live pages → component
 * views → above-pages overlays → devtools cluster → `toolbar` — diffs it
 * against `win.contentView.children`, and applies the delta. It is the
 * only site that calls `addChildView` / `removeChildView` for pages,
 * component views, and singleton overlays (invariant I1). Page and
 * component factories just mutate `pages[]` / the component-view set and
 * request a layout; the reconcile owns attachment.
 *
 * It is invoked exclusively from `layoutAllViews()` when the 'stack'
 * dirty flag is set.
 */

import type { View, WebContentsView } from 'electron'
import {
  bgView,
  aboveView,
  devtoolsBackgroundView,
  devtoolsHeaderView,
  devtoolsResizeHandleView,
  devtoolsView,
  leftSidebarView,
  toolbarView,
  win,
} from './view-refs'
import { focusPresentationOverride, pages } from './runtime-context'
import { listComponentViews } from './component-page-factory'
import type { Page } from './runtime-entities'

export type LayerId =
  | 'bgView'
  | 'aboveView'
  | 'leftSidebar'
  | 'devtoolsBackground'
  | 'devtools'
  | 'devtoolsHeader'
  | 'devtoolsResizeHandle'
  | 'toolbar'

/**
 * Bottom → top. `bgView` pinned to index 0; everything else stacks above pages.
 *
 * `aboveView` is the sole above-pages overlay WCV. It owns marquee,
 * annotations, comments, presence, drawing, the floating-ui menus, and
 * the input-gate forwarding that interaction-overlay used to handle.
 */
export const LAYER_STACK: readonly LayerId[] = [
  'bgView',
  // pages live here (added at creation time by page-factory)
  'aboveView',
  'leftSidebar',
  'devtoolsBackground',
  'devtools',
  'devtoolsHeader',
  'devtoolsResizeHandle',
  'toolbar',
] as const

function resolve(id: LayerId): WebContentsView | null {
  switch (id) {
    case 'bgView': return bgView
    case 'aboveView': return aboveView
    case 'leftSidebar': return leftSidebarView
    case 'devtoolsBackground': return devtoolsBackgroundView
    case 'devtools': return devtoolsView
    case 'devtoolsHeader': return devtoolsHeaderView
    case 'devtoolsResizeHandle': return devtoolsResizeHandleView
    case 'toolbar': return toolbarView
  }
}

/**
 * Resolve LAYER_STACK to the ordered IDs present in the current view registry.
 * Pure (no Electron calls) — used for unit-test determinism.
 */
export function resolveStackOrder(
  refs: Partial<Record<LayerId, unknown>> = {},
): LayerId[] {
  const get = (id: LayerId) => (id in refs ? refs[id] : resolve(id))
  return LAYER_STACK.filter((id) => get(id) != null)
}

export function orderedPagesForStack<T extends { id: string }>(
  inputPages: readonly T[],
  focusedPageId: string | null | undefined,
): T[] {
  if (!focusedPageId) return [...inputPages]
  const focused = inputPages.find((page) => page.id === focusedPageId)
  if (!focused) return [...inputPages]
  return [
    ...inputPages.filter((page) => page.id !== focusedPageId),
    focused,
  ]
}

/**
 * Compute the desired ordered child list, bottom → top:
 * `bgView` → pages (`frameView` + `pageView` + inactive `devtoolsHostView`)
 * → component views → above-pages overlays → devtools cluster → `toolbar`.
 *
 * The active devtools host (`devtoolsView`) is placed with the devtools
 * cluster; every other page's `devtoolsHostView` parks in the per-page
 * section (it is hidden off-screen anyway).
 */
function desiredChildOrder(): View[] {
  const order: View[] = []
  for (const id of resolveStackOrder()) {
    const view = resolve(id)
    if (!view) continue
    order.push(view)
    if (id === 'bgView') {
      const orderedPages: Page[] = orderedPagesForStack(
        pages,
        focusPresentationOverride?.pageId,
      )
      for (const page of orderedPages) {
        order.push(page.frameView, page.pageView)
        if (page.devtoolsHostView && page.devtoolsHostView !== devtoolsView) {
          order.push(page.devtoolsHostView)
        }
      }
      for (const cv of listComponentViews()) order.push(cv.view)
    }
  }
  return order
}

export function applyStack(): void {
  if (!win) return
  const desired = desiredChildOrder()
  const actual = [...win.contentView.children]
  const unchanged =
    actual.length === desired.length &&
    actual.every((view, index) => view === desired[index])
  if (unchanged) return

  // Detach views no longer in the desired set (closed pages / components).
  for (const child of actual) {
    if (!desired.includes(child)) win.contentView.removeChildView(child)
  }
  // Re-add every desired view in order — `addChildView` on a view that is
  // already a child moves it, so appending in sequence yields the exact
  // desired z-order.
  for (const view of desired) {
    win.contentView.addChildView(view)
  }
}
