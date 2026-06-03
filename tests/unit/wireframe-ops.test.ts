import { describe, expect, it } from 'vitest'
import type { WireframeFile, WireframeNode } from '../../src/shared/wireframe/wireframe-types'
import {
  createNodeIdGenerator,
  deleteNode,
  duplicateNode,
  findNodeById,
  insertNode,
  reorderNode,
  updateNodeProps,
  updateNodeText,
  validateWireframe,
} from '../../src/shared/wireframe/wireframe-ops'

// A small but representative tree: a vertical root frame holding a heading, a
// nested row frame with two buttons, and a checkbox.
function sampleFile(): WireframeFile {
  return {
    version: '1.0',
    root: {
      id: 'root',
      type: 'frame',
      direction: 'vertical',
      children: [
        { id: 'title', type: 'text', text: 'Hello', level: 'h1' },
        {
          id: 'row',
          type: 'frame',
          direction: 'horizontal',
          children: [
            { id: 'btn-a', type: 'button', text: 'A', variant: 'primary' },
            { id: 'btn-b', type: 'button', text: 'B', variant: 'secondary' },
          ],
        },
        { id: 'agree', type: 'checkbox', label: 'Agree', checked: false },
      ],
    },
  }
}

function collectIds(node: WireframeNode, acc: string[] = []): string[] {
  acc.push(node.id)
  if (node.type === 'frame') for (const c of node.children) collectIds(c, acc)
  return acc
}

describe('insertNode', () => {
  it('places the node at the requested index', () => {
    const node: WireframeNode = { id: 'new', type: 'divider' }
    const next = insertNode(sampleFile(), 'root', 1, node)
    const root = next.root as Extract<WireframeNode, { type: 'frame' }>
    expect(root.children.map((c) => c.id)).toEqual(['title', 'new', 'row', 'agree'])
  })

  it('clamps an out-of-range index to the end', () => {
    const node: WireframeNode = { id: 'new', type: 'divider' }
    const next = insertNode(sampleFile(), 'root', 99, node)
    const root = next.root as Extract<WireframeNode, { type: 'frame' }>
    expect(root.children.map((c) => c.id)).toEqual(['title', 'row', 'agree', 'new'])
  })

  it('clamps a negative index to the front', () => {
    const node: WireframeNode = { id: 'new', type: 'divider' }
    const next = insertNode(sampleFile(), 'root', -5, node)
    const root = next.root as Extract<WireframeNode, { type: 'frame' }>
    expect(root.children[0].id).toBe('new')
  })

  it('rejects a non-frame parent (no-op)', () => {
    const file = sampleFile()
    const node: WireframeNode = { id: 'new', type: 'divider' }
    expect(insertNode(file, 'title', 0, node)).toEqual(file)
  })
})

describe('deleteNode', () => {
  it('removes the whole subtree', () => {
    const next = deleteNode(sampleFile(), 'row')
    expect(findNodeById(next.root, 'row')).toBeNull()
    expect(findNodeById(next.root, 'btn-a')).toBeNull()
    expect(findNodeById(next.root, 'btn-b')).toBeNull()
    expect(findNodeById(next.root, 'title')).not.toBeNull()
  })

  it('is a no-op for an unknown id', () => {
    const file = sampleFile()
    expect(deleteNode(file, 'nope')).toEqual(file)
  })

  it('refuses to delete the root', () => {
    const file = sampleFile()
    expect(deleteNode(file, 'root')).toEqual(file)
  })
})

