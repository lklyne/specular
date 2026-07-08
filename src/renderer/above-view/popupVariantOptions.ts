// ADR 0008 §5 — shared variant/stroke-width options across popups + toolbar.

import type { ComponentType } from 'react'
import type { DrawingBrushType } from '../../shared/types'
import { PenMarkerIcon, PenSlimIcon } from '../shared/CustomIcons'

export const BRUSH_VARIANT_OPTIONS: Array<{
  kind: DrawingBrushType
  label: string
  Icon: ComponentType<{ size?: number; ink?: string; selected?: boolean }>
}> = [
  { kind: 'pen', label: 'Pen', Icon: PenSlimIcon },
  { kind: 'highlight', label: 'Highlighter', Icon: PenMarkerIcon },
]

// Ordered thin → thick.
const STROKE_WIDTH_PRESETS = [2, 4] as const
const HIGHLIGHT_STROKE_WIDTH_PRESETS = [8, 16] as const

export function strokeWidthPresetsFor(
  brushType: DrawingBrushType | undefined,
): readonly number[] {
  return brushType === 'highlight' ? HIGHLIGHT_STROKE_WIDTH_PRESETS : STROKE_WIDTH_PRESETS
}

/** Closest preset to a given width — used to highlight the current swatch. */
export function nearestStrokeWidthPreset(
  value: number,
  presets: readonly number[] = STROKE_WIDTH_PRESETS,
): number {
  let best = presets[0]
  let bestDist = Math.abs(value - best)
  for (const preset of presets) {
    const dist = Math.abs(value - preset)
    if (dist < bestDist) {
      best = preset
      bestDist = dist
    }
  }
  return best
}
