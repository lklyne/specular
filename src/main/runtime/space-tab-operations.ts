import { randomUUID } from 'crypto'
import { markDirty } from './layout-dirty'
import { fileEntities, updateFileEntity } from './file-entity-state'
import { textEntities, updateTextEntity } from './text-entity-state'
import { drawingEntities, updateDrawingEntity } from './drawing-entity-state'
import { isRenamableNotePath, renameNoteFile } from './note-assets'
import type {
  PersistedWorkspaceTab,
  WorkspaceSnapshot,
} from '../../shared/types'
import {
  getUiState,
  replaceUiState,
  devtoolsPanelTab as uiDevtoolsPanelTab,
  devtoolsWidth as uiDevtoolsWidth,
  setDevtoolsOpen as setUiDevtoolsOpen,
} from '../ui-state'
import { withSuppressedDocSync } from './space-doc'
import {
  scheduleSpaceAutosave,
  withSpacePersistenceSuspended,
} from './space-autosave'
import { requestLayout } from './viewport-control'
import { setZoom, setPan } from './viewport-control'
import {
  activeSpaceTabId,
  setActiveSpaceTabId,
  workspaceAnnotations,
  workspaceEdges,
  workspaceGroups,
  spaceTabs,
} from './space-model'
import {
  cloneAnnotationsForPersistence,
  cloneWorkspaceSnapshot,
} from './runtime-serialization'
import {
  ensureSpaceTabsInitialized,
  syncActiveTabRecord,
  makeEmptyTabSnapshot,
} from './space-tabs'
import {
  DEFAULT_TAB_NAME,
  deleteCanvasFile,
  makeSpaceTabId,
} from './space-persistence'
import { spaceDir } from './space-dir'
import { findPageById } from './runtime-context'
import { destroyActivePages } from './space-restore'
import { restoreWorkspaceSnapshot, transitionToTab } from './space-restore'
import { clearInspectTargets, syncInspectionState, notifyDevtoolsPanelData } from './inspect-session'
import { sendInteractiveState } from './overlay-manager'
import { cancelActive as cancelActiveInteraction } from './interaction-controller'

function makePageId(): string {
  return `page_${randomUUID()}`
}

/**
 * Side-effect half of a tab switch: reset the per-tab UI state — selection,
 * active tool, devtools, overlays.
 */
function resetUiStateForTabSwitch(): void {
  replaceUiState({
    ...getUiState(),
    selection: { kind: 'none' },
    activeTool: { kind: 'select' },
    devtools: {
      ...getUiState().devtools,
      open: false,
      activeTab: uiDevtoolsPanelTab(),
      focusedAnnotationId: null,
      width: uiDevtoolsWidth(),
    },
    overlays: {
      commentOverlayVisible: false,
      selectionMarqueeVisible: false,
    },
  })
}

/**
 * Data half of a tab switch: swap the tab's annotations, groups, edges, and
 * entities into the runtime arrays. Returns whether the snapshot carried
 * content worth restoring — when it did not, the caller still owns the
 * viewport/panel state that `restoreWorkspaceSnapshot` would have applied
 * (see `applyEmptyTabViewState`).
 *
 * `destroyActivePages()` lives here rather than with the UI side effects
 * because it is what clears the runtime entity arrays, and it must run after
 * the annotation swap: it reaches `clearPageAnchorsForPage`, which strips
 * `pageAnchor` from `workspaceAnnotations`.
 */
export function hydrateTabRuntimeState(tab: PersistedWorkspaceTab): boolean {
  workspaceAnnotations.length = 0
  workspaceAnnotations.push(...cloneAnnotationsForPersistence(tab.annotations))
  destroyActivePages()
  workspaceGroups.length = 0
  workspaceEdges.length = 0
  if (tab.snapshot.pages.length || (tab.snapshot.entities && Object.keys(tab.snapshot.entities).length)) {
    restoreWorkspaceSnapshot(tab.snapshot)
    return true
  }
  return false
}

/**
 * Side-effect half for a tab with nothing to restore: apply the viewport and
 * panel state that `restoreWorkspaceSnapshot` applies on the populated path.
 */
function applyEmptyTabViewState(snapshot: WorkspaceSnapshot): void {
  setZoom(snapshot.zoom)
  setPan(snapshot.pan.x, snapshot.pan.y)
  setUiDevtoolsOpen(false)
  clearInspectTargets()
  sendInteractiveState()
  syncInspectionState()
  notifyDevtoolsPanelData()
}

export function applyTabState(tab: PersistedWorkspaceTab): void {
  // Tab switch is a hard transition — drop any in-flight inline edit
  // before swapping entities. The renderer's blur handler saves the
  // text on unmount; this just clears the editing-entity mode token.
  cancelActiveInteraction('tab-switch')
  withSpacePersistenceSuspended(() => {
    resetUiStateForTabSwitch()
    if (!hydrateTabRuntimeState(tab)) applyEmptyTabViewState(tab.snapshot)
  })
}

function newWorkspaceTabRecord(name: string): PersistedWorkspaceTab {
  return {
    id: makeSpaceTabId(),
    name,
    updatedAt: new Date().toISOString(),
    snapshot: makeEmptyTabSnapshot(),
    annotations: [],
    expanded: true,
  }
}

