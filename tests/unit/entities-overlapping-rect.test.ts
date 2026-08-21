import { describe, expect, it } from 'vitest'
import { entitiesOverlappingRect } from '../../src/shared/gesture-utils'
import type {
  CanvasSceneEntity,
  CanvasSceneGroupEntity,
  CanvasScenePageEntity,
} from '../../src/shared/types'

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

function group(
  over: Partial<CanvasSceneGroupEntity> & { id: string },
): CanvasSceneGroupEntity {
  return {
    id: over.id,
    kind: 'group',
    label: 'Group',
    canvasX: over.screenX ?? 0,
    canvasY: over.screenY ?? 0,
    width: over.screenWidth ?? 300,
    height: over.screenHeight ?? 300,
    screenX: 0,
    screenY: 0,
    screenWidth: 300,
    screenHeight: 300,
    layoutMode: 'freeform',
    managedLayout: false,
    entityIds: [],
    ...over,
  }
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
    const ids = entitiesOverlappingRect(
      entities,
      { left: 40, top: 40, width: 220, height: 120 },
      { mode: 'contain', excludedIds: new Set(['c']) },
    )
    expect(ids).toEqual([])

    expect(
      entitiesOverlappingRect(
        entities,
        { left: -10, top: -10, width: 320, height: 170 },
        { mode: 'contain', excludedIds: new Set(['a']) },
      ),
    ).toEqual(['b', 'c'])
  })

  it('returns a fully enclosed group instead of its overlapping children', () => {
    const grouped: CanvasSceneEntity[] = [
      page({ id: 'child-a', screenX: 20, screenY: 20, parentGroupId: 'group-1' }),
      page({ id: 'child-b', screenX: 180, screenY: 180, parentGroupId: 'group-1' }),
      group({
        id: 'group-1',
        screenX: 0,
        screenY: 0,
        screenWidth: 300,
        screenHeight: 300,
        entityIds: ['child-a', 'child-b'],
      }),
    ]

    expect(
      entitiesOverlappingRect(grouped, { left: -10, top: -10, width: 320, height: 320 }),
    ).toEqual(['group-1'])
  })

  it('selects individual children when the marquee hits nothing outside their group', () => {
    const grouped: CanvasSceneEntity[] = [
      page({ id: 'child-a', screenX: 20, screenY: 20, parentGroupId: 'group-1' }),
      page({ id: 'child-b', screenX: 180, screenY: 180, parentGroupId: 'group-1' }),
      group({
        id: 'group-1',
        screenX: 0,
        screenY: 0,
        screenWidth: 300,
        screenHeight: 300,
        entityIds: ['child-a', 'child-b'],
      }),
    ]

    expect(
      entitiesOverlappingRect(grouped, { left: 10, top: 10, width: 120, height: 120 }),
    ).toEqual(['child-a'])
  })

  it('prefers a fully enclosed outer group over its nested group', () => {
    const nested: CanvasSceneEntity[] = [
      page({ id: 'child', screenX: 80, screenY: 80, parentGroupId: 'inner' }),
      group({
        id: 'inner',
        screenX: 50,
        screenY: 50,
        screenWidth: 200,
        screenHeight: 200,
        parentGroupId: 'outer',
        entityIds: ['child'],
      }),
      group({
        id: 'outer',
        screenX: 0,
        screenY: 0,
        screenWidth: 300,
        screenHeight: 300,
        entityIds: ['inner'],
      }),
    ]

    expect(
      entitiesOverlappingRect(nested, { left: -10, top: -10, width: 320, height: 320 }),
    ).toEqual(['outer'])
  })

  it('batches multiple fully enclosed groups', () => {
    const grouped: CanvasSceneEntity[] = [
      page({ id: 'a', screenX: 20, screenY: 20, parentGroupId: 'group-a' }),
      group({
        id: 'group-a',
        screenX: 0,
        screenY: 0,
        screenWidth: 140,
        screenHeight: 140,
        entityIds: ['a'],
      }),
      page({ id: 'b', screenX: 220, screenY: 20, parentGroupId: 'group-b' }),
      group({
        id: 'group-b',
        screenX: 200,
        screenY: 0,
        screenWidth: 140,
        screenHeight: 140,
        entityIds: ['b'],
      }),
    ]

    expect(
      entitiesOverlappingRect(grouped, { left: -10, top: -10, width: 360, height: 160 }),
    ).toEqual(['group-a', 'group-b'])
  })

  it('promotes a partially crossed group to a unit when the marquee also hits outside it', () => {
    const grouped: CanvasSceneEntity[] = [
      page({ id: 'loose', screenX: 380, screenY: 20 }),
      page({ id: 'inside', screenX: 20, screenY: 20, parentGroupId: 'group-1' }),
      page({ id: 'outside-marquee', screenX: 20, screenY: 380, parentGroupId: 'group-1' }),
      group({
        id: 'group-1',
        screenX: 0,
        screenY: 0,
        screenWidth: 300,
        screenHeight: 500,
        entityIds: ['inside', 'outside-marquee'],
      }),
    ]

    // Marquee covers 'inside' (one group child) plus the loose page — the
    // group must come along whole, never a partial slice of its contents.
    expect(
      entitiesOverlappingRect(grouped, { left: -10, top: -10, width: 500, height: 160 }),
    ).toEqual(['loose', 'group-1'])
  })

  it('scopes to the deepest common group: sibling subgroups promote as units', () => {
    const nested: CanvasSceneEntity[] = [
      page({ id: 'a', screenX: 20, screenY: 20, parentGroupId: 'sub-a' }),
      group({
        id: 'sub-a',
        screenX: 10,
        screenY: 10,
        screenWidth: 120,
        screenHeight: 200,
        parentGroupId: 'outer',
        entityIds: ['a'],
      }),
      page({ id: 'b', screenX: 160, screenY: 20, parentGroupId: 'sub-b' }),
      group({
        id: 'sub-b',
        screenX: 150,
        screenY: 10,
        screenWidth: 120,
        screenHeight: 200,
        parentGroupId: 'outer',
        entityIds: ['b'],
      }),
      group({
        id: 'outer',
        screenX: 0,
        screenY: 0,
        screenWidth: 300,
        screenHeight: 300,
        entityIds: ['sub-a', 'sub-b'],
      }),
    ]

    // Marquee stays inside 'outer' but crosses both subgroups partially:
    // scope is 'outer', so each subgroup promotes to a unit within it.
    expect(
      entitiesOverlappingRect(nested, { left: 15, top: 15, width: 200, height: 110 }),
    ).toEqual(['sub-a', 'sub-b'])
  })

  it('batches a full group with intersected children from a partial group', () => {
    const grouped: CanvasSceneEntity[] = [
      page({ id: 'inside-full', screenX: 20, screenY: 20, parentGroupId: 'full' }),
      group({
        id: 'full',
        screenX: 0,
        screenY: 0,
        screenWidth: 140,
        screenHeight: 140,
        entityIds: ['inside-full'],
      }),
      page({ id: 'inside-partial', screenX: 220, screenY: 20, parentGroupId: 'partial' }),
      page({ id: 'outside-marquee', screenX: 380, screenY: 20, parentGroupId: 'partial' }),
      group({
        id: 'partial',
        screenX: 200,
        screenY: 0,
        screenWidth: 320,
        screenHeight: 140,
        entityIds: ['inside-partial', 'outside-marquee'],
      }),
    ]

    // 'inside-partial' cannot come out alone — the hit set spans beyond its
    // group ('full' is also hit), so 'partial' promotes to a unit.
    expect(
      entitiesOverlappingRect(grouped, { left: -10, top: -10, width: 350, height: 160 }),
    ).toEqual(['full', 'partial'])
  })
})
