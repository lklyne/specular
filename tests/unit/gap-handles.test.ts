/**
 * Gap-handle geometry tests (ADR 0015 Milestone 2 — draggable gap handles).
 *
 * `collectGapHandleZones` is the single geometry/eligibility source behind the
 * hit-tester (`gap-handle` layer) and the renderer painter (`GapHandlesLayer`):
 * one strip per gap between adjacent items of a line, spanning the full
 * cross-axis extent. Two doors — a managed row/column group whose group or
 * child is selected (`groupId` set), and a loose equal-gap multi-selection
 * (`groupId` null). `packedGapPositions` is the pure preview kernel the
 * renderer packs items with while a `resizing-gap` drag is in flight.
 *
 * Mutation-verified by:
 *   - swapping the axis ternary in `collectGapHandleZones`'s rect construction
 *     — the row/column zone cases fail.
 *   - dropping the `GAP_HANDLE_MIN_HIT_PX` expansion — the tight-gap case fails.
 *   - removing the `'reorder-handle'`-before-`'gap-handle'` ordering in
 *     HIT_LAYER_ORDER — the priority case fails.
 */

import { describe, expect, it } from 'vitest'
import { collectGapHandleZones, packedGapPositions } from '../../src/shared/gap-handles'
import { GAP_HANDLE_MIN_HIT_PX } from '../../src/shared/canvas-hit-geometry'
import { hitTest } from '../../src/shared/hit-test'
import type { CanvasSceneGroupEntity, CanvasSceneTextEntity } from '../../src/shared/types'

// Canvas and screen coords coincide so zone rects can be asserted directly.
function box(id: string, x: number, y: number, w = 100, h = 40): CanvasSceneTextEntity {
  return {
    kind: 'text',
    id,
    text: 't',
    color: '#000',
    canvasX: x,
    canvasY: y,
    width: w,
    height: h,
    screenX: x,
    screenY: y,
    screenWidth: w,
    screenHeight: h,
  }
}

function managedGroup(
  id: string,
  entityIds: string[],
  layoutMode: 'row' | 'column' | 'freeform' = 'row',
  managedLayout = true,
): CanvasSceneGroupEntity {
  return {
    kind: 'group',
    id,
    label: 'g',
    canvasX: -20,
    canvasY: -20,
    width: 1000,
    height: 200,
    screenX: -20,
    screenY: -20,
    screenWidth: 1000,
    screenHeight: 200,
    layoutMode,
    managedLayout,
    entityIds,
  }
}

describe('collectGapHandleZones — row groups', () => {
  it('emits one strip per gap, spanning the gap and the full cross extent', () => {
    // Children at x 0 / 180 / 360, widths 100 — gaps 80 wide. Middle child is
    // taller, so the cross extent runs from y 0 to y 100.
    const entities = [
      box('a', 0, 0),
      box('b', 180, 0, 100, 100),
      box('c', 360, 0),
      managedGroup('g', ['a', 'b', 'c']),
    ]
    const zones = collectGapHandleZones({
      entities,
      selectedEntityIds: [],
      selectedGroupId: 'g',
    })
    expect(zones).toEqual([
      { groupId: 'g', axis: 'x', index: 0, rect: { x: 100, y: 0, width: 80, height: 100 } },
      { groupId: 'g', axis: 'x', index: 1, rect: { x: 280, y: 0, width: 80, height: 100 } },
    ])
  })

  it('expands a tight gap to the minimum hit thickness, centered on the seam', () => {
    const entities = [box('a', 0, 0), box('b', 102, 0), managedGroup('g', ['a', 'b'])] // gap 2
    const [zone] = collectGapHandleZones({
      entities,
      selectedEntityIds: [],
      selectedGroupId: 'g',
    })
    expect(zone.rect.width).toBe(GAP_HANDLE_MIN_HIT_PX)
    // Centered on the 2px seam midpoint (x 101).
    expect(zone.rect.x).toBe(101 - GAP_HANDLE_MIN_HIT_PX / 2)
  })
})

describe('collectGapHandleZones — column groups', () => {
  it('emits horizontal strips between vertically stacked children', () => {
    const entities = [
      box('a', 0, 0),
      box('b', 0, 120),
      managedGroup('g', ['a', 'b'], 'column'),
    ]
    const zones = collectGapHandleZones({
      entities,
      selectedEntityIds: [],
      selectedGroupId: 'g',
    })
    expect(zones).toEqual([
      { groupId: 'g', axis: 'y', index: 0, rect: { x: 0, y: 40, width: 100, height: 80 } },
    ])
  })
})

