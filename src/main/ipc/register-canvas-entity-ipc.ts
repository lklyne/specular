import { ipcChannels } from '../../shared/ipc-contract'
import { clipboard, ipcMain, Menu, nativeImage, shell, type MenuItemConstructorOptions } from 'electron'
import { VIEWPORT_PRESETS } from '../../shared/constants'
import type { AnnotationCreateRequest, BatchLayoutMode, CanvasEntityKind, PageColorScheme } from '../../shared/types'
import { getEntityKind, hasEntityKind } from '../entities/contract'
import { CLIPBOARD_PREFIX, pasteFromClipboard } from '../clipboard-paste'
import { pages } from '../runtime/page-runtime'
import { aboveView } from '../runtime/view-refs'
import { beginEditingEntity } from '../runtime/editing-entity-runtime'
import { setPendingFocus } from '../runtime/runtime-context'
import { executeRegionSelect } from '../runtime/region-select'
import { queryElementAtPoint } from '../runtime/page-queries'
import {
  pageAtWindowPoint,
  windowPointToCanvasPoint,
} from '../runtime/window-coords'
import { setCommentOverlayActive } from '../runtime/runtime-core'
import { textEntities } from '../runtime/text-entity-state'
import { fileEntities } from '../runtime/file-entity-state'
import { drawingEntities } from '../runtime/drawing-entity-state'
import { shapeEntities } from '../runtime/shape-entity-state'
import {
  getStickyDefaultColor,
  getPlainTextDefaultColor,
  getShapeDefaults,
  getTextDefaultSize,
  getStickyDefaultSize,
} from '../runtime/tool-defaults'
import { createNoteFile } from '../runtime/note-assets'
import {
  createDrawingEntity,
  createFileEntity,
  createShapeEntity,
  createTextEntity,
  deleteDrawingEntity,
  deleteShapeEntity,
  deleteTextEntity,
  deleteFileEntity,
  setPageCustom,
  setPageColorScheme,
  setDeviceOrientation,
  setFileDeviceOrientation,
  setPagePreset,
  toggleDeviceShell,
  toggleFileDeviceShell,
  resizeMultiSelection,
  groupSelectedEntities,
  ungroupSelectedGroup,
  updateResizeGuides,
} from '../runtime/document-commands'
import type { MultiResizeEntry } from '../runtime/document-commands'
import { writeNoteFile } from '../runtime/note-assets'
import { commitNoteContent } from '../runtime/note-commands'
import {
  activeTool,
  finishOneShotPlacement,
  focusCanvasBounds,
  getSelectedEntityIds,
  selectEntity,
  openDevToolsForSelectedPage,
  selectPage,
  selectPageById,
  selectedPageId,
  setActiveTool,
  setSelectedEntities,
} from '../runtime/ui-actions'
import { requestLayout } from '../runtime/viewport-control'
import { markDirty } from '../runtime/layout-dirty'
import { pageBodyCanvasBounds, pageContentSize } from '../runtime/runtime-geometry'
import {
  scheduleWorkspaceAutosave,
} from '../runtime/workspace-autosave'
import { navigatePage, setSyncForSelection, unsyncPage } from '../navigation-sync'
import {
  deviceIdFromMetadata,
  pageUsesCustomSize,
  setCustomPageSizeMetadata,
  setDeviceIdMetadata,
} from '../runtime/runtime-entities'
import { createAnnotation } from '../workspace-annotations'
import {
  deletePages,
} from '../workspace-entities'
import {
  createPageAtPosition,
  duplicateEntity,
  duplicatePageFromSource,
} from '../workspace-pages'
import { deleteGroups, duplicateGroup, ungroupUserGroup } from '../workspace-groups'
import { copyableSelectionPayload } from '../workspace-clipboard'
import { workspaceGroups } from '../runtime/workspace-model'
import { selectGroup } from '../runtime/selection-controller'
import { deleteSelection } from '../runtime/delete-selection'
import { arrangeEntities } from '../runtime/document-commands'
import { selectedEntityIds } from '../ui-state'
import { duplicateSelection } from '../runtime/duplicate-selection'
import { reorderStackOrder, type StackOrderAction } from '../runtime/entity-order-state'

