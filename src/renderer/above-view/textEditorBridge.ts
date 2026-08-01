/**
 * Publishes whichever text-bearing entity (sticky note or .md file) is
 * currently being edited so its selection popup can render formatting
 * toggle state and dispatch commands without threading callbacks through
 * the selection popup table.
 *
 * Module-scope single slot: main owns `editingEntityId`, and only one
 * entity can be in inline-edit mode at a time, so one slot suffices — no
 * per-entity map. `useSyncExternalStore` means only the popover re-renders
 * on cursor moves, not the whole editing card.
 */

import { useSyncExternalStore } from 'react'
import type { StateCommand } from '@codemirror/state'
import type { StickyFormatState } from '../shared/markdown/markdown-format-state'

export interface ActiveTextEditor {
  entityId: string
  format: StickyFormatState
  exec: (command: StateCommand) => void
}

let current: ActiveTextEditor | null = null
const listeners = new Set<() => void>()

export function publishActiveTextEditor(entry: ActiveTextEditor | null): void {
  current = entry
  for (const listener of listeners) listener()
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

function getSnapshot(): ActiveTextEditor | null {
  return current
}

export function useActiveTextEditor(): ActiveTextEditor | null {
  return useSyncExternalStore(subscribe, getSnapshot)
}
