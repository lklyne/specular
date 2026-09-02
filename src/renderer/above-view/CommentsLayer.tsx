import type { ProjectedLayoutData } from '../../shared/scene-projection'
import { memo } from 'react'
import type { Annotation, WorkspaceBounds } from '../../shared/types'
import {
  annotationElementScreenRect,
  canvasRectToScreenRect,
  pendingElementScreenRect,
  type AnnotationLiveBboxLookup,
  type PendingAnnotation,
} from './annotationMath'
import { CommentInput } from '../shared/CommentPrimitives'

const REGION_COMPOSER_WIDTH = 320
const REGION_COMPOSER_MARGIN = 12

/**
 * Single pending-annotation composer (ADR 0006). One component handles all
 * three anchor types — element, canvas-point, region — with placement keyed
 * off the anchor: above-right of the element bbox, adjacent to the click
 * point, or above-right of the region rect. Element/canvas-point drafts
 * arrive via `pendingAnnotation`; region drafts arrive via
 * `pendingRegionRect`. Only one is set at a time.
 *
 * `pendingPosition` is computed at render time by the caller — for element
 * pendings it consults the live bbox the page reports on scroll so the
 * composer follows page content (ADR 0006).
 */
export function PendingAnnotationComposer({
  clearDraft,
  commentInputRef,
  commentText,
  elementNameDraft,
  layoutData,
  pendingAnnotation,
  pendingPosition,
  pendingRegionRect,
  pendingRegionSelectionIds,
  setCommentText,
  setElementNameDraft,
  submitPendingAnnotation,
  submitRegionAnnotation,
}: {
  clearDraft: () => void
  commentInputRef: React.RefObject<HTMLTextAreaElement | null>
  commentText: string
  elementNameDraft: string
  layoutData: ProjectedLayoutData
  pendingAnnotation: PendingAnnotation | null
  pendingPosition: { left: number; top: number; width: number } | null
  pendingRegionRect: WorkspaceBounds | null
  /** Non-null exactly for a selection-born region draft (the popup's Annotate
   *  button). Unlike the comment-tool's region drag, this draft isn't backed
   *  by an active tool gesture, so the router (`useCanvasPointerRouter`)
   *  stands down entirely while it's open (I8' — `annotation-overlay` pointer
   *  owner) and nothing else dismisses it on an outside click. This backdrop
   *  supplies that: same commit-if-typed / discard-if-empty rule the
   *  comment-tool's own click-away path uses (`runCommentGesture`'s
   *  `hasEmptyDraft` check). */
  pendingRegionSelectionIds: string[] | null
  setCommentText: React.Dispatch<React.SetStateAction<string>>
  setElementNameDraft: React.Dispatch<React.SetStateAction<string>>
  submitPendingAnnotation: () => void
  submitRegionAnnotation: () => void
}) {
  if (pendingAnnotation) {
    return (
      <PointDraft
        clearDraft={clearDraft}
        commentInputRef={commentInputRef}
        commentText={commentText}
        elementNameDraft={elementNameDraft}
        pendingAnnotation={pendingAnnotation}
        pendingPosition={pendingPosition}
        setCommentText={setCommentText}
        setElementNameDraft={setElementNameDraft}
        submit={submitPendingAnnotation}
      />
    )
  }
  if (pendingRegionRect) {
    return (
      <RegionDraft
        clearDraft={clearDraft}
        commentInputRef={commentInputRef}
        commentText={commentText}
        layoutData={layoutData}
        rect={pendingRegionRect}
        selectionIds={pendingRegionSelectionIds}
        setCommentText={setCommentText}
        submit={submitRegionAnnotation}
      />
    )
  }
  return null
}

/** Element- and canvas-point drafts: the composer alone, at the anchor. */
function PointDraft({
  clearDraft,
  commentInputRef,
  commentText,
  elementNameDraft,
  pendingAnnotation,
  pendingPosition,
  setCommentText,
  setElementNameDraft,
  submit,
}: {
  clearDraft: () => void
  commentInputRef: React.RefObject<HTMLTextAreaElement | null>
  commentText: string
  elementNameDraft: string
  pendingAnnotation: PendingAnnotation
  pendingPosition: { left: number; top: number; width: number } | null
  setCommentText: React.Dispatch<React.SetStateAction<string>>
  setElementNameDraft: React.Dispatch<React.SetStateAction<string>>
  submit: () => void
}) {
  // Only element anchors carry a nameable target, so the name field is theirs.
  const isElementAnchor = pendingAnnotation.request.anchor.type === 'element'
  return (
    <ComposerBox
      clearDraft={clearDraft}
      commentInputRef={commentInputRef}
      commentText={commentText}
      left={pendingPosition?.left ?? pendingAnnotation.composerX}
      top={pendingPosition?.top ?? pendingAnnotation.composerY}
      width={pendingPosition?.width ?? pendingAnnotation.composerWidth}
      setCommentText={setCommentText}
      submit={submit}
      submitLabel="Submit comment"
      elementNameDraft={isElementAnchor ? elementNameDraft : undefined}
      setElementNameDraft={isElementAnchor ? setElementNameDraft : undefined}
    />
  )
}

/** Region drafts: the dashed rect, the composer below it, and — for a
 *  selection-born draft — the backdrop that stands in for the tool gesture. */
