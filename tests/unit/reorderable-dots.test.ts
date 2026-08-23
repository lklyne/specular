/**
 * Shared reorder-dot selector tests (ADR 0015 D7, Phase C).
 *
 * `reorderableDots` is the single eligibility brain behind the hit-tester and the
 * renderer painter — the union of the selection door (equal-gap loose selection)
 * and the managed door (a selected managed-row group's children). Eligibility
 * runs on canvas geometry; the returned center is the entity's screen center.
 */

import { describe, expect, it } from 'vitest'
import { reorderableDots } from '../../src/shared/reorderable-dots'
import type {
  CanvasSceneGroupEntity,
  CanvasScenePageEntity,
  CanvasSceneTextEntity,
} from '../../src/shared/types'

// Canvas and screen coords coincide so the detector and the dot center agree.
function box(id: string, x: number, w = 100): CanvasSceneTextEntity {
  return {
    kind: 'text',
    id,
    text: 't',
    color: '#000',
    canvasX: x,
    canvasY: 200,
    width: w,
    height: 40,
    screenX: x,
    screenY: 200,
    screenWidth: w,
    screenHeight: 40,
  }
}

function managedRow(id: string, entityIds: string[]): CanvasSceneGroupEntity {
  return {
    kind: 'group',
    id,
    label: 'g',
    canvasX: 0,
    canvasY: 0,
    width: 1000,
    height: 200,
    screenX: 0,
    screenY: 0,
    screenWidth: 1000,
    screenHeight: 200,
    layoutMode: 'row',
    managedLayout: true,
    entityIds,
  }
}

/**
 * A device-framed page at zoom 1: canvas `width` is the web-content size, but
 * the shell it occupies (and the dot center) adds a `bezel` on each side. The
 * shell size is carried on the screen bounds; `x` is the shell's left edge.
 */
function framedPage(id: string, x: number, contentW: number, bezel: number): CanvasScenePageEntity {
  const shellW = contentW + bezel * 2
  return {
    kind: 'page',
    id,
    label: 'p',
    url: 'https://example.com',
    canGoBack: false,
    canGoForward: false,
    isLoading: false,
    canvasX: x,
    canvasY: 200,
    width: contentW,
    height: 400,
    presetIndex: 0,
    synced: false,
    screenX: x,
    screenY: 200,
    screenWidth: shellW,
    screenHeight: 400,
    showDeviceFrame: true,
  }
}

const ids = (dots: { id: string }[]) => dots.map((d) => d.id).sort()

describe('reorderableDots — selection door', () => {
  it('lights every item of an equal-gap multi-selection', () => {
    const entities = [box('a', 100), box('b', 250), box('c', 400)] // gap 50
    const dots = reorderableDots({ entities, selectedEntityIds: ['a', 'b', 'c'] })
    expect(ids(dots)).toEqual(['a', 'b', 'c'])
  })

  it('returns the entity screen center', () => {
    const entities = [box('a', 100), box('b', 250)]
    const [dot] = reorderableDots({ entities, selectedEntityIds: ['a', 'b'] }).filter(
      (d) => d.id === 'a',
    )
    expect(dot.center).toEqual({ x: 150, y: 220 })
  })

  it('lights nothing on an unequal-gap selection', () => {
    const entities = [box('a', 100), box('b', 250), box('c', 900)] // second gap huge
    expect(reorderableDots({ entities, selectedEntityIds: ['a', 'b', 'c'] })).toEqual([])
  })

  it('lights nothing on a single selection', () => {
    const entities = [box('a', 100), box('b', 250)]
    expect(reorderableDots({ entities, selectedEntityIds: ['a'] })).toEqual([])
  })

  it('lights a shell-even row of mixed-frame pages (content gaps differ)', () => {
    // Shells packed at a constant 40px gap: a[0..200], b[240..440], c[480..680].
    // But 'a' is unframed and 'b','c' carry a 30px bezel, so the *content*-box
    // gaps come out unequal — the pre-screen-box detector would drop the dots.
    const entities = [
      framedPage('a', 0, 200, 0),
      framedPage('b', 240, 140, 30),
      framedPage('c', 480, 140, 30),
    ]
    const dots = reorderableDots({ entities, selectedEntityIds: ['a', 'b', 'c'] })
    expect(ids(dots)).toEqual(['a', 'b', 'c'])
    // Dot rides the shell center, not the content center.
    const a = dots.find((d) => d.id === 'a')!
    expect(a.center.x).toBe(100)
  })
})

describe('reorderableDots — managed door', () => {
  it('lights the children of a selected managed-row group', () => {
    const entities = [managedRow('g', ['a', 'b']), box('a', 100), box('b', 250)]
    const dots = reorderableDots({ entities, selectedEntityIds: [], selectedGroupId: 'g' })
    expect(ids(dots)).toEqual(['a', 'b'])
  })

  it('lights a managed child when the child itself is selected', () => {
    const entities = [managedRow('g', ['a', 'b']), box('a', 100), box('b', 250)]
    const dots = reorderableDots({ entities, selectedEntityIds: ['a'] })
    expect(ids(dots)).toEqual(['a'])
  })

  it('lights nothing when neither group nor child is selected', () => {
    const entities = [managedRow('g', ['a', 'b']), box('a', 100), box('b', 250)]
    expect(reorderableDots({ entities, selectedEntityIds: [] })).toEqual([])
  })
})

describe('reorderableDots — union', () => {
  it('dedupes a child covered by both doors', () => {
    // g's children a,b are an equal-gap row AND a managed group; selecting both
    // children would arm both doors. Each id appears once.
    const entities = [managedRow('g', ['a', 'b']), box('a', 100), box('b', 250)]
    const dots = reorderableDots({ entities, selectedEntityIds: ['a', 'b'], selectedGroupId: 'g' })
    expect(ids(dots)).toEqual(['a', 'b'])
  })
})
