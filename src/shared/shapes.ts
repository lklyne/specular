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
 * A shape that needs true (undistorted) curves at any aspect ratio adds a
 * `geometry(width, height)` builder instead: the renderer draws that path at
 * the entity's real size, in a viewBox to match, so nothing stretches. `path`
 * stays the normalized silhouette every picker glyph draws.
 *
 * ponytail: the remaining stretched rounded shapes (pill/cylinder/cloud)
 * distort their curves under non-square scaling — acceptable for flowchart use.
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
  /** True-size path builder, in canvas units. Present = draw this instead of
   *  stretching `path`. */
  geometry?: (width: number, height: number) => string
}

/**
 * Corner radius, in canvas units, for shapes with circular corners. Resizing a
 * shape lengthens its flat edges and leaves the corners alone; only once the
 * box is too small for a full corner does the radius shrink, evenly on all
 * four, to half the shorter side.
 */
export const CORNER_RADIUS = 24

function roundedRectPath(width: number, height: number, radius: number): string {
  const r = Math.max(0, Math.min(radius, width / 2, height / 2))
  if (r === 0) return `M0,0 H${width} V${height} H0 Z`
  return (
    `M${r},0 H${width - r} A${r},${r} 0 0 1 ${width},${r}` +
    ` V${height - r} A${r},${r} 0 0 1 ${width - r},${height}` +
    ` H${r} A${r},${r} 0 0 1 0,${height - r}` +
    ` V${r} A${r},${r} 0 0 1 ${r},0 Z`
  )
}

export const SHAPE_DEFS = [
  { kind: 'rectangle', label: 'Rectangle', path: 'M0,0 H100 V100 H0 Z' },
  {
    kind: 'rounded',
    label: 'Rounded rectangle',
    path: 'M15,0 H85 A15,15 0 0 1 100,15 V85 A15,15 0 0 1 85,100 H15 A15,15 0 0 1 0,85 V15 A15,15 0 0 1 15,0 Z',
    geometry: (w, h) => roundedRectPath(w, h, CORNER_RADIUS),
  },
  { kind: 'ellipse', label: 'Ellipse', path: 'M0,50 A50,50 0 1 1 100,50 A50,50 0 1 1 0,50 Z' },
  {
    kind: 'diamond',
    label: 'Diamond',
    path: 'M50,0 L100,50 L50,100 L0,50 Z',
    textInset: { x: 25, y: 25, w: 50, h: 50 },
  },
  {
    // Equilateral: height = width·√3/2, centered vertically in the square box.
    kind: 'triangle',
    label: 'Triangle',
    path: 'M50,6.7 L100,93.3 L0,93.3 Z',
    textInset: { x: 20, y: 48, w: 60, h: 42 },
  },
  {
    // Regular hexagon: height = width·√3/2, centered vertically in the square box.
    kind: 'hexagon',
    label: 'Hexagon',
    path: 'M25,6.7 H75 L100,50 L75,93.3 H25 L0,50 Z',
  },
  {
    kind: 'pill',
    label: 'Pill',
    // A pill is a stadium — semicircular caps on the short axis — so the glyph
    // draws it wide rather than filling the square box (which would read as a
    // circle).
    path: 'M30,20 H70 A30,30 0 0 1 70,80 H30 A30,30 0 0 1 30,20 Z',
    geometry: (w, h) => roundedRectPath(w, h, Math.min(w, h) / 2),
  },
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

export interface ShapeRender {
  d: string
  line?: string
  viewBox: string
}

/**
 * What to draw for a shape at a given size, in canvas units. `geometry` shapes
 * get a box-sized viewBox so their curves keep their true radius; the rest keep
 * the normalized box and stretch as before.
 */
export function shapeRender(def: ShapeDef, width: number, height: number): ShapeRender {
  if (def.geometry && width > 0 && height > 0) {
    return { d: def.geometry(width, height), viewBox: `0 0 ${width} ${height}` }
  }
  return { d: def.path, line: def.line, viewBox: '0 0 100 100' }
}

export function isShapeKind(value: unknown): value is ShapeKind {
  return typeof value === 'string' && BY_KIND.has(value)
}
