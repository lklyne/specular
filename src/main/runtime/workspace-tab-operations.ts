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
import { withSuppressedDocSync } from './workspace-doc'
import {
  scheduleWorkspaceAutosave,
  withWorkspacePersistenceSuspended,
} from './workspace-autosave'
import { requestLayout } from './viewport-control'
import { setZoom, setPan } from './viewport-control'
import {
  activeWorkspaceTabId,
  setActiveWorkspaceTabId,
  workspaceAnnotations,
  workspaceEdges,
  workspaceGroups,
  workspaceTabs,
} from './workspace-model'
import {
  cloneAnnotationsForPersistence,
  cloneWorkspaceSnapshot,
} from './runtime-serialization'
import {
  ensureWorkspaceTabsInitialized,
  syncActiveTabRecord,
  makeEmptyTabSnapshot,
} from './workspace-tabs'
import {
  DEFAULT_TAB_NAME,
  deleteCanvasFile,
  makeWorkspaceTabId,
} from './workspace-persistence'
import { spaceDir } from './space-dir'
import { findPageById } from './runtime-context'
import { destroyActivePages } from './workspace-restore'
import { restoreWorkspaceSnapshot, transitionToTab } from './workspace-restore'
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
  withWorkspacePersistenceSuspended(() => {
    resetUiStateForTabSwitch()
    if (!hydrateTabRuntimeState(tab)) applyEmptyTabViewState(tab.snapshot)
  })
}

function newWorkspaceTabRecord(name: string): PersistedWorkspaceTab {
  return {
    id: makeWorkspaceTabId(),
    name,
    updatedAt: new Date().toISOString(),
    snapshot: makeEmptyTabSnapshot(),
    annotations: [],
    expanded: true,
  }
}

export function createWorkspaceTab(name?: string): string {
  ensureWorkspaceTabsInitialized()
  syncActiveTabRecord()
  const nextTab = newWorkspaceTabRecord(name?.trim() || `Canvas ${workspaceTabs.length + 1}`)
  workspaceTabs.push(nextTab)
  setActiveWorkspaceTab(nextTab.id)
  scheduleWorkspaceAutosave()
  return nextTab.id
}

export type CreateBackgroundTabResult = { ok: true; id: string } | { ok: false; error: string }

/**
 * Create a tab without moving the user's focus to it — the agent-facing
 * counterpart of `createWorkspaceTab`.
 *
 * Duplicate names are refused because a tab ref resolves by exact name: a
 * second tab called `notes` makes `--tab notes` ambiguous for every caller.
 */
export function createBackgroundWorkspaceTab(name: string): CreateBackgroundTabResult {
  const trimmed = name.trim()
  if (!trimmed) return { ok: false, error: 'tab name is required' }
  ensureWorkspaceTabsInitialized()
  syncActiveTabRecord()
  if (workspaceTabs.some((tab) => tab.name.trim() === trimmed)) {
    return { ok: false, error: `a tab named '${trimmed}' already exists` }
  }
  const nextTab = newWorkspaceTabRecord(trimmed)
  workspaceTabs.push(nextTab)
  markDirty('sidebar')
  requestLayout()
  scheduleWorkspaceAutosave()
  return { ok: true, id: nextTab.id }
}

export function renameWorkspaceTab(tabId: string, name: string): boolean {
  const tab = workspaceTabs.find((candidate) => candidate.id === tabId)
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
  scheduleWorkspaceAutosave()
  return true
}

export function renameWorkspacePage(pageId: string, name: string): boolean {
  const page = findPageById(pageId)
  const trimmed = name.trim()
  if (!page || !trimmed) return false
  page.name = trimmed
  markDirty('sidebar')
  requestLayout()
  scheduleWorkspaceAutosave()
  return true
}

export function renameWorkspaceGroup(groupId: string, name: string): boolean {
  const group = workspaceGroups.find((candidate) => candidate.id === groupId)
  const trimmed = name.trim()
  if (!group || !trimmed) return false
  group.label = trimmed
  markDirty('sidebar')
  requestLayout()
  scheduleWorkspaceAutosave()
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
  scheduleWorkspaceAutosave()
  return true
}

export function renameWorkspaceTextEntity(entityId: string, name: string): boolean {
  const entity = textEntities.find((candidate) => candidate.id === entityId)
  const trimmed = name.trim()
  if (!entity || !trimmed) return false
  if (trimmed === entity.label) return true
  updateTextEntity(entity.id, { label: trimmed })
  requestLayout()
  scheduleWorkspaceAutosave()
  return true
}

