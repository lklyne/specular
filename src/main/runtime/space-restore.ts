// fallow-ignore-file circular-dependencies
// Suppressed: see #141. space-tab-operations imports space-restore creating a mutual dependency
import { markAllDirty } from './layout-dirty'
import type {
  PersistedWorkspaceRecord,
  WorkspaceSnapshot,
} from '../../shared/types'
import {
  selectedPageIndex as uiSelectedPageIndex,
  setDevtoolsOpen as setUiDevtoolsOpen,
  setDevtoolsWidth as setUiDevtoolsWidth,
  setLeftSidebarOpen as setUiLeftSidebarOpen,
  setDevtoolsPanelTab as setUiDevtoolsPanelTab,
  selectedEntityId as uiSelectedEntityId,
  selectedGroupId as uiSelectedGroupId,
  createDefaultUiState,
  resetUiState,
} from '../ui-state'
import {
  getActiveDoc,
  withSuppressedDocSync,
  rewriteDocToSnapshot,
  DOC_MAP_VIEWPORT,
  DOC_ENTITY_MAP_NAMES,
  DOC_ARRAY_ENTITY_ORDER,
} from './space-doc'
import { markUndoBoundary } from './space-undo'
import { resetDocSync } from './space-observers'
import {
  scheduleSpaceAutosave,
  withSpacePersistenceSuspended,
} from './space-autosave'
import { setZoom, setPan, requestLayout } from './viewport-control'
import {
  activeSpaceTabId,
  setActiveSpaceTabId,
  workspaceAnnotations,
  workspaceEdges,
  workspaceGroups,
  spaceTabs,
} from './space-model'
import {
  pages,
  setInspectHoveredTarget,
  setInspectSelectedTarget,
  setInspectActivePageId,
  spaceAutosaveTimer,
  setSpaceAutosaveTimer,
  setSelectionOverlayActive,
} from './runtime-context'
import {
  clearInspectTargets,
  syncInspectionState,
  notifyDevtoolsPanelData,
} from './inspect-session'
import { sendInteractiveState } from './overlay-manager'
import {
  clonePersistedWorkspaceTabs,
} from './space-persistence'
import {
  ensureSpaceTabsInitialized,
} from './space-tabs'
import {
  normalizePresetIndex,
} from './runtime-serialization'
import {
  clampDevtoolsWidth,
  normalizeDevtoolsPanelTab,
} from './preferences'
import { createPage, removePageAtIndex } from './page-factory'
import {
  clearTextEntities,
  createTextEntity as createTextEntityInState,
} from './text-entity-state'
import {
  clearFileEntities,
  createFileEntity as createFileEntityInState,
} from './file-entity-state'
import {
  clearDrawingEntities,
  createDrawingEntity as createDrawingEntityInState,
} from './drawing-entity-state'
import {
  clearShapeEntities,
  createShapeEntity as createShapeEntityInState,
} from './shape-entity-state'
import {
  deselectAll,
  selectPage,
  setSelectedPages,
} from './selection-state'
import {
  selectGroup as commitSelectGroup,
  selectPageById as commitSelectPageById,
} from './selection-controller'
import { toggleDevTools } from './devtools-panel'
import { layoutCache, resetLayoutCache } from './layout-cache'
import {
  setBgView,
  setLeftSidebarView,
  setToolbarView,
  setAboveView,
  setDevtoolsBackgroundView,
  setDevtoolsHeaderView,
  setDevtoolsView,
  setDevtoolsResizeHandleView,
  setWin,
  win,
} from './view-refs'
import {
  DEVTOOLS_DEFAULT_WIDTH,
  TOOLBAR_HEIGHT,
} from './runtime-constants'
import { initWindow } from './window-init'
import { applyTabState } from './space-tab-operations'
import { migrateSnapshotEntityOrderForRestore } from './space-restore-migration'

