import type { BindingContext, BindingId } from '../../shared/bindings'
import { GRID_SIZE, NUDGE_STEP } from '../../shared/constants'
import { setActiveTool } from './tool-mode'
import { applyToolDefaultPatch } from './tool-defaults'
import { undo, redo } from './space-undo'
import { setZoom, setPan, focusSelection, restoreFocusCamera } from './viewport-control'
import { groupSelectedEntities, makeAutoLayoutFromSelection, nudgeSelection, ungroupSelectedGroup } from './document-commands'
import { selectEntities, selectNone } from './selection-controller'
import { requestLayout } from './viewport-control'
import { interactivePageId, pages, selectedPageId } from './runtime-context'
import { exitPageInteractive } from './overlay-manager'
import { deletePages } from '../workspace-entities'
import { textEntities } from './text-entity-state'
import { fileEntities } from './file-entity-state'
import { drawingEntities } from './drawing-entity-state'
import { shapeEntities } from './shape-entity-state'
import { deleteSelection } from './delete-selection'
import { duplicateSelection } from './duplicate-selection'
import { reorderStackOrder } from './entity-order-state'
import { createBlankFrameFromSource } from '../workspace-pages'
import { beginEditingEntity } from './editing-entity-runtime'
import { getUiState } from '../ui-state'

type MainBindingId = Exclude<BindingId, 'annotation-close-thread' | 'annotation-clear-draft'>

export const mainHandlers: Record<MainBindingId, (ctx: BindingContext) => void> = {
  'tool-select': () => {
    setActiveTool({ kind: 'select' })
  },
  'tool-hand': () => {
    setActiveTool({ kind: 'hand' })
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
    setActiveTool({ kind: 'draw' })
    applyToolDefaultPatch({ scope: 'draw', key: 'brushType', value: 'pen' })
  },
  'tool-draw-highlight': () => {
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
    createBlankFrameFromSource({ sourcePageId: pageId })
  },
  'edit-selection': () => {
    const selection = getUiState().selection
    if (
      selection.kind === 'single-entity' &&
      (selection.entityKind === 'text' || selection.entityKind === 'group')
    ) {
      beginEditingEntity(selection.entityId)
    }
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
  'nudge-left': () => {
    nudgeSelection(-NUDGE_STEP, 0)
  },
  'nudge-right': () => {
    nudgeSelection(NUDGE_STEP, 0)
  },
  'nudge-up': () => {
    nudgeSelection(0, -NUDGE_STEP)
  },
  'nudge-down': () => {
    nudgeSelection(0, NUDGE_STEP)
  },
  'nudge-left-grid': () => {
    nudgeSelection(-GRID_SIZE, 0)
  },
  'nudge-right-grid': () => {
    nudgeSelection(GRID_SIZE, 0)
  },
  'nudge-up-grid': () => {
    nudgeSelection(0, -GRID_SIZE)
  },
  'nudge-down-grid': () => {
    nudgeSelection(0, GRID_SIZE)
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
    // Select-first / interact-second (#124): first Escape exits interactive
    // mode back to selected (keeps the outline); a page only owns keyboard
    // while entered, so this is the path that fires from page focus.
    if (interactivePageId()) {
      exitPageInteractive()
      return
    }
    selectNone()
  },
  'close-tab': (ctx) => {
    const pageId = selectedPageId()
    if (!pageId) return
    deletePages({ pageIds: [pageId] })
  },
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
