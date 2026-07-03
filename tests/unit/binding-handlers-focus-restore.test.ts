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

vi.mock('../../src/shared/featureFlags', () => ({ DRAWING_FEATURE_ENABLED: true }))
vi.mock('../../src/main/runtime/tool-mode', () => ({ setActiveTool: vi.fn() }))
vi.mock('../../src/main/runtime/tool-defaults', () => ({ applyToolDefaultPatch: vi.fn() }))
vi.mock('../../src/main/runtime/workspace-undo', () => ({ undo: vi.fn(), redo: vi.fn() }))
vi.mock('../../src/main/runtime/document-commands', () => ({
  groupSelectedEntities: vi.fn(),
  makeAutoLayoutFromSelection: vi.fn(),
  ungroupSelectedGroup: vi.fn(),
}))
vi.mock('../../src/main/runtime/selection-state', () => ({ selectAdjacentPage: vi.fn() }))
vi.mock('../../src/main/runtime/selection-controller', () => ({
  selectEntities: vi.fn(),
  selectNone: vi.fn(),
}))
vi.mock('../../src/main/runtime/layout-dirty', () => ({ markDirty: vi.fn() }))
vi.mock('../../src/main/runtime/runtime-context', () => ({
  arrowNavigationLocked: false,
  pages: [],
  selectedPageId: vi.fn(() => null),
  setArrowNavigationLocked: vi.fn(),
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

const { mainHandlers } = await import('../../src/main/runtime/binding-handlers')

describe('binding handlers focus restore', () => {
  beforeEach(() => {
    vi.clearAllMocks()
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

})
