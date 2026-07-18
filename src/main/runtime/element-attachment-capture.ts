/**
 * Element-attachment enrichment (ADR 0030).
 *
 * When a page-anchored item is placed or moved, a fire-and-forget preload
 * query finds the DOM element under the item's document-space center and
 * stamps it onto the anchor as `pageAnchor.element`. Anchoring itself stays
 * synchronous and geometric (page-anchor-state.ts); this module is the async
 * tail that decorates the anchor once the page answers.
 *
 * Two invariants shape the code:
 *
 * - **Stale-safe.** The item may move, reanchor, or drag off a page while the
 *   query is in flight. Each fire bumps a per-item token; the resolve only
 *   stamps when its token is still current and the anchor still names the same
 *   page/url it was fired for. A newer reanchor fired its own query and wins.
 *
 * - **Outside undo.** The user never chose the element — placement did — so the
 *   stamp must not occupy an undo slot (same reasoning as viewport zoom/pan).
 *   The runtime object is mutated and the attachment is written to a dedicated
 *   Y.Doc field under an untracked origin. The undoable geometric anchor is
 *   never replaced by enrichment. Persistence still flows from runtime state,
 *   where the public shape remains `pageAnchor.element`.
 */

import type * as Y from 'yjs'
import type { PageAnchor } from '../../shared/page-anchor'
import { captureElementAtPageDocumentPoint } from './page-queries'
import { canvasRectToPageDocRect, findAnchorableEntity } from './page-anchor-state'
import { workspaceAnnotations } from './workspace-model'
import { requestAttachmentSubscriptionRefresh } from './element-attachment-subscriptions'
import {
  getActiveDoc,
  DOC_MAP_ENTITIES,
  DOC_MAP_ANNOTATIONS,
} from './workspace-doc'
import { scheduleWorkspaceAutosave } from './workspace-autosave'

/** Untracked transaction origin for the enrichment doc write — outside the
 *  UndoManager's `trackedOrigins` ({null, 'user'}), so the stamp is never an
 *  undo step (mirrors `note-seed` in note-content-state.ts). */
export const ANCHOR_ELEMENT_CAPTURE_ORIGIN = 'anchor-element-capture'

type CapturedElement = NonNullable<PageAnchor['element']>

interface CaptureSnapshot {
  pageId: string
  pageUrl: string | undefined
  token: number
}

// Monotonic per-entity fire counter: a later fire supersedes an earlier one so
// an out-of-order resolve (create then drag-end) can't stamp stale geometry.
const entityCaptureTokens = new Map<string, number>()

function nextEntityToken(entityId: string): number {
  const token = (entityCaptureTokens.get(entityId) ?? 0) + 1
  entityCaptureTokens.set(entityId, token)
  return token
}

/** Coerce a preload capture response to a well-formed element attachment. */
function toCapturedElement(data: unknown): CapturedElement | null {
  if (!data || typeof data !== 'object') return null
  const record = data as Record<string, unknown>
  if (typeof record.selector !== 'string') return null
  if (typeof record.docX !== 'number' || typeof record.docY !== 'number') return null
  return {
    selector: record.selector,
    docX: record.docX,
    docY: record.docY,
    ...(record.viewportPositioned === true ? { viewportPositioned: true } : {}),
  }
}

/** Whether the anchor still names the same page/url the query was fired for. */
function anchorMatchesSnapshot(anchor: PageAnchor, snapshot: CaptureSnapshot): boolean {
  return anchor.pageId === snapshot.pageId && anchor.pageUrl === snapshot.pageUrl
}

/**
 * Write derived attachment metadata into its own Y.Map field under the
 * untracked origin. The diff-sync strips `element` from the undoable
 * `pageAnchor` field. Scheduling autosave persists the runtime value to disk.
 */
function writeAnchorElementToDoc(mapName: string, itemId: string, anchor: PageAnchor): void {
  const doc = getActiveDoc()
  const ymap = doc.getMap(mapName) as Y.Map<Y.Map<unknown>>
  const yItem = ymap.get(itemId)
  if (yItem) {
    doc.transact(() => {
      yItem.set('pageAnchorElement', anchor.element)
    }, ANCHOR_ELEMENT_CAPTURE_ORIGIN)
  }
  scheduleWorkspaceAutosave()
}