describe('duplicateNode', () => {
  it('produces a structurally-equal subtree with all-fresh ids, inserted after the source', () => {
    const file = sampleFile()
    const next = duplicateNode(file, 'row', createNodeIdGenerator('dup'))

    const root = next.root as Extract<WireframeNode, { type: 'frame' }>
    // Inserted directly after the original 'row'.
    expect(root.children.map((c) => c.id)).toEqual(['title', 'row', 'dup-1', 'agree'])

    const clone = findNodeById(next.root, 'dup-1') as Extract<WireframeNode, { type: 'frame' }>
    const original = findNodeById(file.root, 'row') as Extract<WireframeNode, { type: 'frame' }>

    // Same shape (direction + child types/props), different ids.
    expect(clone.direction).toBe(original.direction)
    expect(clone.children.map((c) => c.type)).toEqual(original.children.map((c) => c.type))

    // No id collision with the source tree.
    const sourceIds = new Set(collectIds(file.root))
    for (const id of collectIds(clone)) {
      expect(sourceIds.has(id)).toBe(false)
    }
    // Every node in the clone got a fresh id (deep clone, not a shallow alias).
    expect(collectIds(clone)).toEqual(['dup-1', 'dup-2', 'dup-3'])
  })

  it('is a no-op for an unknown id', () => {
    const file = sampleFile()
    expect(duplicateNode(file, 'nope', createNodeIdGenerator('dup'))).toEqual(file)
  })

  it('is a no-op for the root (no parent to duplicate into)', () => {
    const file = sampleFile()
    expect(duplicateNode(file, 'root', createNodeIdGenerator('dup'))).toEqual(file)
  })
})

describe('updateNodeProps', () => {
  it('patches only the named node', () => {
    const file = sampleFile()
    const next = updateNodeProps(file, 'btn-a', { variant: 'ghost' })
    expect(findNodeById(next.root, 'btn-a')).toMatchObject({ variant: 'ghost' })
    // Sibling untouched.
    expect(findNodeById(next.root, 'btn-b')).toMatchObject({ variant: 'secondary' })
  })

  it('rejects a prop that is illegal for the node type', () => {
    const file = sampleFile()
    // 'level' belongs to text, not button.
    expect(() => updateNodeProps(file, 'btn-a', { level: 'h1' })).toThrow()
  })

  it('is a no-op for an unknown id', () => {
    const file = sampleFile()
    expect(updateNodeProps(file, 'nope', { variant: 'ghost' })).toEqual(file)
  })
})

describe('validateWireframe', () => {
  it('round-trips a valid file as ok', () => {
    expect(validateWireframe(sampleFile())).toEqual({ ok: true })
  })

  it('rejects a missing version', () => {
    const { version, ...rest } = sampleFile()
    void version
    const result = validateWireframe(rest)
    expect(result.ok).toBe(false)
    if (!result.ok) expect(result.errors.join(' ')).toMatch(/version/i)
  })

  it('rejects a non-frame root', () => {
    const result = validateWireframe({ version: '1.0', root: { id: 'r', type: 'text', text: 'x' } })
    expect(result.ok).toBe(false)
    if (!result.ok) expect(result.errors.join(' ')).toMatch(/root.*frame/i)
  })

  it('rejects an unknown node type', () => {
    const result = validateWireframe({
      version: '1.0',
      root: { id: 'r', type: 'frame', children: [{ id: 'x', type: 'widget' }] },
    })
    expect(result.ok).toBe(false)
    if (!result.ok) expect(result.errors.join(' ')).toMatch(/unknown node type/i)
  })

  it('rejects a child on a non-frame node', () => {
    const result = validateWireframe({
      version: '1.0',
      root: {
        id: 'r',
        type: 'frame',
        children: [{ id: 'x', type: 'text', text: 'x', children: [{ id: 'y', type: 'divider' }] }],
      },
    })
    expect(result.ok).toBe(false)
    if (!result.ok) expect(result.errors.join(' ')).toMatch(/cannot have children/i)
  })
})

// Refactor guard: the ops lifted from the renderer still behave identically.
describe('lifted ops (refactor guard)', () => {
  it('reorderNode moves a child to a new index', () => {
    const next = reorderNode(sampleFile(), 'agree', 'root', 0)
    const root = next.root as Extract<WireframeNode, { type: 'frame' }>
    expect(root.children[0].id).toBe('agree')
  })

  it('reorderNode moves a node across frames', () => {
    const next = reorderNode(sampleFile(), 'btn-a', 'root', 0)
    const root = next.root as Extract<WireframeNode, { type: 'frame' }>
    expect(root.children[0].id).toBe('btn-a')
    const row = findNodeById(next.root, 'row') as Extract<WireframeNode, { type: 'frame' }>
    expect(row.children.map((c) => c.id)).toEqual(['btn-b'])
  })

  it('updateNodeText patches the named node only', () => {
    const next = updateNodeText(sampleFile(), 'title', 'Goodbye')
    expect(findNodeById(next.root, 'title')).toMatchObject({ text: 'Goodbye' })
  })
})