export function createSpaceTab(name?: string): string {
  ensureSpaceTabsInitialized()
  syncActiveTabRecord()
  const nextTab = newWorkspaceTabRecord(name?.trim() || `Canvas ${spaceTabs.length + 1}`)
  spaceTabs.push(nextTab)
  setActiveSpaceTab(nextTab.id)
  scheduleSpaceAutosave()
  return nextTab.id
}

export type CreateBackgroundTabResult = { ok: true; id: string } | { ok: false; error: string }

/**
 * Create a tab without moving the user's focus to it — the agent-facing
 * counterpart of `createSpaceTab`.
 *
 * Duplicate names are refused because a tab ref resolves by exact name: a
 * second tab called `notes` makes `--tab notes` ambiguous for every caller.
 */
export function createBackgroundSpaceTab(name: string): CreateBackgroundTabResult {
  const trimmed = name.trim()
  if (!trimmed) return { ok: false, error: 'tab name is required' }
  ensureSpaceTabsInitialized()
  syncActiveTabRecord()
  if (spaceTabs.some((tab) => tab.name.trim() === trimmed)) {
    return { ok: false, error: `a tab named '${trimmed}' already exists` }
  }
  const nextTab = newWorkspaceTabRecord(trimmed)
  spaceTabs.push(nextTab)
  markDirty('sidebar')
  requestLayout()
  scheduleSpaceAutosave()
  return { ok: true, id: nextTab.id }
}

export function renameSpaceTab(tabId: string, name: string): boolean {
  const tab = spaceTabs.find((candidate) => candidate.id === tabId)
  const trimmed = name.trim()
  if (!tab || !trimmed) return false
  // Delete old .canvas file before renaming (next autosave writes the new one)
  const oldName = tab.name
  if (oldName !== trimmed) {
    deleteCanvasFile(spaceDir(), { id: tab.id, name: oldName })
  }
  tab.name = trimmed
  tab.updatedAt = new Date().toISOString()
  markDirty('sidebar')
  requestLayout()
  scheduleSpaceAutosave()
  return true
}

export function renameWorkspacePage(pageId: string, name: string): boolean {
  const page = findPageById(pageId)
  const trimmed = name.trim()
  if (!page || !trimmed) return false
  page.name = trimmed
  markDirty('sidebar')
  requestLayout()
  scheduleSpaceAutosave()
  return true
}

export function renameWorkspaceGroup(groupId: string, name: string): boolean {
  const group = workspaceGroups.find((candidate) => candidate.id === groupId)
  const trimmed = name.trim()
  if (!group || !trimmed) return false
  group.label = trimmed
  markDirty('sidebar')
  requestLayout()
  scheduleSpaceAutosave()
  return true
}

export function renameWorkspaceFileEntity(entityId: string, name: string): boolean {
  const entity = fileEntities.find((candidate) => candidate.id === entityId)
  const trimmed = name.trim()
  if (!entity || !trimmed) return false
  if (!isRenamableNotePath(entity.file)) return false
  const newPath = renameNoteFile(entity.file, trimmed)
  if (!newPath) return false
  if (newPath === entity.file) return true
  updateFileEntity(entity.id, { file: newPath })
  requestLayout()
  scheduleSpaceAutosave()
  return true
}

export function renameWorkspaceTextEntity(entityId: string, name: string): boolean {
  const entity = textEntities.find((candidate) => candidate.id === entityId)
  const trimmed = name.trim()
  if (!entity || !trimmed) return false
  if (trimmed === entity.label) return true
  updateTextEntity(entity.id, { label: trimmed })
  requestLayout()
  scheduleSpaceAutosave()
  return true
}

export function renameWorkspaceDrawingEntity(entityId: string, name: string): boolean {
  const entity = drawingEntities.find((candidate) => candidate.id === entityId)
  const trimmed = name.trim()
  if (!entity || !trimmed) return false
  if (trimmed === entity.label) return true
  updateDrawingEntity(entity.id, { label: trimmed })
  requestLayout()
  scheduleSpaceAutosave()
  return true
}

