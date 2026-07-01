import { describe, expect, it } from 'vitest'
import { entitiesOverlappingRect } from '../../src/shared/gesture-utils'
import type { Camera } from '../../src/shared/coords'
import type { CanvasSceneEntity, CanvasScenePageEntity } from '../../src/shared/types'

const IDENTITY_CAMERA: Camera = { pan: { x: 0, y: 0 }, zoom: 1, canvasOrigin: { x: 0, y: 0 } }

function page(
  over: Partial<CanvasScenePageEntity> & { id: string; canvasX: number; canvasY: number },
): CanvasScenePageEntity {
  const width = over.width ?? 100
  const height = over.height ?? 100
  return {
    id: over.id,
    kind: 'page',
    canvasX: over.canvasX,
    canvasY: over.canvasY,
    width,
    height,
    visualCanvasX: over.canvasX,
    visualCanvasY: over.canvasY,
    visualWidth: width,
    visualHeight: height,
    presetIndex: 0,
    rendererTag: 'web',
    ...over,
  } as CanvasScenePageEntity
}

describe('entitiesOverlappingRect', () => {
  const entities: CanvasSceneEntity[] = [
    page({ id: 'a', canvasX: 0, canvasY: 0, width: 100, height: 100 }),
    page({ id: 'b', canvasX: 200, canvasY: 0, width: 100, height: 100 }),
    page({ id: 'c', canvasX: 50, canvasY: 50, width: 100, height: 100 }),
  ]

  it('returns ids of entities the rect overlaps', () => {
    const ids = entitiesOverlappingRect(
      entities,
      { left: 40, top: 40, width: 80, height: 80 },
      IDENTITY_CAMERA,
    )
    expect(ids).toEqual(['a', 'c'])
  })

  it('returns all entities when the rect is large enough to enclose them', () => {
    const ids = entitiesOverlappingRect(
      entities,
      { left: -10, top: -10, width: 1000, height: 1000 },
      IDENTITY_CAMERA,
    )
    expect(ids).toEqual(['a', 'b', 'c'])
  })

  it('returns an empty array when the rect misses everything', () => {
    const ids = entitiesOverlappingRect(
      entities,
      { left: 500, top: 500, width: 50, height: 50 },
      IDENTITY_CAMERA,
    )
    expect(ids).toEqual([])
  })

  it('treats edge-touching as non-overlap (matches old marquee preview)', () => {
    // Page a sits at [0,100) × [0,100). A rect starting exactly at x=100 must miss.
    const ids = entitiesOverlappingRect(
      [entities[0]],
      { left: 100, top: 0, width: 50, height: 50 },
      IDENTITY_CAMERA,
    )
    expect(ids).toEqual([])
  })

  it('preserves entity input order', () => {
    const ids = entitiesOverlappingRect(
      entities,
      { left: 0, top: 0, width: 300, height: 300 },
      IDENTITY_CAMERA,
    )
    expect(ids).toEqual(['a', 'b', 'c'])
  })
})
