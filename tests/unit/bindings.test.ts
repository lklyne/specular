import { describe, expect, it } from 'vitest'
import {
  BINDINGS,
  ALL_VIEWS,
  CANVAS_REGION,
  acceleratorString,
  dispatchKey,
  normalizeElectronInput,
  type BindingContext,
} from '../../src/shared/bindings'
import type { ToolKind } from '../../src/shared/tool'

const BASE_CTX: BindingContext = {
  activeTool: { kind: 'select' },
  isTextEditing: false,
  pageFocusActive: false,
  sourceView: 'canvasBg',
  viewMode: 'canvas',
  hasOpenAnnotationThread: false,
  hasPendingAnnotation: false,
}

// Active (non-select) tool context for shortcut dispatch tests
const ACTIVE_CTX: BindingContext = { ...BASE_CTX, activeTool: { kind: 'comment' } }

function matchesToolKind(bindingId: string, kind: ToolKind): boolean {
  const stripped = bindingId.replace(/^tool-/, '')
  return stripped === kind || stripped.startsWith(kind + '-')
}

describe('BINDINGS table', () => {
  it('has entries for every bindable tool kind', () => {
    const bindableKinds: ToolKind[] = [
      'select',
      'add-page',
      'add-text',
      'add-sticky',
      'add-shape',
      'comment',
      'draw',
      'inspect',
    ]
    for (const kind of bindableKinds) {
      expect(
        BINDINGS.some((b) => matchesToolKind(b.id, kind)),
        `no binding for tool kind "${kind}"`,
      ).toBe(true)
    }
  })

  it('has no duplicate (key+modifiers+scope-view) among bindings without when predicates', () => {
    const noWhen = BINDINGS.filter((b) => b.when === undefined)
    const seen = new Set<string>()
    for (const binding of noWhen) {
      for (const view of binding.scope) {
        const signature = `${binding.defaultKey.key}|${binding.defaultKey.cmd}|${binding.defaultKey.shift}|${binding.defaultKey.alt}|${view}`
        expect(seen.has(signature), `duplicate binding for ${signature} (id: ${binding.id})`).toBe(
          false,
        )
        seen.add(signature)
      }
    }
  })

  it('all bindings have non-empty label and id', () => {
    for (const binding of BINDINGS) {
      expect(binding.id.length).toBeGreaterThan(0)
      expect(binding.label.length).toBeGreaterThan(0)
    }
  })

  it('all binding scopes are non-empty arrays of valid views', () => {
    const validViews = new Set<string>(ALL_VIEWS)
    for (const binding of BINDINGS) {
      expect(binding.scope.length).toBeGreaterThan(0)
      for (const view of binding.scope) {
        expect(validViews.has(view), `unknown view "${view}" in binding ${binding.id}`).toBe(true)
      }
    }
  })
})

describe('dispatchKey — tool bindings', () => {
  it('returns tool-select for "v" from canvasBg', () => {
    expect(
      dispatchKey(BINDINGS, { key: 'v', cmd: false, alt: false, shift: false }, BASE_CTX),
    ).toBe('tool-select')
  })

  it('returns tool-comment for "c" from canvasBg', () => {
    expect(
      dispatchKey(BINDINGS, { key: 'c', cmd: false, alt: false, shift: false }, BASE_CTX),
    ).toBe('tool-comment')
  })

  it('returns tool-add-shape-diamond for Shift+R from canvasBg', () => {
    expect(
      dispatchKey(BINDINGS, { key: 'r', cmd: false, alt: false, shift: true }, BASE_CTX),
    ).toBe('tool-add-shape-diamond')
  })

  it('returns tool-draw-highlight for Shift+M from canvasBg', () => {
    expect(
      dispatchKey(BINDINGS, { key: 'm', cmd: false, alt: false, shift: true }, BASE_CTX),
    ).toBe('tool-draw-highlight')
  })

  it('does not fire tool key from leftSidebar', () => {
    expect(
      dispatchKey(
        BINDINGS,
        { key: 'v', cmd: false, alt: false, shift: false },
        { ...BASE_CTX, sourceView: 'leftSidebar' },
      ),
    ).toBeNull()
  })

  it('does not fire tool key from page view', () => {
    expect(
      dispatchKey(
        BINDINGS,
        { key: 'v', cmd: false, alt: false, shift: false },
        { ...BASE_CTX, sourceView: 'page' },
      ),
    ).toBeNull()
  })

  it('fires tool keys from all CANVAS_REGION views', () => {
    for (const view of CANVAS_REGION) {
      expect(
        dispatchKey(
          BINDINGS,
          { key: 'v', cmd: false, alt: false, shift: false },
          { ...BASE_CTX, sourceView: view },
        ),
      ).toBe('tool-select')
    }
  })
})

