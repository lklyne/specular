/**
 * Shared reveal orchestration for page-bound comments and canvas items.
 *
 * A reveal has two independent jobs:
 * 1. restore the page document recorded by `pageAnchor`, when necessary;
 * 2. smooth-scroll that document to the bound content after it is ready.
 *
 * Callers still own their surface-specific selection and canvas focus. Keeping
 * document restoration here prevents sidebar comments and anchored entities
 * from growing separate navigation lifecycles.
 */

import { ipcChannels } from '../../shared/ipc-contract'
import type { PageAnchor } from '../../shared/page-anchor'
import { navigatePage } from '../navigation-sync'
import type { AnchorableEntity } from './anchorable-entity-store'
import { offPageDocument } from './document-binding'
import { pageAnchorElementShift } from './page-anchor-scroll'
import { pages } from './runtime-context'
import { pageBodyCanvasBounds } from './runtime-geometry'
import { sendPageIpc } from './page-ipc'

export interface PageDocumentScrollTarget {
  pageId: string
  /** Position in the page's document CSS px to bring into view. */
  documentY: number
}

/** Land revealed content below sticky site headers, with useful context above. */
const VIEWPORT_ANCHOR_FRACTION = 1 / 3

/**
 * Restore an anchor's recorded document, then reveal its content. When the
 * document is already current, `revealContent` runs immediately. Returns false
 * only when the anchor's page no longer exists.
 */
export function revealPageAnchoredContent(
  anchor: PageAnchor | undefined,
  revealContent: () => void | Promise<void>,
): boolean {
  if (!anchor) return false
  const page = pages.find((candidate) => candidate.id === anchor.pageId)
  if (!page || page.pageView.webContents.isDestroyed()) return false

  const reveal = () => {
    void Promise.resolve(revealContent()).catch(() => {})
  }

  if (anchor.pageUrl && offPageDocument(anchor.pageId, anchor.pageUrl)) {
    page.pageView.webContents.once('did-finish-load', reveal)
    navigatePage(page, { type: 'load-url', url: anchor.pageUrl })
    return true
  }

  reveal()
  return true
}

/**
 * Convert an anchored entity's stored canvas Y into its current document Y.
 * The recorded scroll offset recovers the placement-time document position;
 * the live element correction follows later page reflow. Legacy frame-pinned
 * anchors use the current scroll, preserving their present viewport position.
 */
export function computeEntityScrollTarget(
  entity: Pick<AnchorableEntity, 'canvasY' | 'pageAnchor'>,
): PageDocumentScrollTarget | null {
  const anchor = entity.pageAnchor
  if (!anchor) return null
  const page = pages.find((candidate) => candidate.id === anchor.pageId)
  if (!page) return null
  const body = pageBodyCanvasBounds(page)
  const element = pageAnchorElementShift(anchor)
  return {
    pageId: anchor.pageId,
    documentY:
      entity.canvasY -
      body.y +
      (anchor.scrollY ?? page.scrollY ?? 0) -
      element.y,
  }
}

/** Reuse the page preload's eased scroll ramp for any page-bound content. */
export async function dispatchScrollToDocumentTarget(
  target: PageDocumentScrollTarget,
): Promise<void> {
  const page = pages.find((candidate) => candidate.id === target.pageId)
  if (!page) return
  const body = pageBodyCanvasBounds(page)
  const targetScrollY = Math.max(
    0,
    target.documentY - body.height * VIEWPORT_ANCHOR_FRACTION,
  )
  const deltaY = targetScrollY - (page.scrollY ?? 0)
  await sendPageIpc(page.id, ipcChannels.dispatchScroll, {
    x: body.width / 2,
    y: body.height / 2,
    deltaX: 0,
    deltaY,
  }).catch(() => {})
}

export async function dispatchScrollToEntity(
  entity: Pick<AnchorableEntity, 'canvasY' | 'pageAnchor'>,
): Promise<void> {
  const target = computeEntityScrollTarget(entity)
  if (target) await dispatchScrollToDocumentTarget(target)
}
