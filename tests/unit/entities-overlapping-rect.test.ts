import { describe, expect, it } from 'vitest'
import { entitiesOverlappingRect } from '../../src/shared/gesture-utils'
import { entityIdsInScreenRect } from '../../src/shared/marquee-selection'
import type { CanvasSceneEntity, CanvasScenePageEntity } from '../../src/shared/types'

function page(over: Partial<CanvasScenePageEntity> & { id: string }): CanvasScenePageEntity {
  return {
    id: over.id,
    kind: 'page',
    canvasX: 0,
    canvasY: 0,
    width: over.screenWidth ?? 100,
    height: over.screenHeight ?? 100,
    screenX: 0,
    screenY: 0,
    screenWidth: 100,
    screenHeight: 100,
    presetIndex: 0,
    rendererTag: 'web',
    ...over,
  } as CanvasScenePageEntity
}

describe('entitiesOverlappingRect', () => {
  const entities: CanvasSceneEntity[] = [
    page({ id: 'a', screenX: 0, screenY: 0, screenWidth: 100, screenHeight: 100 }),
    page({ id: 'b', screenX: 200, screenY: 0, screenWidth: 100, screenHeight: 100 }),
    page({ id: 'c', screenX: 50, screenY: 50, screenWidth: 100, screenHeight: 100 }),
  ]

  it('returns ids of entities the rect overlaps', () => {
    const ids = entitiesOverlappingRect(entities, { left: 40, top: 40, width: 80, height: 80 })
    expect(ids).toEqual(['a', 'c'])
  })

  it('returns all entities when the rect is large enough to enclose them', () => {
    const ids = entitiesOverlappingRect(entities, { left: -10, top: -10, width: 1000, height: 1000 })
    expect(ids).toEqual(['a', 'b', 'c'])
  })

  it('returns an empty array when the rect misses everything', () => {
    const ids = entitiesOverlappingRect(entities, { left: 500, top: 500, width: 50, height: 50 })
    expect(ids).toEqual([])
  })

  it('treats edge-touching as non-overlap (matches old marquee preview)', () => {
    // Page a sits at [0,100) × [0,100). A rect starting exactly at x=100 must miss.
    const ids = entitiesOverlappingRect([entities[0]], { left: 100, top: 0, width: 50, height: 50 })
    expect(ids).toEqual([])
  })

  it('preserves entity input order', () => {
    const ids = entitiesOverlappingRect(entities, { left: 0, top: 0, width: 300, height: 300 })
    expect(ids).toEqual(['a', 'b', 'c'])
  })

  it('can require full containment and exclude the item under the first click', () => {
    const ids = entityIdsInScreenRect(
      entities,
      { left: 40, top: 40, width: 220, height: 120 },
      'contain',
      new Set(['c']),
    )
    expect(ids).toEqual([])

    expect(
      entityIdsInScreenRect(
        entities,
        { left: -10, top: -10, width: 320, height: 170 },
        'contain',
        new Set(['a']),
      ),
    ).toEqual(['b', 'c'])
  })
})
