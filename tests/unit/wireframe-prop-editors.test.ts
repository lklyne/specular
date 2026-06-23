import { describe, expect, it } from 'vitest'
import {
  editorDisplayValue,
  editorsForNodeType,
  patchForEditorChange,
  type WireframePropEditor,
} from '../../src/shared/wireframe/wireframe-prop-editors'
import { updateNodeProps } from '../../src/shared/wireframe/wireframe-ops'
import type { WireframeFile } from '../../src/shared/wireframe/wireframe-types'

function editor(type: string, key: string): WireframePropEditor {
  const found = editorsForNodeType(type).find((e) => e.key === key)
  if (!found) throw new Error(`no ${key} editor for ${type}`)
  return found
}

describe('editorsForNodeType', () => {
  it('exposes the documented editors per node type', () => {
    expect(editorsForNodeType('frame').map((e) => e.key)).toEqual([
      'direction',
      'gap',
      'padding',
      'width',
      'height',
    ])
    expect(editorsForNodeType('text').map((e) => e.key)).toEqual(['level'])
    expect(editorsForNodeType('button').map((e) => e.key)).toEqual(['variant'])
    expect(editorsForNodeType('input').map((e) => e.key)).toEqual(['label'])
    expect(editorsForNodeType('dropdown').map((e) => e.key)).toEqual(['label', 'options'])
  })

  it('has no editors for leaf nodes or unknown types', () => {
    expect(editorsForNodeType('divider')).toEqual([])
    expect(editorsForNodeType('spacer')).toEqual([])
    expect(editorsForNodeType('mystery')).toEqual([])
  })
})

describe('editorDisplayValue', () => {
  it('reads the current value off the node, blank when unset', () => {
    const node = { id: 'f', type: 'frame', direction: 'horizontal', gap: 8 }
    expect(editorDisplayValue(node, editor('frame', 'direction'))).toBe('horizontal')
    expect(editorDisplayValue(node, editor('frame', 'gap'))).toBe('8')
    expect(editorDisplayValue(node, editor('frame', 'padding'))).toBe('')
  })

  it('joins dropdown options into a comma list', () => {
    const node = { id: 'd', type: 'dropdown', options: ['One', 'Two'] }
    expect(editorDisplayValue(node, editor('dropdown', 'options'))).toBe('One, Two')
  })
})

// The controller mapping: an editor change → the `setProps` patch the panel
// dispatches. Coercion per control type is the load-bearing behavior.
describe('patchForEditorChange', () => {
  it('select / text pass the raw string through', () => {
    expect(patchForEditorChange(editor('frame', 'direction'), 'horizontal')).toEqual({
      direction: 'horizontal',
    })
    expect(patchForEditorChange(editor('button', 'variant'), 'ghost')).toEqual({ variant: 'ghost' })
    expect(patchForEditorChange(editor('input', 'label'), 'Email')).toEqual({ label: 'Email' })
  })

  it('number coerces and rejects non-numeric / empty input', () => {
    expect(patchForEditorChange(editor('frame', 'gap'), '12')).toEqual({ gap: 12 })
    expect(patchForEditorChange(editor('frame', 'gap'), '')).toBeNull()
    expect(patchForEditorChange(editor('frame', 'gap'), 'wide')).toBeNull()
  })

  it('sizing accepts fill/hug verbatim or a numeric px', () => {
    expect(patchForEditorChange(editor('frame', 'width'), 'fill')).toEqual({ width: 'fill' })
    expect(patchForEditorChange(editor('frame', 'height'), 'hug')).toEqual({ height: 'hug' })
    expect(patchForEditorChange(editor('frame', 'width'), '320')).toEqual({ width: 320 })
    expect(patchForEditorChange(editor('frame', 'width'), '')).toBeNull()
  })

  it('options splits a comma list, trimming and dropping empties', () => {
    expect(patchForEditorChange(editor('dropdown', 'options'), 'A, B ,, C')).toEqual({
      options: ['A', 'B', 'C'],
    })
    expect(patchForEditorChange(editor('dropdown', 'options'), '')).toEqual({ options: [] })
  })

  it('produces patches that updateNodeProps accepts (legal keys per type)', () => {
    const file: WireframeFile = {
      version: '1.0',
      root: {
        id: 'root',
        type: 'frame',
        direction: 'vertical',
        children: [{ id: 'cta', type: 'button', text: 'Go' }],
      },
    }
    const dir = patchForEditorChange(editor('frame', 'direction'), 'horizontal')!
    const variant = patchForEditorChange(editor('button', 'variant'), 'secondary')!
    const next = updateNodeProps(updateNodeProps(file, 'root', dir), 'cta', variant)
    expect((next.root as { direction: string }).direction).toBe('horizontal')
  })
})
