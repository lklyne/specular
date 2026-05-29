import { describe, expect, it } from 'vitest'
import {
  bringToFront,
  enforceGroupContiguity,
  moveBackward,
  moveForward,
  sendToBack,
} from '../../src/shared/entity-order-math'

describe('entity-order stack mutations', () => {
  it('moves a multi-selection forward as one block', () => {
    expect(moveForward(['a', 'b', 'c', 'd'], ['b', 'c'])).toEqual(['a', 'd', 'b', 'c'])
  })

  it('moves a multi-selection backward as one block', () => {
    expect(moveBackward(['a', 'b', 'c', 'd'], ['b', 'c'])).toEqual(['b', 'c', 'a', 'd'])
  })

  it('brings a multi-selection to front while preserving relative order', () => {
    expect(bringToFront(['a', 'b', 'c', 'd'], ['b', 'd'])).toEqual(['a', 'c', 'b', 'd'])
  })

  it('sends a multi-selection to back while preserving relative order', () => {
    expect(sendToBack(['a', 'b', 'c', 'd'], ['b', 'd'])).toEqual(['b', 'd', 'a', 'c'])
  })

  it('keeps nested group runs contiguous after a mutation', () => {
    const groups = [
      { id: 'outer', parentGroupId: null, childIds: ['a', 'inner'] },
      { id: 'inner', parentGroupId: 'outer', childIds: ['b'] },
    ]
    const next = enforceGroupContiguity(
      moveForward(['a', 'outer', 'x', 'b', 'inner', 'y'], ['outer']),
      groups,
    )

    expect(next).toEqual(['x', 'a', 'b', 'inner', 'outer', 'y'])
  })
})
