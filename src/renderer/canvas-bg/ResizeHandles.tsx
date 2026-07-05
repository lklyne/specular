import type { ResizeCorner, ResizeEdge } from './entityConstants'
import { HANDLE_SIZE, CORNER_CURSORS, EDGE_CURSORS } from './entityConstants'
import { selectionColor } from './canvasBgConstants'

// Visual-only handles — canvas resize gestures are routed through aboveView's
// canvas pointer router, which hit-tests the `data-resize-handle` attribute.

const HALF = HANDLE_SIZE / 2
const NEG_HALF = -(HANDLE_SIZE / 2)

export function CornerResizeHandle({
  corner,
  isDark,
}: {
  corner: ResizeCorner
  isDark: boolean
}) {
  const pos: React.CSSProperties =
    corner === 'top-left' ? { top: NEG_HALF, left: NEG_HALF } :
    corner === 'top-right' ? { top: NEG_HALF, right: NEG_HALF } :
    corner === 'bottom-left' ? { bottom: NEG_HALF, left: NEG_HALF } :
    { bottom: NEG_HALF, right: NEG_HALF }

  return (
    <div
      data-resize-handle
      data-overlay-ui
      style={{
        position: 'absolute',
        ...pos,
        width: HANDLE_SIZE,
        height: HANDLE_SIZE,
        boxSizing: 'border-box',
        background: 'white',
        border: `1px solid ${selectionColor(isDark)}`,
        borderRadius: 0,
        cursor: CORNER_CURSORS[corner],
        pointerEvents: 'auto',
        zIndex: 1,
      }}
    />
  )
}

export function EdgeResizeHandle({
  edge,
}: {
  edge: ResizeEdge
}) {
  const isHorizontal = edge === 'top' || edge === 'bottom'
  const pos: React.CSSProperties =
    edge === 'top' ? { top: NEG_HALF, left: HALF, right: HALF } :
    edge === 'bottom' ? { bottom: NEG_HALF, left: HALF, right: HALF } :
    edge === 'left' ? { left: NEG_HALF, top: HALF, bottom: HALF } :
    { right: NEG_HALF, top: HALF, bottom: HALF }

  return (
    <div
      data-resize-handle
      data-overlay-ui
      style={{
        position: 'absolute',
        ...pos,
        width: isHorizontal ? undefined : HANDLE_SIZE,
        height: isHorizontal ? HANDLE_SIZE : undefined,
        cursor: EDGE_CURSORS[edge],
        pointerEvents: 'auto',
        zIndex: 1,
      }}
    />
  )
}