export function destroyActivePages(): void {
  clearTextEntities()
  clearFileEntities()
  clearDrawingEntities()
  clearShapeEntities()
  while (pages.length) {
    removePageAtIndex(pages.length - 1)
  }
}

function selectPageById(id: string): boolean {
  return commitSelectPageById(id)
}

function restoreDocEntityOrder(snapshot: WorkspaceSnapshot): void {
  if (!snapshot.entityOrder) return
  const doc = getActiveDoc()
  const order = doc.getArray<string>(DOC_ARRAY_ENTITY_ORDER)
  if (JSON.stringify(order.toArray()) === JSON.stringify(snapshot.entityOrder)) return
  doc.transact(() => {
    if (order.length) order.delete(0, order.length)
    if (snapshot.entityOrder?.length) order.push(snapshot.entityOrder)
  }, 'restore')
}

export function restoreWorkspaceSnapshot(snapshot: WorkspaceSnapshot): boolean {
  const migratedEntityOrder = migrateSnapshotEntityOrderForRestore(snapshot)
  const hasEntities = snapshot.entities && Object.keys(snapshot.entities).length > 0
  if (!snapshot.pages.length && !hasEntities) return false

  // fallow-ignore-next-line complexity
  withSpacePersistenceSuspended(() => {
    setZoom(snapshot.zoom)
    setPan(snapshot.pan.x, snapshot.pan.y)
    setUiLeftSidebarOpen(snapshot.leftSidebarOpen ?? true)
    setUiDevtoolsWidth(clampDevtoolsWidth(snapshot.devtoolsWidth))
    const normalizedPanelTab = normalizeDevtoolsPanelTab(snapshot.devtoolsPanelTab)
    if (normalizedPanelTab) {
      setUiDevtoolsPanelTab(normalizedPanelTab)
    }
    workspaceGroups.length = 0
    workspaceEdges.length = 0
    if (snapshot.groups) {
      workspaceGroups.push(
        ...snapshot.groups.map((group) => ({
          ...group,
          pageIds: group.pageIds ? [...group.pageIds] : undefined,
          metadata: group.metadata ? { ...group.metadata } : undefined,
        })),
      )
    }
    if (snapshot.entities) {
      for (const id of snapshot.entityOrder ?? Object.keys(snapshot.entities)) {
        const entity = snapshot.entities[id]
        if (entity?.kind === 'group' && !workspaceGroups.some((group) => group.id === entity.id)) {
          workspaceGroups.push({
            id: entity.id,
            kind: 'group',
            label: entity.label,
            canvasX: entity.canvasX,
            canvasY: entity.canvasY,
            width: entity.width,
            height: entity.height,
            parentGroupId: entity.parentGroupId,
            color: entity.color,
            layoutMode: entity.layoutMode,
            layoutGap: entity.layoutGap,
            managedLayout: entity.managedLayout,
            sourceTaskId: entity.sourceTaskId,
            metadata: entity.metadata ? { ...entity.metadata } : undefined,
          })
        }
      }
    }
    if (snapshot.edges) {
      workspaceEdges.push(
        ...snapshot.edges.map((edge) => ({
          ...edge,
          metadata: edge.metadata ? { ...edge.metadata } : undefined,
        })),
      )
    }

    const restoredPageIds = new Set<string>()
    for (const page of snapshot.pages) {
      createPage({
        id: page.id,
        name: page.name,
        url: page.url,
        presetIndex: normalizePresetIndex(page.presetIndex),
        canvasX: page.canvasX,
        canvasY: page.canvasY,
        syncId: page.syncId ?? null,
        source: page.source ?? 'manual',
        parentGroupId: page.parentGroupId ?? page.groupId,
        groupId: page.parentGroupId ?? page.groupId,
        metadata: page.metadata,
        colorScheme: page.colorScheme,
      })
      if (page.id) restoredPageIds.add(page.id)
    }

    // Restore text and file entities from snapshot
    if (snapshot.entities) {
      for (const id of snapshot.entityOrder ?? Object.keys(snapshot.entities)) {
        const entity = snapshot.entities[id]
        if (entity?.kind === 'page' && !restoredPageIds.has(entity.id)) {
          createPage({
            id: entity.id,
            name: entity.name,
            url: entity.url,
            presetIndex: entity.presetIndex,
            canvasX: entity.canvasX,
            canvasY: entity.canvasY,
            syncId: entity.syncId ?? null,
            source: entity.source ?? 'manual',
            parentGroupId: entity.parentGroupId ?? entity.groupId,
            groupId: entity.parentGroupId ?? entity.groupId,
            metadata: entity.metadata,
            colorScheme: entity.colorScheme,
          })
        } else if (entity?.kind === 'text' || (entity as any)?.kind === 'sticky-note') {
          createTextEntityInState({
            id: entity.id,
            canvasX: entity.canvasX,
            canvasY: entity.canvasY,
            text: (entity as any).text,
            color: (entity as any).color,
            textStyle: (entity as any).textStyle,
            widthMode: (entity as any).widthMode,
            textSize: (entity as any).textSize,
            width: (entity as any).width,
            height: (entity as any).height,
            parentGroupId: (entity as any).parentGroupId,
            pageAnchor: (entity as any).pageAnchor,
          })
        } else if (entity?.kind === 'file') {
          createFileEntityInState({
            id: entity.id,
            canvasX: entity.canvasX,
            canvasY: entity.canvasY,
            file: (entity as any).file,
            subpath: (entity as any).subpath,
            width: (entity as any).width,
            height: (entity as any).height,
            parentGroupId: (entity as any).parentGroupId,
            presetIndex: (entity as any).presetIndex,
            metadata: (entity as any).metadata,
            objectFit: (entity as any).objectFit,
          })
        } else if (entity?.kind === 'drawing') {
          createDrawingEntityInState({
            id: entity.id,
            canvasX: entity.canvasX,
            canvasY: entity.canvasY,
            width: (entity as any).width,
            height: (entity as any).height,
            strokes: (entity as any).strokes ?? [],
            parentGroupId: (entity as any).parentGroupId,
            pageAnchor: (entity as any).pageAnchor,
          })
        } else if (entity?.kind === 'shape') {
          createShapeEntityInState({
            id: entity.id,
            canvasX: entity.canvasX,
            canvasY: entity.canvasY,
            width: (entity as any).width,
            height: (entity as any).height,
            shapeKind: (entity as any).shapeKind,
            text: (entity as any).text,
            color: (entity as any).color,
            strokeWidth: (entity as any).strokeWidth,
            textSize: (entity as any).textSize,
            theme: (entity as any).theme,
            parentGroupId: (entity as any).parentGroupId,
            pageAnchor: (entity as any).pageAnchor,
            label: (entity as any).label,
          })
        }
      }
    }

    if (snapshot.selectedPageId) {
      selectPageById(snapshot.selectedPageId)
    } else if (snapshot.selectedPageIds?.length) {
      setSelectedPages(snapshot.selectedPageIds)
    } else if (
      snapshot.selectedPageIndex !== null &&
      snapshot.selectedPageIndex >= 0 &&
      snapshot.selectedPageIndex < pages.length
    ) {
      selectPage(snapshot.selectedPageIndex)
    } else {
      deselectAll()
    }

    if (snapshot.selectedGroupId) {
      commitSelectGroup(snapshot.selectedGroupId)
    }

    // Legacy browser-mode snapshots targeted a page. Browser mode no longer
    // exists, so keep the page selected and remain in the canvas view.
    if (snapshot.browserTabMode === 'page' || snapshot.browserTabMode === 'responsive') {
      const pageId = snapshot.selectedPageId ?? uiSelectedEntityId()
      if (pageId) {
        selectPageById(pageId)
      }
    }

    if (snapshot.devtoolsOpen && uiSelectedPageIndex(pages.map((p) => p.id)) !== null) {
      toggleDevTools()
    }
  })

  restoreDocEntityOrder(snapshot)
  if (migratedEntityOrder) markAllDirty()
  return true
}

