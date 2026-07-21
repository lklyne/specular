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
