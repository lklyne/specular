import type { BindingContext, BindingId } from '../../shared/bindings'
import { DRAWING_FEATURE_ENABLED } from '../../shared/featureFlags'
import { setActiveTool } from './tool-mode'
import { applyToolDefaultPatch } from './tool-defaults'
import { undo, redo } from './workspace-undo'
import { setZoom, setPan, focusSelection, restoreFocusCamera } from './viewport-control'
import { groupSelectedEntities, makeAutoLayoutFromSelection, ungroupSelectedGroup } from './document-commands'
import { selectAdjacentPage } from './selection-state'
import { selectEntities, selectNone } from './selection-controller'
import { markDirty } from './layout-dirty'
import { requestLayout } from './surface-layout'
import { arrowNavigationLocked, setArrowNavigationLocked, pages, selectedPageId } from './runtime-context'
import { deletePages } from '../workspace-entities'
import { textEntities } from './text-entity-state'
import { fileEntities } from './file-entity-state'
import { drawingEntities } from './drawing-entity-state'
import { shapeEntities } from './shape-entity-state'
import { deleteSelection } from './delete-selection'
import { duplicateSelection } from './duplicate-selection'
import { reorderStackOrder } from './entity-order-state'
import { createBlankFrameFromSource } from '../workspace-pages'

type MainBindingId = Exclude<BindingId, 'annotation-close-thread' | 'annotation-clear-draft'>

export const mainHandlers: Record<MainBindingId, (ctx: BindingContext) => void> = {
  'tool-select': () => {
    setActiveTool({ kind: 'select' })
  },
  'tool-add-page': () => {
    setActiveTool({ kind: 'add-page' })
  },
  'tool-add-text': () => {
    setActiveTool({ kind: 'add-text' })
  },
  'tool-add-sticky': () => {
    setActiveTool({ kind: 'add-sticky' })
  },
  'tool-add-shape-rectangle': () => {
    setActiveTool({ kind: 'add-shape' })
    applyToolDefaultPatch({ scope: 'add-shape', key: 'shapeKind', value: 'rectangle' })
  },
  'tool-add-shape-ellipse': () => {
    setActiveTool({ kind: 'add-shape' })
    applyToolDefaultPatch({ scope: 'add-shape', key: 'shapeKind', value: 'ellipse' })
  },
  'tool-add-shape-diamond': () => {
    setActiveTool({ kind: 'add-shape' })
    applyToolDefaultPatch({ scope: 'add-shape', key: 'shapeKind', value: 'diamond' })
  },
  'tool-comment': () => {
    setActiveTool({ kind: 'comment' })
  },
  'tool-draw-pen': () => {
    if (!DRAWING_FEATURE_ENABLED) return
    setActiveTool({ kind: 'draw' })
    applyToolDefaultPatch({ scope: 'draw', key: 'brushType', value: 'pen' })
  },
  'tool-draw-highlight': () => {
    if (!DRAWING_FEATURE_ENABLED) return
    setActiveTool({ kind: 'draw' })
    applyToolDefaultPatch({ scope: 'draw', key: 'brushType', value: 'highlight' })
  },
  'tool-inspect': () => {
    setActiveTool({ kind: 'inspect' })
  },
  'undo': () => {
    undo()
  },
  'redo': () => {
    redo()
  },
  'reset-viewport': () => {
    if (restoreFocusCamera()) return
    setZoom(1.0)
    if (!focusSelection({ storeReturnCamera: false, animate: false })) {
      setPan(0, 0)
      requestLayout()
    }
  },
  'group': () => {
    groupSelectedEntities()
  },
  'ungroup': () => {
    ungroupSelectedGroup()
  },
  'make-auto-layout': (ctx) => {
    makeAutoLayoutFromSelection()
  },
  'select-all': () => {
    selectAllEntities()
  },
  'duplicate': () => {
    duplicateSelection()
  },
  'new-frame': () => {
    const pageId = selectedPageId()
    if (!pageId) return
    createBlankFrameFromSource({ sourcePageId: pageId, focusAddressBar: true })
  },
  'delete-selection': () => {
    deleteSelection()
  },
  'stack-bring-forward': (ctx) => {
    reorderStackOrder('bring-forward')
  },
  'stack-send-backward': (ctx) => {
    reorderStackOrder('send-backward')
  },
  'stack-bring-to-front': (ctx) => {
    reorderStackOrder('bring-to-front')
  },
  'stack-send-to-back': (ctx) => {
    reorderStackOrder('send-to-back')
  },
  'nav-left': () => {
    selectAdjacentPageOnce('left')
  },
  'nav-right': () => {
    selectAdjacentPageOnce('right')
  },
  'nav-up': () => {
    selectAdjacentPageOnce('up')
  },
  'nav-down': () => {
    selectAdjacentPageOnce('down')
  },
  'escape-tool': (ctx) => {
    // While text editing, the renderer commits the edit natively via DOM keydown.
    // Returning here prevents the tool from also being reset on the same keypress.
    if (ctx.isTextEditing) return
    setActiveTool({ kind: 'select' })
  },
  'restore-focus-camera': () => {
    restoreFocusCamera()
  },
  'escape-page-focus': () => {
    selectNone()
    markDirty('canvas')
    requestLayout()
  },
  'close-tab': (ctx) => {
    const pageId = selectedPageId()
    if (!pageId) return
    deletePages({ pageIds: [pageId] })
  },
}

type ArrowDirection = 'left' | 'right' | 'up' | 'down'

function selectAdjacentPageOnce(direction: ArrowDirection): void {
  if (arrowNavigationLocked) return
  const changed = selectAdjacentPage(direction)
  if (!changed) return
  setArrowNavigationLocked(true)
  setTimeout(() => {
    setArrowNavigationLocked(false)
  }, 0)
}

export function selectAllEntities(): void {
  const entityIds = [
    ...pages.map((p) => p.id),
    ...textEntities.map((e) => e.id),
    ...fileEntities.map((e) => e.id),
    ...shapeEntities.map((e) => e.id),
    ...drawingEntities.map((e) => e.id),
  ]
  if (!entityIds.length) return
  selectEntities(entityIds)
}
