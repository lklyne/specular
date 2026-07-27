import { useCallback, useEffect, useState } from 'react'
import type { AnnotationAnchor, AnnotationElementSelectionPayload, LayoutUpdateData, WorkspaceBounds } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { canvasToScreenX, canvasToScreenY, toOverlayY } from '../../shared/gesture-utils'
import { pageViewportToScreen } from '../../shared/page-space'
import {
  drawingBounds,
  elementAnchoredComposerPosition,
  type DrawingSession,
  type PendingAnnotation,
} from './annotationMath'

const VIEWPORT_PADDING = 8
const COMPOSER_MARGIN = 8
const COMPOSER_MIN_HEIGHT = 52
const CANVAS_POINT_COMPOSER_WIDTH = 320

/** What opening a draft sets. Anything left out returns to the closed value. */
interface DraftOpen {
  pending: PendingAnnotation | null
  regionRect: WorkspaceBounds | null
  regionSelectionIds: string[] | null
  elementName: string
}

const NO_DRAFT: DraftOpen = {
  pending: null,
  regionRect: null,
  regionSelectionIds: null,
  elementName: '',
}

export function useAnnotationDraftState({
  api,
  layoutData,
  layoutRef,
  commentInputRef,
  activeStrokeRef,
}: {
  api: CanvasBgElectronAPI
  layoutData: LayoutUpdateData
  layoutRef: React.MutableRefObject<LayoutUpdateData>
  commentInputRef: React.RefObject<HTMLTextAreaElement | null>
  activeStrokeRef: React.MutableRefObject<{ pointerId: number; strokeId: string } | null>
}) {
  const [pendingAnnotation, setPendingAnnotation] = useState<PendingAnnotation | null>(null)
  const [pendingRegionRect, setPendingRegionRect] = useState<WorkspaceBounds | null>(null)
  // Selection popup's Annotate button vs. the comment-tool region drag: both
  // land in `pendingRegionRect`, but only the selection-born one carries these
  // ids and routes its submit to `api.annotateSelection` instead of
  // `api.createRegionAnnotation` — see `submitRegionAnnotation` below.
  const [pendingRegionSelectionIds, setPendingRegionSelectionIds] = useState<string[] | null>(null)
  const [commentText, setCommentText] = useState('')
  const [elementNameDraft, setElementNameDraft] = useState('')

  const drawing = useDrawingSession(api, activeStrokeRef)
  const { drawingSession, setDrawingSession, clearDrawing, commitDrawing } = drawing

  const clearDraft = useCallback(() => {
    clearDrawing()
    setPendingAnnotation(null)
    setPendingRegionRect(null)
    setPendingRegionSelectionIds(null)
    setCommentText('')
    setElementNameDraft('')
  }, [clearDrawing])

  /**
   * Open one draft, closing every other. The composer is singular — a draft of
   * any kind supersedes whatever was open, and always starts with empty text.
   */
  const openDraft = useCallback((next: Partial<DraftOpen>) => {
    const draft = { ...NO_DRAFT, ...next }
    setPendingAnnotation(draft.pending)
    setPendingRegionRect(draft.regionRect)
    setPendingRegionSelectionIds(draft.regionSelectionIds)
    clearDrawing()
    setCommentText('')
    setElementNameDraft(draft.elementName)
  }, [clearDrawing])

  // Renderer-local handoff for the selection popup's Annotate button (ADR
  // 0019 one door): pre-anchors the same region composer the comment tool's
  // drag gesture opens, over the selection's union bounds instead of a drag
  // rect. No IPC round-trip — the popup already has everything it needs
  // (the layout broadcast's entities) to compute the rect itself.
  const beginSelectionAnnotation = useCallback(
    (entityIds: string[], rect: WorkspaceBounds) => {
      openDraft({ regionRect: rect, regionSelectionIds: entityIds })
    },
    [openDraft],
  )

  const submitPendingAnnotation = useCallback(() => {
    if (!pendingAnnotation) return
    const nextText = commentText.trim()
    if (!nextText) return
    api.createAnnotation({
      ...pendingAnnotation.request,
      text: nextText,
      ...elementNameField(pendingAnnotation, elementNameDraft),
    })
    clearDraft()
  }, [api, clearDraft, commentText, elementNameDraft, pendingAnnotation])

  const submitRegionAnnotation = useCallback(() => {
    if (!pendingRegionRect) return
    const nextText = commentText.trim()
    if (!nextText) return
    sendRegionAnnotation(api, pendingRegionRect, pendingRegionSelectionIds, nextText)
    clearDraft()
  }, [api, clearDraft, commentText, pendingRegionRect, pendingRegionSelectionIds])

  const submitDrawing = useCallback(() => {
    commitDrawing()
    clearDraft()
  }, [clearDraft, commitDrawing])

  useEffect(
    () =>
      api.onAnnotateElementSelected((payload) => {
        const pending = buildPendingAnnotation(payload, layoutRef.current)
        if (!pending) return
        openDraft({ pending, elementName: payload.name?.trim() })
      }),
    [api, layoutRef, openDraft],
  )

  useEffect(
    () =>
      api.onRegionSelectCommitted(({ canvasRect }) => {
        openDraft({ regionRect: canvasRect })
      }),
    [api, openDraft],
  )

  useEffect(
    () =>
      // ADR 0006: comment-tool click that landed off-page (or in a page slot
      // with no DOM element) becomes a canvas-point pending annotation. We
      // mount the composer adjacent to the click in screen coords.
      api.onCommentCanvasPointCommitted(({ canvasX, canvasY }) => {
        openDraft({
          pending: buildCanvasPointPendingAnnotation(canvasX, canvasY, layoutRef.current),
        })
      }),
    [api, layoutRef, openDraft],
  )

  const activeToolKind = layoutData.activeTool.kind
  useEffect(() => {
    if (activeToolKind === 'comment') {
      // Comment tool now owns both element/canvas-point clicks and region
      // drags (ADR 0006). Drafts of either kind persist across these
      // gestures; only the (mutually exclusive) drawing session is cleared.
      if (drawingSession) {
        clearDrawing()
        setCommentText('')
      }
      return
    }
    if (pendingAnnotation) {
      setPendingAnnotation(null)
      setCommentText('')
    }
    // A selection-born region draft (the popup's Annotate button) isn't tied
    // to the comment tool being active — it opens over whatever tool the
    // selection popup was mounted under, so it's exempt from this
    // left-comment-tool cleanup. Cleared by clearDraft (submit / Escape /
    // click-outside) instead.
    if (pendingRegionRect && !pendingRegionSelectionIds) {
      setPendingRegionRect(null)
      setCommentText('')
    }
  }, [
    clearDrawing,
    drawingSession,
    activeToolKind,
    pendingAnnotation,
    pendingRegionRect,
    pendingRegionSelectionIds,
  ])

  // Leaving the draw tool commits whatever was drawn rather than dropping it.
  useEffect(() => {
    if (activeToolKind === 'draw' || !drawingSession) return
    commitDrawing()
    clearDraft()
  }, [clearDraft, commitDrawing, drawingSession, activeToolKind])

  useEffect(() => {
    if (!pendingAnnotation && !pendingRegionRect) return
    const id = window.requestAnimationFrame(() => {
      commentInputRef.current?.focus({ preventScroll: true })
    })
    return () => window.cancelAnimationFrame(id)
  }, [commentInputRef, pendingAnnotation, pendingRegionRect])

  return {
    beginSelectionAnnotation,
    clearDraft,
    commentText,
    drawingSession,
    drawingStrokeActive: drawing.drawingStrokeActive,
    elementNameDraft,
    pendingAnnotation,
    pendingRegionRect,
    pendingRegionSelectionIds,
    setCommentText,
    setDrawingSession,
    setDrawingStrokeActive: drawing.setDrawingStrokeActive,
    setElementNameDraft,
    setPendingAnnotation,
    submitDrawing,
    submitPendingAnnotation,
    submitRegionAnnotation,
    undoLastStroke: drawing.undoLastStroke,
  }
}

