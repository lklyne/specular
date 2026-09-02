import { memo } from 'react'
import type { SelectionOverlayPayload } from '../../shared/types'

/**
 * Marquee rectangle layer — pointer-events: none visual only.
 * Driven by the `canvas-selection-overlay` IPC which canvas-bg's gesture
 * code sends via setSelectionOverlayRect.
 */
export const MarqueeLayer = memo(function MarqueeLayer({ overlay }: { overlay: SelectionOverlayPayload | null }) {
  if (!overlay) return null
  // Place-shape drag renders a live shape preview elsewhere; suppress the
  // marquee box so they don't double up.
  if (overlay.variant === 'place-shape') return null
  return (
    <div
      style={{
        position: 'absolute',
        left: overlay.rect.left,
        top: overlay.rect.top,
        width: overlay.rect.width,
        height: overlay.rect.height,
        boxSizing: 'border-box',
        // Same blue as the inspect-tool / per-item hover highlight, so the
        // marquee bbox reads like a "this is what you'd be selecting"
        // companion to the inner item outlines painted by each page.
        border: '1px solid rgba(59, 130, 246, 0.9)',
        background: 'rgba(59, 130, 246, 0.12)',
        pointerEvents: 'none',
        // Body layers (file/sticky/shape) wrap their cards in a viewport
        // div with `transform: scale`, which creates a stacking context.
        // Without an explicit z-index the marquee paints under opaque
        // bodies because it appears earlier in DOM order.
        zIndex: 10,
      }}
    />
  )
})
