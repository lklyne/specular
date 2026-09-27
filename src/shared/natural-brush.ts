// Presets for the `brush` draw type, rendered by webgpu-brush. Each id is a
// built-in webgpu-brush brush name, so it is passed straight to `brush.set()`.
// `spray` and `pastel` are left out: their scatter runs 6–30x the stroke
// width, which reads as noise at annotation sizes.

export const NATURAL_BRUSH_PRESETS = [
  'charcoal',
  '2B',
  'HB',
  'cpencil',
  'crayon',
  'marker',
  'rotring',
] as const

export type NaturalBrushPreset = (typeof NATURAL_BRUSH_PRESETS)[number]

export const DEFAULT_NATURAL_BRUSH_PRESET: NaturalBrushPreset = 'charcoal'

export const NATURAL_BRUSH_PRESET_LABELS: Record<NaturalBrushPreset, string> = {
  charcoal: 'Charcoal',
  '2B': 'Soft pencil',
  HB: 'Pencil',
  cpencil: 'Colored pencil',
  crayon: 'Crayon',
  marker: 'Marker',
  rotring: 'Fineliner',
}

/**
 * Each preset's tip size per unit of webgpu-brush weight (its `param.weight`).
 * Dividing a stroke's pixel width by this keeps every preset at the width the
 * picker shows, instead of the brush's own scale (marker is ~7x pen).
 */
export const NATURAL_BRUSH_TIP_SIZE: Record<NaturalBrushPreset, number> = {
  charcoal: 0.35,
  '2B': 0.3,
  HB: 0.3,
  cpencil: 0.35,
  crayon: 0.33,
  marker: 2,
  rotring: 0.15,
}

export function isNaturalBrushPreset(value: unknown): value is NaturalBrushPreset {
  return (NATURAL_BRUSH_PRESETS as readonly unknown[]).includes(value)
}
