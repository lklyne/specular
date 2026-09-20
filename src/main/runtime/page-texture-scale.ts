/**
 * Texture-resolution LOD for offscreen pages.
 *
 * A page's texture is its view size × the host's device scale factor, and
 * every frame pays for those pixels three times over: the capturer's
 * full-frame blit, canvas-bg's ImageBitmap copy, and the pooled IOSurfaces
 * that back both (measured at 60 pages: 3.7GB of GPU-process footprint at
 * full size, 1.2GB at a quarter). A page showing at a fraction of its CSS
 * size cannot display that detail, so its host view shrinks by `k` while
 * device emulation keeps the document laid out at the full CSS viewport
 * (`applyTextureScale` in `page-host.ts`).
 *
 * `k` never drops below the on-screen scale, so a texture always carries at
 * least the pixels the screen shows and sharpness is never traded.
 */

export const FULL_TEXTURE_SCALE = 1

/** Ordered large-to-small. */
const SCALES: readonly number[] = [FULL_TEXTURE_SCALE, 0.5, 0.25]

/**
 * How far below a boundary the display scale must fall before the texture
 * shrinks to it. Growing has no margin — a texture thinner than the screen
 * shows as blur, so it is owed at once; shrinking is only ever a saving.
 */
const SHRINK_MARGIN = 0.8

/** The smallest texture scale that still out-resolves `displayScale`. */
function smallestSharpScale(displayScale: number): number {
  let best = FULL_TEXTURE_SCALE
  for (const scale of SCALES) if (scale >= displayScale) best = scale
  return best
}

/**
 * The texture scale a page showing at `displayScale` (screen px per CSS px)
 * should paint at, given the scale it paints at now. Pass the returned value
 * back as `currentScale` on the next evaluation — the pair is what keeps a
 * zoom hovering near a boundary from resizing the view back and forth.
 */
export function textureScaleForDisplayScale(displayScale: number, currentScale: number): number {
  const sharp = smallestSharpScale(displayScale)
  if (sharp >= currentScale) return sharp
  return Math.min(currentScale, smallestSharpScale(displayScale / SHRINK_MARGIN))
}

/** Slack on a frame's coded size, for the rounding between CSS and device px. */
export const TEXTURE_SIZE_TOLERANCE_PX = 2

/**
 * The CSS length a frame was painted for, read back from its texture. A view
 * resize lands some frames after it is asked for, and the frames in between
 * still show the old viewport; only their texture size says so. `expectedCss`
 * wins inside the rounding slack, so a settled page reports its exact size.
 */
export function paintedCssLength(
  codedLength: number,
  expectedCss: number,
  devicePxPerCssPx: number,
): number {
  if (Math.abs(codedLength - expectedCss * devicePxPerCssPx) <= TEXTURE_SIZE_TOLERANCE_PX) {
    return expectedCss
  }
  return Math.max(1, Math.round(codedLength / devicePxPerCssPx))
}
