/**
 * The §3 "change space" flow (ADR 0033): pick a folder, ask about migration
 * when it's warranted, then reopen the window against the new root.
 *
 * Changing a space is a reopen, not a live re-point (§2) — the workspace
 * tears down and reloads from the new root so nothing mutates under a live
 * Y.Doc, open pages, or an in-flight autosave.
 */

import { type BrowserWindow, dialog } from 'electron'
import { mkdirSync } from 'fs'
import { basename } from 'path'
import { DEFAULT_PAGES } from '../../shared/constants'
import { createPage } from './page-factory'
import { spacePickerDefaultPath } from './picker-defaults'
import { setSpacePath } from './preferences'
import { spaceDir } from './space-dir'
import { hasCanvasFiles, migrateSpace } from './space-migration'
import { requestLayout } from './viewport-control'
import { flushSpaceAutosaveSync, loadSpace, withSpacePersistenceSuspended } from './space-autosave'
import {
  activeSpaceTabId,
  setActiveSpaceTabId,
  workspaceAnnotations,
  workspaceEdges,
  workspaceGroups,
  spaceTabs,
} from './space-model'
import {
  DOC_ALL_MAP_NAMES,
  getActiveDoc,
  rewriteDocToSnapshot,
  withSuppressedDocSync,
} from './space-doc'
import { resetDocSync } from './space-observers'
import { makeEmptyWorkspaceSnapshot } from './space-persistence'
import { destroyActivePages, restorePersistedSpace, rebuildWindowFromSnapshot } from './space-restore'
import { clearUndoHistory } from './space-undo'

/**
 * Drives the folder picker and the migration prompts, then persists and
 * reopens. Returns the new resolved space path, or null if the user
 * canceled at any step (picker or a dialog's Cancel button).
 */
export async function changeSpaceViaPicker(win: BrowserWindow): Promise<string | null> {
  const currentSpace = spaceDir()

  const picked = await dialog.showOpenDialog(win, {
    title: 'Choose a space folder',
    properties: ['openDirectory', 'createDirectory'],
    defaultPath: spacePickerDefaultPath(),
  })
  if (picked.canceled || !picked.filePaths.length) return null
  const destination = picked.filePaths[0]
  if (destination === currentSpace) return null

  // "Move my canvases" already flushes and migrates the old root's files
  // below; every other path (opening an existing space, starting fresh)
  // still has unflushed in-memory edits sitting against `currentSpace`, so
  // the switch below must flush there first.
  let alreadyFlushedOldRoot = false

  if (hasCanvasFiles(destination)) {
    // Someone re-opening an existing space — not a migration question.
    // Merging two populated spaces is out of scope (ADR 0033 §3).
    const { response } = await dialog.showMessageBox(win, {
      type: 'question',
      buttons: ['Open this space', 'Cancel'],
      defaultId: 0,
      cancelId: 1,
      message: 'Open this space?',
      detail: `"${basename(destination)}" already has canvases in it. Specular will open it as-is.`,
    })
    if (response !== 0) return null
  } else if (hasCanvasFiles(currentSpace)) {
    const { response } = await dialog.showMessageBox(win, {
      type: 'question',
      buttons: ['Move my canvases', 'Start fresh here', 'Cancel'],
      defaultId: 0,
      cancelId: 2,
      message: 'Move your canvases to the new folder?',
      detail: `Your current space, "${currentSpace}", has canvases in it.`,
    })
    if (response === 2) return null
    if (response === 0) {
      // Flush first so the files migrateSpace copies reflect the latest
      // in-memory state, not whatever last hit disk on the debounce.
      // migrateSpace() then deletes the originals from currentSpace — the
      // switch below must NOT flush again before setSpacePath, or it would
      // write the (still-old-space) runtime state straight back into the
      // now-empty currentSpace it just cleaned up.
      flushSpaceAutosaveSync()
      migrateSpace(currentSpace, destination)
      alreadyFlushedOldRoot = true
    } else {
      mkdirSync(destination, { recursive: true })
      await dialog.showMessageBox(win, {
        type: 'info',
        buttons: ['OK'],
        message: 'Starting fresh',
        detail: `Your existing canvases are still at "${currentSpace}".`,
      })
    }
  }

  mkdirSync(destination, { recursive: true })
  changeSpaceTo(destination, { flushOldRoot: !alreadyFlushedOldRoot })
  return spaceDir()
}

/**
 * Re-point `spaceDir()` at `destination` and reopen the window against it.
 *
 * Ordering matters: `spaceDir()` is a resolver, not cached state, so the
 * instant `setSpacePath()` runs, every persistence call — including any
 * autosave that fires mid-teardown while the runtime arrays still hold the
 * *old* space's content — starts writing into the *new* root. Flushing
 * against the old root must happen strictly before `setSpacePath()`, and
 * persistence must stay suspended from `setSpacePath()` until the new
 * root's content has been hydrated, so nothing in between can land
 * old-space state in the new folder.
 */