/**
 * Stamp a captured element onto an anchorable entity's anchor, guarding against
 * a superseding fire or a reanchor that happened while the query was in flight.
 * Exported for direct unit-style coverage of the stale guards.
 */
export function stampEntityElement(
  entityId: string,
  snapshot: CaptureSnapshot,
  data: unknown,
): void {
  const captured = toCapturedElement(data)
  if (!captured) return
  if (entityCaptureTokens.get(entityId) !== snapshot.token) return
  const entity = findAnchorableEntity(entityId)
  const anchor = entity?.pageAnchor
  if (!entity || !anchor || !anchorMatchesSnapshot(anchor, snapshot)) return
  // Fresh object — the diff-sync detects anchor changes by field identity.
  entity.pageAnchor = { ...anchor, element: captured }
  writeAnchorElementToDoc(DOC_MAP_ENTITIES, entityId, entity.pageAnchor)
  // The page must now track this selector — the stamp bypasses the mutation
  // seam (it is outside undo), so it refreshes subscriptions itself.
  requestAttachmentSubscriptionRefresh()
}

/**
 * Fire a capture query for an anchored entity from its document-space center.
 * No-op when the entity is free or its page is gone. Fire-and-forget: rejection
 * (timeout, destroyed page) is swallowed.
 */
export function captureElementForEntity(entityId: string): void {
  const entity = findAnchorableEntity(entityId)
  const anchor = entity?.pageAnchor
  if (!entity || !anchor) return
  const docRect = canvasRectToPageDocRect(
    { x: entity.canvasX, y: entity.canvasY, width: entity.width, height: entity.height },
    anchor.pageId,
  )
  if (!docRect) return
  const docX = docRect.x + docRect.width / 2
  const docY = docRect.y + docRect.height / 2
  const snapshot: CaptureSnapshot = {
    pageId: anchor.pageId,
    pageUrl: anchor.pageUrl,
    token: nextEntityToken(entityId),
  }
  void captureElementAtPageDocumentPoint(anchor.pageId, docX, docY)
    .then((data) => stampEntityElement(entityId, snapshot, data))
    .catch(() => {})
}

/**
 * Stamp a captured element onto a region annotation's anchor. Annotations
 * capture once at creation, so there is no token race — only the existence and
 * same-anchor guards. Exported for direct coverage.
 */
export function stampAnnotationElement(
  annotationId: string,
  snapshot: CaptureSnapshot,
  data: unknown,
): void {
  const captured = toCapturedElement(data)
  if (!captured) return
  const annotation = workspaceAnnotations.find((candidate) => candidate.id === annotationId)
  const anchor = annotation?.pageAnchor
  if (!annotation || !anchor || !anchorMatchesSnapshot(anchor, snapshot)) return
  annotation.pageAnchor = { ...anchor, element: captured }
  writeAnchorElementToDoc(DOC_MAP_ANNOTATIONS, annotationId, annotation.pageAnchor)
  requestAttachmentSubscriptionRefresh()
}

/**
 * Fire a one-shot capture for a freshly created region annotation from its
 * region's document-space center. No-op unless the annotation carries a page
 * anchor and a `docRect` region (a region's binding is written once — the
 * attachment is tracking, not binding; ADR 0030).
 */
export function captureElementForAnnotation(annotationId: string): void {
  const annotation = workspaceAnnotations.find((candidate) => candidate.id === annotationId)
  const anchor = annotation?.pageAnchor
  if (!annotation || !anchor) return
  if (annotation.anchor.type !== 'region' || !('docRect' in annotation.anchor)) return
  const { docRect } = annotation.anchor
  const docX = docRect.x + docRect.width / 2
  const docY = docRect.y + docRect.height / 2
  const snapshot: CaptureSnapshot = { pageId: anchor.pageId, pageUrl: anchor.pageUrl, token: 0 }
  void captureElementAtPageDocumentPoint(anchor.pageId, docX, docY)
    .then((data) => stampAnnotationElement(annotationId, snapshot, data))
    .catch(() => {})
}
