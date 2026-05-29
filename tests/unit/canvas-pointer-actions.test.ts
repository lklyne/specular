import { describe, expect, it } from 'vitest'
import { hitTest, type HitInputs } from '../../src/shared/hit-test'
import {
  routePointerDown,
  type CanvasPointerContext,
} from '../../src/shared/canvas-pointer-actions'
import type {
  CanvasSceneEntity,
  CanvasSceneFileEntity,
  CanvasScenePageEntity,
  CanvasSceneShapeEntity,
  CanvasSceneTextEntity,
} from '../../src/shared/types'

function page(over: Partial<CanvasScenePageEntity> = {}): CanvasScenePageEntity {
  return {
    id: 'f1',
    kind: 'page',
    canvasX: 0,
    canvasY: 0,
    width: 800,
    height: 600,
    screenX: 100,
    screenY: 100,
    screenWidth: 800,
    screenHeight: 600,
    presetIndex: 0,
    rendererTag: 'web',
    ...over,
  } as CanvasScenePageEntity
}

function text(over: Partial<CanvasSceneTextEntity> = {}): CanvasSceneTextEntity {
  return {
    id: 't1',
    kind: 'text',
    canvasX: 0,
    canvasY: 0,
    width: 200,
    height: 80,
    screenX: 1100,
    screenY: 100,
    screenWidth: 200,
    screenHeight: 80,
    text: 'hi',
    ...over,
  } as CanvasSceneTextEntity
}

function inputs(entities: CanvasSceneEntity[], selected: string[] = []): HitInputs {
  return { entities, edges: [], selectedEntityIds: selected, zoom: 1 }
}

const baseCtx: CanvasPointerContext = {
  selectedEntityIds: [],
  isPrimaryButton: true,
  button: 'left',
  modifiers: { shift: false, meta: false, ctrl: false },
  spaceHeld: false,
  altHeld: false,
  editingEntityId: null,
}

function shape(over: Partial<CanvasSceneShapeEntity> = {}): CanvasSceneShapeEntity {
  return {
    id: 's1',
    kind: 'shape',
    canvasX: 0,
    canvasY: 0,
    width: 200,
    height: 80,
    screenX: 1400,
    screenY: 100,
    screenWidth: 200,
    screenHeight: 80,
    shapeKind: 'rect',
    text: '',
    ...over,
  } as CanvasSceneShapeEntity
}

function file(over: Partial<CanvasSceneFileEntity> = {}): CanvasSceneFileEntity {
  return {
    id: 'fi1',
    kind: 'file',
    file: 'note.md',
    canvasX: 0,
    canvasY: 0,
    width: 200,
    height: 80,
    screenX: 1700,
    screenY: 100,
    screenWidth: 200,
    screenHeight: 80,
    rendererTag: 'markdown',
    rendererEditable: true,
    ...over,
  } as CanvasSceneFileEntity
}

