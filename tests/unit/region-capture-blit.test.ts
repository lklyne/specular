/**
 * A region wider or taller than the window can only capture the part of a view
 * that is actually on screen. The composite places those pixels at the offset
 * they came from and leaves the canvas-background fill everywhere else — an
 * all-or-nothing dimension check instead dropped the whole layer, which is how
 * a selection of notes with no pages in it produced a blank gray screenshot.
 *
 * Mutation-verified by: ignoring capture.offsetX/offsetY (the offset case puts
 * pixels at 0,0 and fails); dropping the rowEnd/colEnd clamps (the overflow
 * case throws / writes past the buffer).
 */

import { describe, expect, it } from 'vitest'
import { blitCapture } from '../../src/main/runtime/region-capture'

/** Solid-color RGBA bitmap. */
function bitmap(w: number, h: number, rgb: [number, number, number], alpha = 255): Buffer {
  const buf = Buffer.alloc(w * h * 4)
  for (let i = 0; i < buf.length; i += 4) {
    buf[i] = rgb[0]; buf[i + 1] = rgb[1]; buf[i + 2] = rgb[2]; buf[i + 3] = alpha
  }
  return buf
}

function pixel(buf: Buffer, w: number, x: number, y: number): [number, number, number] {
  const i = (y * w + x) * 4
  return [buf[i], buf[i + 1], buf[i + 2]]
}

const FILL: [number, number, number] = [0xf5, 0xf5, 0xf5]

function fill(w: number, h: number): Buffer {
  return bitmap(w, h, FILL)
}

describe('blitCapture', () => {
  it('places a partial capture at its offset and leaves the rest filled', () => {
    const out = fill(4, 4)
    // Only the bottom-right 2×2 of the region was on screen.
    blitCapture(
      { bitmap: bitmap(2, 2, [10, 20, 30]), width: 2, height: 2, offsetX: 2, offsetY: 2 },
      out,
      4,
      4,
      { blend: false },
    )
    expect(pixel(out, 4, 3, 3)).toEqual([10, 20, 30])
    expect(pixel(out, 4, 2, 2)).toEqual([10, 20, 30])
    expect(pixel(out, 4, 0, 0)).toEqual(FILL)
    expect(pixel(out, 4, 1, 3)).toEqual(FILL)
  })

  it('clips a capture that overflows the output instead of writing past it', () => {
    const out = fill(2, 2)
    blitCapture(
      { bitmap: bitmap(4, 4, [1, 2, 3]), width: 4, height: 4, offsetX: 1, offsetY: 1 },
      out,
      2,
      2,
      { blend: false },
    )
    expect(pixel(out, 2, 1, 1)).toEqual([1, 2, 3])
    expect(pixel(out, 2, 0, 0)).toEqual(FILL)
    expect(out.length).toBe(2 * 2 * 4)
  })

  it('blends translucent overlay pixels and skips fully transparent ones', () => {
    const out = fill(1, 2)
    const src = Buffer.concat([
      bitmap(1, 1, [0, 0, 0], 0), // transparent — must not paint
      bitmap(1, 1, [0, 0, 0], 255), // opaque black
    ])
    blitCapture({ bitmap: src, width: 1, height: 2 }, out, 1, 2, { blend: true })
    expect(pixel(out, 1, 0, 0)).toEqual(FILL)
    expect(pixel(out, 1, 0, 1)).toEqual([0, 0, 0])
  })
})