/**
 * The in-progress freehand drawing. Mutually exclusive with an annotation
 * draft, but its own concern: strokes accumulate across pointer gestures and
 * commit as one entity when the drawing ends.
 */
function useDrawingSession(
  api: CanvasBgElectronAPI,
  activeStrokeRef: React.MutableRefObject<{ pointerId: number; strokeId: string } | null>,
) {
  const [drawingSession, setDrawingSession] = useState<DrawingSession | null>(null)
  const [drawingStrokeActive, setDrawingStrokeActive] = useState(false)

  const clearDrawing = useCallback(() => {
    activeStrokeRef.current = null
    setDrawingSession(null)
    setDrawingStrokeActive(false)
  }, [activeStrokeRef])

  /** Writes the strokes out as a drawing entity. A stroke-less session is a
   *  no-op, so callers can commit unconditionally before clearing. */
  const commitDrawing = useCallback(() => {
    if (!drawingSession?.strokes.length) return
    api.createDrawing({
      canvasX: drawingSession.bounds.x,
      canvasY: drawingSession.bounds.y,
      width: drawingSession.bounds.width,
      height: drawingSession.bounds.height,
      strokes: drawingSession.strokes,
    })
  }, [api, drawingSession])

  const undoLastStroke = useCallback(() => {
    setDrawingSession((current) => {
      const remaining = current?.strokes.slice(0, -1)
      if (!remaining?.length) return null
      return { strokes: remaining, bounds: drawingBounds(remaining) }
    })
  }, [])

  return {
    clearDrawing,
    commitDrawing,
    drawingSession,
    drawingStrokeActive,
    setDrawingSession,
    setDrawingStrokeActive,
    undoLastStroke,
  }
}

