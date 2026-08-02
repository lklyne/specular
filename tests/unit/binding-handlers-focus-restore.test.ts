import { beforeEach, describe, expect, it, vi } from 'vitest'

const viewport = vi.hoisted(() => ({
  focusSelection: vi.fn(),
  restoreFocusCamera: vi.fn(),
  setPan: vi.fn(),
  setZoom: vi.fn(),
}))

const layout = vi.hoisted(() => ({
  requestLayout: vi.fn(),
}))

const editing = vi.hoisted(() => ({
  beginEditingEntity: vi.fn(),
}))

const ui = vi.hoisted(() => ({
  getUiState: vi.fn(),
}))

vi.mock('../../src/shared/featureFlags', () => ({ DRAWING_FEATURE_ENABLED: true }))
vi.mock('../../src/main/runtime/tool-mode', () => ({ setActiveTool: vi.fn() }))
vi.mock('../../src/main/runtime/tool-defaults', () => ({ applyToolDefaultPatch: vi.fn() }))
vi.mock('../../src/main/runtime/space-undo', () => ({ undo: vi.fn(), redo: vi.fn() }))
vi.mock('../../src/main/runtime/document-commands', () => ({
  groupSelectedEntities: vi.fn(),
  makeAutoLayoutFromSelection: vi.fn(),
  nudgeSelection: vi.fn(),
  ungroupSelectedGroup: vi.fn(),
}))
vi.mock('../../src/main/runtime/selection-controller', () => ({
  selectEntities: vi.fn(),
  selectNone: vi.fn(),
}))
vi.mock('../../src/main/runtime/layout-dirty', () => ({ markDirty: vi.fn() }))
vi.mock('../../src/main/runtime/runtime-context', () => ({
  pages: [],
  selectedPageId: vi.fn(() => null),
}))
vi.mock('../../src/main/workspace-entities', () => ({ deletePages: vi.fn() }))
vi.mock('../../src/main/runtime/text-entity-state', () => ({ textEntities: [] }))
vi.mock('../../src/main/runtime/file-entity-state', () => ({ fileEntities: [] }))
vi.mock('../../src/main/runtime/drawing-entity-state', () => ({ drawingEntities: [] }))
vi.mock('../../src/main/runtime/shape-entity-state', () => ({ shapeEntities: [] }))
vi.mock('../../src/main/runtime/delete-selection', () => ({ deleteSelection: vi.fn() }))
vi.mock('../../src/main/runtime/duplicate-selection', () => ({ duplicateSelection: vi.fn() }))
vi.mock('../../src/main/runtime/entity-order-state', () => ({ reorderStackOrder: vi.fn() }))
vi.mock('../../src/main/runtime/viewport-control', () => ({ ...viewport, ...layout }))
vi.mock('../../src/main/runtime/editing-entity-runtime', () => editing)
vi.mock('../../src/main/ui-state', () => ui)

const { mainHandlers } = await import('../../src/main/runtime/binding-handlers')

describe('binding handlers focus restore', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    ui.getUiState.mockReturnValue({ selection: { kind: 'none' } })
  })

  it('reset viewport restores the focus camera instead of applying a manual camera move', () => {
    viewport.restoreFocusCamera.mockReturnValue(true)

    mainHandlers['reset-viewport']({
      focusReturnCameraActive: true,
      isTextEditing: false,
      view: 'aboveView',
    })

    expect(viewport.restoreFocusCamera).toHaveBeenCalledOnce()
    expect(viewport.setZoom).not.toHaveBeenCalled()
    expect(viewport.setPan).not.toHaveBeenCalled()
    expect(layout.requestLayout).not.toHaveBeenCalled()
  })

  it('reset viewport keeps the normal reset behavior when no focus return camera exists', () => {
    viewport.restoreFocusCamera.mockReturnValue(false)
    viewport.focusSelection.mockReturnValue(false)

    mainHandlers['reset-viewport']({
      focusReturnCameraActive: false,
      isTextEditing: false,
      view: 'aboveView',
    })

    expect(viewport.setZoom).toHaveBeenCalledWith(1)
    expect(viewport.focusSelection).toHaveBeenCalledWith({
      storeReturnCamera: false,
      animate: false,
    })
    expect(viewport.setPan).toHaveBeenCalledWith(0, 0)
    expect(layout.requestLayout).toHaveBeenCalledOnce()
  })

  it.each([
    ['text', 'text-1'],
    ['group', 'group-1'],
  ] as const)('Enter begins editing a selected %s', (entityKind, entityId) => {
    ui.getUiState.mockReturnValue({
      selection: { kind: 'single-entity', entityId, entityKind },
    })

    mainHandlers['edit-selection']({} as never)

    expect(editing.beginEditingEntity).toHaveBeenCalledWith(entityId)
  })

  it('Enter ignores non-editable and multi-selections', () => {
    ui.getUiState.mockReturnValue({
      selection: { kind: 'single-entity', entityId: 'page-1', entityKind: 'page' },
    })
    mainHandlers['edit-selection']({} as never)
    ui.getUiState.mockReturnValue({
      selection: { kind: 'multi-entity', entityIds: ['text-1', 'text-2'] },
    })
    mainHandlers['edit-selection']({} as never)

    expect(editing.beginEditingEntity).not.toHaveBeenCalled()
  })

})