/**
 * Write the new tab's state into Y.Doc as a tracked transaction.
 * UndoManager captures the diff so tab switches are undoable.
 */
export function transitionToTab(snapshot: WorkspaceSnapshot, tabId: string): void {
  const doc = getActiveDoc()
  rewriteDocToSnapshot(doc, {
    mapNames: [DOC_MAP_VIEWPORT, ...DOC_ENTITY_MAP_NAMES],
    origin: 'user',
    tab: { id: tabId, snapshot },
    tabs: spaceTabs.map((t) => ({ id: t.id, name: t.name })),
  })
  markAllDirty()
  markUndoBoundary()
  resetDocSync()
}

export function restorePersistedSpace(
  record: PersistedWorkspaceRecord,
): boolean {
  spaceTabs.length = 0
  spaceTabs.push(...clonePersistedWorkspaceTabs(record.tabs))
  let migratedEntityOrder = false
  for (const tab of spaceTabs) {
    migratedEntityOrder = migrateSnapshotEntityOrderForRestore(tab.snapshot) || migratedEntityOrder
  }
  setActiveSpaceTabId(
    record.activeTabId && spaceTabs.some((tab) => tab.id === record.activeTabId)
      ? record.activeTabId
      : spaceTabs[0]?.id ?? null,
  )
  const activeTab = spaceTabs.find((tab) => tab.id === activeSpaceTabId)
  if (!activeTab) return false
  applyTabState(activeTab)
  if (record.viewMode === 'browser') {
    const pageId = activeTab.snapshot.selectedPageId ?? activeTab.snapshot.selectedPageIds?.[0]
    if (pageId) selectPageById(pageId)
  }
  // Startup path: UndoManager not yet created, so this initial hydration
  // won't generate an undo step. initializeDocObservers() handles the
  // initial sync, and clearUndoHistory() is called after to wipe any
  // phantom entries.
  if (migratedEntityOrder) {
    markAllDirty()
    scheduleSpaceAutosave()
  }
  return true
}

