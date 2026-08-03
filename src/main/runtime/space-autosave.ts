import { app } from 'electron'
import type { PersistedWorkspaceRecord } from '../../shared/types'
import { requestDocSync } from './space-observers'
import { projectAllNoteContentToDisk } from './note-content-state'
import { spaceDir } from './space-dir'
import {
  pages,
  spaceAutosaveTimer,
  spacePersistenceSuspendCount,
  setSpaceAutosaveTimer,
  incrementSpacePersistenceSuspendCount,
  decrementSpacePersistenceSuspendCount,
} from './runtime-context'
import { spaceTabs } from './space-model'
import {
  activePersistedWorkspace as resolveActivePersistedWorkspace,
  flushSpaceAutosaveSync as flushAutosaveNow,
  loadSpaceFromCanvasFiles,
  loadWorkspaceStore as loadPersistedWorkspaceStore,
  scheduleSpaceAutosave as scheduleAutosave,
  workspaceStorePath,
  writeAllTabsAsCanvasFiles,
  writeSpaceMetaSync,
} from './space-persistence'
import { buildPersistedWorkspaceRecord } from './space-tabs'

function shouldPersistSpace(): boolean {
  return (
    spacePersistenceSuspendCount === 0 &&
    (pages.length > 0 || spaceTabs.length > 0)
  )
}

/**
 * Load a space from .canvas files (primary), falling back to legacy workspace-store.json.
 */
export function loadSpace(): PersistedWorkspaceRecord | null {
  try {
    // Primary: load from .canvas files
    const record = loadSpaceFromCanvasFiles(spaceDir())
    if (record) return record
  } catch (error) {
    console.error('Failed to load space from .canvas files:', error)
  }
  try {
    // Fallback: load from legacy workspace-store.json. Predates the .canvas
    // format and always lived at the userData root, not inside a space — an
    // unset spacePath's legacy fallback is nested under it instead.
    const store = loadPersistedWorkspaceStore(workspaceStorePath(app.getPath('userData')))
    if (store) {
      console.log('Loaded from legacy workspace-store.json — next save will write .canvas files')
      return resolveActivePersistedWorkspace(store)
    }
  } catch (error) {
    console.error('Failed to load legacy workspace store:', error)
  }
  return null
}

/**
 * Save space state to .canvas files + workspace-meta.json.
 * workspace-store.json is no longer written.
 */
export function saveSpaceStore(): void {
  if (!shouldPersistSpace()) return
  try {
    const record = buildPersistedWorkspaceRecord()
    const spacePath = spaceDir()
    writeAllTabsAsCanvasFiles(spacePath, record.tabs)
    projectAllNoteContentToDisk()
    writeSpaceMetaSync(spacePath, {
      activeTabId: record.activeTabId,
      tabs: record.tabs.map((t) => ({
        id: t.id,
        name: t.name,
        updatedAt: t.updatedAt,
        expanded: t.expanded,
      })),
    })
  } catch (error) {
    console.error('Failed to save space:', error)
  }
}

export function scheduleSpaceAutosave(): void {
  requestDocSync()
  scheduleAutosave({
    autosaveTimer: spaceAutosaveTimer,
    shouldPersist: shouldPersistSpace,
    setAutosaveTimer: setSpaceAutosaveTimer,
    saveSpaceStore,
  })
}

export function flushSpaceAutosaveSync(): void {
  flushAutosaveNow({
    autosaveTimer: spaceAutosaveTimer,
    setAutosaveTimer: setSpaceAutosaveTimer,
    saveSpaceStore,
  })
}

export function withSpacePersistenceSuspended<T>(callback: () => T): T {
  incrementSpacePersistenceSuspendCount()
  try {
    return callback()
  } finally {
    decrementSpacePersistenceSuspendCount()
  }
}
