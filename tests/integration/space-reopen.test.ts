/**
 * Reopening a space after a change (ADR 0033 §2/§3) — the window-independent
 * core: `reloadWorkspaceDataFromCurrentSpace()` in
 * src/main/runtime/space-change.ts. It loads whatever `spaceDir()` currently
 * resolves to and clears anything left over from the previous space so
 * autosave never straddles two roots.
 *
 * The window-teardown half (`reopenAtCurrentSpace()`, via
 * `rebuildWindowFromSnapshot()` -> `initWindow()`) calls Electron machinery
 * (loadRenderer, screen listeners, real BrowserWindow construction) with no
 * in-process test double in this harness, so it's out of scope here — see
 * tests/README.md's boot-suite carve-out for view-geometry-adjacent code.
 *
 * Mutation-verified by: deleting the empty-space fallback's
 * `destroyActivePages()` call in `reloadWorkspaceDataFromCurrentSpace` —
 * "switching to an empty space replaces old-space pages instead of piling
 * default pages on top of them" fails because the old space's text entity
 * survives alongside the two DEFAULT_PAGES entries.
 */

import { mkdtempSync, rmSync } from 'fs'
import { join } from 'path'
import { tmpdir } from 'os'
import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { createTextEntity, getTextEntities } from '../../src/main/runtime/document-commands'
import { pages } from '../../src/main/runtime/runtime-context'
import { setSpacePath } from '../../src/main/runtime/preferences'
import { reloadWorkspaceDataFromCurrentSpace } from '../../src/main/runtime/space-change'
import { readCanvasFile } from '../../src/main/runtime/workspace-persistence'

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
