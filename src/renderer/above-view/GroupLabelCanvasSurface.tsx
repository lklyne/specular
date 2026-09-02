/**
 * GroupLabelCanvasSurface: group titles drawn on a full-window canvas at the
 * screen geometry this renderer projected. Same pattern as canvas-bg's
 * ChromeCanvasSurface: text renders at display scale each frame, so labels
 * keep their fixed 11px size and stay crisp during a zoom gesture instead of
 * being drawn once as DOM and scaled.
 *
 * Mounted in aboveView (not the canvas-bg chrome canvas) because labels must
 * stay visible above native page views (ADR 0002). Purely visual. Pointer
 * input routes through the shared hit-test's `group-label` target, and a
 * group being renamed skips canvas drawing so the DOM input shows instead.
 */
import { memo, useCallback, useEffect, useRef } from 'react'
import {
  GROUP_LABEL_BOTTOM_GAP,
  GROUP_LABEL_FONT,
  GROUP_LABEL_LINE_HEIGHT,
} from '../../shared/group-label-geometry'
import type { ProjectedGroupEntity } from '../../shared/scene-projection'
import { prepareScreenCanvas } from '../shared/screenCanvas'

function groupLabelColor(
  group: ProjectedGroupEntity,
  isDark: boolean,
): string {
  if (group.color) return isDark ? '#f4f4f5' : '#18181b' // zinc-100 / zinc-900
  return isDark ? '#d4d4d8' : '#3f3f46' // zinc-300 / zinc-700
}

function drawGroupLabels({
  canvas,
  groups,
  originY,
  isDark,
  skipGroupId,
  devicePixelRatio,
}: {
  canvas: HTMLCanvasElement
  groups: ProjectedGroupEntity[]
  originY: number
  isDark: boolean
  skipGroupId: string | null
  devicePixelRatio: number
}): void {
  const prepared = prepareScreenCanvas(canvas, devicePixelRatio)
  if (!prepared) return
  const { ctx } = prepared

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
    const x = group.screenX
    const groupTop = group.screenY - originY
    ctx.fillStyle = groupLabelColor(group, isDark)
    ctx.fillText(group.label, x, groupTop - GROUP_LABEL_BOTTOM_GAP - baselineFromBottom)
  }
}

export const GroupLabelCanvasSurface = memo(function GroupLabelCanvasSurface({
  groups,
  originY,
  isDark,
  editingEntityId,
}: {
  groups: ProjectedGroupEntity[]
  originY: number
  isDark: boolean
  editingEntityId: string | null
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null)

  const drawInputs = useRef({ groups, originY, isDark, editingEntityId })
  drawInputs.current = { groups, originY, isDark, editingEntityId }

  const draw = useCallback(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const { groups, originY, isDark, editingEntityId } = drawInputs.current
    drawGroupLabels({
      canvas,
      groups,
      originY,
      isDark,
      skipGroupId: editingEntityId,
      devicePixelRatio: window.devicePixelRatio || 1,
    })
  }, [])

  useEffect(() => {
    draw()
  }, [groups, originY, isDark, editingEntityId, draw])

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
})
