/**
 * Shared scaffold for the aboveView body layers (file, shape, sticky, group
 * bounds): a viewport transform wrapper plus the per-entity shell div.
 */

import type { SceneView } from '../../shared/scene-projection'

/**
 * Wraps a body layer's cards in a viewport transform so they live in
 * canvas-coordinate space. AboveView's WCV origin already sits at
 * `canvasOrigin.y` (the toolbar inset), so the translate omits that axis
 * — only `canvasOrigin.x` and `pan` apply.
 */
export function CanvasViewportLayer({
  view,
  children,
}: {
  view: SceneView
  children: React.ReactNode
}) {
  return (
    <div
      className="pointer-events-none absolute left-0 top-0 origin-top-left"
      style={{
        ['--canvas-zoom' as string]: view.zoom,
        transform: `translate(${view.canvasOrigin.x + view.pan.x}px, ${view.pan.y}px) scale(${view.zoom})`,
      }}
    >
      {children}
    </div>
  )
}

/**
 * The absolutely-positioned per-entity container inside a viewport layer.
 * `data-entity-id` is what the pointer router's DOM lookups key on. Sizing,
 * background, and overflow are per-entity-kind, passed via `style`.
 */
export function EntityShell({
  id,
  canvasX,
  canvasY,
  style,
  shellRef,
  children,
}: {
  id: string
  canvasX: number
  canvasY: number
  style?: React.CSSProperties
  shellRef?: React.Ref<HTMLDivElement>
  children: React.ReactNode
}) {
  return (
    <div
      ref={shellRef}
      data-entity-id={id}
      className="absolute pointer-events-auto"
      style={{
        left: canvasX,
        top: canvasY,
        cursor: 'default',
        touchAction: 'none',
        ...style,
      }}
    >
      {children}
    </div>
  )
}
