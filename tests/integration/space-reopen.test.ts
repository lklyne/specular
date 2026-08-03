/**
 * Reopening a space after a change (ADR 0033 §2/§3) — the window-independent
 * core in src/main/runtime/space-change.ts:
 *
 *  - `reloadWorkspaceDataFromCurrentSpace()` loads whatever `spaceDir()`
 *    currently resolves to and clears anything left over from the previous
 *    space so autosave never straddles two roots.
 *  - `reopenSpaceAt(destination, reload, opts)` owns the ordering around
 *    that: flush the OLD root (spaceDir() must still resolve there),
 *    suspend persistence, re-point spaceDir() via setSpacePath(), run
 *    `reload`, resume. `changeSpaceTo()` (the production entry point) is
 *    this function with `reload = reopenAtCurrentSpace`; tests below pass
 *    `reloadWorkspaceDataFromCurrentSpace` directly to exercise the same
 *    ordering without a window.
 *
 * The window-teardown half (`reopenAtCurrentSpace()`, via
 * `rebuildWindowFromSnapshot()` -> `initWindow()`) calls Electron machinery
 * (loadRenderer, screen listeners, real BrowserWindow construction) with no
 * in-process test double in this harness, so it's out of scope here — see
 * tests/README.md's boot-suite carve-out for view-geometry-adjacent code.
 *
 * Mutation-verified by:
 *  - deleting the empty-space fallback's `destroyActivePages()` call in
 *    `reloadWorkspaceDataFromCurrentSpace` — "switching to an empty space
 *    replaces old-space pages instead of piling default pages on top of
 *    them" fails because the old space's text entity survives alongside
 *    the two DEFAULT_PAGES entries.
 *  - swapping `reopenSpaceAt`'s order to `setSpacePath()` then
 *    `flushSpaceAutosaveSync()` — "flushes the old root before re-pointing"
 *    fails because the pending mutation gets written into the new root
 *    instead of the old one (this is the exact regression a real user hit:
 *    "Start fresh here" wrote every old canvas into the new folder).
 */

import { mkdtempSync, readdirSync, rmSync } from 'fs'
import { join } from 'path'
import { tmpdir } from 'os'
import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { createTextEntity, getTextEntities } from '../../src/main/runtime/document-commands'
import { pages } from '../../src/main/runtime/runtime-context'
import { setSpacePath } from '../../src/main/runtime/preferences'
import { spaceDir } from '../../src/main/runtime/space-dir'
import {
  reloadWorkspaceDataFromCurrentSpace,
  reopenSpaceAt,
} from '../../src/main/runtime/space-change'
import { readCanvasFile } from '../../src/main/runtime/space-persistence'

let harness: WorkspaceHarness
const chosenDirs: string[] = []

function freshSpaceDir(): string {
  const dir = mkdtempSync(join(tmpdir(), 'specular-reopen-'))
  chosenDirs.push(dir)
  return dir
}

describe('reopening a space', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    setSpacePath(undefined)
    harness.reset()
  })

  afterAll(() => {
    harness?.dispose()
    for (const dir of chosenDirs) rmSync(dir, { recursive: true, force: true })
  })

  it('flushing then switching spaces leaves the old root file untouched and writes new mutations only to the new root', async () => {
    const oldEntity = createTextEntity({ canvasX: 0, canvasY: 0, text: 'in the old space' })
    await settleSync()
    harness.flush()
    const oldPath = harness.diskPath()
    const oldDocBefore = readCanvasFile(oldPath)
    expect(oldDocBefore?.nodes.some((n) => n.id === oldEntity.id)).toBe(true)

    const newSpace = freshSpaceDir()
    setSpacePath(newSpace)
    reloadWorkspaceDataFromCurrentSpace()

    const newEntity = createTextEntity({ canvasX: 10, canvasY: 10, text: 'in the new space' })
    await settleSync()
    harness.flush()

    // The old file on disk is exactly what it was before the switch — the
    // new-space mutation never landed there.
    expect(readCanvasFile(oldPath)).toEqual(oldDocBefore)

    // The new mutation landed under the new space's root.
    const newPath = harness.diskPath()
    expect(newPath.startsWith(newSpace)).toBe(true)
    const newDoc = readCanvasFile(newPath)
    expect(newDoc?.nodes.some((n) => n.id === newEntity.id)).toBe(true)
    expect(newDoc?.nodes.some((n) => n.id === oldEntity.id)).toBe(false)
  })

  it('reopenSpaceAt flushes the old root before re-pointing spaceDir(), so a pending mutation lands in the old root, not the new one', async () => {
    const oldSpaceDir = spaceDir() // legacy default — fixed for this test run, unlike the random new-space temp dir
    const oldEntity = createTextEntity({ canvasX: 0, canvasY: 0, text: 'unflushed old content' })
    await settleSync()
    // Deliberately no harness.flush() here — the 350ms debounce is still
    // pending. If reopenSpaceAt flushed AFTER setSpacePath, this mutation
    // would be written into the new root's directory instead of the old
    // one, because spaceDir() would already resolve to the new folder.

    const newSpace = freshSpaceDir()
    reopenSpaceAt(newSpace, reloadWorkspaceDataFromCurrentSpace)

    // Old root: the flush ordered before setSpacePath wrote the pending
    // mutation there, into the existing per-tab .canvas file.
    const oldCanvasFiles = readdirSync(oldSpaceDir).filter((f) => f.endsWith('.canvas'))
    expect(oldCanvasFiles.length).toBeGreaterThan(0)
    const oldDoc = readCanvasFile(join(oldSpaceDir, oldCanvasFiles[0]))
    expect(oldDoc?.nodes.some((n) => n.id === oldEntity.id)).toBe(true)

    // New root: nothing landed there before reloadWorkspaceDataFromCurrentSpace
    // hydrated it — the old space's content never leaked across.
    const newCanvasFilesBeforeAutosave = readdirSync(newSpace).filter((f) => f.endsWith('.canvas'))
    expect(newCanvasFilesBeforeAutosave).toEqual([])

    // Once the new (empty -> default-pages) state autosaves, the new root
    // gets only that content — never the old entity.
    harness.flush()
    const newDoc = readCanvasFile(harness.diskPath())
    expect(newDoc?.nodes.some((n) => n.id === oldEntity.id)).toBe(false)
    expect(pages.length).toBe(2)
  })

  it('switching to an empty space replaces old-space pages instead of piling default pages on top of them', () => {
    createTextEntity({ canvasX: 0, canvasY: 0, text: 'old space text' })
    expect(getTextEntities().length).toBe(1)

    setSpacePath(freshSpaceDir())
    reloadWorkspaceDataFromCurrentSpace()

    // Empty new space -> falls back to the DEFAULT_PAGES starter set, with
    // no trace of the old space's text entity.
    expect(getTextEntities().length).toBe(0)
    expect(pages.length).toBe(2)
  })
})