function RegionDraft({
  clearDraft,
  commentInputRef,
  commentText,
  layoutData,
  rect,
  selectionIds,
  setCommentText,
  submit,
}: {
  clearDraft: () => void
  commentInputRef: React.RefObject<HTMLTextAreaElement | null>
  commentText: string
  layoutData: ProjectedLayoutData
  rect: WorkspaceBounds
  selectionIds: string[] | null
  setCommentText: React.Dispatch<React.SetStateAction<string>>
  submit: () => void
}) {
  const screen = canvasRectToScreenRect(layoutData, rect)
  const overlayTop = screen.top - layoutData.canvasOrigin.y
  const composerX = Math.min(
    Math.max(screen.left, 8),
    window.innerWidth - REGION_COMPOSER_WIDTH - 8,
  )
  return (
    <>
      {selectionIds ? (
        <div
          className="pointer-events-auto absolute inset-0 z-30"
          data-overlay-ui
          onPointerDown={(event) => {
            if (event.pointerType === 'mouse' && event.button !== 0) return
            if (commentText.trim()) submit()
            else clearDraft()
          }}
        />
      ) : null}
      <div
        className="pointer-events-none absolute rounded border-2 border-dashed border-blue-500/90 bg-blue-500/10"
        style={{ left: screen.left, top: overlayTop, width: screen.width, height: screen.height }}
      />
      <ComposerBox
        clearDraft={clearDraft}
        commentInputRef={commentInputRef}
        commentText={commentText}
        left={composerX}
        top={overlayTop + screen.height + REGION_COMPOSER_MARGIN}
        width={REGION_COMPOSER_WIDTH}
        setCommentText={setCommentText}
        submit={submit}
        submitLabel="Submit region annotation"
      />
    </>
  )
}

function ComposerBox({
  clearDraft,
  commentInputRef,
  commentText,
  left,
  top,
  width,
  setCommentText,
  submit,
  submitLabel,
  elementNameDraft,
  setElementNameDraft,
}: {
  clearDraft: () => void
  commentInputRef: React.RefObject<HTMLTextAreaElement | null>
  commentText: string
  left: number
  top: number
  width: number
  setCommentText: React.Dispatch<React.SetStateAction<string>>
  submit: () => void
  submitLabel?: string
  elementNameDraft?: string
  setElementNameDraft?: React.Dispatch<React.SetStateAction<string>>
}) {
  const showElementName = elementNameDraft !== undefined && setElementNameDraft !== undefined
  return (
    <div
      className="pointer-events-auto absolute z-50"
      data-overlay-ui
      style={{ left, top, width }}
    >
      <div className="flex flex-col gap-1 rounded-[8px] border border-[var(--surface-popover-border)] bg-[var(--surface-popover-subtle)] px-1.5 pt-1.5 shadow-lg">
        {showElementName ? (
          <input
            type="text"
            value={elementNameDraft}
            onChange={(event) => setElementNameDraft(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Escape') { event.preventDefault(); clearDraft() }
              if (event.key === 'Enter') { event.preventDefault(); commentInputRef.current?.focus() }
              event.stopPropagation()
            }}
            placeholder="Element name"
            aria-label="Element name"
            className="w-full rounded-[6px] bg-transparent px-2 py-1 text-[12px] font-medium text-[var(--surface-foreground)] outline-none placeholder:text-[var(--surface-foreground-muted)]"
          />
        ) : null}
        <div className="relative pl-1.5 pb-1.5">
          <CommentInput
            inputRef={commentInputRef}
            autoFocus={!showElementName}
            value={commentText}
            onChange={setCommentText}
            onSubmit={submit}
            submitLabel={submitLabel}
            onKeyDown={(event) => {
              if (event.key === 'Escape') { event.preventDefault(); clearDraft() }
              event.stopPropagation()
            }}
          />
        </div>
      </div>
    </div>
  )
}

/**
 * Outline drawn around the element targeted by a pending element-anchored
 * comment. Single-click element selection through the comment tool sets a
 * `pendingAnnotation`, which suppresses the page-paints hover preview — so
 * without this outline the user has no visual confirmation of what they
 * just selected. The region case keeps its outlines because each page
 * paints them from the held marquee rect.
 */
export const PendingElementOutline = memo(function PendingElementOutline({
  pending,
  layoutData,
  liveBboxes,
}: {
  pending: PendingAnnotation | null
  layoutData: ProjectedLayoutData
  liveBboxes: AnnotationLiveBboxLookup
}) {
  if (!pending) return null
  const rect = pendingElementScreenRect(pending, layoutData, liveBboxes)
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
})

/**
 * Highlight ring on the focused thread's anchor. The conversation itself
 * lives in the right panel; this is the canvas's only trace of an open
 * thread — the annotated element stays fully visible while the agent works
 * on it. Element anchors only: regions keep their resting overlay, and
 * canvas points are marked by their badge.
 */
export const FocusedThreadOutline = memo(function FocusedThreadOutline({
  annotation,
  layoutData,
  liveBboxes,
}: {
  annotation: Annotation | null
  layoutData: ProjectedLayoutData
  liveBboxes: AnnotationLiveBboxLookup
}) {
  if (!annotation) return null
  const rect = annotationElementScreenRect(annotation, layoutData, liveBboxes)
  if (!rect) return null
  return (
    <div
      className="pointer-events-none absolute"
      style={{
        left: rect.left,
        top: rect.top,
        width: Math.max(1, rect.width),
        height: Math.max(1, rect.height),
        border: '1.5px solid rgba(59, 130, 246, 0.95)',
        borderRadius: 3,
        boxShadow: '0 0 0 1px rgba(255, 255, 255, 0.22) inset',
        boxSizing: 'border-box',
        zIndex: 40,
      }}
    />
  )
})
