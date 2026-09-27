import type { ProjectedLayoutData } from '../../shared/scene-projection'
import { memo } from 'react'
import type { CommentDraft } from '../../shared/comment-draft'
import {
  canvasRectToScreenRect,
  pendingElementScreenRect,
  type AnnotationLiveBboxLookup,
} from './annotationMath'
import { canvasToScreenX, canvasToScreenY, toOverlayY } from '../../shared/gesture-utils'

/**
 * The comment draft's canvas marker — a passive visual only. The composer
 * that finishes the draft lives in the right panel (ADR 0006 amendment); the
 * canvas just shows the user what a Save would attach to.
 */
export const CommentDraftMarker = memo(function CommentDraftMarker({
  draft,
  layoutData,
  liveBboxes,
}: {
  draft: CommentDraft | null
  layoutData: ProjectedLayoutData
  liveBboxes: AnnotationLiveBboxLookup
}) {
  if (!draft) return null
  if (draft.kind === 'element') {
    return <PendingElementOutline draft={draft} layoutData={layoutData} liveBboxes={liveBboxes} />
  }
  if (draft.kind === 'point') {
    return <PendingPointMarker draft={draft} layoutData={layoutData} />
  }
  return <PendingRegionOutline rect={draft.canvasRect} layoutData={layoutData} />
})

/**
 * Outline drawn around the element targeted by a pending element-anchored
 * comment. Single-click element selection through the comment tool opens a
 * draft, which suppresses the page-paints hover preview — so without this
 * outline the user has no visual confirmation of what they just selected.
 */
function PendingElementOutline({
  draft,
  layoutData,
  liveBboxes,
}: {
  draft: Extract<CommentDraft, { kind: 'element' }>
  layoutData: ProjectedLayoutData
  liveBboxes: AnnotationLiveBboxLookup
}) {
  const rect = pendingElementScreenRect(draft, layoutData, liveBboxes)
  if (!rect) return null
  return (
    <div
      className="pointer-events-none absolute"
      style={{
        left: rect.left,
        top: rect.top,
        width: Math.max(1, rect.width),
        height: Math.max(1, rect.height),
        border: '1px dashed rgba(59, 130, 246, 0.95)',
        background: 'rgba(59, 130, 246, 0.14)',
        boxShadow: '0 0 0 1px rgba(255, 255, 255, 0.22) inset',
        boxSizing: 'border-box',
        zIndex: 40,
      }}
    />
  )
}

/** A small pin at the click point for a canvas-point draft. */
function PendingPointMarker({
  draft,
  layoutData,
}: {
  draft: Extract<CommentDraft, { kind: 'point' }>
  layoutData: ProjectedLayoutData
}) {
  const x = canvasToScreenX(layoutData, draft.canvasX)
  const y = toOverlayY(layoutData, canvasToScreenY(layoutData, draft.canvasY))
  return (
    <div
      className="pointer-events-none absolute rounded-full border-2 border-blue-500/95 bg-blue-500/20"
      style={{
        left: x - 6,
        top: y - 6,
        width: 12,
        height: 12,
        boxShadow: '0 0 0 1px rgba(255, 255, 255, 0.22) inset',
        zIndex: 40,
      }}
    />
  )
}

/** The dashed rect for a region or selection draft — the same resting visual
 *  a region drag leaves, minus the composer that used to sit below it. */
function PendingRegionOutline({
  rect,
  layoutData,
}: {
  rect: { x: number; y: number; width: number; height: number }
  layoutData: ProjectedLayoutData
}) {
  const screen = canvasRectToScreenRect(layoutData, rect)
  const overlayTop = screen.top - layoutData.canvasOrigin.y
  return (
    <div
      className="pointer-events-none absolute rounded border-2 border-dashed border-blue-500/90 bg-blue-500/10"
      style={{ left: screen.left, top: overlayTop, width: screen.width, height: screen.height }}
    />
  )
}
