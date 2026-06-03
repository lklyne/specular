/**
 * External-edit watcher for `.wireframe.json` assets (plan 3.5).
 *
 * Watches the workspace note directory; when a `.wireframe.json` file changes on
 * disk out-of-band (an agent `Write`, a git checkout), the change is debounced
 * and handed to `importExternalWireframeEdit`, which validates it and folds it
 * into the Y.Doc as one undoable transaction. Writes this process made (the
 * projection step) are recognized by content hash inside the import and ignored,
 * so the write → watch → write loop never closes.
 *
 * One directory watcher, not one per file: it survives entity create/delete and
 * resolves the target entity by path at event time.
 */

import { watch, type FSWatcher } from 'fs'
import { join } from 'path'
import { workspaceNoteDir } from './note-assets'
import { importExternalWireframeEdit } from './wireframe-commands'
import { isWireframeFilePath, seedAllWireframeBaselines } from './wireframe-content-state'

// Coalesce the burst of events a single save produces (tmp+rename writes fire
// more than once); short enough to feel live, long enough to settle.
const DEBOUNCE_MS = 80

let watcher: FSWatcher | null = null
const debounceTimers = new Map<string, ReturnType<typeof setTimeout>>()

export function startWireframeWatcher(): void {
  if (watcher) return

  let dir: string
  try {
    dir = workspaceNoteDir()
  } catch {
    return
  }

  // Hold every existing wireframe's at-rest tree in the store + Y.Doc before any
  // external edit can race in, so an import is a clean diff back to it.
  seedAllWireframeBaselines()

  try {
    watcher = watch(dir, (_event, filename) => {
      if (!filename) return
      const name = filename.toString()
      if (!isWireframeFilePath(name)) return
      const filePath = join(dir, name)
      const existing = debounceTimers.get(filePath)
      if (existing) clearTimeout(existing)
      debounceTimers.set(
        filePath,
        setTimeout(() => {
          debounceTimers.delete(filePath)
          try {
            importExternalWireframeEdit(filePath)
          } catch {
            // Transient read/parse race — the next event re-imports.
          }
        }, DEBOUNCE_MS),
      )
    })
    watcher.on('error', () => stopWireframeWatcher())
  } catch {
    watcher = null
  }
}

export function stopWireframeWatcher(): void {
  if (watcher) {
    watcher.close()
    watcher = null
  }
  for (const timer of debounceTimers.values()) clearTimeout(timer)
  debounceTimers.clear()
}