function isStackOrderAction(action: string): action is StackOrderAction {
  return (
    action === 'bring-forward' ||
    action === 'send-backward' ||
    action === 'bring-to-front' ||
    action === 'send-to-back'
  )
}

function stackOrderMenuItems(targetId: string): MenuItemConstructorOptions[] {
  return [
    {
      label: 'Bring forward',
      accelerator: 'CmdOrCtrl+]',
      click: () => reorderStackOrder('bring-forward', targetId),
    },
    {
      label: 'Send backward',
      accelerator: 'CmdOrCtrl+[',
      click: () => reorderStackOrder('send-backward', targetId),
    },
    {
      label: 'Bring to front',
      accelerator: 'CmdOrCtrl+Shift+]',
      click: () => reorderStackOrder('bring-to-front', targetId),
    },
    {
      label: 'Send to back',
      accelerator: 'CmdOrCtrl+Shift+[',
      click: () => reorderStackOrder('send-to-back', targetId),
    },
  ]
}

export function registerCanvasEntityIpc(): void {
  ipcMain.on(
    ipcChannels.canvasPlacePendingEntity,
    (
      _event,
      payload: {
        canvasX: number
        canvasY: number
        dragRect?: { x: number; y: number; width: number; height: number } | null
      },
    ) => {
      const { canvasX, canvasY } = payload
      const dragRect = payload.dragRect ?? null
      const tool = activeTool()
      if (tool.kind === 'add-text') {
        const created = createTextEntity({
          canvasX,
          canvasY,
          textStyle: 'plain',
          color: getPlainTextDefaultColor() ?? undefined,
          textSize: getTextDefaultSize(),
        })
        selectEntity(created.id, 'text')
        beginEditingEntity(created.id)
      } else if (tool.kind === 'add-document') {
        // Stamps a markdown file entity backed by a fresh empty `.md` note.
        const filePath = createNoteFile()
        const created = createFileEntity({ canvasX, canvasY, file: filePath })
        selectEntity(created.id, 'file')
        beginEditingEntity(created.id)
      } else if (tool.kind === 'add-sticky') {
        const created = createTextEntity({
          canvasX,
          canvasY,
          textStyle: 'sticky',
          color: getStickyDefaultColor(),
          textSize: getStickyDefaultSize(),
        })
        selectEntity(created.id, 'text')
        beginEditingEntity(created.id)
      } else if (tool.kind === 'add-shape') {
        const defaults = getShapeDefaults()
        const created = dragRect
          ? createShapeEntity({
              canvasX: dragRect.x,
              canvasY: dragRect.y,
              width: dragRect.width,
              height: dragRect.height,
              shapeKind: defaults.shapeKind,
              color: defaults.color,
              strokeWidth: defaults.strokeWidth,
              textSize: defaults.textSize,
            })
          : createShapeEntity({
              canvasX,
              canvasY,
              shapeKind: defaults.shapeKind,
              color: defaults.color,
              strokeWidth: defaults.strokeWidth,
              textSize: defaults.textSize,
            })
        selectEntity(created.id, 'shape')
        beginEditingEntity(created.id)
      } else if (tool.kind === 'add-page') {
        createPageAtPosition({
          sourcePageId: tool.sourcePageId,
          presetIndex: tool.presetIndex ?? 0,
          customSize: tool.customSize ?? false,
          canvasX,
          canvasY,
          mode: 'add_from_toolbar',
          focus: true,
        })
      } else {
        return
      }
      finishOneShotPlacement()
    },
  )

  ipcMain.on(ipcChannels.canvasDeleteSelection, () => {
    deleteSelection()
  })

  ipcMain.on(ipcChannels.canvasDeletePage, (_event, { pageId }: { pageId: string }) => {
    if (!pages.some((candidate) => candidate.id === pageId)) return
    deletePages({ pageIds: [pageId] })
  })

  ipcMain.on(ipcChannels.canvasArrangeSelection, (_event, mode: BatchLayoutMode) => {
    arrangeEntities(selectedEntityIds(), mode)
  })

  ipcMain.on(ipcChannels.canvasNavigatePage, (_event, { pageId, url }: { pageId: string; url: string }) => {
    const page = pages.find((candidate) => candidate.id === pageId)
    if (!page) return
    navigatePage(page, { type: 'load-url', url })
  })

  ipcMain.on(ipcChannels.canvasBackPage, (_event, { pageId }: { pageId: string }) => {
    const page = pages.find((candidate) => candidate.id === pageId)
    if (!page) return
    navigatePage(page, { type: 'go-back', fallbackUrl: page.pageView.webContents.getURL() })
  })

  ipcMain.on(ipcChannels.canvasForwardPage, (_event, { pageId }: { pageId: string }) => {
    const page = pages.find((candidate) => candidate.id === pageId)
    if (!page) return
    navigatePage(page, { type: 'go-forward', fallbackUrl: page.pageView.webContents.getURL() })
  })

  ipcMain.on(ipcChannels.canvasReloadPage, (_event, { pageId }: { pageId: string }) => {
    const page = pages.find((candidate) => candidate.id === pageId)
    if (!page) return
    navigatePage(page, { type: 'reload', fallbackUrl: page.pageView.webContents.getURL() })
  })

  ipcMain.on(
    ipcChannels.canvasRevealEntity,
    (_event, { entityId, entityKind }: { entityId: string; entityKind: string }) => {
      if (entityKind === 'page') {
        if (!selectPageById(entityId)) return
        const page = pages.find((candidate) => candidate.id === entityId)
        if (page) focusCanvasBounds(pageBodyCanvasBounds(page))
        return
      }
      selectEntity(entityId, entityKind)
      const te = textEntities.find((t) => t.id === entityId)
      const fe = fileEntities.find((f) => f.id === entityId)
      const de = drawingEntities.find((d) => d.id === entityId)
      const se = shapeEntities.find((s) => s.id === entityId)
      const entity = te ?? fe ?? de ?? se
      if (entity) {
        focusCanvasBounds({ x: entity.canvasX, y: entity.canvasY, width: entity.width, height: entity.height })
      }
    },
  )

  ipcMain.on(
    ipcChannels.canvasDeleteEntity,
    (_event, { entityId, entityKind }: { entityId: string; entityKind: string }) => {
      if (entityKind === 'text') {
        deleteTextEntity(entityId)
      } else if (entityKind === 'file') {
        deleteFileEntity(entityId)
      } else if (entityKind === 'drawing') {
        deleteDrawingEntity(entityId)
      } else if (entityKind === 'shape') {
        deleteShapeEntity(entityId)
      }
      requestLayout()
    },
  )

  ipcMain.on(ipcChannels.canvasRevealGroup, (_event, { groupId }: { groupId: string }) => {
    const group = workspaceGroups.find((candidate) => candidate.id === groupId)
    if (!group) return
    selectGroup(groupId)
    focusCanvasBounds({
      x: group.canvasX,
      y: group.canvasY,
      width: group.width,
      height: group.height,
    })
    requestLayout()
  })

  ipcMain.on(ipcChannels.canvasUngroupGroup, (_event, { groupId }: { groupId: string }) => {
    const group = workspaceGroups.find((g) => g.id === groupId)
    if (!group) return
    selectGroup(groupId)
    ungroupSelectedGroup()
  })

  ipcMain.on(
    ipcChannels.canvasSetPagePreset,
    (_event, { pageId, index }: { pageId: string; index: number }) => {
      if (index < 0 || index >= VIEWPORT_PRESETS.length) return
      const idx = pages.findIndex((candidate) => candidate.id === pageId)
      if (idx === -1) return
      selectPage(idx)
      setPagePreset(pageId, index)
    },
  )

  ipcMain.on(ipcChannels.canvasSetPageCustom, (_event, { pageId }: { pageId: string }) => {
    setPageCustom(pageId)
  })

  ipcMain.on(
    ipcChannels.canvasSetPageColorScheme,
    (_event, { pageId, colorScheme }: { pageId: string; colorScheme: PageColorScheme | null }) => {
      if (colorScheme !== null && colorScheme !== 'light' && colorScheme !== 'dark') return
      setPageColorScheme(pageId, colorScheme)
    },
  )

  ipcMain.on(
    ipcChannels.canvasSetDeviceOrientation,
    (_event, { pageId, orientation }: { pageId: string; orientation: string }) => {
      if (orientation !== 'portrait' && orientation !== 'landscape') return
      setDeviceOrientation(pageId, orientation)
    },
  )

  ipcMain.on(ipcChannels.canvasToggleDeviceShell, (_event, { pageId }: { pageId: string }) => {
    toggleDeviceShell(pageId)
  })

  ipcMain.on(
    ipcChannels.canvasUpdatePageBounds,
    (
      _event,
      {
        pageId,
        patch,
      }: {
        pageId: string
        patch: { width?: number; height?: number; canvasX?: number; canvasY?: number }
      },
    ) => {
      const page = pages.find((candidate) => candidate.id === pageId)
      if (!page) return
      const currentSize = pageContentSize(page)
      const nextSize = {
        width: patch.width ?? currentSize.width,
        height: patch.height ?? currentSize.height,
      }
      const sizeWasResized = patch.width !== undefined || patch.height !== undefined
      const sizeChanged =
        nextSize.width !== currentSize.width || nextSize.height !== currentSize.height
      if (pageUsesCustomSize(page.metadata) || (sizeWasResized && sizeChanged)) {
        let meta = setCustomPageSizeMetadata(page.metadata, nextSize)
        // Resizing away from a device preset clears the device — keeps shell as generic page
        if (sizeChanged && deviceIdFromMetadata(meta)) {
          meta = setDeviceIdMetadata(meta, null)
        }
        page.metadata = meta
      }
      if (patch.canvasX !== undefined) page.canvasX = patch.canvasX
      if (patch.canvasY !== undefined) page.canvasY = patch.canvasY
      updateResizeGuides(pageId)
      scheduleWorkspaceAutosave()
      markDirty('canvas')
      requestLayout()
    },
  )

  ipcMain.on(ipcChannels.canvasDuplicatePage, (_event, { pageId }: { pageId: string }) => {
    if (!pages.some((candidate) => candidate.id === pageId)) return
    duplicatePageFromSource({
      sourcePageId: pageId,
      focus: true,
    })
  })

  ipcMain.on(ipcChannels.canvasShowPageContextMenu, (_event, { pageId }: { pageId: string }) => {
    const page = pages.find((candidate) => candidate.id === pageId)
    if (!page) return
    const canGoBack = page.pageView.webContents.canGoBack()
    const canGoForward = page.pageView.webContents.canGoForward()
    const menu = Menu.buildFromTemplate([
      {
        label: 'Back',
        enabled: canGoBack,
        click: () => navigatePage(page, { type: 'go-back', fallbackUrl: page.pageView.webContents.getURL() }),
      },
      {
        label: 'Forward',
        enabled: canGoForward,
        click: () => navigatePage(page, { type: 'go-forward', fallbackUrl: page.pageView.webContents.getURL() }),
      },
      {
        label: 'Reload',
        click: () => navigatePage(page, { type: 'reload', fallbackUrl: page.pageView.webContents.getURL() }),
      },
      { type: 'separator' },
      {
        label: 'Duplicate',
        click: () => {
          duplicatePageFromSource({ sourcePageId: pageId, focus: true })
        },
      },
      { type: 'separator' },
      ...stackOrderMenuItems(pageId),
      { type: 'separator' },
      {
        label: 'Delete',
        click: () => {
          deletePages({ pageIds: [pageId] })
        },
      },
    ])
    menu.popup()
  })

  ipcMain.on(
    ipcChannels.canvasReorderStack,
    (_event, { action, targetId }: { action: string; targetId?: string }) => {
      if (!isStackOrderAction(action)) return
      reorderStackOrder(action, targetId)
    },
  )

  ipcMain.on(ipcChannels.canvasRevealPage, (_event, { pageId }: { pageId: string }) => {
    if (!selectPageById(pageId)) return
    const page = pages.find((candidate) => candidate.id === pageId)
    if (page) focusCanvasBounds(pageBodyCanvasBounds(page))
  })

  ipcMain.on(ipcChannels.canvasSetSelectionPreset, (_event, index: number) => {
    const pageId = selectedPageId()
    if (!pageId) return
    setPagePreset(pageId, index)
  })

  ipcMain.on(ipcChannels.canvasOpenDevtoolsSelection, () => {
    if (!selectedPageId()) return
    openDevToolsForSelectedPage()
  })

  ipcMain.on(ipcChannels.canvasDuplicateSelection, () => {
    duplicateSelection()
  })

  ipcMain.on(ipcChannels.canvasCopySelection, () => {
    const payload = copyableSelectionPayload()
    if (!payload) return
    clipboard.writeText(`${CLIPBOARD_PREFIX}${JSON.stringify(payload)}`)
  })


  ipcMain.on(
    ipcChannels.canvasPasteSelection,
    (_event, { canvasX, canvasY }: { canvasX: number; canvasY: number }) => {
      pasteFromClipboard({ canvasX, canvasY })
    },
  )

  ipcMain.on(ipcChannels.canvasToggleSyncSelection, () => {
    setSyncForSelection(getSelectedEntityIds())
  })

  ipcMain.on(ipcChannels.canvasUnsyncPage, (_event, pageId: string) => {
    unsyncPage(pageId)
  })

  ipcMain.on(ipcChannels.canvasToggleAnnotateMode, () => {
    const next = activeTool().kind === 'comment' ? { kind: 'select' as const } : { kind: 'comment' as const }
    setActiveTool(next)
  })

  ipcMain.on(ipcChannels.canvasToggleDrawMode, () => {
    const next = activeTool().kind === 'draw' ? { kind: 'select' as const } : { kind: 'draw' as const }
    setActiveTool(next)
  })

  ipcMain.on(ipcChannels.canvasCreateAnnotation, (_event, request: AnnotationCreateRequest) => {
    createAnnotation(request)
  })

  ipcMain.on(ipcChannels.canvasCreateDrawing, (_event, input: {
    canvasX: number
    canvasY: number
    width: number
    height: number
    strokes: import('../../shared/types').AnnotationDrawingStroke[]
  }) => {
    createDrawingEntity(input)
  })

  ipcMain.on(
    ipcChannels.canvasCommitRegionSelect,
    (_event, canvasRect: { x: number; y: number; width: number; height: number }) => {
      // Forward to annotation overlay to show comment composer.
      setCommentOverlayActive(true)
      setPendingFocus({ kind: 'aboveView' })
      requestLayout()
      if (aboveView && !aboveView.webContents.isDestroyed()) {
        aboveView.webContents.send(ipcChannels.regionSelectCommitted, { canvasRect })
      }
    },
  )

  ipcMain.on(
    ipcChannels.canvasCreateRegionAnnotation,
    (_event, payload: { canvasRect: { x: number; y: number; width: number; height: number }; text: string }) => {
      executeRegionSelect(payload.canvasRect, payload.text).catch((err) => {
        console.error('[region-select] failed:', err)
      })
    },
  )

  // Comment tool — click below the drag threshold (ADR 0006). Resolve the
  // page under the click; if a DOM element is at the page-local point,
  // route to the existing `annotate-element-selected` flow. Otherwise
  // fall back to a canvas-point anchor, broadcast on a sibling channel.
  ipcMain.on(
    ipcChannels.canvasCommentClickAt,
    (_event, payload: { windowX?: number; windowY?: number } | undefined) => {
      const windowX = payload?.windowX
      const windowY = payload?.windowY
      if (typeof windowX !== 'number' || typeof windowY !== 'number') return

      const fireCanvasPoint = () => {
        const canvasPoint = windowPointToCanvasPoint(windowX, windowY)
        setCommentOverlayActive(true)
        setPendingFocus({ kind: 'aboveView' })
        requestLayout()
        if (aboveView && !aboveView.webContents.isDestroyed()) {
          aboveView.webContents.send(ipcChannels.commentCanvasPointCommitted, {
            canvasX: canvasPoint.x,
            canvasY: canvasPoint.y,
          })
        }
      }

      const hit = pageAtWindowPoint(windowX, windowY)
      if (!hit) {
        fireCanvasPoint()
        return
      }
      queryElementAtPoint(hit.pageId, hit.localX, hit.localY)
        .then((data) => {
          if (data) {
            setCommentOverlayActive(true)
            setPendingFocus({ kind: 'aboveView' })
            requestLayout()
            if (aboveView && !aboveView.webContents.isDestroyed()) {
              aboveView.webContents.send(ipcChannels.annotateElementSelected, {
                pageId: hit.pageId,
                ...data,
              })
            }
            return
          }
          fireCanvasPoint()
        })
        .catch(() => {
          fireCanvasPoint()
        })
    },
  )

// Generic interactive update — one channel dispatching through the registry's
  // per-kind `update` (create/delete stay per-kind: create payloads diverge,
  // delete routes through `deleteSelection`). The renderer types the patch by
  // kind (`EntityUpdatePatchMap`), so an ill-typed patch is a compile error.
  ipcMain.on(
    ipcChannels.canvasUpdateEntity,
    (_event, { kind, id, patch }: { kind: CanvasEntityKind; id: string; patch: Record<string, unknown> }) => {
      if (!hasEntityKind(kind)) return
      getEntityKind(kind).update(id, patch, {})
    },
  )

  // --- Text Entity IPC ---

  ipcMain.on(ipcChannels.canvasDeleteTextEntity, (_event, { id }: { id: string }) => {
    deleteTextEntity(id)
  })

  ipcMain.on(ipcChannels.canvasDeleteDrawingEntity, (_event, { id }: { id: string }) => {
    deleteDrawingEntity(id)
  })

  ipcMain.on(ipcChannels.canvasDuplicateDrawingEntity, (_event, { id }: { id: string }) => {
    duplicateEntity({ entityId: id, focus: true })
  })

  ipcMain.on(ipcChannels.canvasDuplicateTextEntity, (_event, { id }: { id: string }) => {
    duplicateEntity({ entityId: id, focus: true })
  })

  // --- Shape Entity IPC ---

  ipcMain.on(ipcChannels.canvasDeleteShape, (_event, { id }: { id: string }) => {
    deleteShapeEntity(id)
  })

  ipcMain.on(ipcChannels.canvasDuplicateShape, (_event, { id }: { id: string }) => {
    duplicateEntity({ entityId: id, focus: true })
  })

  // --- File Entity IPC ---

  ipcMain.on(ipcChannels.canvasDeleteFileEntity, (_event, { id }: { id: string }) => {
    deleteFileEntity(id)
  })

  ipcMain.on(ipcChannels.canvasDuplicateFileEntity, (_event, { id }: { id: string }) => {
    duplicateEntity({ entityId: id, focus: true })
  })

  ipcMain.on(
    ipcChannels.canvasSetFileDeviceOrientation,
    (_event, { fileId, orientation }: { fileId: string; orientation: string }) => {
      if (orientation !== 'portrait' && orientation !== 'landscape') return
      setFileDeviceOrientation(fileId, orientation)
    },
  )

  ipcMain.on(
    ipcChannels.canvasToggleFileDeviceShell,
    (_event, { fileId }: { fileId: string }) => {
      toggleFileDeviceShell(fileId)
    },
  )

  ipcMain.on(ipcChannels.canvasShowFileInFinder, (_event, { filePath }: { filePath: string }) => {
    shell.showItemInFolder(filePath)
  })

  ipcMain.on(ipcChannels.canvasCopyFileAsPng, (_event, { filePath }: { filePath: string }) => {
    clipboard.writeImage(nativeImage.createFromPath(filePath))
  })

  // Raw disk write for non-Y.Doc-backed note content (issue #262 non-goals).
  ipcMain.handle(ipcChannels.writeNoteFile, (_event, { filePath, content }: { filePath: string; content: string }) => {
    writeNoteFile(filePath, content)
    return true
  })

  // Markdown note edits: routed through the Y.Doc so they participate in
  // the unified UndoManager (issue #262).
  ipcMain.handle(
    ipcChannels.applyNoteContent,
    (_event, { entityId, content }: { entityId: string; content: string }) => {
      return commitNoteContent(entityId, content)
    },
  )

  ipcMain.on(ipcChannels.canvasDuplicateGroup, (_event, { id }: { id: string }) => {
    duplicateGroup({ groupId: id, focus: true })
  })

  ipcMain.on(ipcChannels.canvasDeleteGroup, (_event, { id }: { id: string }) => {
    deleteGroups({ groupIds: [id] })
  })

  ipcMain.on(ipcChannels.canvasGroupSelection, () => {
    groupSelectedEntities()
  })

  ipcMain.on(ipcChannels.canvasUngroupSelection, () => {
    ungroupSelectedGroup()
  })

  ipcMain.on(
    ipcChannels.canvasResizeMultiSelection,
    (_event, { entries }: { entries: MultiResizeEntry[] }) => {
      resizeMultiSelection(entries)
    },
  )
}
