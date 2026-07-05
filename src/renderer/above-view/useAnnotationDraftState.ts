import { useCallback, useEffect, useState } from 'react'
import type { AnnotationAnchor, AnnotationElementSelectionPayload, LayoutUpdateData, WorkspaceBounds } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { canvasToScreenX, canvasToScreenY, toOverlayY } from '../../shared/gesture-utils'
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
  const [drawingSession, setDrawingSession] = useState<DrawingSession | null>(null)
  const [drawingStrokeActive, setDrawingStrokeActive] = useState(false)
  const [commentText, setCommentText] = useState('')
  const [elementNameDraft, setElementNameDraft] = useState('')

  const clearDraft = useCallback(() => {
    activeStrokeRef.current = null
    setPendingAnnotation(null)
    setPendingRegionRect(null)
    setDrawingSession(null)
    setDrawingStrokeActive(false)
    setCommentText('')
    setElementNameDraft('')
  }, [activeStrokeRef])

  const submitPendingAnnotation = useCallback(() => {
    if (!pendingAnnotation) return
    const nextText = commentText.trim()
    if (!nextText) return
    const trimmedName = elementNameDraft.trim()
    api.createAnnotation({
      ...pendingAnnotation.request,
      text: nextText,
      ...(pendingAnnotation.request.anchor.type === 'element' && trimmedName
        ? { elementName: trimmedName }
        : {}),
    })
    clearDraft()
  }, [api, clearDraft, commentText, elementNameDraft, pendingAnnotation])

  const submitRegionAnnotation = useCallback(() => {
    if (!pendingRegionRect) return
    const nextText = commentText.trim()
    if (!nextText) return
    api.createRegionAnnotation(pendingRegionRect, nextText)
    clearDraft()
  }, [api, clearDraft, commentText, pendingRegionRect])

  const submitDrawing = useCallback(() => {
    if (!drawingSession || !drawingSession.strokes.length) return
    api.createDrawing({
      canvasX: drawingSession.bounds.x,
      canvasY: drawingSession.bounds.y,
      width: drawingSession.bounds.width,
      height: drawingSession.bounds.height,
      strokes: drawingSession.strokes,
    })
    clearDraft()
  }, [api, clearDraft, drawingSession])

  const undoLastStroke = useCallback(() => {
    setDrawingSession((current) => {
      if (!current || current.strokes.length === 0) return null
      const nextStrokes = current.strokes.slice(0, -1)
      if (!nextStrokes.length) return null
      return {
        strokes: nextStrokes,
        bounds: drawingBounds(nextStrokes),
      }
    })
  }, [])

  useEffect(() => {
    const cleanup = api.onAnnotateElementSelected((payload) => {
      const pending = buildPendingAnnotation(payload, layoutRef.current)
      if (!pending) return
      setPendingAnnotation(pending)
      setDrawingSession(null)
      setCommentText('')
      setElementNameDraft(payload.name?.trim() ?? '')
    })
    return cleanup
  }, [api, layoutRef])

  useEffect(() => {
    const cleanup = api.onRegionSelectCommitted(({ canvasRect }) => {
      setPendingRegionRect(canvasRect)
      setPendingAnnotation(null)
      setDrawingSession(null)
      setCommentText('')
      setElementNameDraft('')
    })
    return cleanup
  }, [api])

  useEffect(() => {
    // ADR 0006: comment-tool click that landed off-page (or in a page slot
    // with no DOM element) becomes a canvas-point pending annotation. We
    // mount the composer adjacent to the click in screen coords.
    const cleanup = api.onCommentCanvasPointCommitted(({ canvasX, canvasY }) => {
      const pending = buildCanvasPointPendingAnnotation(canvasX, canvasY, layoutRef.current)
      setPendingAnnotation(pending)
      setPendingRegionRect(null)
      setDrawingSession(null)
      setCommentText('')
      setElementNameDraft('')
    })
    return cleanup
  }, [api, layoutRef])

  const activeToolKind = layoutData.activeTool.kind
  useEffect(() => {
    if (activeToolKind === 'comment') {
      // Comment tool now owns both element/canvas-point clicks and region
      // drags (ADR 0006). Drafts of either kind persist across these
      // gestures; only the (mutually exclusive) drawing session is cleared.
      if (drawingSession) {
        activeStrokeRef.current = null
        setDrawingSession(null)
        setCommentText('')
      }
      return
    }
    if (pendingAnnotation) {
      setPendingAnnotation(null)
      setCommentText('')
    }
    if (pendingRegionRect) {
      setPendingRegionRect(null)
      setCommentText('')
    }
  }, [activeStrokeRef, drawingSession, activeToolKind, pendingAnnotation, pendingRegionRect])

  useEffect(() => {
    if (activeToolKind === 'draw') return
    if (!drawingSession) return
    if (drawingSession.strokes.length > 0) {
      api.createDrawing({
        canvasX: drawingSession.bounds.x,
        canvasY: drawingSession.bounds.y,
        width: drawingSession.bounds.width,
        height: drawingSession.bounds.height,
        strokes: drawingSession.strokes,
      })
    }
    clearDraft()
  }, [api, clearDraft, drawingSession, activeToolKind])

  useEffect(() => {
    if (!pendingAnnotation && !pendingRegionRect) return
    const id = window.requestAnimationFrame(() => {
      commentInputRef.current?.focus({ preventScroll: true })
    })
    return () => window.cancelAnimationFrame(id)
  }, [commentInputRef, pendingAnnotation, pendingRegionRect])

  return {
    clearDraft,
    commentText,
    drawingSession,
    drawingStrokeActive,
    elementNameDraft,
    pendingAnnotation,
    pendingRegionRect,
    setCommentText,
    setDrawingSession,
    setDrawingStrokeActive,
    setElementNameDraft,
    setPendingAnnotation,
    submitDrawing,
    submitPendingAnnotation,
    submitRegionAnnotation,
    undoLastStroke,
  }
}

function buildPendingAnnotation(
  payload: AnnotationElementSelectionPayload,
  layout: LayoutUpdateData,
): PendingAnnotation | null {
  const page = layout.entities.find((candidate) => candidate.id === payload.pageId)
  if (!page) return null
  const bb = payload.boundingBox
  const contentScreenX =
    'contentScreenX' in page && page.contentScreenX != null ? page.contentScreenX : page.screenX
  const contentScreenY =
    'contentScreenY' in page && page.contentScreenY != null ? page.contentScreenY : page.screenY
  const contentScreenWidth =
    'contentScreenWidth' in page && page.contentScreenWidth != null
      ? page.contentScreenWidth
      : page.screenWidth
  const contentScreenHeight =
    'contentScreenHeight' in page && page.contentScreenHeight != null
      ? page.contentScreenHeight
      : page.screenHeight
  const scaleX = contentScreenWidth / page.width
  const scaleY = contentScreenHeight / page.height
  const elementLeft = contentScreenX + (bb ? bb.x * scaleX : contentScreenWidth / 2)
  const elementTop = toOverlayY(
    layout,
    contentScreenY + (bb ? bb.y * scaleY : contentScreenHeight / 2),
  )
  const elementHeight = bb ? bb.height * scaleY : 0
  const composerWidth = Math.min(420, window.innerWidth - VIEWPORT_PADDING * 2)
  const { composerX, composerY } = elementAnchoredComposerPosition({
    elementLeft,
    elementTop,
    elementHeight,
    composerWidth,
  })
  const anchor: AnnotationAnchor = {
    type: 'element',
    pageId: payload.pageId,
    selector: payload.elementPath,
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
