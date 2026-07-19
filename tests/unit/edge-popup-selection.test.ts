import { describe, expect, it } from 'vitest'
import type {
  CanvasSelectableTarget,
  WorkspaceEdge,
} from '../../src/shared/types'
import { edgeForPopup } from '../../src/renderer/above-view/edgePopupSelection'

const edge = {
  id: 'edge-1',
  fromEntityId: 'shape-1',
  toEntityId: 'shape-2',
} as WorkspaceEdge

function subject(selection: CanvasSelectableTarget[]) {
  return edgeForPopup(selection, [edge])
}

describe('edgeForPopup', () => {
  it('returns the edge for a single-edge selection', () => {
    expect(subject([{ kind: 'edge', id: edge.id }])).toBe(edge)
  })

  it('suppresses the edge popup when an edge is selected with canvas items', () => {
    expect(
      subject([
        { kind: 'shape', id: 'shape-1' },
        { kind: 'edge', id: edge.id },
        { kind: 'page', id: 'page-1' },
      ]),
    ).toBeNull()
  })

  it('keeps the current single-edge popup from representing multiple edges', () => {
    expect(
      subject([
        { kind: 'edge', id: edge.id },
        { kind: 'edge', id: 'edge-2' },
      ]),
    ).toBeNull()
  })
})