describe('dispatchKey — modifier shortcuts', () => {
  it('returns undo for Cmd+Z from canvasBg', () => {
    expect(
      dispatchKey(BINDINGS, { key: 'z', cmd: true, alt: false, shift: false }, BASE_CTX),
    ).toBe('undo')
  })

  it('returns redo for Cmd+Shift+Z', () => {
    expect(
      dispatchKey(BINDINGS, { key: 'z', cmd: true, alt: false, shift: true }, BASE_CTX),
    ).toBe('redo')
  })

  it('returns reset-viewport for Cmd+1', () => {
    expect(
      dispatchKey(BINDINGS, { key: '1', cmd: true, alt: false, shift: false }, BASE_CTX),
    ).toBe('reset-viewport')
  })

  it('returns group for Cmd+G', () => {
    expect(
      dispatchKey(BINDINGS, { key: 'g', cmd: true, alt: false, shift: false }, BASE_CTX),
    ).toBe('group')
  })

  it('returns ungroup for Cmd+Shift+G', () => {
    expect(
      dispatchKey(BINDINGS, { key: 'g', cmd: true, alt: false, shift: true }, BASE_CTX),
    ).toBe('ungroup')
  })

  it('returns delete-selection for Delete key', () => {
    expect(
      dispatchKey(BINDINGS, { key: 'delete', cmd: false, alt: false, shift: false }, BASE_CTX),
    ).toBe('delete-selection')
  })

  it('returns delete-selection for Backspace key', () => {
    expect(
      dispatchKey(BINDINGS, { key: 'backspace', cmd: false, alt: false, shift: false }, BASE_CTX),
    ).toBe('delete-selection')
  })

  it('returns close-tab for Cmd+W from any view', () => {
    for (const view of ALL_VIEWS) {
      expect(
        dispatchKey(
          BINDINGS,
          { key: 'w', cmd: true, alt: false, shift: false },
          { ...BASE_CTX, sourceView: view },
        ),
      ).toBe('close-tab')
    }
  })

  it('returns stack-order shortcuts in canvas mode', () => {
    expect(
      dispatchKey(BINDINGS, { key: ']', cmd: true, alt: false, shift: false }, BASE_CTX),
    ).toBe('stack-bring-forward')
    expect(
      dispatchKey(BINDINGS, { key: '[', cmd: true, alt: false, shift: false }, BASE_CTX),
    ).toBe('stack-send-backward')
    expect(
      dispatchKey(BINDINGS, { key: ']', cmd: true, alt: false, shift: true }, BASE_CTX),
    ).toBe('stack-bring-to-front')
    expect(
      dispatchKey(BINDINGS, { key: '[', cmd: true, alt: false, shift: true }, BASE_CTX),
    ).toBe('stack-send-to-back')
  })

  it('suppresses stack-order shortcuts in browser mode', () => {
    expect(
      dispatchKey(
        BINDINGS,
        { key: ']', cmd: true, alt: false, shift: false },
        { ...BASE_CTX, viewMode: 'browser' },
      ),
    ).toBeNull()
  })

  it('maps left-sidebar arrows to stack-order actions in canvas mode', () => {
    const ctx: BindingContext = { ...BASE_CTX, sourceView: 'leftSidebar' }
    expect(
      dispatchKey(BINDINGS, { key: 'arrowup', cmd: false, alt: false, shift: false }, ctx),
    ).toBe('stack-bring-forward')
    expect(
      dispatchKey(BINDINGS, { key: 'arrowdown', cmd: false, alt: false, shift: false }, ctx),
    ).toBe('stack-send-backward')
  })
})