describe('routePointerDown', () => {
  it('page body pointerdown → page-body-press', () => {
    const f = page()
    const target = hitTest(inputs([f]), { x: 500, y: 400 })
    const action = routePointerDown(target, baseCtx)
    expect(action).toEqual({ kind: 'page-body-press', entityId: 'f1', preserveSelection: false })
  })

  it('page body pointerdown on single-selected page → forward-pointer-down', () => {
    const f = page()
    const target = hitTest(inputs([f], ['f1']), { x: 500, y: 400 })
    const action = routePointerDown(target, { ...baseCtx, selectedEntityIds: ['f1'] })
    expect(action).toEqual({ kind: 'forward-pointer-down', entityId: 'f1', button: 'left' })
  })

  it('alt-click on single-selected page body → page-body-press (alt-copy semantics preserved)', () => {
    const f = page()
    const target = hitTest(inputs([f], ['f1']), { x: 500, y: 400 })
    const action = routePointerDown(target, {
      ...baseCtx,
      selectedEntityIds: ['f1'],
      altHeld: true,
    })
    expect(action).toEqual({ kind: 'page-body-press', entityId: 'f1', preserveSelection: true })
  })

  it('right-click on single-selected page body → forward-pointer-down (right)', () => {
    const f = page()
    const target = hitTest(inputs([f], ['f1']), { x: 500, y: 400 })
    const action = routePointerDown(target, {
      ...baseCtx,
      selectedEntityIds: ['f1'],
      isPrimaryButton: false,
      button: 'right',
    })
    expect(action).toEqual({ kind: 'forward-pointer-down', entityId: 'f1', button: 'right' })
  })

  it('page body pointerdown when page is in multi-selection → page-body-press (drag)', () => {
    const f = page()
    const t = text()
    const target = hitTest(inputs([f, t], ['f1', 't1']), { x: 500, y: 400 })
    const action = routePointerDown(target, {
      ...baseCtx,
      selectedEntityIds: ['f1', 't1'],
    })
    expect(action).toMatchObject({ kind: 'page-body-press', entityId: 'f1' })
  })

  it('shift-click on single-selected page body → toggle-select (extends selection, does not forward)', () => {
    const f = page()
    const target = hitTest(inputs([f], ['f1']), { x: 500, y: 400 })
    const action = routePointerDown(target, {
      ...baseCtx,
      selectedEntityIds: ['f1'],
      modifiers: { shift: true, meta: false, ctrl: false },
    })
    expect(action).toEqual({ kind: 'toggle-select', entityId: 'f1', entityKind: 'page' })
  })

  it('cmd-click on unselected page body → toggle-select (extends selection)', () => {
    const f = page()
    const t = text()
    const target = hitTest(inputs([f, t], ['t1']), { x: 500, y: 400 })
    const action = routePointerDown(target, {
      ...baseCtx,
      selectedEntityIds: ['t1'],
      modifiers: { shift: false, meta: true, ctrl: false },
    })
    expect(action).toEqual({ kind: 'toggle-select', entityId: 'f1', entityKind: 'page' })
  })

  it('shift-click on multi-selected page body → toggle-select (drops it from selection)', () => {
    const f = page()
    const t = text()
    const target = hitTest(inputs([f, t], ['f1', 't1']), { x: 500, y: 400 })
    const action = routePointerDown(target, {
      ...baseCtx,
      selectedEntityIds: ['f1', 't1'],
      modifiers: { shift: true, meta: false, ctrl: false },
    })
    expect(action).toEqual({ kind: 'toggle-select', entityId: 'f1', entityKind: 'page' })
  })

  it('chrome click on page → begin-entity-drag', () => {
    const f = page()
    // Chrome is the 36px strip above screenY.
    const target = hitTest(inputs([f]), { x: 500, y: f.screenY - 10 })
    const action = routePointerDown(target, baseCtx)
    expect(action).toMatchObject({ kind: 'begin-entity-drag', entityId: 'f1', entityKind: 'page' })
  })

  it('shift-click chrome → toggle-select (no drag)', () => {
    const f = page()
    const target = hitTest(inputs([f]), { x: 500, y: f.screenY - 10 })
    const action = routePointerDown(target, {
      ...baseCtx,
      modifiers: { shift: true, meta: false, ctrl: false },
    })
    expect(action).toEqual({ kind: 'toggle-select', entityId: 'f1', entityKind: 'page' })
  })

  it('anchor click → begin-edge-drag', () => {
    const f = page()
    const target = hitTest(
      inputs([f], ['f1']),
      // Right-side anchor sits past the resize edge strip (pages extend the
      // resize hit band to entity.right + 12 for the outline padding).
      { x: f.screenX + f.screenWidth + 20, y: f.screenY + f.screenHeight / 2 },
    )
    const action = routePointerDown(target, { ...baseCtx, selectedEntityIds: ['f1'] })
    expect(action).toMatchObject({ kind: 'begin-edge-drag', entityId: 'f1', side: 'right' })
  })

  it('resize handle (selected entity) → begin-resize', () => {
    const f = page()
    const target = hitTest(
      inputs([f], ['f1']),
      { x: f.screenX, y: f.screenY }, // nw handle
    )
    const action = routePointerDown(target, { ...baseCtx, selectedEntityIds: ['f1'] })
    expect(action).toMatchObject({ kind: 'begin-resize', entityId: 'f1', handle: 'nw' })
  })

  it('background click (no modifiers) → background-click', () => {
    const target = hitTest(inputs([]), { x: 50, y: 50 })
    const action = routePointerDown(target, baseCtx)
    expect(action).toEqual({ kind: 'background-click' })
  })

  it('shift on background → background-click (additive deselect/no-op)', () => {
    const target = hitTest(inputs([]), { x: 50, y: 50 })
    const action = routePointerDown(target, {
      ...baseCtx,
      modifiers: { shift: true, meta: false, ctrl: false },
    })
    expect(action).toEqual({ kind: 'background-click' })
  })

  it('space-held + background → begin-pan', () => {
    const target = hitTest(inputs([]), { x: 50, y: 50 })
    const action = routePointerDown(target, { ...baseCtx, spaceHeld: true })
    expect(action).toEqual({ kind: 'begin-pan' })
  })

  it('text body click → begin-entity-drag', () => {
    const t = text()
    const target = hitTest(inputs([t]), { x: t.screenX + 50, y: t.screenY + 30 })
    const action = routePointerDown(target, baseCtx)
    expect(action).toMatchObject({ kind: 'begin-entity-drag', entityId: 't1', entityKind: 'text' })
  })

  it('non-primary button on background → noop', () => {
    const target = hitTest(inputs([]), { x: 50, y: 50 })
    const action = routePointerDown(target, { ...baseCtx, isPrimaryButton: false })
    expect(action).toEqual({ kind: 'noop' })
  })

  it('multi-bbox SE handle → begin-multi-resize (no entityId on the action)', () => {
    const t1 = text({ id: 't1', screenX: 100, screenY: 100, screenWidth: 50, screenHeight: 50 })
    const t2 = text({ id: 't2', screenX: 200, screenY: 200, screenWidth: 80, screenHeight: 40 })
    // Multi-bbox SE corner sits at (280+8, 240+8) = (288, 248).
    const target = hitTest(inputs([t1, t2], ['t1', 't2']), { x: 288, y: 248 })
    const action = routePointerDown(target, {
      ...baseCtx,
      selectedEntityIds: ['t1', 't2'],
    })
    expect(action).toEqual({ kind: 'begin-multi-resize', handle: 'se' })
  })

  // --- Issue #49: click-on-solo-selected → begin-entity-press (deferred) ---
  describe('begin-entity-press (issue #49)', () => {
    it('click on solo-selected text body → begin-entity-press', () => {
      const t = text()
      const target = hitTest(inputs([t], ['t1']), { x: t.screenX + 50, y: t.screenY + 30 })
      const action = routePointerDown(target, { ...baseCtx, selectedEntityIds: ['t1'] })
      expect(action).toEqual({ kind: 'begin-entity-press', entityId: 't1', entityKind: 'text' })
    })

    it('click on solo-selected shape body → begin-entity-press', () => {
      const s = shape()
      const target = hitTest(inputs([s], ['s1']), { x: s.screenX + 50, y: s.screenY + 30 })
      const action = routePointerDown(target, { ...baseCtx, selectedEntityIds: ['s1'] })
      expect(action).toEqual({ kind: 'begin-entity-press', entityId: 's1', entityKind: 'shape' })
    })

    it('click on unselected text body → begin-entity-drag (no press deferral)', () => {
      const t = text()
      const target = hitTest(inputs([t]), { x: t.screenX + 50, y: t.screenY + 30 })
      const action = routePointerDown(target, baseCtx)
      expect(action).toMatchObject({ kind: 'begin-entity-drag', entityId: 't1' })
    })

    it('click on text in multi-selection → begin-entity-drag (no press deferral)', () => {
      const t1 = text({ id: 't1' })
      const t2 = text({ id: 't2', screenX: 1500 })
      const target = hitTest(
        inputs([t1, t2], ['t1', 't2']),
        { x: t1.screenX + 50, y: t1.screenY + 30 },
      )
      const action = routePointerDown(target, {
        ...baseCtx,
        selectedEntityIds: ['t1', 't2'],
      })
      expect(action).toMatchObject({ kind: 'begin-entity-drag', entityId: 't1' })
    })

    it('shift-click on solo-selected text → toggle-select (no press deferral)', () => {
      const t = text()
      const target = hitTest(inputs([t], ['t1']), { x: t.screenX + 50, y: t.screenY + 30 })
      const action = routePointerDown(target, {
        ...baseCtx,
        selectedEntityIds: ['t1'],
        modifiers: { shift: true, meta: false, ctrl: false },
      })
      expect(action).toEqual({ kind: 'toggle-select', entityId: 't1', entityKind: 'text' })
    })

    it('cmd-click on solo-selected shape → toggle-select (no press deferral)', () => {
      const s = shape()
      const target = hitTest(inputs([s], ['s1']), { x: s.screenX + 50, y: s.screenY + 30 })
      const action = routePointerDown(target, {
        ...baseCtx,
        selectedEntityIds: ['s1'],
        modifiers: { shift: false, meta: true, ctrl: false },
      })
      expect(action).toEqual({ kind: 'toggle-select', entityId: 's1', entityKind: 'shape' })
    })

    it('alt-click on solo-selected text → begin-entity-drag (alt-clone semantics preserved)', () => {
      const t = text()
      const target = hitTest(inputs([t], ['t1']), { x: t.screenX + 50, y: t.screenY + 30 })
      const action = routePointerDown(target, {
        ...baseCtx,
        selectedEntityIds: ['t1'],
        altHeld: true,
      })
      expect(action).toMatchObject({ kind: 'begin-entity-drag', entityId: 't1' })
    })

    it('space-click on solo-selected text → begin-entity-drag (hold-to-pan modifier preserves drag)', () => {
      const t = text()
      const target = hitTest(inputs([t], ['t1']), { x: t.screenX + 50, y: t.screenY + 30 })
      const action = routePointerDown(target, {
        ...baseCtx,
        selectedEntityIds: ['t1'],
        spaceHeld: true,
      })
      expect(action).toMatchObject({ kind: 'begin-entity-drag', entityId: 't1' })
    })

    it('click on solo-selected text while another entity is editing → begin-entity-drag (deferral suppressed)', () => {
      const t = text()
      const target = hitTest(inputs([t], ['t1']), { x: t.screenX + 50, y: t.screenY + 30 })
      const action = routePointerDown(target, {
        ...baseCtx,
        selectedEntityIds: ['t1'],
        editingEntityId: 'other-entity',
      })
      expect(action).toMatchObject({ kind: 'begin-entity-drag', entityId: 't1' })
    })

    it('non-primary (right-click) on solo-selected text → noop (deferral is left-button only)', () => {
      const t = text()
      const target = hitTest(inputs([t], ['t1']), { x: t.screenX + 50, y: t.screenY + 30 })
      const action = routePointerDown(target, {
        ...baseCtx,
        selectedEntityIds: ['t1'],
        isPrimaryButton: false,
        button: 'right',
      })
      expect(action).toEqual({ kind: 'noop' })
    })

    // Issue #49 follow-up: editable file renderers (markdown, wireframe,
    // video) opt into the same press-deferral. Image / component
    // placeholders gracefully fall through to drag.
    it('click on solo-selected editable file body → begin-entity-press', () => {
      const f = file({ rendererEditable: true })
      const target = hitTest(inputs([f], ['fi1']), { x: f.screenX + 50, y: f.screenY + 30 })
      const action = routePointerDown(target, { ...baseCtx, selectedEntityIds: ['fi1'] })
      expect(action).toEqual({ kind: 'begin-entity-press', entityId: 'fi1', entityKind: 'file' })
    })

    it('click on solo-selected non-editable file body (image) → begin-entity-drag', () => {
      const f = file({
        id: 'img',
        file: 'photo.png',
        rendererTag: 'image',
        rendererEditable: false,
      })
      const target = hitTest(inputs([f], ['img']), { x: f.screenX + 50, y: f.screenY + 30 })
      const action = routePointerDown(target, { ...baseCtx, selectedEntityIds: ['img'] })
      expect(action).toMatchObject({ kind: 'begin-entity-drag', entityId: 'img' })
    })

    it('click on solo-selected file with missing rendererEditable (unclaimed) → begin-entity-drag', () => {
      const f = file({ id: 'unk', file: 'foo.bin', rendererEditable: undefined })
      const target = hitTest(inputs([f], ['unk']), { x: f.screenX + 50, y: f.screenY + 30 })
      const action = routePointerDown(target, { ...baseCtx, selectedEntityIds: ['unk'] })
      expect(action).toMatchObject({ kind: 'begin-entity-drag', entityId: 'unk' })
    })
  })

  // --- Issue #41 regression: chrome wins over anchor in their overlap zone ---
  it('issue #41: click at the chrome/top-anchor overlap zone goes to chrome, not anchor', () => {
    // Chrome strip is the 36px band above screenY: x=[100,900], y=[64,100].
    // Top anchor (when selected) is centred above the page midpoint, with a
    // 4px gap. At zoom=1 it's 56×24, centred at (500, 84): x=[472,528],
    // y=[72,96]. The anchor's hit ring dips into the chrome y-range — that's
    // the #41 bug class. Per ADR 0001's priority table, chrome wins.
    const f = page()
    const target = hitTest(inputs([f], ['f1']), { x: 500, y: 84 })
    expect(target.payload.kind).toBe('chrome')
    const action = routePointerDown(target, { ...baseCtx, selectedEntityIds: ['f1'] })
    expect(action.kind).toBe('begin-entity-drag')
  })
})

describe('routePointerDown — auto-layout reorder dot (ADR 0015)', () => {
  it('routes a reorder-handle hit to begin-reorder-drag', () => {
    const action = routePointerDown(
      {
        layer: 'reorder-handle',
        region: { kind: 'rect', rect: { x: 0, y: 0, width: 28, height: 28 } },
        payload: { kind: 'reorder-handle', entityId: 'c1', entityKind: 'text', groupId: 'g1' },
      },
      baseCtx,
    )
    expect(action).toEqual({
      kind: 'begin-reorder-drag',
      childId: 'c1',
      groupId: 'g1',
      entityKind: 'text',
    })
  })
})
