/**
 * Duplicate-tab-name file-identity integration tests.
 *
 * Covers the fix in src/main/runtime/workspace-persistence.ts +
 * workspace-tab-operations.ts for #361: `.canvas` file paths used to be
 * re-derived from the tab *name* on every save/load, so two tabs sharing a
 * name silently shared one file (last writer wins, rename/delete of one
 * clobbered the other). Each tab now carries an explicit `file` field that
 * is the authoritative id -> file mapping; same-named tabs are suffixed
 * (`scratch.canvas`, `scratch-2.canvas`) instead of colliding.
 *
 * Mutation-verified by commenting out `assignWorkspaceTabFiles(params.workspaceTabs)`
 * in `buildPersistedWorkspaceRecord` (workspace-persistence.ts) — "two
 * same-named tabs persist to distinct files" fails because both tabs
 * resolve to `scratch.canvas` and the second tab's flush overwrites the
 * first tab's content.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { createTextEntity } from '../../src/main/runtime/document-commands'
import {
  createWorkspaceTab,
  deleteWorkspaceTab,
  renameWorkspaceTab,
} from '../../src/main/runtime/workspace-tab-operations'
import {
  DEFAULT_WORKSPACE_ID,
  canvasFilePathForTab,
  loadWorkspaceFromCanvasFiles,
  readCanvasFile,
  readWorkspaceMeta,
} from '../../src/main/runtime/workspace-persistence'
import type { JsonCanvasDocument } from '../../src/shared/json-canvas-types'

let harness: WorkspaceHarness

function hasTextId(doc: JsonCanvasDocument | null, id: string): boolean {
  return (doc?.nodes ?? []).some((n) => n.id === id)
}

describe('duplicate tab names get distinct .canvas files', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('two same-named tabs persist to distinct .canvas files and round-trip with distinct content', async () => {
    const tabAId = createWorkspaceTab('scratch')
    const entityA = createTextEntity({ canvasX: 0, canvasY: 0, text: 'tab-a-content' })
    await settleSync()

    const tabBId = createWorkspaceTab('scratch')
    const entityB = createTextEntity({ canvasX: 0, canvasY: 0, text: 'tab-b-content' })
    await settleSync()

    harness.flush()

    const meta = readWorkspaceMeta(harness.userDataPath, DEFAULT_WORKSPACE_ID)
    const tabAMeta = meta?.tabs.find((t) => t.id === tabAId)
    const tabBMeta = meta?.tabs.find((t) => t.id === tabBId)
    expect(tabAMeta?.file).toBeTruthy()
    expect(tabBMeta?.file).toBeTruthy()
    expect(tabAMeta?.file).not.toBe(tabBMeta?.file)

    const docA = readCanvasFile(canvasFilePathForTab(harness.userDataPath, DEFAULT_WORKSPACE_ID, tabAMeta!))
    const docB = readCanvasFile(canvasFilePathForTab(harness.userDataPath, DEFAULT_WORKSPACE_ID, tabBMeta!))
    expect(hasTextId(docA, entityA.id)).toBe(true)
    expect(hasTextId(docA, entityB.id)).toBe(false)
    expect(hasTextId(docB, entityB.id)).toBe(true)
    expect(hasTextId(docB, entityA.id)).toBe(false)

    // Round-trip through a fresh load, as if the app relaunched.
    const reloaded = loadWorkspaceFromCanvasFiles(harness.userDataPath, DEFAULT_WORKSPACE_ID)
    const reloadedA = reloaded?.tabs.find((t) => t.id === tabAId)
    const reloadedB = reloaded?.tabs.find((t) => t.id === tabBId)
    expect(reloadedA?.snapshot.entities?.[entityA.id]).toBeTruthy()
    expect(reloadedA?.snapshot.entities?.[entityB.id]).toBeFalsy()
    expect(reloadedB?.snapshot.entities?.[entityB.id]).toBeTruthy()
    expect(reloadedB?.snapshot.entities?.[entityA.id]).toBeFalsy()
  })

  it('renaming one of two same-named tabs never deletes or rewrites the other file', async () => {
    const tabAId = createWorkspaceTab('scratch')
    const entityA = createTextEntity({ canvasX: 0, canvasY: 0, text: 'tab-a-content' })
    await settleSync()

    const tabBId = createWorkspaceTab('scratch')
    createTextEntity({ canvasX: 0, canvasY: 0, text: 'tab-b-content' })
    await settleSync()
    harness.flush()

    const metaBefore = readWorkspaceMeta(harness.userDataPath, DEFAULT_WORKSPACE_ID)
    const tabAMeta = metaBefore!.tabs.find((t) => t.id === tabAId)!
    const tabAPath = canvasFilePathForTab(harness.userDataPath, DEFAULT_WORKSPACE_ID, tabAMeta)

    renameWorkspaceTab(tabBId, 'scratch-renamed')

    // The old-file cleanup on rename runs synchronously, before the next
    // autosave flush would otherwise re-heal a wrongly-deleted sibling file —
    // assert here, not after a flush, so a wrong (name-based) deletion is
    // actually caught instead of being masked by the next full save.
    expect(hasTextId(readCanvasFile(tabAPath), entityA.id)).toBe(true)

    harness.flush()
    expect(hasTextId(readCanvasFile(tabAPath), entityA.id)).toBe(true)

    const metaAfter = readWorkspaceMeta(harness.userDataPath, DEFAULT_WORKSPACE_ID)
    const tabAMetaAfter = metaAfter!.tabs.find((t) => t.id === tabAId)!
    expect(tabAMetaAfter.file).toBe(tabAMeta.file)
  })

  it('deleting one of two same-named tabs leaves the other file intact', async () => {
    const tabAId = createWorkspaceTab('scratch')
    const entityA = createTextEntity({ canvasX: 0, canvasY: 0, text: 'tab-a-content' })
    await settleSync()

    const tabBId = createWorkspaceTab('scratch')
    createTextEntity({ canvasX: 0, canvasY: 0, text: 'tab-b-content' })
    await settleSync()
    harness.flush()

    const metaBefore = readWorkspaceMeta(harness.userDataPath, DEFAULT_WORKSPACE_ID)
    const tabAMeta = metaBefore!.tabs.find((t) => t.id === tabAId)!
    const tabBMeta = metaBefore!.tabs.find((t) => t.id === tabBId)!
    const tabAPath = canvasFilePathForTab(harness.userDataPath, DEFAULT_WORKSPACE_ID, tabAMeta)
    const tabBPath = canvasFilePathForTab(harness.userDataPath, DEFAULT_WORKSPACE_ID, tabBMeta)

    deleteWorkspaceTab(tabBId)

    // Assert before the next flush — a wrong (name-based) deletion of tab
    // A's file would otherwise be masked by the next full save rewriting it.
    expect(hasTextId(readCanvasFile(tabAPath), entityA.id)).toBe(true)

    harness.flush()
    expect(hasTextId(readCanvasFile(tabAPath), entityA.id)).toBe(true)
    expect(readCanvasFile(tabBPath)).toBeNull()
  })

  it('existing workspaces without a file field load unchanged and backfill file on next save', async () => {
    const metaBeforeSave = readWorkspaceMeta(harness.userDataPath, DEFAULT_WORKSPACE_ID)
    expect(metaBeforeSave?.tabs[0]?.file).toBeUndefined()

    const entity = createTextEntity({ canvasX: 0, canvasY: 0, text: 'legacy tab content' })
    await settleSync()
    harness.flush()

    const metaAfterSave = readWorkspaceMeta(harness.userDataPath, DEFAULT_WORKSPACE_ID)
    const tabMeta = metaAfterSave!.tabs[0]
    expect(tabMeta.file).toBe('Blank.canvas')
    expect(hasTextId(readCanvasFile(canvasFilePathForTab(harness.userDataPath, DEFAULT_WORKSPACE_ID, tabMeta)), entity.id)).toBe(true)
  })
})
