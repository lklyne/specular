import type { CanvasSceneEntity } from './types'

export function groupDropTargetAt(
  entities: readonly CanvasSceneEntity[],
  point: { x: number; y: number },
  excludedIds: ReadonlySet<string>,
): string | null {
  const groups = entities
    .filter((entity) => entity.kind === 'group')
    .filter((group) => !excludedIds.has(group.id))
    .filter((group) => (
      point.x >= group.screenX &&
      point.x <= group.screenX + group.screenWidth &&
      point.y >= group.screenY &&
      point.y <= group.screenY + group.screenHeight
    ))

  // Nested groups can overlap. The smallest containing box is the most
  // specific target; paint order breaks equal-area ties front-most first.
  groups.sort((a, b) => {
    const areaDelta =
      a.screenWidth * a.screenHeight - b.screenWidth * b.screenHeight
    if (areaDelta !== 0) return areaDelta
    return entities.indexOf(b) - entities.indexOf(a)
  })
  return groups[0]?.id ?? null
}
