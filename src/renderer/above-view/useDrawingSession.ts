import { useCallback, useEffect, useState } from 'react'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import type { ToolKind } from '../../shared/tool'
import { drawingBounds, type DrawingSession } from './annotationMath'

/**
 * The in-progress freehand drawing. Strokes accumulate across pointer
 * gestures and commit as one entity when the drawing ends. Leaving the draw
 * tool mid-stroke commits whatever was drawn rather than dropping it.
 */
export function useDrawingSession(
  api: CanvasBgElectronAPI,
  activeStrokeRef: React.MutableRefObject<{ pointerId: number; strokeId: string } | null>,
  activeToolKind: ToolKind,
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

  useEffect(() => {
    if (activeToolKind === 'draw' || !drawingSession) return
    commitDrawing()
    clearDrawing()
  }, [activeToolKind, clearDrawing, commitDrawing, drawingSession])

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
