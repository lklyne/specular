import { ipcMain } from 'electron'
import { DRAWING_FEATURE_ENABLED } from '../../shared/featureFlags'
import type { CanvasEntityKind, SelectionModifiers, SidebarSectionKey } from '../../shared/types'
import type { EdgeSide } from '../../shared/types'
import { selectionMutationMode } from '../../shared/selection-modifiers'
import { pages } from '../runtime/page-runtime'
import { setCommentOverlayActive } from '../runtime/runtime-core'
import { setHoverEntity, setHoveredPage } from '../runtime/runtime-core'
import { activeTool as uiActiveTool } from '../ui-state'
import {
  canvasOrigin,
  pan,
  requestLayout,
  zoom,
} from '../runtime/surface-layout'
import { saveImageBuffer } from '../runtime/image-assets'
import { htmlDefaultSize, imageSizeFromBuffer } from '../runtime/image-sizing'
import {
  focusSelectedPage,
  getSelectedEntityIds,
  selectBrowserTab,
  selectEntity,
  selectPage,
  selectPageById,
  selectedPageId,
  setBrowserMode,
  setCanvasMode,
  setSelectedEntities,
} from '../runtime/ui-actions'
import {
  interactionBlocksPageHover,
  interactionBlocksPageSelection,
} from '../runtime/interaction-state'
import { commitActive } from '../runtime/interaction-controller'
import {
  beginEditingEntity,
  cancelEditingEntity,
  commitEditingEntity,
} from '../runtime/editing-entity-runtime'
import { setTextEditingActive, setAnnotationState } from '../runtime/binding-dispatcher'
import {
  forwardPointerToPage,
  forwardWheelToPage,
  type ForwardPointerPayload,
  type ForwardWheelPayload,
} from '../runtime/page-input-forwarding'
import {
  createWorkspaceTab,
  deleteWorkspaceTab,
  duplicateWorkspaceTab,
  renameWorkspaceDrawingEntity,
  renameWorkspaceFileEntity,
  renameWorkspacePage,
  renameWorkspaceGroup,
  renameWorkspaceTab,
  renameWorkspaceTextEntity,
  reorderWorkspaceTab,
  scheduleWorkspaceAutosave,
  setActiveWorkspaceTab,
  setWorkspaceTabExpanded,
} from '../runtime/workspace-session'
import {
  setPageBrowserSizeMode,
  type BrowserSizeMode,
} from '../runtime/runtime-entities'
import { createEdges, deleteEdges } from '../workspace-edges'
import { selectEntitiesInRect } from '../workspace-entities'
import { createFileEntity } from '../runtime/document-commands'
import {
  applyEntitySelectionMutation,
  enterGroup,
  resolveEntityKind,
  selectGroup,
  selectNone,
} from '../runtime/selection-controller'
import { consumeDragId } from '../runtime/drop-owner'
import { registerCanvasDragIpc } from './register-canvas-drag-ipc'
import { registerCanvasEntityIpc } from './register-canvas-entity-ipc'
import { reorderSidebarStackOrder } from '../runtime/entity-order-state'