export function duplicateSpaceTab(tabId: string): string | null {
  ensureSpaceTabsInitialized()
  syncActiveTabRecord()
  const source = spaceTabs.find((candidate) => candidate.id === tabId)
  if (!source) return null
  const now = new Date().toISOString()
  const snapshot = cloneWorkspaceSnapshot(source.snapshot)
  snapshot.pages = snapshot.pages.map((page) => ({ ...page, id: makePageId() }))
  const pageIdMap = new Map<string, string>()
  source.snapshot.pages.forEach((page, index) => {
    const nextId = snapshot.pages[index]?.id
    if (page.id && nextId) pageIdMap.set(page.id, nextId)
  })
  snapshot.selectedPageId =
    (snapshot.selectedPageId && pageIdMap.get(snapshot.selectedPageId)) ?? null
  snapshot.selectedPageIds = snapshot.selectedPageIds
    ?.map((pageId) => pageIdMap.get(pageId) ?? pageId)
    .filter(Boolean) as string[] | undefined
  snapshot.groups = snapshot.groups?.map((group) => ({
    ...group,
    pageIds: group.pageIds?.map((pageId) => pageIdMap.get(pageId) ?? pageId),
  }))
  snapshot.edges = snapshot.edges?.map((edge) => ({
    ...edge,
    fromEntityId: pageIdMap.get(edge.fromEntityId) ?? edge.fromEntityId,
    toEntityId: pageIdMap.get(edge.toEntityId) ?? edge.toEntityId,
  }))
  const annotations = cloneAnnotationsForPersistence(source.annotations).map((annotation) => {
    const anchor =
      annotation.anchor.type === 'canvas' || annotation.anchor.type === 'region'
        ? annotation.anchor
        : { ...annotation.anchor, pageId: pageIdMap.get(annotation.anchor.pageId) ?? annotation.anchor.pageId }
    return { ...annotation, id: `annotation_${randomUUID()}`, anchor }
  })
  const duplicate: PersistedWorkspaceTab = {
    id: makeSpaceTabId(),
    name: `${source.name} Copy`,
    updatedAt: now,
    snapshot,
    annotations,
    expanded: source.expanded ?? true,
  }
  const sourceIndex = spaceTabs.findIndex((candidate) => candidate.id === tabId)
  spaceTabs.splice(sourceIndex + 1, 0, duplicate)
  setActiveSpaceTab(duplicate.id)
  scheduleSpaceAutosave()
  return duplicate.id
}

export function deleteSpaceTab(tabId: string): boolean {
  ensureSpaceTabsInitialized()
  syncActiveTabRecord()
  const index = spaceTabs.findIndex((candidate) => candidate.id === tabId)
  if (index === -1) return false
  const deletedTab = spaceTabs[index]
  if (spaceTabs.length === 1) {
    // Delete old canvas file if the tab is being reset to defaults with a new name
    if (deletedTab.name !== DEFAULT_TAB_NAME) {
      deleteCanvasFile(spaceDir(), deletedTab)
    }
    spaceTabs[index] = {
      ...spaceTabs[index],
      name: DEFAULT_TAB_NAME,
      updatedAt: new Date().toISOString(),
      snapshot: makeEmptyTabSnapshot(),
      annotations: [],
      expanded: true,
    }
    setActiveSpaceTabId(spaceTabs[index].id)
    withSuppressedDocSync(() => applyTabState(spaceTabs[index]))
    transitionToTab(spaceTabs[index].snapshot, spaceTabs[index].id)
    markDirty('sidebar')
    requestLayout()
    scheduleSpaceAutosave()
    return true
  }
  // Delete the .canvas file for the removed tab
  deleteCanvasFile(spaceDir(), deletedTab)
  // Removing a canvas the user is not looking at is a bookkeeping change: drop
  // the record and leave their view where it is. Only losing the active tab
  // forces a move, and then the neighbour is the least surprising landing spot.
  if (tabId !== activeSpaceTabId) {
    spaceTabs.splice(index, 1)
    markDirty('sidebar')
    scheduleSpaceAutosave()
    return true
  }
  const fallback = spaceTabs[index + 1] ?? spaceTabs[index - 1] ?? null
  spaceTabs.splice(index, 1)
  if (!fallback) return false
  setActiveSpaceTabId(fallback.id)
  withSuppressedDocSync(() => applyTabState(fallback))
  transitionToTab(fallback.snapshot, fallback.id)
  markDirty('sidebar')
  requestLayout()
  scheduleSpaceAutosave()
  return true
}

export function reorderSpaceTab(tabId: string, toIndex: number): boolean {
  const fromIndex = spaceTabs.findIndex((candidate) => candidate.id === tabId)
  if (fromIndex === -1) return false
  const clamped = Math.max(0, Math.min(toIndex, spaceTabs.length - 1))
  if (fromIndex === clamped) return false
  const [tab] = spaceTabs.splice(fromIndex, 1)
  spaceTabs.splice(clamped, 0, tab)
  markDirty('sidebar')
  requestLayout()
  scheduleSpaceAutosave()
  return true
}

export function setSpaceTabExpanded(tabId: string, expanded: boolean): boolean {
  const tab = spaceTabs.find((candidate) => candidate.id === tabId)
  if (!tab) return false
  tab.expanded = expanded
  tab.updatedAt = new Date().toISOString()
  markDirty('sidebar')
  requestLayout()
  scheduleSpaceAutosave()
  return true
}

export function setActiveSpaceTab(tabId: string): boolean {
  ensureSpaceTabsInitialized()
  if (tabId === activeSpaceTabId) {
    requestLayout()
    return true
  }
  syncActiveTabRecord()
  const nextTab = spaceTabs.find((candidate) => candidate.id === tabId)
  if (!nextTab) return false
  setActiveSpaceTabId(nextTab.id)
  withSuppressedDocSync(() => applyTabState(nextTab))
  transitionToTab(nextTab.snapshot, nextTab.id)
  markDirty('sidebar')
  requestLayout()
  scheduleSpaceAutosave()
  return true
}