/** Only element anchors carry a nameable target, and only a typed name counts. */
function elementNameField(
  pending: PendingAnnotation,
  draft: string,
): { elementName?: string } {
  const name = draft.trim()
  if (pending.request.anchor.type !== 'element' || !name) return {}
  return { elementName: name }
}

/**
 * A selection-born draft names the entities it was drawn over, so it goes
 * through the annotate-selection door; a comment-tool drag only has the rect.
 */
function sendRegionAnnotation(
  api: CanvasBgElectronAPI,
  rect: WorkspaceBounds,
  selectionIds: string[] | null,
  text: string,
): void {
  if (selectionIds) api.annotateSelection({ entityIds: selectionIds, text })
  else api.createRegionAnnotation(rect, text)
}

function buildPendingAnnotation(
  payload: AnnotationElementSelectionPayload,
  layout: LayoutUpdateData,
): PendingAnnotation | null {
  const page = layout.entities.find((candidate) => candidate.id === payload.pageId)
  if (!page) return null
  // No bounding box → anchor the composer to the page-content center (a
  // zero-size rect at the viewport midpoint maps to exactly that point).
  const rect = pageViewportToScreen(
    payload.boundingBox ?? { x: page.width / 2, y: page.height / 2, width: 0, height: 0 },
    page,
    layout,
  )
  const composerWidth = Math.min(420, window.innerWidth - VIEWPORT_PADDING * 2)
  const { composerX, composerY } = elementAnchoredComposerPosition({
    elementLeft: rect.left,
    elementTop: rect.top,
    elementHeight: rect.height,
    composerWidth,
  })
  const anchor: AnnotationAnchor = {
    type: 'element',
    pageId: payload.pageId,
    selector: payload.uniqueSelector || payload.elementPath,
    elementPath: payload.fullPath,
    boundingBox: payload.boundingBox,
  }
  return {
    draftId: makeDraftId(),
    request: {
      anchor,
      text: '',
      metadata: {
        inspectContext: payload,
      },
    },
    composerX,
    composerY,
    composerWidth,
  }
}

function makeDraftId(): string {
  return `draft:${Math.random().toString(36).slice(2, 10)}:${Date.now().toString(36)}`
}

function buildCanvasPointPendingAnnotation(
  canvasX: number,
  canvasY: number,
  layout: LayoutUpdateData,
): PendingAnnotation {
  // Anchor the composer just below + right of the click point in screen
  // coords. Coords are converted from canvas via the live layout (zoom +
  // pan) so the composer stays put even if pan/zoom changes between the
  // commit and the next layout broadcast.
  const screenX = canvasToScreenX(layout, canvasX)
  const screenY = canvasToScreenY(layout, canvasY)
  const composerWidth = Math.min(CANVAS_POINT_COMPOSER_WIDTH, window.innerWidth - VIEWPORT_PADDING * 2)
  const composerX = Math.min(
    Math.max(screenX + COMPOSER_MARGIN, VIEWPORT_PADDING),
    window.innerWidth - composerWidth - VIEWPORT_PADDING,
  )
  const overlayY = toOverlayY(layout, screenY) + COMPOSER_MARGIN
  const composerY = Math.min(
    Math.max(overlayY, VIEWPORT_PADDING),
    window.innerHeight - COMPOSER_MIN_HEIGHT - VIEWPORT_PADDING,
  )
  return {
    draftId: makeDraftId(),
    request: {
      anchor: { type: 'canvas', canvasX, canvasY },
      text: '',
    },
    composerX,
    composerY,
    composerWidth,
  }
}
