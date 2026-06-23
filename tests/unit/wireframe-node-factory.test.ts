import { describe, expect, it } from 'vitest'
import {
  createWireframeNode,
  type WireframePaletteType,
} from '../../src/shared/wireframe/wireframe-node-factory'
import { createNodeIdGenerator, validateWireframe } from '../../src/shared/wireframe/wireframe-ops'
import type { WireframeFile } from '../../src/shared/wireframe/wireframe-types'

describe('createWireframeNode', () => {
  it('takes its id from genId (deterministic-by-input)', () => {
    const node = createWireframeNode('text', createNodeIdGenerator('add'))
    expect(node.id).toBe('add-1')
  })

  it('maps the palette "page" alias to an empty vertical frame', () => {
    const node = createWireframeNode('page', () => 'n')
    expect(node).toEqual({ id: 'n', type: 'frame', direction: 'vertical', children: [] })
  })

  it('frame is empty and ready to receive children', () => {
    const node = createWireframeNode('frame', () => 'n')
    expect(node.type).toBe('frame')
    if (node.type === 'frame') expect(node.children).toEqual([])
  })

  const cases: { type: WireframePaletteType; type_: string }[] = [
    { type: 'text', type_: 'text' },
    { type: 'button', type_: 'button' },
    { type: 'input', type_: 'input' },
    { type: 'dropdown', type_: 'dropdown' },
    { type: 'checkbox', type_: 'checkbox' },
    { type: 'toggle', type_: 'toggle' },
    { type: 'image', type_: 'image' },
    { type: 'divider', type_: 'divider' },
    { type: 'spacer', type_: 'spacer' },
  ]

  it.each(cases)('builds a valid $type node that the schema accepts', ({ type, type_ }) => {
    const node = createWireframeNode(type, () => `id-${type}`)
    expect(node.type).toBe(type_)
    expect(node.id).toBe(`id-${type}`)
    // The node must drop cleanly into a tree the validator accepts.
    const file: WireframeFile = {
      version: '1.0',
      root: { id: 'root', type: 'frame', children: [node] },
    }
    expect(validateWireframe(file)).toEqual({ ok: true })
  })
})
