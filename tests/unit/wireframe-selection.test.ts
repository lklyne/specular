import { describe, expect, it } from 'vitest'
import type { WireframeFile } from '../../src/shared/wireframe/wireframe-types'
import {
  EMPTY_WIREFRAME_SELECTION,
  nodeHasEditableText,
  wireframeSelectionReducer,
  type WireframeSelectionState,
} from '../../src/shared/wireframe/wireframe-selection'

// A small tree: a vertical frame with a heading (editable text), a primary
// button (editable text), and a divider (no editable text).
const file: WireframeFile = {
  version: '1.0',
  root: {
    id: 'root',
    type: 'frame',
    direction: 'vertical',
    children: [
      { id: 'title', type: 'text', text: 'Hello', level: 'h1' },
      { id: 'cta', type: 'button', text: 'Submit', variant: 'primary' },
      { id: 'rule', type: 'divider' },
    ],
  },
}

const selected = (id: string): WireframeSelectionState => ({
  selectedNodeId: id,
  editingNodeId: null,
})
const editing = (id: string): WireframeSelectionState => ({
  selectedNodeId: id,
  editingNodeId: id,
})

describe('nodeHasEditableText', () => {
  it('is true for text/button/input/dropdown/checkbox/toggle, false otherwise', () => {
    expect(nodeHasEditableText(file.root.children![0])).toBe(true) // text
    expect(nodeHasEditableText(file.root.children![1])).toBe(true) // button
    expect(nodeHasEditableText(file.root.children![2])).toBe(false) // divider
    expect(nodeHasEditableText(file.root)).toBe(false) // frame
  })
})

describe('wireframeSelectionReducer', () => {
  it('select-node maps a click (hit data) to the selected node id', () => {
    const next = wireframeSelectionReducer(
      EMPTY_WIREFRAME_SELECTION,
      { kind: 'select-node', nodeId: 'title' },
      file,
    )
    expect(next).toEqual({ selectedNodeId: 'title', editingNodeId: null })
  })

  it('select-node returns the same reference when already selected (no edit)', () => {
    const state = selected('title')
    const next = wireframeSelectionReducer(state, { kind: 'select-node', nodeId: 'title' }, file)
    expect(next).toBe(state)
  })

  it('select-node drops out of an active edit and selects the new node', () => {
    const next = wireframeSelectionReducer(
      editing('title'),
      { kind: 'select-node', nodeId: 'cta' },
      file,
    )
    expect(next).toEqual({ selectedNodeId: 'cta', editingNodeId: null })
  })

  it('request-edit promotes a selected editable node to edit (double-click / Enter)', () => {
    const next = wireframeSelectionReducer(
      selected('title'),
      { kind: 'request-edit', nodeId: 'title' },
      file,
    )
    expect(next).toEqual({ selectedNodeId: 'title', editingNodeId: 'title' })
  })

  it('request-edit on a non-editable node only selects it (no edit)', () => {
    const next = wireframeSelectionReducer(
      EMPTY_WIREFRAME_SELECTION,
      { kind: 'request-edit', nodeId: 'rule' },
      file,
    )
    expect(next).toEqual({ selectedNodeId: 'rule', editingNodeId: null })
  })

  it('request-edit on an unknown node id only selects it (no crash, no edit)', () => {
    const next = wireframeSelectionReducer(
      EMPTY_WIREFRAME_SELECTION,
      { kind: 'request-edit', nodeId: 'ghost' },
      file,
    )
    expect(next).toEqual({ selectedNodeId: 'ghost', editingNodeId: null })
  })

  it('commit-edit keeps the node selected and exits edit', () => {
    const next = wireframeSelectionReducer(editing('cta'), { kind: 'commit-edit' }, file)
    expect(next).toEqual({ selectedNodeId: 'cta', editingNodeId: null })
  })

  it('escape steps out of edit first (keeps selection), then clears on a second press', () => {
    const afterFirst = wireframeSelectionReducer(editing('title'), { kind: 'escape' }, file)
    expect(afterFirst).toEqual({ selectedNodeId: 'title', editingNodeId: null })

    const afterSecond = wireframeSelectionReducer(afterFirst, { kind: 'escape' }, file)
    expect(afterSecond).toEqual(EMPTY_WIREFRAME_SELECTION)
  })

  it('select-background clears selection and edit', () => {
    expect(
      wireframeSelectionReducer(editing('title'), { kind: 'select-background' }, file),
    ).toEqual(EMPTY_WIREFRAME_SELECTION)
  })

  it('select-background is a no-op (same reference) when nothing is selected', () => {
    const next = wireframeSelectionReducer(
      EMPTY_WIREFRAME_SELECTION,
      { kind: 'select-background' },
      file,
    )
    expect(next).toBe(EMPTY_WIREFRAME_SELECTION)
  })
})
