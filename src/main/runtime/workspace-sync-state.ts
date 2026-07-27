/**
 * Pure in-memory cloud-sync state: the workspace's binding to its cloud doc
 * and the connection status. Kept free of electron/fs imports so consumers
 * that only need to *read* the binding (e.g. scene building's asset
 * resolution) stay loadable outside the electron process. Persistence,
 * registry, and publish/load flows live in `workspace-sync.ts`.
 */

export type SyncStatus = 'off' | 'connecting' | 'connected' | 'error'

export interface SyncBinding {
  docId: string
  url: string
}

interface SyncState {
  binding: SyncBinding | null
  status: SyncStatus
}

const state: SyncState = { binding: null, status: 'off' }

export function getSyncBinding(): SyncBinding | null {
  return state.binding
}

export function getSyncStatus(): SyncStatus {
  return state.status
}

export function setSyncStatus(status: SyncStatus): void {
  state.status = status
}

/** Set the in-memory binding without persisting (used by the load path). */
export function setSyncBinding(binding: SyncBinding | null): void {
  state.binding = binding
  if (!binding) state.status = 'off'
}

/** Reset the in-memory sync state (workspace unload / test isolation). */
export function resetSyncState(): void {
  state.binding = null
  state.status = 'off'
}

/** Direct mutation door for the publish flow in `workspace-sync.ts`. */
export function adoptBinding(binding: SyncBinding, status: SyncStatus): void {
  state.binding = binding
  state.status = status
}

// ---------------------------------------------------------------------------
// Remote-origin registry
//
// The network transport (`workspace-sync-transport.ts`) applies remote Yjs
// updates with its provider instance as the transaction origin — not the
// static `REMOTE_SYNC_ORIGIN` symbol that `applyRemoteUpdate` uses. The
// Y.Doc→runtime observer must treat both as "remote" so remote work patches
// the runtime arrays; the UndoManager (tracked origins {null,'user'}) ignores
// both automatically, since a provider instance is neither. Kept here — pure,
// electron-free — so the observer can consult it without importing transport.
// ---------------------------------------------------------------------------

const remoteOrigins = new Set<unknown>()

export function registerRemoteOrigin(origin: unknown): void {
  remoteOrigins.add(origin)
}

export function unregisterRemoteOrigin(origin: unknown): void {
  remoteOrigins.delete(origin)
}

export function isRemoteOrigin(origin: unknown): boolean {
  return remoteOrigins.has(origin)
}