describe('dispatchKey — Escape resolution', () => {
  it('returns annotation-close-thread when open thread exists (aboveView)', () => {
    const ctx: BindingContext = {
      ...ACTIVE_CTX,
      sourceView: 'aboveView',
      hasOpenAnnotationThread: true,
    }
    expect(
      dispatchKey(BINDINGS, { key: 'escape', cmd: false, alt: false, shift: false }, ctx),
    ).toBe('annotation-close-thread')
  })

  it('returns annotation-clear-draft when pending annotation exists (aboveView)', () => {
    const ctx: BindingContext = {
      ...ACTIVE_CTX,
      sourceView: 'aboveView',
      hasPendingAnnotation: true,
    }
    expect(
      dispatchKey(BINDINGS, { key: 'escape', cmd: false, alt: false, shift: false }, ctx),
    ).toBe('annotation-clear-draft')
  })

  it('returns escape-tool when no annotation state from aboveView', () => {
    const ctx: BindingContext = {
      ...ACTIVE_CTX,
      sourceView: 'aboveView',
      hasOpenAnnotationThread: false,
      hasPendingAnnotation: false,
    }
    expect(
      dispatchKey(BINDINGS, { key: 'escape', cmd: false, alt: false, shift: false }, ctx),
    ).toBe('escape-tool')
  })

  it('returns escape-page-focus for Escape from page view when page focused', () => {
    const ctx: BindingContext = {
      ...ACTIVE_CTX,
      sourceView: 'page',
      pageFocusActive: true,
    }
    expect(
      dispatchKey(BINDINGS, { key: 'escape', cmd: false, alt: false, shift: false }, ctx),
    ).toBe('escape-page-focus')
  })

  it('returns escape-tool for Escape from canvasBg when tool is active', () => {
    const ctx: BindingContext = { ...ACTIVE_CTX, sourceView: 'canvasBg' }
    expect(
      dispatchKey(BINDINGS, { key: 'escape', cmd: false, alt: false, shift: false }, ctx),
    ).toBe('escape-tool')
  })

  it('returns null for Escape when already on select tool', () => {
    const ctx: BindingContext = { ...BASE_CTX, sourceView: 'canvasBg' }
    expect(
      dispatchKey(BINDINGS, { key: 'escape', cmd: false, alt: false, shift: false }, ctx),
    ).toBeNull()
  })
})

describe('dispatchKey — firesWhileTyping', () => {
  it('suppresses plain tool keys while typing', () => {
    const ctx: BindingContext = { ...BASE_CTX, isTextEditing: true }
    expect(
      dispatchKey(BINDINGS, { key: 'v', cmd: false, alt: false, shift: false }, ctx),
    ).toBeNull()
    expect(
      dispatchKey(BINDINGS, { key: 'c', cmd: false, alt: false, shift: false }, ctx),
    ).toBeNull()
    expect(
      dispatchKey(BINDINGS, { key: 'm', cmd: false, alt: false, shift: false }, ctx),
    ).toBeNull()
  })

  it('suppresses group/ungroup while typing', () => {
    const ctx: BindingContext = { ...BASE_CTX, isTextEditing: true }
    expect(
      dispatchKey(BINDINGS, { key: 'g', cmd: true, alt: false, shift: false }, ctx),
    ).toBeNull()
  })

  it('fires undo while typing', () => {
    const ctx: BindingContext = { ...BASE_CTX, isTextEditing: true }
    expect(
      dispatchKey(BINDINGS, { key: 'z', cmd: true, alt: false, shift: false }, ctx),
    ).toBe('undo')
  })

  it('fires redo while typing', () => {
    const ctx: BindingContext = { ...BASE_CTX, isTextEditing: true }
    expect(
      dispatchKey(BINDINGS, { key: 'z', cmd: true, alt: false, shift: true }, ctx),
    ).toBe('redo')
  })

  it('fires reset-viewport while typing', () => {
    const ctx: BindingContext = { ...BASE_CTX, isTextEditing: true }
    expect(
      dispatchKey(BINDINGS, { key: '1', cmd: true, alt: false, shift: false }, ctx),
    ).toBe('reset-viewport')
  })

  it('fires close-tab while typing', () => {
    const ctx: BindingContext = { ...BASE_CTX, isTextEditing: true }
    expect(
      dispatchKey(BINDINGS, { key: 'w', cmd: true, alt: false, shift: false }, ctx),
    ).toBe('close-tab')
  })

  it('fires escape-tool while typing', () => {
    const ctx: BindingContext = { ...ACTIVE_CTX, isTextEditing: true }
    expect(
      dispatchKey(BINDINGS, { key: 'escape', cmd: false, alt: false, shift: false }, ctx),
    ).toBe('escape-tool')
  })
})

