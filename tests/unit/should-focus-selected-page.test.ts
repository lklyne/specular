import { describe, expect, it } from 'vitest'
import {
  shouldFocusSelectedPage,
  type ShouldFocusSelectedPageInputs,
} from '../../src/shared/should-focus-selected-page'
import type { Tool } from '../../src/shared/tool'

function inputs(
  overrides: Partial<ShouldFocusSelectedPageInputs> = {},
): ShouldFocusSelectedPageInputs {
  return {
    selection: { kind: 'single-entity', entityId: 'f1', entityKind: 'page' },
    interactionKind: 'idle',
    activeTool: { kind: 'select' },
    commentOverlayActive: false,
    interactivePageId: 'f1',
    ...overrides,
  }
}

describe('shouldFocusSelectedPage — happy path', () => {
  it('returns the page id when the selected page has been entered', () => {
    expect(shouldFocusSelectedPage(inputs())).toBe('f1')
  })

  it('select-first / interact-second: a selected-but-not-entered page owns no keyboard', () => {
    expect(shouldFocusSelectedPage(inputs({ interactivePageId: null }))).toBeNull()
  })

  it('returns null when the entered page is a different page than the selection', () => {
    expect(shouldFocusSelectedPage(inputs({ interactivePageId: 'f2' }))).toBeNull()
  })
})

describe('shouldFocusSelectedPage — selection shape', () => {
  it('returns null when nothing is selected', () => {
    expect(shouldFocusSelectedPage(inputs({ selection: { kind: 'none' } }))).toBeNull()
  })

  it('returns null when single-selected entity is not a page', () => {
    expect(
      shouldFocusSelectedPage(
        inputs({
          selection: { kind: 'single-entity', entityId: 't1', entityKind: 'text' },
        }),
      ),
    ).toBeNull()
  })

  it('returns null when a page is part of a multi-selection', () => {
    expect(
      shouldFocusSelectedPage(
        inputs({
          selection: { kind: 'multi-entity', entityIds: ['f1', 'f2'] },
        }),
      ),
    ).toBeNull()
  })
})

describe('shouldFocusSelectedPage — divergence cases (plan §8 Phase A)', () => {
  it('case 1: inline entity editor active — interactionKind=editing-entity excludes', () => {
    expect(
      shouldFocusSelectedPage(inputs({ interactionKind: 'editing-entity' })),
    ).toBeNull()
  })

  it('case 2: activeTool=draw with a page selected excludes', () => {
    expect(shouldFocusSelectedPage(inputs({ activeTool: { kind: 'draw' } }))).toBeNull()
  })

  it('case 3: active drag of the single-selected page excludes', () => {
    expect(
      shouldFocusSelectedPage(inputs({ interactionKind: 'dragging-entities' })),
    ).toBeNull()
  })

  it('case 4: activeTool=inspect with a page selected excludes', () => {
    expect(shouldFocusSelectedPage(inputs({ activeTool: { kind: 'inspect' } }))).toBeNull()
  })

  it('case 4: activeTool=comment with a page selected excludes', () => {
    expect(
      shouldFocusSelectedPage(inputs({ activeTool: { kind: 'comment' } })),
    ).toBeNull()
  })

  it('case 4 follow-up: comment composer open (commentOverlayActive) excludes even when activeTool is select', () => {
    expect(
      shouldFocusSelectedPage(inputs({ commentOverlayActive: true })),
    ).toBeNull()
  })
})

describe('shouldFocusSelectedPage — interaction modes other than idle exclude', () => {
  for (const interactionKind of [
    'panning',
    'marquee',
    'dragging-entities',
    'resizing-entity',
    'resizing-multi-selection',
    'dragging-edge',
    'editing-entity',
  ] as const) {
    it(`returns null when interactionKind=${interactionKind}`, () => {
      expect(shouldFocusSelectedPage(inputs({ interactionKind }))).toBeNull()
    })
  }
})

describe('shouldFocusSelectedPage — tools other than select exclude', () => {
  const tools: Tool[] = [
    { kind: 'inspect' },
    { kind: 'comment' },
    { kind: 'draw' },
    { kind: 'add-page' },
    { kind: 'add-text' },
    { kind: 'add-sticky' },
    { kind: 'add-shape' },
  ]
  for (const tool of tools) {
    it(`returns null when activeTool=${tool.kind}`, () => {
      expect(shouldFocusSelectedPage(inputs({ activeTool: tool }))).toBeNull()
    })
  }
})
