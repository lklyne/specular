// One glyph for any shape kind, drawn from the same 0–100 path the canvas
// renders. Used by every shape picker so the icon always matches the drawn
// shape (and no per-shape icon asset is needed).

import { shapeDef } from '../../shared/shapes'

export function ShapeGlyph({
  kind,
  size = 14,
}: {
  kind: string
  size?: number
}) {
  const def = shapeDef(kind)
  return (
    <svg width={size} height={size} viewBox="-6 -6 112 112" fill="none" aria-hidden>
      <path
        d={def.path}
        fill="none"
        stroke="currentColor"
        strokeWidth={8}
        strokeLinejoin="round"
        strokeLinecap="round"
      />
      {def.line ? (
        <path d={def.line} fill="none" stroke="currentColor" strokeWidth={8} strokeLinecap="round" />
      ) : null}
    </svg>
  )
}
