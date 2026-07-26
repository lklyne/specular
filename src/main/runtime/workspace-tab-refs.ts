// ---------------------------------------------------------------------------
// Tab reference resolution
// ---------------------------------------------------------------------------
// A "ref" is whatever an agent typed to name a canvas: a tab id or an exact
// tab name. One resolver, main-side, so every surface that targets a tab
// (`tab switch`, `--tab`) agrees on what a ref means and errors identically.

import type { PersistedWorkspaceTab } from '../../shared/types'
import { workspaceTabs } from './workspace-model'

export type ResolvedWorkspaceTabRef =
  | { ok: true; tab: PersistedWorkspaceTab }
  | { ok: false; error: string }

function describeAvailableTabs(): string {
  return workspaceTabs.map((tab) => `${tab.id} (${tab.name})`).join(', ')
}

/**
 * Resolve a tab ref: id exact match first, then exact name (post-trim).
 *
 * Never guesses. An ambiguous name lists the matching ids and an unknown ref
 * lists every tab, so a caller can retry without another round-trip.
 */
export function resolveWorkspaceTabRef(ref: string): ResolvedWorkspaceTabRef {
  const trimmed = ref.trim()
  if (!trimmed) return { ok: false, error: 'a tab ref (id or name) is required' }

  const byId = workspaceTabs.find((tab) => tab.id === trimmed)
  if (byId) return { ok: true, tab: byId }

  const byName = workspaceTabs.filter((tab) => tab.name.trim() === trimmed)
  if (byName.length === 1) return { ok: true, tab: byName[0] }
  if (byName.length > 1) {
    return {
      ok: false,
      error: `tab name '${trimmed}' matches ${byName.length} tabs: ${byName
        .map((tab) => tab.id)
        .join(', ')} — use an id`,
    }
  }
  return { ok: false, error: `unknown tab '${trimmed}' — available: ${describeAvailableTabs()}` }
}
