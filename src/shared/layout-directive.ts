import type { SpacingToken } from './types'

// All multiples of GRID_SIZE (20px) so token-spaced gaps stay snap-aligned.
export const SPACING_TOKEN_PIXELS: Record<SpacingToken, number> = {
  xs: 20,
  s: 40,
  m: 60,
  l: 100,
  xl: 160,
}

export function resolveSpacing(value: number | SpacingToken | undefined, fallback: number): number {
  if (value === undefined) return fallback
  if (typeof value === 'number') return value
  return SPACING_TOKEN_PIXELS[value] ?? fallback
}

const SPACING_TOKEN_NAMES: ReadonlySet<string> = new Set(Object.keys(SPACING_TOKEN_PIXELS))

function isSpacingValue(v: unknown): boolean {
  return typeof v === 'number' || (typeof v === 'string' && SPACING_TOKEN_NAMES.has(v))
}

/**
 * Validate an unknown value as a `LayoutDirective`. Returns null on success,
 * or a human-readable error string describing the first problem found. Call
 * at the boundary (CLI/HTTP) so bad agent input fails loudly instead of
 * silently falling through to defaults.
 */
export function validateLayoutDirective(value: unknown): string | null {
  if (!value || typeof value !== 'object') return 'layout: expected an object'
  const d = value as Record<string, unknown>
  if (d.kind !== 'row' && d.kind !== 'column' && d.kind !== 'grid') {
    return `layout.kind: expected 'row' | 'column' | 'grid', got ${JSON.stringify(d.kind)}`
  }
  for (const key of ['gap', 'rowGap', 'colGap'] as const) {
    if (d[key] !== undefined && !isSpacingValue(d[key])) {
      return `layout.${key}: expected number or one of ${[...SPACING_TOKEN_NAMES].join('|')}, got ${JSON.stringify(d[key])}`
    }
  }
  if (d.cols !== undefined && (typeof d.cols !== 'number' || !Number.isInteger(d.cols) || d.cols < 1)) {
    return `layout.cols: expected positive integer, got ${JSON.stringify(d.cols)}`
  }
  for (const key of ['originX', 'originY'] as const) {
    if (d[key] !== undefined && typeof d[key] !== 'number') {
      return `layout.${key}: expected number, got ${JSON.stringify(d[key])}`
    }
  }
  if (d.near !== undefined && typeof d.near !== 'string') {
    return `layout.near: expected entity id string, got ${JSON.stringify(d.near)}`
  }
  if ((d.originX === undefined) !== (d.originY === undefined)) {
    return 'layout: originX and originY must be specified together'
  }
  return null
}