describe('dispatchKey — firesFromPageFocus', () => {
  it('suppresses most bindings when page is focused', () => {
    const ctx: BindingContext = { ...BASE_CTX, pageFocusActive: true, sourceView: 'page' }
    expect(
      dispatchKey(BINDINGS, { key: 'z', cmd: true, alt: false, shift: false }, ctx),
    ).toBeNull()
    expect(
      dispatchKey(BINDINGS, { key: 'g', cmd: true, alt: false, shift: false }, ctx),
    ).toBeNull()
    expect(
      dispatchKey(BINDINGS, { key: 'w', cmd: true, alt: false, shift: false }, ctx),
    ).toBeNull()
  })

  it('fires delete-selection from page focus when the page is not typing', () => {
    const ctx: BindingContext = { ...BASE_CTX, pageFocusActive: true, sourceView: 'page' }
    expect(
      dispatchKey(BINDINGS, { key: 'delete', cmd: false, alt: false, shift: false }, ctx),
    ).toBe('delete-selection')
  })

  it('suppresses delete-selection from page focus while typing', () => {
    const ctx: BindingContext = {
      ...BASE_CTX,
      isTextEditing: true,
      pageFocusActive: true,
      sourceView: 'page',
    }
    expect(
      dispatchKey(BINDINGS, { key: 'delete', cmd: false, alt: false, shift: false }, ctx),
    ).toBeNull()
  })

  it('fires reset-viewport from page focus', () => {
    const ctx: BindingContext = { ...BASE_CTX, pageFocusActive: true, sourceView: 'page' }
    expect(
      dispatchKey(BINDINGS, { key: '1', cmd: true, alt: false, shift: false }, ctx),
    ).toBe('reset-viewport')
  })

  it('fires escape-page-focus from page focus when predicate holds', () => {
    const ctx: BindingContext = { ...BASE_CTX, pageFocusActive: true, sourceView: 'page' }
    expect(
      dispatchKey(BINDINGS, { key: 'escape', cmd: false, alt: false, shift: false }, ctx),
    ).toBe('escape-page-focus')
  })
})

