/**
 * Sizes a full-window canvas to its CSS box at `devicePixelRatio` and hands
 * back a context in CSS-pixel units, cleared. Null when the context is
 * unavailable. Every screen-space canvas layer (grid, chrome, group labels,
 * drag freeze) starts its draw here.
 */
export function prepareScreenCanvas(
  canvas: HTMLCanvasElement,
  devicePixelRatio: number,
): { ctx: CanvasRenderingContext2D; width: number; height: number; dpr: number } | null {
  const dpr = Math.max(devicePixelRatio || 1, 1)
  const width = canvas.clientWidth
  const height = canvas.clientHeight
  const targetWidth = Math.max(1, Math.ceil(width * dpr))
  const targetHeight = Math.max(1, Math.ceil(height * dpr))
  if (canvas.width !== targetWidth) canvas.width = targetWidth
  if (canvas.height !== targetHeight) canvas.height = targetHeight

  const ctx = canvas.getContext('2d')
  if (!ctx) return null
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  ctx.clearRect(0, 0, width, height)
  return { ctx, width, height, dpr }
}
