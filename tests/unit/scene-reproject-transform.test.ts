import { describe, expect, it } from 'vitest'
import {
  sceneReprojectTransform,
  type Camera,
} from '../../src/renderer/shared/hooks/useSceneCamera'

// The scene container holds children at their payload-camera positions; the
// transform must reproject them to the live camera. Local position of a canvas
// point is `origin + canvas*zoom + pan`, with origin.x = canvasOrigin.x and
// origin.y = originLocalY (0 for the inset overlays, canvasOrigin.y for bgView).
function parse(transform: string): { s: number; tx: number; ty: number } {
  const m = transform.match(/translate\((-?[\d.]+)px, (-?[\d.]+)px\) scale\((-?[\d.]+)\)/)
  if (!m) throw new Error(`unparseable transform: ${transform}`)
  return { tx: Number(m[1]), ty: Number(m[2]), s: Number(m[3]) }
}

function localPos(
  canvas: { x: number; y: number },
  cam: Camera,
  canvasOrigin: { x: number; y: number },
  originLocalY: number,
) {
  return {
    x: canvasOrigin.x + canvas.x * cam.zoom + cam.pan.x,
    y: originLocalY + canvas.y * cam.zoom + cam.pan.y,
  }
}

const canvasOrigin = { x: 180, y: 44 }
const CANVAS_POINTS = [
  { x: 0, y: 0 },
  { x: 320, y: 240 },
  { x: -1500, y: 900 },
]

describe('sceneReprojectTransform', () => {
  for (const originLocalY of [0, canvasOrigin.y]) {
    for (const [z0, z1] of [
      [1, 1], // pan only
      [1, 2],
      [2, 1],
      [0.5, 1.75],
      [3, 0.4],
    ]) {
      it(`round-trips canvas points at zoom ${z0}->${z1}, originLocalY=${originLocalY}`, () => {
        const payload = { pan: { x: -200, y: 130 }, zoom: z0, canvasOrigin }
        const live: Camera = { pan: { x: 640, y: -75 }, zoom: z1 }
        const { s, tx, ty } = parse(sceneReprojectTransform(payload, live, originLocalY))

        for (const c of CANVAS_POINTS) {
          const l0 = localPos(c, payload, canvasOrigin, originLocalY)
          const l1 = localPos(c, live, canvasOrigin, originLocalY)
          // Applying the CSS transform (about the top-left origin) to the
          // payload position must land on the live position, within 0.5px.
          expect(s * l0.x + tx).toBeCloseTo(l1.x, 1)
          expect(s * l0.y + ty).toBeCloseTo(l1.y, 1)
        }
      })
    }
  }

  it('degrades to a pure translate by the pan delta when zoom is unchanged', () => {
    const payload = { pan: { x: 100, y: 100 }, zoom: 1.5, canvasOrigin }
    const live: Camera = { pan: { x: 260, y: 40 }, zoom: 1.5 }
    const { s, tx, ty } = parse(sceneReprojectTransform(payload, live, 0))
    expect(s).toBeCloseTo(1, 6)
    expect(tx).toBeCloseTo(160, 6) // 260 - 100
    expect(ty).toBeCloseTo(-60, 6) // 40 - 100
  })

  it('is identity when the live camera equals the payload camera', () => {
    const payload = { pan: { x: 12, y: 34 }, zoom: 2, canvasOrigin }
    const { s, tx, ty } = parse(
      sceneReprojectTransform(payload, { pan: { x: 12, y: 34 }, zoom: 2 }, canvasOrigin.y),
    )
    expect(s).toBeCloseTo(1, 6)
    expect(tx).toBeCloseTo(0, 6)
    expect(ty).toBeCloseTo(0, 6)
  })
})