describe('collectGapHandleZones — eligibility', () => {
  const entities = () => [box('a', 0, 0), box('b', 180, 0), managedGroup('g', ['a', 'b'])]

  it('lights nothing when neither the group nor a child is selected', () => {
    expect(
      collectGapHandleZones({ entities: entities(), selectedEntityIds: [], selectedGroupId: null }),
    ).toEqual([])
  })

  it('lights when a child is selected (matching the reorder dots managed door)', () => {
    expect(
      collectGapHandleZones({ entities: entities(), selectedEntityIds: ['b'], selectedGroupId: null }),
    ).toHaveLength(1)
  })

  it('ignores unmanaged and freeform groups (no selected entities → no selection door either)', () => {
    const freeform = [box('a', 0, 0), box('b', 180, 0), managedGroup('g', ['a', 'b'], 'freeform')]
    expect(
      collectGapHandleZones({ entities: freeform, selectedEntityIds: [], selectedGroupId: 'g' }),
    ).toEqual([])
    const unmanaged = [box('a', 0, 0), box('b', 180, 0), managedGroup('g', ['a', 'b'], 'row', false)]
    expect(
      collectGapHandleZones({ entities: unmanaged, selectedEntityIds: [], selectedGroupId: 'g' }),
    ).toEqual([])
  })
})

describe('collectGapHandleZones — selection door', () => {
  it('emits strips with a null groupId for a loose equal-gap selection', () => {
    const entities = [box('a', 0, 0), box('b', 180, 0), box('c', 360, 0)]
    const zones = collectGapHandleZones({
      entities,
      selectedEntityIds: ['a', 'b', 'c'],
      selectedGroupId: null,
    })
    expect(zones).toEqual([
      { groupId: null, axis: 'x', index: 0, rect: { x: 100, y: 0, width: 80, height: 40 } },
      { groupId: null, axis: 'x', index: 1, rect: { x: 280, y: 0, width: 80, height: 40 } },
    ])
  })

  it('emits nothing for an unequal-gap selection (mirrors the reorder dots gate)', () => {
    const entities = [box('a', 0, 0), box('b', 180, 0), box('c', 420, 0)] // gaps 80 / 140
    expect(
      collectGapHandleZones({ entities, selectedEntityIds: ['a', 'b', 'c'], selectedGroupId: null }),
    ).toEqual([])
  })

  it('leaves managed children to the managed door — no duplicate strips', () => {
    const entities = [box('a', 0, 0), box('b', 180, 0), managedGroup('g', ['a', 'b'])]
    const zones = collectGapHandleZones({
      entities,
      selectedEntityIds: ['a', 'b'],
      selectedGroupId: null,
    })
    expect(zones).toHaveLength(1)
    expect(zones[0].groupId).toBe('g')
  })
})

describe('hit-test integration', () => {
  it('routes a point in the gap strip to gap-handle, not the group body', () => {
    const entities = [box('a', 0, 0), box('b', 180, 0), managedGroup('g', ['a', 'b'])]
    const target = hitTest(
      {
        entities,
        edges: [],
        selectedEntityIds: [],
        selectedGroupId: 'g',
        zoom: 1,
      },
      { x: 140, y: 20 }, // mid-gap, inside the group's body rect
    )
    expect(target.payload).toEqual({ kind: 'gap-handle', groupId: 'g' })
  })

  it('the reorder dot wins where dot and gap strip overlap', () => {
    // Flush tiny children: b's dot square (4px around center x 15) overlaps the
    // expanded gap strip (10px around the seam at x 10) on x ∈ [13, 15].
    const entities = [
      box('a', 0, 0, 10, 10),
      box('b', 10, 0, 10, 10),
      managedGroup('g', ['a', 'b']),
    ]
    const point = { x: 14, y: 5 } // inside both b's dot square and the strip
    const target = hitTest(
      { entities, edges: [], selectedEntityIds: [], selectedGroupId: 'g', zoom: 1 },
      point,
    )
    expect(target.payload.kind).toBe('reorder-handle')
  })
})

describe('packedGapPositions', () => {
  it('repacks a row at the new gap, anchored at the first child', () => {
    const children = [box('a', 0, 0), box('b', 180, 0), box('c', 360, 0)] // gap 80
    const changed = packedGapPositions(children, 'x', 20)
    expect(changed.get('a')).toBeUndefined() // anchor unchanged
    expect(changed.get('b')).toEqual({ x: 120, y: 0 })
    expect(changed.get('c')).toEqual({ x: 240, y: 0 })
  })

  it('repacks a column along y, keeping each child cross coordinate', () => {
    const children = [box('a', 5, 0), box('b', 5, 120)] // gap 80 along y
    const changed = packedGapPositions(children, 'y', 0)
    expect(changed.get('b')).toEqual({ x: 5, y: 40 })
  })

  it('returns an empty map when the gap already matches', () => {
    const children = [box('a', 0, 0), box('b', 180, 0)]
    expect(packedGapPositions(children, 'x', 80).size).toBe(0)
  })

  it('keepCross preserves each item cross coordinate (selection door)', () => {
    const children = [box('a', 0, 0), box('b', 180, 10), box('c', 360, 20)]
    const changed = packedGapPositions(children, 'x', 20, { keepCross: true })
    expect(changed.get('b')).toEqual({ x: 120, y: 10 })
    expect(changed.get('c')).toEqual({ x: 240, y: 20 })
  })
})
