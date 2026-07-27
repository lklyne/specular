/**
 * Workspace cloud-sync seam.
 *
 * Owns the in-memory binding between the workspace `Y.Doc` (the synced unit —
 * one Durable Object per workspace) and its cloud rendezvous, plus the single
 * door remote transactions enter through. The real network transport is a
 * later step; this module is shaped so one can subscribe to local doc updates
 * (filtered by origin) and feed remote ones in via `applyRemoteUpdate` without
 * any change here.
 *
 * Two invariants are load-bearing (ADR 0018 §1/§4):
 *   1. Remote edits carry `REMOTE_SYNC_ORIGIN`, which is neither `'user'` nor
 *      `null`, so the UndoManager (tracked origins: {null, 'user'}) never
 *      captures them — remote work stays out of the local undo stack — and the
 *      Y.Doc→runtime observer can tell them apart from local mutations.
 *   2. The binding lives in the `.canvas` file (a `specular.server` block), not
 *      the Y.Doc: the workspace map is undo-tracked, and the binding must never
 *      be undoable nor sync as content. Tokens never touch the file.
 */

import * as Y from 'yjs'
import { existsSync, readFileSync, renameSync, writeFileSync } from 'fs'
import { join } from 'path'
import { app } from 'electron'
import { getActiveDoc } from './workspace-doc'
import {
  DEFAULT_WORKSPACE_ID,
  readWorkspaceServerBinding,
  workspacesDir,
} from './workspace-persistence'
import { scheduleWorkspaceAutosave } from './workspace-autosave'
import { adoptBinding, setSyncBinding, type SyncBinding } from './workspace-sync-state'

/**
 * Origin stamped on every remote transaction. A unique symbol cannot collide
 * with `'user'`, `null`, or the UndoManager instance, and is stable for the
 * life of the process — the two properties the observer guard and the undo
 * exclusion both rely on.
 */
export const REMOTE_SYNC_ORIGIN: unique symbol = Symbol('specular:remote-sync')

export {
  getSyncBinding,
  getSyncStatus,
  setSyncStatus,
  setSyncBinding,
  resetSyncState,
} from './workspace-sync-state'
export type { SyncBinding, SyncStatus } from './workspace-sync-state'

// ---------------------------------------------------------------------------
// Remote update ingress
// ---------------------------------------------------------------------------

/**
 * Apply a remote Yjs update to the active workspace doc. The transaction's
 * origin is `REMOTE_SYNC_ORIGIN`, so the afterTransaction observer patches the
 * runtime arrays (as it does for undo) and the UndoManager ignores it.
 */
export function applyRemoteUpdate(update: Uint8Array): void {
  Y.applyUpdate(getActiveDoc(), update, REMOTE_SYNC_ORIGIN)
}

// ---------------------------------------------------------------------------
// docId → workspace-path registry (userData/sync-registry.json)
//
// A `.canvas` copied to a new path carries the source docId. Sharing both to
// one Durable Object would silently cross-sync two workspaces, so the registry
// records which path owns each docId; a second path claiming a known docId is
// forked (its binding cleared) on load until it republishes.
// ---------------------------------------------------------------------------

const SYNC_REGISTRY_FILE = 'sync-registry.json'

export function syncRegistryPath(userDataPath: string): string {
  return join(userDataPath, SYNC_REGISTRY_FILE)
}

export function readSyncRegistry(userDataPath: string): Record<string, string> {
  const file = syncRegistryPath(userDataPath)
  if (!existsSync(file)) return {}
  try {
    const parsed = JSON.parse(readFileSync(file, 'utf8'))
    return parsed && typeof parsed === 'object' ? (parsed as Record<string, string>) : {}
  } catch {
    return {}
  }
}

function writeSyncRegistry(userDataPath: string, registry: Record<string, string>): void {
  const file = syncRegistryPath(userDataPath)
  const tmp = `${file}.tmp`
  writeFileSync(tmp, JSON.stringify(registry, null, 2), 'utf8')
  renameSync(tmp, file)
}

export function registerDoc(userDataPath: string, docId: string, workspacePath: string): void {
  const registry = readSyncRegistry(userDataPath)
  if (registry[docId] === workspacePath) return
  registry[docId] = workspacePath
  writeSyncRegistry(userDataPath, registry)
}

/**
 * Fork-on-duplicate guard, run on workspace load. If the loaded binding's
 * docId is already registered to a different path, drop the binding (the copy
 * becomes an unpublished fork until republished with a fresh docId). A
 * first-seen or same-path docId is registered and the binding kept.
 */
export function resolveBindingOnLoad(
  userDataPath: string,
  workspacePath: string,
  binding: SyncBinding | null,
): SyncBinding | null {
  if (!binding) return null
  const registry = readSyncRegistry(userDataPath)
  const owner = registry[binding.docId]
  if (owner && owner !== workspacePath) return null
  if (!owner) registerDoc(userDataPath, binding.docId, workspacePath)
  return binding
}

function workspacePathFor(userDataPath: string, workspaceId: string): string {
  return join(workspacesDir(userDataPath), workspaceId)
}

// ---------------------------------------------------------------------------
// Publish / clear / load — persist through the normal autosave path
// ---------------------------------------------------------------------------

/**
 * Publish (or rebind) the workspace to a cloud doc. Registers the docId, sets
 * the in-memory binding, and schedules the autosave that writes the
 * `specular.server` block into the `.canvas` files. The transport (later step)
 * attaches separately.
 */
export function publishBinding(binding: SyncBinding, workspaceId: string = DEFAULT_WORKSPACE_ID): void {
  const userDataPath = app.getPath('userData')
  registerDoc(userDataPath, binding.docId, workspacePathFor(userDataPath, workspaceId))
  adoptBinding(binding, 'connecting')
  scheduleWorkspaceAutosave()
}

/** Clear the binding (un-publish locally) and persist the removal. */
export function clearSyncBinding(): void {
  setSyncBinding(null)
  scheduleWorkspaceAutosave()
}

/**
 * Resolve and adopt the binding recorded in a workspace's `.canvas` files,
 * applying the fork guard. Called from the load path so both app boot and the
 * test harness pick the binding up.
 */
export function loadSyncBinding(userDataPath: string, workspaceId: string = DEFAULT_WORKSPACE_ID): void {
  const disk = readWorkspaceServerBinding(userDataPath, workspaceId)
  const resolved = resolveBindingOnLoad(userDataPath, workspacePathFor(userDataPath, workspaceId), disk)
  setSyncBinding(resolved)
}