export function registerCanvasIpc(): void {
  registerCanvasDragIpc()
  registerCanvasEntityIpc()

  // --- Selection ---

  ipcMain.on(
    'canvas-select-in-rect',
    (
      _event,
      payload: {
        x: number
        y: number
        width: number
        height: number
        modifiers?: SelectionModifiers
      },
    ) => {
      const { modifiers, ...bounds } = payload
      selectEntitiesInRect(bounds, { mode: selectionMutationMode(modifiers) })
    },
  )

  ipcMain.on('canvas-clear-annotate-hover', () => {
    for (const page of pages) {
      if (page.pageView.webContents.isDestroyed()) continue
      page.pageView.webContents.send('annotate-clear-hover')
    }
  })

  ipcMain.on(
    'canvas-select-page',
    (
      _event,
      { pageId, modifiers }: { pageId: string; modifiers?: SelectionModifiers },
    ) => {
      if (interactionBlocksPageSelection()) return
      if (!pages.some((candidate) => candidate.id === pageId)) return
      const mode = selectionMutationMode(modifiers)
      if (mode === 'replace') {
        const idx = pages.findIndex((candidate) => candidate.id === pageId)
        if (idx !== -1) selectPage(idx)
        return
      }
      applyEntitySelectionMutation([pageId], mode)
    },
  )

  ipcMain.on(
    'canvas-click-at',
    (
      _event,
      {
        screenX,
        screenY,
        modifiers,
      }: { screenX: number; screenY: number; modifiers?: SelectionModifiers },
    ) => {
      if (interactionBlocksPageSelection()) return
      const origin = canvasOrigin()
      const canvasX = (screenX - origin.x - pan.x) / zoom
      const canvasY = (screenY - origin.y - pan.y) / zoom
      // Use a 1x1 rect at the click point for hit-testing.
      // Drawings are hit-tested in the renderer (SVG stroke hit-paths), not by bbox here.
      selectEntitiesInRect(
        { x: canvasX, y: canvasY, width: 1, height: 1 },
        { includeDrawings: false, mode: selectionMutationMode(modifiers) },
      )
    },
  )

  ipcMain.on(
    'canvas-select-in-screen-rect',
    (
      _event,
      rect: {
        x: number
        y: number
        width: number
        height: number
        modifiers?: SelectionModifiers
      },
    ) => {
      if (interactionBlocksPageSelection()) return
      const origin = canvasOrigin()
      const canvasX = (rect.x - origin.x - pan.x) / zoom
      const canvasY = (rect.y - origin.y - pan.y) / zoom
      selectEntitiesInRect(
        { x: canvasX, y: canvasY, width: rect.width / zoom, height: rect.height / zoom },
        { mode: selectionMutationMode(rect.modifiers) },
      )
    },
  )

  ipcMain.on('canvas-select-entities', (_event, entityIds: string[]) => {
    setSelectedEntities(entityIds)
    requestLayout()
  })

  const VALID_ENTITY_KINDS: ReadonlySet<CanvasEntityKind> = new Set<CanvasEntityKind>(
    DRAWING_FEATURE_ENABLED
      ? ['page', 'text', 'file', 'drawing', 'shape', 'edge']
      : ['page', 'text', 'file', 'shape', 'edge'],
  )
  ipcMain.on(
    'canvas-select-entity',
    (
      _event,
      {
        entityId,
        entityKind,
        modifiers,
      }: { entityId: string; entityKind: string; modifiers?: SelectionModifiers },
    ) => {
      if (!VALID_ENTITY_KINDS.has(entityKind as CanvasEntityKind)) return
      if (entityKind === 'page' && interactionBlocksPageSelection()) return
      const mode = selectionMutationMode(modifiers)
      if (mode === 'replace') {
        selectEntity(entityId, entityKind)
        return
      }
      applyEntitySelectionMutation([entityId], mode)
    },
  )

  ipcMain.on('canvas-select-group', (_event, { groupId }: { groupId: string }) => {
    selectGroup(groupId, { clearInteraction: true })
  })

  ipcMain.on('canvas-enter-group', (_event, { groupId }: { groupId: string }) => {
    enterGroup(groupId, { clearInteraction: true })
  })

  // Unified inline-edit entry point. Dblclick on a sticky/text/shape body
  // (or a group rename label) sends `canvas-request-entity-edit`; main
  // selects the entity, transitions the InteractionController into
  // `editing-entity`, and stashes the token. The renderer derives
  // editing state from the broadcast `interaction` field of the next
  // layout-update — no separate begin-edit channel.
  ipcMain.on(
    'canvas-request-entity-edit',
    (_event, { entityId }: { entityId: string }) => {
      const kind = resolveEntityKind(entityId)
      if (kind === 'edge' || kind === 'drawing') return
      // Selection first so the renderer's selection outline / chrome
      // updates atomically with the edit-mode entry. selectEntity for
      // groups uses selectGroup; pages use selectPageById.
      if (kind === 'group') {
        selectGroup(entityId, { clearInteraction: true })
      } else {
        selectEntity(entityId, kind)
      }
      beginEditingEntity(entityId)
    },
  )

  ipcMain.on('canvas-commit-entity-edit', () => {
    commitEditingEntity()
  })

  ipcMain.on('canvas-cancel-entity-edit', () => {
    cancelEditingEntity('escape')
  })

  ipcMain.on('canvas-hover-page', (_event, { pageId }: { pageId: string | null }) => {
    if (interactionBlocksPageHover()) return
    // Comment tool captures pointerdown in the overlay — page hover would
    // race with the comment gesture's element preview (ADR 0006).
    if (uiActiveTool().kind === 'comment') return
    setHoveredPage(pageId)
  })

  // PoC: aboveView forwards wheel/pointer events that hit the body of the
  // single-selected page so the page reacts as if clicked/scrolled directly.
  // See docs/plans/aboveview-interactive-layer-poc.md.
  ipcMain.on(
    'canvas-forward-wheel',
    (_event, { pageId, payload }: { pageId: string; payload: ForwardWheelPayload }) => {
      forwardWheelToPage(pageId, payload)
    },
  )
  ipcMain.on(
    'canvas-forward-pointer',
    (_event, { pageId, payload }: { pageId: string; payload: ForwardPointerPayload }) => {
      forwardPointerToPage(pageId, payload)
    },
  )

  // Renderers report whether a typing target (textarea / contenteditable /
  // input) is currently focused so canvas-mode keyboard shortcuts back
  // off (Cmd+Z, single-letter tool hotkeys, etc.). Edit-mode lifecycle
  // (`editing-entity`) is owned by the request/commit/cancel-entity-edit
  // IPC pair — this handler stays focus-tracking-only.
  ipcMain.on('canvas-set-text-editing', (event, { active }: { active: boolean }) => {
    setTextEditingActive(event.sender, active)
  })

  ipcMain.on(
    'canvas-set-annotation-state',
    (_event, { hasOpenThread, hasPending }: { hasOpenThread: boolean; hasPending: boolean }) => {
      setAnnotationState(hasOpenThread, hasPending)
    },
  )

  // --- Browser mode ---

  ipcMain.on('canvas-select-browser-tab', (_event, { pageId }: { pageId: string }) => {
    selectBrowserTab(pageId)
  })

  ipcMain.on(
    'canvas-set-browser-size-mode',
    (_event, { pageId, mode }: { pageId: string; mode: BrowserSizeMode }) => {
      const page = pages.find((candidate) => candidate.id === pageId)
      if (!page) return
      page.metadata = setPageBrowserSizeMode(page.metadata, mode)
      scheduleWorkspaceAutosave()
      requestLayout()
    },
  )

  ipcMain.on(
    'canvas-set-browser-mode',
    (_event, { mode }: { mode: 'canvas' | 'browser' }) => {
      if (mode === 'browser') {
        setBrowserMode()
        return
      }
      setCanvasMode()
    },
  )

  // --- Tab management ---

  ipcMain.on('canvas-select-tab', (_event, { tabId }: { tabId: string }) => {
    setActiveWorkspaceTab(tabId)
  })

  ipcMain.on('canvas-create-tab', () => {
    createWorkspaceTab()
  })

  ipcMain.on(
    'canvas-rename-tab',
    (_event, { tabId, name }: { tabId: string; name: string }) => {
      renameWorkspaceTab(tabId, name)
    },
  )

  ipcMain.on(
    'canvas-rename-page',
    (_event, { pageId, name }: { pageId: string; name: string }) => {
      renameWorkspacePage(pageId, name)
    },
  )

  ipcMain.on(
    'canvas-rename-group',
    (_event, { groupId, name }: { groupId: string; name: string }) => {
      renameWorkspaceGroup(groupId, name)
    },
  )

  ipcMain.on(
    'canvas-rename-file-entity',
    (_event, { entityId, name }: { entityId: string; name: string }) => {
      renameWorkspaceFileEntity(entityId, name)
    },
  )

  ipcMain.on(
    'canvas-rename-text-entity',
    (_event, { entityId, name }: { entityId: string; name: string }) => {
      renameWorkspaceTextEntity(entityId, name)
    },
  )

  ipcMain.on(
    'canvas-rename-drawing-entity',
    (_event, { entityId, name }: { entityId: string; name: string }) => {
      renameWorkspaceDrawingEntity(entityId, name)
    },
  )

  ipcMain.on('canvas-duplicate-tab', (_event, { tabId }: { tabId: string }) => {
    duplicateWorkspaceTab(tabId)
  })

  ipcMain.on('canvas-delete-tab', (_event, { tabId }: { tabId: string }) => {
    deleteWorkspaceTab(tabId)
  })

  ipcMain.on(
    'canvas-reorder-tab',
    (_event, { tabId, toIndex }: { tabId: string; toIndex: number }) => {
      reorderWorkspaceTab(tabId, toIndex)
    },
  )

  ipcMain.on(
    'canvas-reorder-sidebar-item',
    (
      _event,
      payload: {
        section: SidebarSectionKey
        draggedId: string
        anchorId: string | null
        position: 'before' | 'after'
        parentId: string | null
      },
    ) => {
      if (payload.section !== 'notes' && payload.section !== 'pages') return
      if (payload.position !== 'before' && payload.position !== 'after') return
      reorderSidebarStackOrder(payload)
    },
  )

  ipcMain.on(
    'canvas-set-tab-expanded',
    (_event, { tabId, expanded }: { tabId: string; expanded: boolean }) => {
      setWorkspaceTabExpanded(tabId, expanded)
    },
  )

  // --- Edge operations ---

  ipcMain.on(
    'canvas-create-edge',
    (
      _event,
      {
        fromEntityId,
        toEntityId,
        fromSide,
        toSide,
      }: {
        fromEntityId: string
        toEntityId: string
        fromSide?: EdgeSide
        toSide?: EdgeSide
      },
    ) => {
      if (!fromSide || !toSide) return
      createEdges({
        edges: [
          {
            fromEntityId,
            toEntityId,
            fromSide,
            toSide,
            toEnd: 'arrow',
            kind: 'connection',
          },
        ],
      })
      commitActive()
      setHoverEntity(null)
      requestLayout()
    },
  )

  ipcMain.on('canvas-delete-edge', (_event, { edgeId }: { edgeId: string }) => {
    deleteEdges({ edgeIds: [edgeId] })
    requestLayout()
  })

  ipcMain.on('canvas-select-edge', (_event, { edgeId }: { edgeId: string | null }) => {
    if (!edgeId) {
      selectNone()
      return
    }
    selectEntity(edgeId, 'edge')
    requestLayout()
  })

  // --- File drop ---
  //
  // Electron dispatches drop events to every overlapping WCV (gotcha #9).
  // Preferred path: renderer stamps a unique dragId on dragstart and
  // forwards it through — DropOwner.consumeDragId ensures the first
  // delivery wins and the rest are ignored (spec §4.5, invariant I5).
  //
  // Legacy path: payload-hash + 500ms window. Kept as fallback until
  // every preload bridge stamps a dragId (Phase 5 cutover).

  let lastDropKey = ''
  let lastDropTime = 0

  ipcMain.on(
    'canvas-drop-file-buffer',
    (
      _event,
      {
        buffer,
        ext,
        canvasX,
        canvasY,
        dragId,
      }: { buffer: Buffer; ext: string; canvasX: number; canvasY: number; dragId?: string },
    ) => {
      if (dragId) {
        if (consumeDragId(dragId)) return
      } else {
        const dropKey = `${buffer.length}:${ext}:${canvasX}:${canvasY}`
        const now = Date.now()
        if (dropKey === lastDropKey && now - lastDropTime < 500) return
        lastDropKey = dropKey
        lastDropTime = now
      }

      const file = saveImageBuffer(buffer, ext)
      const { width, height } = htmlDefaultSize(`.${ext}`) ?? imageSizeFromBuffer(buffer)
      createFileEntity({ canvasX, canvasY, file, width, height })
    },
  )

  ipcMain.on(
    'canvas-drop-component-path',
    (
      _event,
      {
        absolutePath,
        canvasX,
        canvasY,
        dragId,
      }: { absolutePath: string; canvasX: number; canvasY: number; dragId?: string },
    ) => {
      if (!absolutePath) return
      if (dragId && consumeDragId(dragId)) return
      // No metadata stamp here — componentRenderPlugin.resolveUrl re-derives
      // the repo from entity.file every time, so a file dropped before its
      // repo is connected (or while the wrong parent repo was the only
      // match) heals automatically once the right repo shows up.
      createFileEntity({ canvasX, canvasY, file: absolutePath })
    },
  )

}