export function renameWorkspaceDrawingEntity(entityId: string, name: string): boolean {
  const entity = drawingEntities.find((candidate) => candidate.id === entityId)
  const trimmed = name.trim()
  if (!entity || !trimmed) return false
  if (trimmed === entity.label) return true
  updateDrawingEntity(entity.id, { label: trimmed })
  requestLayout()
  scheduleWorkspaceAutosave()
  return true
}

export function duplicateWorkspaceTab(tabId: string): string | null {
  ensureWorkspaceTabsInitialized()
  syncActiveTabRecord()
  const source = workspaceTabs.find((candidate) => candidate.id === tabId)
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
    id: makeWorkspaceTabId(),
    name: `${source.name} Copy`,
    updatedAt: now,
    snapshot,
    annotations,
    expanded: source.expanded ?? true,
  }
  const sourceIndex = workspaceTabs.findIndex((candidate) => candidate.id === tabId)
  workspaceTabs.splice(sourceIndex + 1, 0, duplicate)
  setActiveWorkspaceTab(duplicate.id)
  scheduleWorkspaceAutosave()
  return duplicate.id
}

export function deleteWorkspaceTab(tabId: string): boolean {
  ensureWorkspaceTabsInitialized()
  syncActiveTabRecord()
  const index = workspaceTabs.findIndex((candidate) => candidate.id === tabId)
  if (index === -1) return false
  const deletedTab = workspaceTabs[index]
  if (workspaceTabs.length === 1) {
    // Delete old canvas file if the tab is being reset to defaults with a new name
    if (deletedTab.name !== DEFAULT_TAB_NAME) {
      deleteCanvasFile(spaceDir(), deletedTab)
    }
    workspaceTabs[index] = {
      ...workspaceTabs[index],
      name: DEFAULT_TAB_NAME,
      updatedAt: new Date().toISOString(),
      snapshot: makeEmptyTabSnapshot(),
      annotations: [],
      expanded: true,
    }
    setActiveWorkspaceTabId(workspaceTabs[index].id)
    withSuppressedDocSync(() => applyTabState(workspaceTabs[index]))
    transitionToTab(workspaceTabs[index].snapshot, workspaceTabs[index].id)
    markDirty('sidebar')
    requestLayout()
    scheduleWorkspaceAutosave()
    return true
  }
  // Delete the .canvas file for the removed tab
  deleteCanvasFile(spaceDir(), deletedTab)
  // Removing a canvas the user is not looking at is a bookkeeping change: drop
  // the record and leave their view where it is. Only losing the active tab
  // forces a move, and then the neighbour is the least surprising landing spot.
  if (tabId !== activeWorkspaceTabId) {
    workspaceTabs.splice(index, 1)
    markDirty('sidebar')
    scheduleWorkspaceAutosave()
    return true
  }
  const fallback = workspaceTabs[index + 1] ?? workspaceTabs[index - 1] ?? null
  workspaceTabs.splice(index, 1)
  if (!fallback) return false
  setActiveWorkspaceTabId(fallback.id)
  withSuppressedDocSync(() => applyTabState(fallback))
  transitionToTab(fallback.snapshot, fallback.id)
  markDirty('sidebar')
  requestLayout()
  scheduleWorkspaceAutosave()
  return true
}

export function reorderWorkspaceTab(tabId: string, toIndex: number): boolean {
  const fromIndex = workspaceTabs.findIndex((candidate) => candidate.id === tabId)
  if (fromIndex === -1) return false
  const clamped = Math.max(0, Math.min(toIndex, workspaceTabs.length - 1))
  if (fromIndex === clamped) return false
  const [tab] = workspaceTabs.splice(fromIndex, 1)
  workspaceTabs.splice(clamped, 0, tab)
  markDirty('sidebar')
  requestLayout()
  scheduleWorkspaceAutosave()
  return true
}

export function setWorkspaceTabExpanded(tabId: string, expanded: boolean): boolean {
  const tab = workspaceTabs.find((candidate) => candidate.id === tabId)
  if (!tab) return false
  tab.expanded = expanded
  tab.updatedAt = new Date().toISOString()
  markDirty('sidebar')
  requestLayout()
  scheduleWorkspaceAutosave()
  return true
}

export function setActiveWorkspaceTab(tabId: string): boolean {
  ensureWorkspaceTabsInitialized()
  if (tabId === activeWorkspaceTabId) {
    requestLayout()
    return true
  }
  syncActiveTabRecord()
  const nextTab = workspaceTabs.find((candidate) => candidate.id === tabId)
  if (!nextTab) return false
  setActiveWorkspaceTabId(nextTab.id)
  withSuppressedDocSync(() => applyTabState(nextTab))
  transitionToTab(nextTab.snapshot, nextTab.id)
  markDirty('sidebar')
  requestLayout()
  scheduleWorkspaceAutosave()
  return true
}
