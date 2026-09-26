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
 *    the starter canvas's own entities.
 *  - swapping `reopenSpaceAt`'s order to `setSpacePath()` then
 *    `flushSpaceAutosaveSync()` — "flushes the old root before re-pointing"
 *    fails because the pending mutation gets written into the new root
 *    instead of the old one (this is the exact regression a real user hit:
 *    "Start fresh here" wrote every old canvas into the new folder).
 *  - deleting `resetDocToCurrentSpace()` from
 *    `reloadWorkspaceDataFromCurrentSpace` — "leaves the Y.Doc holding the
 *    new space" fails because the doc still carries the previous space's
 *    entities and tab list.
 *  - dropping the `adoptUnreferencedCanvasFiles` merge in
 *    `loadSpaceFromCanvasFiles` — "opening a space picks up a .canvas file
 *    its index never listed" fails because the stranded canvas never becomes
 *    a tab and the next autosave writes the index over it (this is the
 *    regression that cost a real user their tab list: a space change left
 *    the runtime holding another space's tabs, and the next autosave wrote
 *    that list over the destination's index, orphaning 15 canvases that were
 *    still sitting on disk).
 */

import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync } from 'fs'
import { basename, join, resolve } from 'path'
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
import {
  canvasFilePath,
  readCanvasFile,
  readSpaceMeta,
  writeCanvasFileSync,
} from '../../src/main/runtime/space-persistence'
import { spaceTabs } from '../../src/main/runtime/space-model'
import { getDocTabList } from '../../src/main/runtime/space-doc'

/** An empty root is seeded with the starter canvas, so the content a fresh
 *  space comes up with is whatever that file holds. Counted from the file
 *  itself rather than hard-coded, so editing the starter can't silently
 *  invalidate the assertions below. */
function starterNodeCount(type: 'link' | 'text'): number {
  const doc = JSON.parse(
    readFileSync(resolve(__dirname, '../../resources/starter-space/Welcome.canvas'), 'utf8'),
  ) as { nodes: { type: string }[] }
  return doc.nodes.filter((node) => node.type === type).length
}

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
    expect(pages.length).toBe(starterNodeCount('link'))
  })

  it('leaves the Y.Doc holding the new space, so nothing can sync the old space back under the new root', async () => {
    // Space B, with content of its own to come back to.
    const spaceB = freshSpaceDir()
    reopenSpaceAt(spaceB, reloadWorkspaceDataFromCurrentSpace)
    const bEntity = createTextEntity({ canvasX: 0, canvasY: 0, text: 'B content' })
    await settleSync()
    harness.flush()
    const bTabIds = spaceTabs.map((tab) => tab.id)

    // Away to A, where the user does some work...
    const spaceA = freshSpaceDir()
    reopenSpaceAt(spaceA, reloadWorkspaceDataFromCurrentSpace)
    const aEntity = createTextEntity({ canvasX: 0, canvasY: 0, text: 'A content' })
    await settleSync()
    harness.flush()

    // ...and back into B. The doc must describe B, not A: while it held A,
    // any doc -> runtime path (undo, reverse sync) would reinstate A's
    // entities and tab list, and the next autosave would write them into B.
    reopenSpaceAt(spaceB, reloadWorkspaceDataFromCurrentSpace)
    const entities = harness.doc.getMap('entities')
    expect(entities.has(aEntity.id)).toBe(false)
    expect(entities.has(bEntity.id)).toBe(true)
    expect(getDocTabList(harness.doc).map((tab) => tab.id)).toEqual(bTabIds)

    // Belt and braces on the outcome that actually cost data: B's index on
    // disk still lists B's tabs after the switch settles.
    await settleSync()
    harness.flush()
    expect(readSpaceMeta(spaceB)?.tabs.map((tab) => tab.id)).toEqual(bTabIds)
  })

  it('opening a space picks up a .canvas file its index never listed, so the next save cannot orphan it', () => {
    const space = freshSpaceDir()
    reopenSpaceAt(space, reloadWorkspaceDataFromCurrentSpace)
    createTextEntity({ canvasX: 0, canvasY: 0, text: 'one tab' })
    harness.flush()

    // A canvas the index knows nothing about — the shape left behind by any
    // drift between the tab list and the space on disk. The user's loss came
    // from exactly this: a tab list belonging to another space, written over
    // an index whose canvases were all still on disk.
    const strandedName = 'Stranded-abcd.canvas'
    writeCanvasFileSync(join(space, strandedName), { nodes: [], edges: [] })

    // Opening the space is where the index is reconciled with the folder.
    reopenSpaceAt(space, reloadWorkspaceDataFromCurrentSpace)
    expect(spaceTabs.map((tab) => tab.name)).toContain('Stranded')

    createTextEntity({ canvasX: 5, canvasY: 5, text: 'later edit' })
    harness.flush()

    const meta = readSpaceMeta(space)
    const files = readdirSync(space).filter((f) => f.endsWith('.canvas'))
    const referenced = new Set(
      (meta?.tabs ?? []).map((tab) => basename(canvasFilePath(space, tab))),
    )
    expect(files.length).toBeGreaterThan(1)
    for (const file of files) expect(referenced.has(file)).toBe(true)
    expect(existsSync(join(space, strandedName))).toBe(true)
  })

  it('switching to an empty space replaces old-space pages instead of piling default pages on top of them', () => {
    createTextEntity({ canvasX: 0, canvasY: 0, text: 'old space text' })
    expect(getTextEntities().length).toBe(1)

    setSpacePath(freshSpaceDir())
    reloadWorkspaceDataFromCurrentSpace()

    // Empty new space -> comes up with the starter canvas, and the old
    // space's text entity is replaced rather than piled on top of.
    expect(getTextEntities().some((entity) => entity.text === 'old space text')).toBe(false)
    expect(getTextEntities().length).toBe(starterNodeCount('text'))
    expect(pages.length).toBe(starterNodeCount('link'))
  })
})
