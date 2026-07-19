import { ipcChannels } from '../../shared/ipc-contract'
import { ipcMain } from 'electron'
import type {
  CanvasEntityKind,
  FocusPresentationMode,
  SelectionModifiers,
  SidebarSectionKey,
} from '../../shared/types'
import type { EdgeEnd, EdgeSide } from '../../shared/types'
import type { MarqueeSelectionMode } from '../../shared/marquee-selection'
import { selectionMutationMode } from '../../shared/selection-modifiers'
import { pages } from '../runtime/page-runtime'
import { setCommentOverlayActive } from '../runtime/runtime-core'
import { setHoverEntity, setHoveredPage } from '../runtime/runtime-core'
import { activeTool as uiActiveTool } from '../ui-state'
import { pan, setPendingFocus, zoom } from '../runtime/runtime-context'
import { requestLayout } from '../runtime/viewport-control'
import { boundCanvasOrigin as canvasOrigin } from '../runtime/runtime-geometry'
import { saveImageBuffer } from '../runtime/image-assets'
import { htmlDefaultSize, imageSizeFromBuffer } from '../runtime/image-sizing'
import {
  focusSelection,
  getSelectedEntityIds,
  selectEntity,
  selectPage,
  selectPageById,
  restoreFocusCamera,
  selectedPageId,
  setFocusPresentationMode,
  setFocusAnnotationsVisible,
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
import { leftSidebarView } from '../runtime/view-refs'
import {
  forwardPointerToPage,
  forwardWheelToPage,
  type ForwardPointerPayload,
  type ForwardWheelPayload,
} from '../runtime/page-input-forwarding'
import {
  createWorkspaceTab,
  deleteWorkspaceTab,
  renameWorkspaceDrawingEntity,
  renameWorkspaceFileEntity,
  renameWorkspacePage,
  renameWorkspaceGroup,
  renameWorkspaceTab,
  renameWorkspaceTextEntity,
  reorderWorkspaceTab,
  setActiveWorkspaceTab,
} from '../runtime/workspace-tab-operations'
import { scheduleWorkspaceAutosave } from '../runtime/workspace-autosave'
import { deleteEdges } from '../workspace-edges'
import { updateEdge } from '../runtime/document-commands'
import { notifyLeftSidebarData } from '../runtime/sidebar-builder'

type EdgeUpdatePatch = {
  fromEnd?: EdgeEnd
  toEnd?: EdgeEnd
  fromSide?: EdgeSide
  toSide?: EdgeSide
  color?: string
  label?: string
  strokeWidth?: number
  lineStyle?: import('../../shared/types').EdgeLineStyle
}
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
import { enterPageInteractive } from '../runtime/overlay-manager'
import { registerCanvasDragIpc } from './register-canvas-drag-ipc'
import { registerCanvasEntityIpc } from './register-canvas-entity-ipc'
import { registerCanvasReorderIpc } from './register-canvas-reorder-ipc'
import { registerCanvasGapIpc } from './register-canvas-gap-ipc'
import { reorderSidebarStackOrder } from '../runtime/entity-order-state'
import { reparentEntities } from '../runtime/group-membership'

export function registerCanvasIpc(): void {
  registerCanvasDragIpc()
  registerCanvasEntityIpc()
  registerCanvasReorderIpc()
  registerCanvasGapIpc()

  // --- Selection ---

  ipcMain.on(
    ipcChannels.canvasSelectInRect,
    (
      _event,
      payload: {
        x: number
        y: number
        width: number
        height: number
        modifiers?: SelectionModifiers
        selectionMode?: MarqueeSelectionMode
        excludedEntityIds?: string[]
      },
    ) => {
      const {
        modifiers,
        selectionMode: requestedMode,
        excludedEntityIds,
        ...bounds
      } = payload
      const selectionMode =
        requestedMode === 'contain' ? 'contain' : 'intersect'
      selectEntitiesInRect(bounds, {
        mode: selectionMutationMode(modifiers),
        selectionMode,
        excludedEntityIds: Array.isArray(excludedEntityIds)
          ? excludedEntityIds.filter((id): id is string => typeof id === 'string')
          : [],
      })
    },
  )

  ipcMain.on(ipcChannels.canvasClearAnnotateHover, () => {
    for (const page of pages) {
      if (page.pageView.webContents.isDestroyed()) continue
      page.pageView.webContents.send(ipcChannels.annotateClearHover)
    }
  })

  ipcMain.on(
    ipcChannels.canvasSelectPage,
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

  ipcMain.on(ipcChannels.canvasSelectEntities, (_event, entityIds: string[]) => {
    setSelectedEntities(entityIds)
    requestLayout()
  })

  ipcMain.on(ipcChannels.canvasFocusSelection, () => {
    focusSelection()
  })

  ipcMain.on(ipcChannels.canvasRestoreFocusCamera, () => {
    restoreFocusCamera()
  })

  ipcMain.on(ipcChannels.canvasSetFocusPresentationMode, (_event, mode: FocusPresentationMode) => {
    if (mode !== 'device' && mode !== 'fit' && mode !== 'fill') return
    setFocusPresentationMode(mode)
  })

  ipcMain.on(ipcChannels.canvasSetFocusAnnotationsVisible, (_event, visible: boolean) => {
    setFocusAnnotationsVisible(Boolean(visible))
  })

  const VALID_ENTITY_KINDS: ReadonlySet<CanvasEntityKind> = new Set<CanvasEntityKind>([
    'page', 'text', 'file', 'drawing', 'shape', 'group', 'edge',
  ])
  ipcMain.on(
    ipcChannels.canvasSelectEntity,
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

  ipcMain.on(ipcChannels.canvasSelectGroup, (_event, { groupId }: { groupId: string }) => {
    selectGroup(groupId, { clearInteraction: true })
  })

  ipcMain.on(ipcChannels.canvasEnterGroup, (_event, { groupId }: { groupId: string }) => {
    enterGroup(groupId, { clearInteraction: true })
  })

  ipcMain.on(ipcChannels.canvasEnterPageInteractive, (_event, { pageId }: { pageId: string }) => {
    enterPageInteractive(pageId)
  })

  // Unified inline-edit entry point. Dblclick on a sticky/text/shape body
  // (or a group rename label) sends `canvas-request-entity-edit`; main
  // selects the entity, transitions the InteractionController into
  // `editing-entity`, and stashes the token. The renderer derives
  // editing state from the broadcast `interaction` field of the next
  // layout-update — no separate begin-edit channel.
  ipcMain.on(
    ipcChannels.canvasRequestEntityEdit,
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

  ipcMain.on(ipcChannels.canvasCommitEntityEdit, () => {
    commitEditingEntity()
  })

  ipcMain.on(ipcChannels.canvasCancelEntityEdit, () => {
    cancelEditingEntity('escape')
  })

  ipcMain.on(ipcChannels.canvasHoverPage, (_event, { pageId }: { pageId: string | null }) => {
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
    ipcChannels.canvasForwardWheel,
    (_event, { pageId, payload }: { pageId: string; payload: ForwardWheelPayload }) => {
      // Focus presentation locks the camera: a focused page scrolls its own
      // content; the wheel never reaches the canvas, so there's no exit to guard.
      forwardWheelToPage(pageId, payload)
    },
  )
  ipcMain.on(
    ipcChannels.canvasForwardPointer,
    (_event, { pageId, payload }: { pageId: string; payload: ForwardPointerPayload }) => {
      forwardPointerToPage(pageId, payload)
    },
  )

  // Renderers report whether a typing target (textarea / contenteditable /
  // input) is currently focused so canvas-mode keyboard shortcuts back
  // off (Cmd+Z, single-letter tool hotkeys, etc.). Edit-mode lifecycle
  // (`editing-entity`) is owned by the request/commit/cancel-entity-edit
  // IPC pair — this handler stays focus-tracking-only.
  ipcMain.on(ipcChannels.canvasSetTextEditing, (event, { active }: { active: boolean }) => {
    setTextEditingActive(event.sender, active)
    // When the sidebar reports a text input becoming active, request a layout
    // pass so reconcileFocus() immediately gives focus to the sidebar —
    // preventing any in-flight layout pass from stealing it back to aboveView.
    if (event.sender === leftSidebarView?.webContents) {
      if (active) setPendingFocus({ kind: 'sidebar' })
      requestLayout()
    }
  })

  ipcMain.on(
    ipcChannels.canvasSetAnnotationState,
    (_event, { hasOpenThread, hasPending }: { hasOpenThread: boolean; hasPending: boolean }) => {
      setAnnotationState(hasOpenThread, hasPending)
    },
  )

  // --- Tab management ---

  ipcMain.on(ipcChannels.canvasSelectTab, (_event, { tabId }: { tabId: string }) => {
    setActiveWorkspaceTab(tabId)
  })

  ipcMain.on(ipcChannels.canvasCreateTab, () => {
    createWorkspaceTab()
  })

  ipcMain.handle(
    ipcChannels.canvasRenameTab,
    (_event, { tabId, name }: { tabId: string; name: string }) => {
      const renamed = renameWorkspaceTab(tabId, name)
      if (renamed) notifyLeftSidebarData()
      return renamed
    },
  )

  ipcMain.handle(
    ipcChannels.canvasRenamePage,
    (_event, { pageId, name }: { pageId: string; name: string }) => {
      const renamed = renameWorkspacePage(pageId, name)
      if (renamed) notifyLeftSidebarData()
      return renamed
    },
  )

  ipcMain.handle(
    ipcChannels.canvasRenameGroup,
    (_event, { groupId, name }: { groupId: string; name: string }) => {
      const renamed = renameWorkspaceGroup(groupId, name)
      if (renamed) notifyLeftSidebarData()
      return renamed
    },
  )

  ipcMain.handle(
    ipcChannels.canvasRenameFileEntity,
    (_event, { entityId, name }: { entityId: string; name: string }) => {
      const renamed = renameWorkspaceFileEntity(entityId, name)
      if (renamed) notifyLeftSidebarData()
      return renamed
    },
  )

  ipcMain.handle(
    ipcChannels.canvasRenameTextEntity,
    (_event, { entityId, name }: { entityId: string; name: string }) => {
      const renamed = renameWorkspaceTextEntity(entityId, name)
      if (renamed) notifyLeftSidebarData()
      return renamed
    },
  )

  ipcMain.handle(
    ipcChannels.canvasRenameDrawingEntity,
    (_event, { entityId, name }: { entityId: string; name: string }) => {
      const renamed = renameWorkspaceDrawingEntity(entityId, name)
      if (renamed) notifyLeftSidebarData()
      return renamed
    },
  )


  ipcMain.on(ipcChannels.canvasDeleteTab, (_event, { tabId }: { tabId: string }) => {
    deleteWorkspaceTab(tabId)
  })

  ipcMain.on(
    ipcChannels.canvasReorderTab,
    (_event, { tabId, toIndex }: { tabId: string; toIndex: number }) => {
      reorderWorkspaceTab(tabId, toIndex)
    },
  )

  ipcMain.on(
    ipcChannels.canvasReorderSidebarItem,
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
    ipcChannels.canvasReparentSidebarItems,
    (
      _event,
      payload: { entityIds: string[]; parentGroupId: string | null },
    ) => {
      if (!Array.isArray(payload.entityIds)) return
      reparentEntities(payload.entityIds, payload.parentGroupId)
    },
  )

  // --- Edge operations ---

  ipcMain.on(ipcChannels.canvasDeleteEdge, (_event, { edgeId }: { edgeId: string }) => {
    deleteEdges({ edgeIds: [edgeId] })
    requestLayout()
  })

  ipcMain.on(
    ipcChannels.canvasUpdateEdge,
    (_event, { edgeId, patch }: { edgeId: string; patch: EdgeUpdatePatch }) => {
      if (!edgeId) return
      updateEdge(edgeId, patch)
      requestLayout()
    },
  )

  ipcMain.on(ipcChannels.canvasSelectEdge, (_event, { edgeId }: { edgeId: string | null }) => {
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
    ipcChannels.canvasDropFileBuffer,
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
    ipcChannels.canvasDropComponentPath,
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
