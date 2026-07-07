/**
 * The shape catalog — one row per shape kind. This is the single source of
 * truth for what shapes exist: `ShapeKind` derives from it, the renderer draws
 * `path`, and every picker (popup grid, tool popup, right panel, sidebar) reads
 * its `label` + `path`. Adding a shape = adding a row here.
 *
 * Each `path` is an SVG path in a normalized 0–100 box. The renderer draws it
 * with `preserveAspectRatio="none"` + `vectorEffect="non-scaling-stroke"`, so
 * the geometry stretches to any width/height while the border stays uniform.
 *
 * ponytail: shapes are one SVG path in a 0–100 box. Straight-edged shapes
 * stretch cleanly; rounded ones (rounded/pill/cylinder/cloud) distort their
 * curves under non-square scaling — acceptable for flowchart use. If a shape
 * ever needs true corners at any aspect ratio, give it a CSS render path.
 */

/** Where a shape's text label sits, as % of the bounding box. Omit = full box. */
export interface ShapeTextInset {
  x: number
  y: number
  w: number
  h: number
}

export interface ShapeDef {
  kind: string
  label: string
  /** Filled silhouette, normalized 0–100. */
  path: string
  /** Optional stroke-only overlay (e.g. a cylinder's top rim). No fill. */
  line?: string
  /** Label box, if the full bounding box would overflow the silhouette. */
  textInset?: ShapeTextInset
}

export const SHAPE_DEFS = [
  { kind: 'rectangle', label: 'Rectangle', path: 'M0,0 H100 V100 H0 Z' },
  {
    kind: 'rounded',
    label: 'Rounded rectangle',
    path: 'M15,0 H85 A15,15 0 0 1 100,15 V85 A15,15 0 0 1 85,100 H15 A15,15 0 0 1 0,85 V15 A15,15 0 0 1 15,0 Z',
  },
  { kind: 'ellipse', label: 'Ellipse', path: 'M0,50 A50,50 0 1 1 100,50 A50,50 0 1 1 0,50 Z' },
  {
    kind: 'diamond',
    label: 'Diamond',
    path: 'M50,0 L100,50 L50,100 L0,50 Z',
    textInset: { x: 25, y: 25, w: 50, h: 50 },
  },
  {
    kind: 'triangle',
    label: 'Triangle',
    path: 'M50,0 L100,100 L0,100 Z',
    textInset: { x: 20, y: 48, w: 60, h: 48 },
  },
  { kind: 'hexagon', label: 'Hexagon', path: 'M25,0 H75 L100,50 L75,100 H25 L0,50 Z' },
  { kind: 'pill', label: 'Pill', path: 'M25,0 H75 A25,50 0 0 1 75,100 H25 A25,50 0 0 1 25,0 Z' },
  {
    kind: 'parallelogram',
    label: 'Parallelogram',
    path: 'M22,0 H100 L78,100 H0 Z',
  },
  {
    kind: 'chevron',
    label: 'Chevron',
    path: 'M0,0 H70 L100,50 L70,100 H0 L30,50 Z',
    textInset: { x: 5, y: 15, w: 60, h: 70 },
  },
  {
    kind: 'cylinder',
    label: 'Cylinder',
    path: 'M0,15 A50,15 0 0 1 100,15 L100,85 A50,15 0 0 1 0,85 Z',
    line: 'M0,15 A50,15 0 0 0 100,15',
    textInset: { x: 8, y: 28, w: 84, h: 58 },
  },
] as const satisfies readonly ShapeDef[]

export type ShapeKind = (typeof SHAPE_DEFS)[number]['kind']

const BY_KIND = new Map<string, ShapeDef>(SHAPE_DEFS.map((d) => [d.kind, d]))

/** The def for a kind; falls back to rectangle for unknown/legacy values. */
export function shapeDef(kind: string): ShapeDef {
  return BY_KIND.get(kind) ?? SHAPE_DEFS[0]
}

export function isShapeKind(value: unknown): value is ShapeKind {
  return typeof value === 'string' && BY_KIND.has(value)
}
