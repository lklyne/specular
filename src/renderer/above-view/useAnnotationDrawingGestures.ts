import type { LayoutSnapshotRef } from '../shared/hooks/useProjectedLayoutRef'
import { useCallback, useRef } from 'react'
import type { LayoutUpdateData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import {
  clientYToWindowY,
  isOverlayUiTarget,
  screenPointToCanvasPoint,
} from '../../shared/gesture-utils'
import { drawingBounds, snapPointTo45Degrees, type DrawingSession } from './annotationMath'

export function useAnnotationDrawingGestures({
  api,
  clearDraft,
  closeThread,
  drawInteractionEnabled,
  layoutData,
  layoutRef,
  pendingAnnotation,
  activeStrokeRef,
  setDrawingSession,
  setDrawingStrokeActive,
  setPendingAnnotation,
}: {
  api: CanvasBgElectronAPI
  clearDraft: () => void
  closeThread: () => void
  drawInteractionEnabled: boolean
  layoutData: LayoutUpdateData
  layoutRef: LayoutSnapshotRef
  pendingAnnotation: unknown
  activeStrokeRef: React.MutableRefObject<{ pointerId: number; strokeId: string } | null>
  setDrawingSession: React.Dispatch<
    React.SetStateAction<import('./annotationMath').DrawingSession | null>
  >
  setDrawingStrokeActive: React.Dispatch<React.SetStateAction<boolean>>
  setPendingAnnotation: React.Dispatch<
    React.SetStateAction<import('./annotationMath').PendingAnnotation | null>
  >
}) {
  const sessionRef = useRef<DrawingSession | null>(null)

  const handleOverlayPointerDown = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      if (drawInteractionEnabled) {
        if (event.pointerType === 'mouse' && event.button !== 0) return
        if (event.clientY < layoutData.canvasOrigin.y) return
        if (event.clientX < layoutData.leftChromeWidth) {
          return
        }
        if (isOverlayUiTarget(event.target)) return

        // Clear any existing selection — draw mode only adds new strokes, never selects.
        if (layoutData.selectedEntityIds.length > 0) {
          api.selectEntities([])
        }

        const strokeId = `stroke_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`
        const startPoint = screenPointToCanvasPoint(
          event.clientX,
          clientYToWindowY(event.clientY, layoutRef.current),
          layoutRef.current,
        )
        activeStrokeRef.current = { pointerId: event.pointerId, strokeId }
        setDrawingStrokeActive(true)
        closeThread()
        setPendingAnnotation(null)
        // Brush, color, and stroke width come from per-tool defaults
        // (ADR 0009). The draw tool's popup writes them; the gesture reads
        // them at stroke-start time. Color is stored raw (a preset number or
        // the 'neutral' sentinel) — DrawingsLayer resolves the palette from
        // the stroke's brush at render time.
        const drawDefaults = layoutRef.current.toolDefaults.draw
        const nextStrokes = [
          {
            id: strokeId,
            color: drawDefaults.color,
            width: drawDefaults.strokeWidth,
            points: [startPoint],
            brushType: drawDefaults.brushType,
          },
        ]
        const nextSession: DrawingSession = {
          strokes: nextStrokes,
          bounds: drawingBounds(nextStrokes),
        }
        sessionRef.current = nextSession
        setDrawingSession(nextSession)
        event.currentTarget.setPointerCapture(event.pointerId)
        event.preventDefault()
        return
      }

      if (!pendingAnnotation) return
      if (event.pointerType === 'mouse' && event.button !== 0) return
      if (isOverlayUiTarget(event.target)) return
      clearDraft()
    },
    [
      activeStrokeRef,
      api,
      clearDraft,
      closeThread,
      drawInteractionEnabled,
      layoutData.leftChromeWidth,
      layoutData.canvasOrigin.y,
      layoutData.selectedEntityIds,
      layoutRef,
      pendingAnnotation,
      setDrawingSession,
      setDrawingStrokeActive,
      setPendingAnnotation,
    ],
  )

  const handleOverlayPointerMove = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      const activeStroke = activeStrokeRef.current
      if (!activeStroke) return
      if (event.pointerId !== activeStroke.pointerId) return
      const pointerPoint = screenPointToCanvasPoint(
        event.clientX,
        clientYToWindowY(event.clientY, layoutRef.current),
        layoutRef.current,
      )
      const current = sessionRef.current
      if (!current) return
      const nextStrokes = current.strokes.map((stroke) =>
        stroke.id === activeStroke.strokeId
          ? {
              ...stroke,
              points: [
                ...stroke.points,
                event.shiftKey
                  ? snapPointTo45Degrees(stroke.points[0], pointerPoint)
                  : pointerPoint,
              ],
            }
          : stroke,
      )
      const nextSession: DrawingSession = {
        strokes: nextStrokes,
        bounds: drawingBounds(nextStrokes),
      }
      sessionRef.current = nextSession
      setDrawingSession(nextSession)
    },
    [activeStrokeRef, api, layoutRef, setDrawingSession],
  )

  const handleOverlayPointerUp = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      if (activeStrokeRef.current?.pointerId !== event.pointerId) return
      activeStrokeRef.current = null
      setDrawingStrokeActive(false)
      if (event.currentTarget.hasPointerCapture(event.pointerId)) {
        event.currentTarget.releasePointerCapture(event.pointerId)
      }
      const finished = sessionRef.current
      sessionRef.current = null
      setDrawingSession(null)
      if (finished && finished.strokes.length) {
        api.createDrawing({
          canvasX: finished.bounds.x,
          canvasY: finished.bounds.y,
          width: finished.bounds.width,
          height: finished.bounds.height,
          strokes: finished.strokes,
        })
      }
    },
    [activeStrokeRef, api, setDrawingSession, setDrawingStrokeActive],
  )

  const handleOverlayPointerCancel = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      if (activeStrokeRef.current?.pointerId !== event.pointerId) return
      activeStrokeRef.current = null
      setDrawingStrokeActive(false)
      if (event.currentTarget.hasPointerCapture(event.pointerId)) {
        event.currentTarget.releasePointerCapture(event.pointerId)
      }
    },
    [activeStrokeRef, setDrawingStrokeActive],
  )

  return {
    handleOverlayPointerCancel,
    handleOverlayPointerDown,
    handleOverlayPointerMove,
    handleOverlayPointerUp,
  }
}
