import type { CanvasSceneEntity, WorkspaceBounds } from './types'

export type MarqueeSelectionMode = 'intersect' | 'contain'

type Rect = { x: number; y: number; width: number; height: number }

export function rectSelectsBounds(
  selection: Rect,
  candidate: Rect,
  mode: MarqueeSelectionMode,
): boolean {
  if (mode === 'contain') {
    return (
      candidate.x >= selection.x &&
      candidate.y >= selection.y &&
      candidate.x + candidate.width <= selection.x + selection.width &&
      candidate.y + candidate.height <= selection.y + selection.height
    )
  }
  return (
    candidate.x < selection.x + selection.width &&
    candidate.x + candidate.width > selection.x &&
    candidate.y < selection.y + selection.height &&
    candidate.y + candidate.height > selection.y
  )
}

export function pointInsideRect(point: { x: number; y: number }, rect: Rect): boolean {
  return (
    point.x >= rect.x &&
    point.x <= rect.x + rect.width &&
    point.y >= rect.y &&
    point.y <= rect.y + rect.height
  )
}

export function entityIdsInScreenRect(
  entities: readonly CanvasSceneEntity[],
  rect: { left: number; top: number; width: number; height: number },
  mode: MarqueeSelectionMode,
  excludedIds: ReadonlySet<string> = new Set(),
): string[] {
  const selection: WorkspaceBounds = {
    x: rect.left,
    y: rect.top,
    width: rect.width,
    height: rect.height,
  }
  return entities
    .filter(
      (entity) =>
        !excludedIds.has(entity.id) &&
        rectSelectsBounds(
          selection,
          {
            x: entity.screenX,
            y: entity.screenY,
            width: entity.screenWidth,
            height: entity.screenHeight,
          },
          mode,
        ),
    )
    .map((entity) => entity.id)
}
