import { describe, expect, it } from 'vitest'
import { applyNodeAction } from '../../src/shared/wireframe/wireframe-node-actions'
import { findNodeById } from '../../src/shared/wireframe/wireframe-ops'
import type { WireframeFile, WireframeNode } from '../../src/shared/wireframe/wireframe-types'

function sampleFile(): WireframeFile {
  return {
    version: '1.0',
    root: {
      id: 'root',
      type: 'frame',
      direction: 'vertical',
      children: [
        { id: 'title', type: 'text', text: 'Hello' },
        {
          id: 'row',
          type: 'frame',
          direction: 'horizontal',
          children: [{ id: 'btn-a', type: 'button', text: 'A' }],
        },
      ],
    },
  }
}

function rootChildIds(file: WireframeFile): string[] {
  const root = file.root as Extract<WireframeNode, { type: 'frame' }>
  return root.children.map((c) => c.id)
}

describe('applyNodeAction', () => {
  it('delete removes the selected subtree and clears the selection', () => {
    const result = applyNodeAction(sampleFile(), 'delete', 'row', 0)
    expect(result).not.toBeNull()
    expect(findNodeById(result!.file.root, 'row')).toBeNull()
    expect(findNodeById(result!.file.root, 'btn-a')).toBeNull()
    expect(result!.nextSelectedNodeId).toBeNull()
  })

  it('duplicate inserts a fresh-id clone after the source and selects the clone', () => {
    const result = applyNodeAction(sampleFile(), 'duplicate', 'row', 1)
    expect(result).not.toBeNull()
    // Clone takes the first generated id and is inserted directly after 'row'.
    expect(rootChildIds(result!.file)).toEqual(['title', 'row', 'dup1-1'])
    expect(result!.nextSelectedNodeId).toBe('dup1-1')
    // The clone is structurally equal but shares no ids with the source.
    const clone = findNodeById(result!.file.root, 'dup1-1') as Extract<
      WireframeNode,
      { type: 'frame' }
    >
    expect(clone.children.map((c) => c.id)).toEqual(['dup1-2'])
  })

  it('the seq makes repeated duplicate ids deterministic and non-colliding', () => {
    const first = applyNodeAction(sampleFile(), 'duplicate', 'title', 1)
    const second = applyNodeAction(sampleFile(), 'duplicate', 'title', 2)
    expect(first!.nextSelectedNodeId).toBe('dup1-1')
    expect(second!.nextSelectedNodeId).toBe('dup2-1')
  })

  it('is a no-op when nothing is selected', () => {
    expect(applyNodeAction(sampleFile(), 'delete', null, 0)).toBeNull()
    expect(applyNodeAction(sampleFile(), 'duplicate', null, 0)).toBeNull()
  })

  it('refuses to delete or duplicate the root', () => {
    expect(applyNodeAction(sampleFile(), 'delete', 'root', 0)).toBeNull()
    expect(applyNodeAction(sampleFile(), 'duplicate', 'root', 1)).toBeNull()
  })

  it('is a no-op for an unknown node id', () => {
    expect(applyNodeAction(sampleFile(), 'delete', 'nope', 0)).toBeNull()
  })
})