describe('normalizeElectronInput', () => {
  it('normalizes a keyDown event', () => {
    expect(
      normalizeElectronInput({
        type: 'keyDown',
        key: 'v',
        code: 'KeyV',
        isAutoRepeat: false,
        shift: false,
        control: false,
        alt: false,
        meta: false,
      }),
    ).toEqual({ key: 'v', cmd: false, shift: false, alt: false })
  })

  it('returns null for keyUp', () => {
    expect(
      normalizeElectronInput({
        type: 'keyUp',
        key: 'v',
        code: 'KeyV',
        isAutoRepeat: false,
        shift: false,
        control: false,
        alt: false,
        meta: false,
      }),
    ).toBeNull()
  })

  it('lowercases the key', () => {
    expect(
      normalizeElectronInput({
        type: 'keyDown',
        key: 'Z',
        code: 'KeyZ',
        isAutoRepeat: false,
        shift: true,
        control: false,
        alt: false,
        meta: true,
      }),
    ).toEqual({ key: 'z', cmd: true, shift: true, alt: false })
  })

  it('treats meta as cmd (macOS)', () => {
    const result = normalizeElectronInput({
      type: 'keyDown',
      key: 'z',
      code: 'KeyZ',
      isAutoRepeat: false,
      shift: false,
      control: false,
      alt: false,
      meta: true,
    })
    expect(result?.cmd).toBe(true)
  })

  it('treats control as cmd (Windows/Linux)', () => {
    const result = normalizeElectronInput({
      type: 'keyDown',
      key: 'z',
      code: 'KeyZ',
      isAutoRepeat: false,
      shift: false,
      control: true,
      alt: false,
      meta: false,
    })
    expect(result?.cmd).toBe(true)
  })

  it('normalizes shifted bracket keys by physical key', () => {
    expect(
      normalizeElectronInput({
        type: 'keyDown',
        key: '}',
        code: 'BracketRight',
        isAutoRepeat: false,
        shift: true,
        control: false,
        alt: false,
        meta: true,
      }),
    ).toEqual({ key: ']', cmd: true, shift: true, alt: false })
  })

  it('round-trips through dispatchKey for every binding', () => {
    for (const binding of BINDINGS) {
      const input = normalizeElectronInput({
        type: 'keyDown',
        key: binding.defaultKey.key,
        code: '',
        isAutoRepeat: false,
        shift: binding.defaultKey.shift,
        control: binding.defaultKey.cmd,
        alt: binding.defaultKey.alt,
        meta: false,
      })
      expect(input).not.toBeNull()
      expect(input).toEqual(binding.defaultKey)
    }
  })
})

describe('acceleratorString', () => {
  it('formats a plain letter', () => {
    expect(acceleratorString({ key: 'v', cmd: false, shift: false, alt: false })).toBe('V')
  })

  it('formats Cmd+Z', () => {
    expect(acceleratorString({ key: 'z', cmd: true, shift: false, alt: false })).toBe('CmdOrCtrl+Z')
  })

  it('formats Cmd+Shift+Z', () => {
    expect(acceleratorString({ key: 'z', cmd: true, shift: true, alt: false })).toBe('CmdOrCtrl+Shift+Z')
  })

  it('formats Shift+R', () => {
    expect(acceleratorString({ key: 'r', cmd: false, shift: true, alt: false })).toBe('Shift+R')
  })

  it('formats Escape', () => {
    expect(acceleratorString({ key: 'escape', cmd: false, shift: false, alt: false })).toBe(
      'Escape',
    )
  })

  it('formats arrow keys using Electron names', () => {
    expect(acceleratorString({ key: 'arrowleft', cmd: false, shift: false, alt: false })).toBe(
      'Left',
    )
    expect(acceleratorString({ key: 'arrowright', cmd: false, shift: false, alt: false })).toBe(
      'Right',
    )
    expect(acceleratorString({ key: 'arrowup', cmd: false, shift: false, alt: false })).toBe('Up')
    expect(acceleratorString({ key: 'arrowdown', cmd: false, shift: false, alt: false })).toBe(
      'Down',
    )
  })

  it('formats Delete and Backspace', () => {
    expect(acceleratorString({ key: 'delete', cmd: false, shift: false, alt: false })).toBe(
      'Delete',
    )
    expect(acceleratorString({ key: 'backspace', cmd: false, shift: false, alt: false })).toBe(
      'Backspace',
    )
  })

  it('produces a non-empty string for every binding defaultKey', () => {
    for (const binding of BINDINGS) {
      const s = acceleratorString(binding.defaultKey)
      expect(s.length, `empty accelerator for ${binding.id}`).toBeGreaterThan(0)
    }
  })

  it('includes all modifier components', () => {
    expect(acceleratorString({ key: 'x', cmd: true, shift: true, alt: true })).toBe(
      'CmdOrCtrl+Alt+Shift+X',
    )
  })
})
