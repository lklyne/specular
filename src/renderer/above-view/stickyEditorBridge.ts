/**
 * Publishes the sticky note currently being edited so the formatting
 * popover (StickyNotePopover) can render its toggle state and dispatch
 * commands without threading callbacks through the selection popup table.
 *
 * Module-scope single slot: main owns `editingEntityId`, and only one
 * sticky can be in inline-edit mode at a time, so one slot suffices — no
 * per-entity map. `useSyncExternalStore` means only the popover re-renders
 * on cursor moves, not the whole editing card.
 */

import { useSyncExternalStore } from 'react'
import type { StateCommand } from '@codemirror/state'
import type { StickyFormatState } from '../shared/markdown/markdown-format-state'

export interface ActiveStickyEditor {
  entityId: string
  format: StickyFormatState
  exec: (command: StateCommand) => void
}

let current: ActiveStickyEditor | null = null
const listeners = new Set<() => void>()

export function publishStickyEditor(entry: ActiveStickyEditor | null): void {
  current = entry
  for (const listener of listeners) listener()
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

function getSnapshot(): ActiveStickyEditor | null {
  return current
}

export function useActiveStickyEditor(): ActiveStickyEditor | null {
  return useSyncExternalStore(subscribe, getSnapshot)
}
