export type MarqueeSelectionMode = 'intersect' | 'contain'

type Rect = { x: number; y: number; width: number; height: number }

export function pointInsideRect(point: { x: number; y: number }, rect: Rect): boolean {
  return (
    point.x >= rect.x &&
    point.x <= rect.x + rect.width &&
    point.y >= rect.y &&
    point.y <= rect.y + rect.height
  )
}
