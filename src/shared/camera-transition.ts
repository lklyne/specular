export interface CanvasCamera {
  zoom: number
  pan: { x: number; y: number }
}

export const DEFAULT_CAMERA_TRANSITION_DURATION_MS = 320
export const CAMERA_TRANSITION_FRAME_MS = 16
const CAMERA_SPRING_STIFFNESS = 8.5
const CAMERA_SPRING_END_VALUE =
  1 - (1 + CAMERA_SPRING_STIFFNESS) * Math.exp(-CAMERA_SPRING_STIFFNESS)

export function easeOutCubic(t: number): number {
  return 1 - (1 - t) ** 3
}

export function cameraSpring(t: number): number {
  const value = clampUnit(t)
  if (value <= 0) return 0
  if (value >= 1) return 1
  const raw =
    1 -
    (1 + CAMERA_SPRING_STIFFNESS * value) *
      Math.exp(-CAMERA_SPRING_STIFFNESS * value)
  return raw / CAMERA_SPRING_END_VALUE
}

export function sampledCameraSpringCss(sampleCount = 13): string {
  const samples = Array.from({ length: sampleCount }, (_, index) => {
    const t = sampleCount <= 1 ? 1 : index / (sampleCount - 1)
    return Number(cameraSpring(t).toFixed(4)).toString()
  })
  return `linear(${samples.join(', ')})`
}

export const CAMERA_SPRING_CSS_EASING = sampledCameraSpringCss()

function clampUnit(value: number): number {
  if (!Number.isFinite(value)) return 1
  return Math.max(0, Math.min(1, value))
}

function lerp(start: number, end: number, t: number): number {
  return start + (end - start) * t
}

export function interpolateCamera(
  start: CanvasCamera,
  target: CanvasCamera,
  progress: number,
  easing: (t: number) => number = cameraSpring,
): CanvasCamera {
  const rawT = clampUnit(progress)
  if (rawT >= 1) return target
  if (rawT <= 0) return start

  const t = clampUnit(easing(rawT))
  return {
    zoom: lerp(start.zoom, target.zoom, t),
    pan: {
      x: Math.round(lerp(start.pan.x, target.pan.x, t)),
      y: Math.round(lerp(start.pan.y, target.pan.y, t)),
    },
  }
}
