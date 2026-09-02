/**
 * Structural sharing is what lets memoized consumers skip an unchanged branch
 * of the scene. The regression it protects against: a layout pass (or an IPC
 * delivery) handing every entity fresh identity even though only one field
 * moved, which re-renders the whole canvas on every hover.
 *
 * Mutation-verified by replacing `unchanged ? previous : shared` with `shared`
 * in `shareObject` and confirming the identity assertions below fail.
 */

import { describe, expect, it } from 'vitest'
import { shareLayoutData, shareStructure } from '../../src/shared/layout-structural-share'
import type { LayoutUpdateData } from '../../src/shared/types'

function entity(id: string, canvasX: number): Record<string, unknown> {
  return { id, kind: 'text', canvasX, canvasY: 0, width: 10, height: 10 }
}

function scene(entities: Record<string, unknown>[], hoverId: string | null) {
  return {
    zoom: 1,
    pan: { x: 0, y: 0 },
    entities,
    hover: hoverId ? { kind: 'entity', id: hoverId } : null,
  }
}

describe('shareStructure', () => {
  it('returns the previous value when the rebuild is deep-equal', () => {
    const previous = scene([entity('a', 0), entity('b', 5)], 'a')
    const next = scene([entity('a', 0), entity('b', 5)], 'a')

    expect(shareStructure(previous, next)).toBe(previous)
  })

  it('gives new identity only to the changed branch and its container', () => {
    const previous = scene([entity('a', 0), entity('b', 5)], null)
    const next = scene([entity('a', 0), entity('b', 99)], null)

    const shared = shareStructure(previous, next)

    expect(shared).not.toBe(previous)
    expect(shared.entities).not.toBe(previous.entities)
    expect(shared.entities[1]).toEqual(next.entities[1])
    expect(shared.entities[0]).toBe(previous.entities[0])
    expect(shared.pan).toBe(previous.pan)
  })

  it('keeps sibling identity when an entity is inserted ahead of them', () => {
    const previous = scene([entity('a', 0), entity('b', 5)], null)
    const next = scene([entity('c', 1), entity('a', 0), entity('b', 5)], null)

    const shared = shareStructure(previous, next)

    expect(shared.entities).toHaveLength(3)
    expect(shared.entities[1]).toBe(previous.entities[0])
    expect(shared.entities[2]).toBe(previous.entities[1])
  })

  it('does not share across a key the rebuild dropped', () => {
    const previous = { a: 1, b: 2 }
    const next = { a: 1 }

    expect(shareStructure(previous, next)).toEqual({ a: 1 })
    expect(shareStructure(previous, next)).not.toBe(previous)
  })

  it('treats a value that turned undefined as a change', () => {
    const previous = { hover: { id: 'a' } }
    const next = { hover: undefined }

    expect(shareStructure(previous, next)).not.toBe(previous)
  })
})

describe('shareLayoutData', () => {
  it('shares across a differing build time, which every pass reports fresh', () => {
    const previous = { ...scene([entity('a', 0)], 'a'), buildMs: 4 } as unknown as LayoutUpdateData
    const next = { ...scene([entity('a', 0)], 'a'), buildMs: 11 } as unknown as LayoutUpdateData

    expect(shareLayoutData(previous, next)).toBe(previous)
  })

  it('shares when the previous pass carried a build time and the new one does not', () => {
    const previous = { ...scene([entity('a', 0)], 'a'), buildMs: 4 } as unknown as LayoutUpdateData
    const next = scene([entity('a', 0)], 'a') as unknown as LayoutUpdateData

    expect(shareLayoutData(previous, next)).toBe(previous)
  })

  it('still reports a real change alongside a differing build time', () => {
    const previous = { ...scene([entity('a', 0)], 'a'), buildMs: 4 } as unknown as LayoutUpdateData
    const next = { ...scene([entity('a', 0)], 'b'), buildMs: 11 } as unknown as LayoutUpdateData

    const shared = shareLayoutData(previous, next)

    expect(shared).not.toBe(previous)
    expect(shared.entities[0]).toBe(previous.entities[0])
    expect(shared.hover).toEqual({ kind: 'entity', id: 'b' })
  })
})
