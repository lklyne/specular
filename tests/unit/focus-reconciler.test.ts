import { describe, expect, it } from 'vitest'
import { expectedFocus, focusKey, type FocusState } from '../../src/main/runtime/focus-reconciler'

function state(overrides: Partial<FocusState> = {}): FocusState {
  return {
    interactionMode: 'idle',
    editingEntityId: null,
    selectedPageId: null,
    commentOverlayActive: false,
    pendingFocus: null,
    focusedPageId: null,
    sidebarTextInputActive: false,
    toolbarTextInputActive: false,
    ...overrides,
  }
}

describe('expectedFocus', () => {
  it('defaults to aboveView in idle canvas mode (Phase F: aboveView is the keyboard owner)', () => {
    expect(expectedFocus(state())).toEqual({ kind: 'aboveView' })
  })

  it('does not focus a selected page unless the page-focus predicate elects it', () => {
    expect(expectedFocus(state({ selectedPageId: 'p1' })))
      .toEqual({ kind: 'aboveView' })
  })

  for (const mode of ['panning', 'marquee', 'dragging-entities', 'resizing-entity', 'resizing-multi-selection', 'dragging-edge'] as const) {
    it(`routes ${mode} to aboveView`, () => {
      expect(expectedFocus(state({ interactionMode: mode }))).toEqual({ kind: 'aboveView' })
    })
  }

  it('editing-entity routes to aboveView (post-Phase-C: inline editors live in aboveView)', () => {
    expect(expectedFocus(state({ interactionMode: 'editing-entity', editingEntityId: 'e1' })))
      .toEqual({ kind: 'aboveView' })
  })

  it('routes to aboveView when comment overlay is active', () => {
    expect(expectedFocus(state({ commentOverlayActive: true }))).toEqual({ kind: 'aboveView' })
  })

  describe('focusedPageId (predicate-derived keyboard target)', () => {
    it('routes to the target page in idle canvas mode', () => {
      expect(expectedFocus(state({ focusedPageId: 'f1' })))
        .toEqual({ kind: 'page', id: 'f1' })
    })
    it('yields to active gestures', () => {
      expect(expectedFocus(state({ focusedPageId: 'f1', interactionMode: 'panning' })))
        .toEqual({ kind: 'aboveView' })
    })
    it('yields to comment overlay', () => {
      expect(expectedFocus(state({ focusedPageId: 'f1', commentOverlayActive: true })))
        .toEqual({ kind: 'aboveView' })
    })
    it('yields to explicit pendingFocus', () => {
      expect(expectedFocus(state({ focusedPageId: 'f1', pendingFocus: { kind: 'toolbar' } })))
        .toEqual({ kind: 'toolbar' })
    })
  })

  it('routes to sidebar when sidebarTextInputActive', () => {
    expect(expectedFocus(state({ sidebarTextInputActive: true }))).toEqual({ kind: 'sidebar' })
  })

  it('sidebarTextInputActive overrides page focus', () => {
    expect(expectedFocus(state({ sidebarTextInputActive: true, focusedPageId: 'p1' })))
      .toEqual({ kind: 'sidebar' })
  })

  it('sidebarTextInputActive yields to explicit pendingFocus', () => {
    expect(expectedFocus(state({ sidebarTextInputActive: true, pendingFocus: { kind: 'toolbar' } })))
      .toEqual({ kind: 'toolbar' })
  })

  it('routes to toolbar when toolbarTextInputActive', () => {
    expect(expectedFocus(state({ toolbarTextInputActive: true }))).toEqual({ kind: 'toolbar' })
  })

  it('toolbarTextInputActive overrides page focus', () => {
    expect(expectedFocus(state({ toolbarTextInputActive: true, focusedPageId: 'p1' })))
      .toEqual({ kind: 'toolbar' })
  })

  it('pendingFocus overrides derivation', () => {
    expect(expectedFocus(state({ pendingFocus: { kind: 'toolbar' } })))
      .toEqual({ kind: 'toolbar' })
    expect(expectedFocus(state({
      selectedPageId: 'p1',
      pendingFocus: { kind: 'bgView' },
    }))).toEqual({ kind: 'bgView' })
  })
})

describe('focusKey', () => {
  it('is stable and distinct across targets', () => {
    expect(focusKey({ kind: 'bgView' })).toBe('bgView')
    expect(focusKey({ kind: 'aboveView' })).toBe('aboveView')
    expect(focusKey({ kind: 'page', id: 'p1' })).toBe('page:p1')
    expect(focusKey({ kind: 'page', id: 'p2' })).toBe('page:p2')
  })
})
