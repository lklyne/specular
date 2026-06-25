import { app } from 'electron'
import type { PersistedWorkspaceRecord } from '../../shared/types'
import { requestDocSync } from './workspace-observers'
import {
  pages,
  workspaceAutosaveTimer,
  workspacePersistenceSuspendCount,
  setWorkspaceAutosaveTimer,
  incrementWorkspacePersistenceSuspendCount,
  decrementWorkspacePersistenceSuspendCount,
} from './runtime-context'
import { workspaceTabs } from './workspace-model'
import {
  activePersistedWorkspace as resolveActivePersistedWorkspace,
  DEFAULT_WORKSPACE_ID,
  flushWorkspaceAutosaveSync as flushAutosaveNow,
  loadWorkspaceFromCanvasFiles,
  loadWorkspaceStore as loadPersistedWorkspaceStore,
  scheduleWorkspaceAutosave as scheduleAutosave,
  workspaceStorePath,
  writeAllTabsAsCanvasFiles,
  writeWorkspaceMetaSync,
} from './workspace-persistence'
import { buildPersistedWorkspaceRecord } from './workspace-tabs'

function shouldPersistWorkspace(): boolean {
  return (
    workspacePersistenceSuspendCount === 0 &&
    (pages.length > 0 || workspaceTabs.length > 0)
  )
}

/**
 * Load workspace from .canvas files (primary), falling back to legacy workspace-store.json.
 */
export function loadWorkspace(): PersistedWorkspaceRecord | null {
  const userDataPath = app.getPath('userData')
  try {
    // Primary: load from .canvas files
    const record = loadWorkspaceFromCanvasFiles(userDataPath, DEFAULT_WORKSPACE_ID)
    if (record) return record
  } catch (error) {
    console.error('Failed to load workspace from .canvas files:', error)
  }
  try {
    // Fallback: load from legacy workspace-store.json
    const store = loadPersistedWorkspaceStore(workspaceStorePath(userDataPath))
    if (store) {
      console.log('Loaded from legacy workspace-store.json — next save will write .canvas files')
      return resolveActivePersistedWorkspace(store)
    }
  } catch (error) {
    console.error('Failed to load workspace store:', error)
  }
  return null
}

/**
 * Save workspace state to .canvas files + workspace-meta.json.
 * workspace-store.json is no longer written.
 */
export function saveWorkspaceStore(): void {
  if (!shouldPersistWorkspace()) return
  try {
    const record = buildPersistedWorkspaceRecord()
    const userDataPath = app.getPath('userData')
    writeAllTabsAsCanvasFiles(userDataPath, record.id, record.tabs)
    writeWorkspaceMetaSync(userDataPath, record.id, {
      activeTabId: record.activeTabId,
      tabs: record.tabs.map((t) => ({
        id: t.id,
        name: t.name,
        updatedAt: t.updatedAt,
        expanded: t.expanded,
      })),
    })
  } catch (error) {
    console.error('Failed to save workspace:', error)
  }
}

export function scheduleWorkspaceAutosave(): void {
  requestDocSync()
  scheduleAutosave({
    autosaveTimer: workspaceAutosaveTimer,
    shouldPersist: shouldPersistWorkspace,
    setAutosaveTimer: setWorkspaceAutosaveTimer,
    saveWorkspaceStore,
  })
}

export function flushWorkspaceAutosaveSync(): void {
  flushAutosaveNow({
    autosaveTimer: workspaceAutosaveTimer,
    setAutosaveTimer: setWorkspaceAutosaveTimer,
    saveWorkspaceStore,
  })
}

export function withWorkspacePersistenceSuspended<T>(callback: () => T): T {
  incrementWorkspacePersistenceSuspendCount()
  try {
    return callback()
  } finally {
    decrementWorkspacePersistenceSuspendCount()
  }
}
