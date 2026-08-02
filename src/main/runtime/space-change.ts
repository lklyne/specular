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
import { setSpacePath } from './preferences'
import { spaceDir } from './space-dir'
import { hasCanvasFiles, migrateSpace } from './space-migration'
import { requestLayout } from './viewport-control'
import { flushSpaceAutosaveSync, loadSpace } from './space-autosave'
import {
  setActiveSpaceTabId,
  workspaceAnnotations,
  workspaceEdges,
  workspaceGroups,
  spaceTabs,
} from './space-model'
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
  })
  if (picked.canceled || !picked.filePaths.length) return null
  const destination = picked.filePaths[0]
  if (destination === currentSpace) return null

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
      migrateSpace(currentSpace, destination)
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
  setSpacePath(destination)
  reopenAtCurrentSpace()
  return spaceDir()
}

/**
 * Tear down the window and reload from whatever `spaceDir()` currently
 * resolves to. Shared by the picker flow above and the boot recovery flow
 * (§4), which sets the space before the first window ever opens and so has
 * no window to reopen — callers on that path skip this and let `initWindow()`
 * pick up the resolved root normally.
 */
export function reopenAtCurrentSpace(): void {
  flushSpaceAutosaveSync()

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
  clearUndoHistory()
  requestLayout()
}