export function changeSpaceTo(destination: string, opts: { flushOldRoot?: boolean } = {}): void {
  reopenSpaceAt(destination, reopenAtCurrentSpace, opts)
}

/**
 * The ordering both `changeSpaceTo()` and the reopen tests exercise: flush
 * the old root (unless the caller already did, e.g. right after a
 * migration that deleted the old root's originals), suspend persistence,
 * re-point `spaceDir()`, run `reload` (hydrate the new root's content —
 * with or without a window rebuild), then resume. Factored out so tests can
 * cover the exact flush/suspend/setSpacePath/reload sequence production
 * code runs, swapping in the window-independent `reload` callback instead
 * of the Electron-dependent window rebuild.
 */
export function reopenSpaceAt(
  destination: string,
  reload: () => void,
  opts: { flushOldRoot?: boolean } = {},
): void {
  if (opts.flushOldRoot !== false) flushSpaceAutosaveSync()
  withSpacePersistenceSuspended(() => {
    setSpacePath(destination)
    reload()
  })
}

/**
 * Tear down the window and reload from whatever `spaceDir()` currently
 * resolves to. Shared by the picker flow above and the boot recovery flow
 * (§4), which sets the space before the first window ever opens and so has
 * no window to reopen — callers on that path skip this and let `initWindow()`
 * pick up the resolved root normally.
 *
 * Does not flush or suspend itself — `reopenSpaceAt()` above owns that
 * ordering, since the flush must happen before `setSpacePath()` re-points
 * `spaceDir()` and this function only ever runs after that.
 */
export function reopenAtCurrentSpace(): void {
  // rebuildWindowFromSnapshot tears down and rebuilds the window shell
  // (views, timers, ui-state, layout cache) the same way the dev-only
  // reload-app IPC does. An empty snapshot is intentional: real content for
  // the new root comes from reloadWorkspaceDataFromCurrentSpace below, not
  // from this rebuild step.
  rebuildWindowFromSnapshot(
    makeEmptyWorkspaceSnapshot({
      leftSidebarOpen: true,
      devtoolsPanelTab: 'inspect',
      devtoolsWidth: 400,
    }),
  )

  reloadWorkspaceDataFromCurrentSpace()
}

/**
 * The window-independent half of a reopen: load whatever `spaceDir()`
 * currently resolves to into the runtime arrays, falling back to the
 * default starter pages when the new root is empty (mirrors the boot
 * fallback in src/main/index.ts), and reset the undo stack so it doesn't
 * carry entries from the old space into the new one. Split out from
 * `reopenAtCurrentSpace()` so it's testable without the window-teardown
 * half, which depends on Electron machinery (loadRenderer, screen, …) that
 * has no in-process test double.
 */
export function reloadWorkspaceDataFromCurrentSpace(): void {
  const record = loadSpace()
  const restored = record ? restorePersistedSpace(record) : false
  if (!restored) {
    // restorePersistedSpace clears the old space's pages/entities/tabs
    // as part of hydrating the new ones; the empty-space fallback has no
    // such hydration step, so it must clear them itself before seeding the
    // starter pages — otherwise they'd sit alongside the old space's content
    // instead of replacing it.
    destroyActivePages()
    workspaceGroups.length = 0
    workspaceEdges.length = 0
    workspaceAnnotations.length = 0
    spaceTabs.length = 0
    setActiveSpaceTabId(null)
    for (const cfg of DEFAULT_PAGES) {
      createPage(cfg)
    }
  }
  // Doc observers and the undo manager are already wired from app boot and
  // operate on the same module-level arrays rebuildWindowFromSnapshot just
  // reset — no need to recreate them.
  //
  // The Y.Doc, though, still holds the *previous* space: unlike a tab switch
  // (`transitionToTab`), `restorePersistedSpace` rebuilds the runtime arrays
  // without touching the doc. Leaving it stale points the two layers at
  // different spaces, and every doc -> runtime path — an undo, a reverse
  // sync — then reinstates the old space's entities and tab list under the
  // new root, where autosave writes them out as if they belonged to it.
  resetDocToCurrentSpace()
  clearUndoHistory()
  requestLayout()
}

/**
 * Rewrite the Y.Doc to match the freshly-loaded space: the active tab's
 * content, the new tab list, and no trace of the space we came from. Runs
 * untracked — this is a reopen, not a user edit, and `clearUndoHistory()`
 * follows it anyway.
 */
function resetDocToCurrentSpace(): void {
  const doc = getActiveDoc()
  const activeTab = spaceTabs.find((tab) => tab.id === activeSpaceTabId)
  withSuppressedDocSync(() => {
    rewriteDocToSnapshot(doc, {
      mapNames: DOC_ALL_MAP_NAMES,
      origin: 'space-reopen',
      tab: activeTab ? { id: activeTab.id, snapshot: activeTab.snapshot } : null,
      tabs: spaceTabs.map((tab) => ({ id: tab.id, name: tab.name })),
    })
  })
  // Drops the previous space's note mirror, which is keyed by entity id and
  // would otherwise be projected back to disk under the new root.
  resetDocSync()
}
