/**
 * GroupLabelCanvasSurface — group titles drawn on a full-window canvas in
 * screen space from the live camera, every pan/zoom tick. Same pattern as
 * canvas-bg's ChromeCanvasSurface: text renders at display scale each frame,
 * so labels keep their fixed 11px size and stay crisp during a zoom gesture
 * instead of riding the CSS scene transform as bitmap-scaled DOM.
 *
 * Mounted in aboveView (not the canvas-bg chrome canvas) because labels must
 * stay visible above native page views (ADR 0002). Purely visual — pointer
 * input routes through the shared hit-test's `group-label` target, and a
 * group being renamed skips canvas drawing so the DOM input shows instead.
 */
import { useCallback, useEffect, useRef } from 'react'
import {
  GROUP_LABEL_BOTTOM_GAP,
  GROUP_LABEL_FONT,
  GROUP_LABEL_LINE_HEIGHT,
} from '../../shared/group-label-geometry'
import type { SceneCameraTransform } from '../../shared/scene-camera-transform'
import type { CanvasSceneGroupEntity } from '../../shared/types'

export function groupLabelColor(
  group: CanvasSceneGroupEntity,
  isDark: boolean,
): string {
  if (group.color) return isDark ? '#f4f4f5' : '#18181b' // zinc-100 / zinc-900
  return isDark ? '#d4d4d8' : '#3f3f46' // zinc-300 / zinc-700
}

export function drawGroupLabels({
  canvas,
  groups,
  transform,
  originY,
  isDark,
  skipGroupId,
  devicePixelRatio,
}: {
  canvas: HTMLCanvasElement
  groups: CanvasSceneGroupEntity[]
  transform: SceneCameraTransform
  originY: number
  isDark: boolean
  skipGroupId: string | null
  devicePixelRatio: number
}): void {
  const dpr = Math.max(devicePixelRatio, 1)
  const width = canvas.clientWidth
  const height = canvas.clientHeight
  const targetWidth = Math.max(1, Math.ceil(width * dpr))
  const targetHeight = Math.max(1, Math.ceil(height * dpr))
  if (canvas.width !== targetWidth) canvas.width = targetWidth
  if (canvas.height !== targetHeight) canvas.height = targetHeight

  const ctx = canvas.getContext('2d')
  if (!ctx) return
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  ctx.clearRect(0, 0, width, height)

  ctx.font = GROUP_LABEL_FONT
  ctx.textBaseline = 'alphabetic'

  // Reproduce the DOM line box: baseline sits half-leading + descent above
  // the bottom of a LINE_HEIGHT-tall box anchored BOTTOM_GAP above the group.
  const metrics = ctx.measureText('Mg')
  const contentHeight =
    metrics.fontBoundingBoxAscent + metrics.fontBoundingBoxDescent
  const baselineFromBottom =
    Math.max(0, (GROUP_LABEL_LINE_HEIGHT - contentHeight) / 2) +
    metrics.fontBoundingBoxDescent

  for (const group of groups) {
    if (!group.label || group.id === skipGroupId) continue
    const x = transform.x + transform.scale * group.screenX
    const groupTop = transform.y + transform.scale * (group.screenY - originY)
    ctx.fillStyle = groupLabelColor(group, isDark)
    ctx.fillText(group.label, x, groupTop - GROUP_LABEL_BOTTOM_GAP - baselineFromBottom)
  }
}

export function GroupLabelCanvasSurface({
  groups,
  transform,
  originY,
  isDark,
  editingEntityId,
}: {
  groups: CanvasSceneGroupEntity[]
  transform: SceneCameraTransform
  originY: number
  isDark: boolean
  editingEntityId: string | null
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null)

  const drawInputs = useRef({ groups, transform, originY, isDark, editingEntityId })
  drawInputs.current = { groups, transform, originY, isDark, editingEntityId }

  const draw = useCallback(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const { groups, transform, originY, isDark, editingEntityId } =
      drawInputs.current
    drawGroupLabels({
      canvas,
      groups,
      transform,
      originY,
      isDark,
      skipGroupId: editingEntityId,
      devicePixelRatio: window.devicePixelRatio || 1,
    })
  }, [])

  useEffect(() => {
    draw()
  }, [groups, transform, originY, isDark, editingEntityId, draw])

  useEffect(() => {
    window.addEventListener('resize', draw)
    // First paint can race system font load; repaint once fonts settle.
    document.fonts?.ready.then(draw).catch(() => {})
    return () => window.removeEventListener('resize', draw)
  }, [draw])

  return (
    <canvas
      ref={canvasRef}
      aria-hidden="true"
      data-group-label-canvas
      className="pointer-events-none absolute inset-0 h-full w-full"
    />
  )
}