function resetWindowState(): void {
  if (layoutCache.layoutTimer) {
    clearTimeout(layoutCache.layoutTimer)
    layoutCache.layoutTimer = null
  }
  if (spaceAutosaveTimer) {
    clearTimeout(spaceAutosaveTimer)
    setSpaceAutosaveTimer(null)
  }

  setBgView(null)
  setLeftSidebarView(null)
  setToolbarView(null)
  setAboveView(null)
  layoutCache.toolbarHeight = TOOLBAR_HEIGHT
  setSelectionOverlayActive(false)
  setDevtoolsBackgroundView(null)
  setDevtoolsHeaderView(null)
  setDevtoolsView(null)
  setDevtoolsResizeHandleView(null)
  resetUiState({
    ...createDefaultUiState(),
    devtools: {
      ...createDefaultUiState().devtools,
      width: DEVTOOLS_DEFAULT_WIDTH,
    },
  })
  setInspectHoveredTarget(null)
  setInspectSelectedTarget(null)
  setInspectActivePageId(null)
  resetLayoutCache()
  pages.length = 0
  workspaceGroups.length = 0
  workspaceEdges.length = 0
  workspaceAnnotations.length = 0
  spaceTabs.length = 0
  setActiveSpaceTabId(null)
  setWin(null)
}

export function rebuildWindowFromSnapshot(snapshot: WorkspaceSnapshot): void {
  const oldWin = win
  const oldBounds = oldWin?.getBounds()

  resetWindowState()
  initWindow()

  if (oldBounds && win) {
    win.setBounds(oldBounds)
  }

  const restored = restoreWorkspaceSnapshot(snapshot)
  if (!restored) return

  requestLayout()

  if (oldWin && !oldWin.isDestroyed()) {
    oldWin.close()
  }
}
