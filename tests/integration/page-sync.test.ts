/**
 * Page sync sets against the real runtime, in-process.
 *
 * A sync set is a shared `syncId` on 2+ pages (independent of groups). Guards:
 * `setSyncForSelection` mints one id across the selection, toggle-merges to
 * unsync when the whole selection already shares a set, auto-dissolves sets
 * that drop below two members, persists `syncId` to the .canvas link node, and
 * round-trips through undo as one step.
 *
 * Mutation-verified by: making `setSyncForSelection` a no-op (early return in
 * src/main/navigation-sync.ts) — "syncs a two-page selection" fails because
 * neither page gains a syncId; and by dropping `syncId` from `persistPage`
 * (src/main/runtime/page-doc-projection.ts) — "persists syncId to disk" fails.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { applyCanvasPatch } from '../../src/main/canvas-apply'
import { findPageById } from '../../src/main/runtime/runtime-context'
import {
  setSyncForSelection,
  unsyncPage,
  isPageSynced,
} from '../../src/main/navigation-sync'
import { undo, redo } from '../../src/main/runtime/workspace-undo'
import { DOC_MAP_PAGES } from '../../src/main/runtime/workspace-doc'

let harness: WorkspaceHarness

function createPage(x: number): string {
  const result = applyCanvasPatch({
    entities: [{ kind: 'page', url: 'https://example.com', canvasX: x, canvasY: 0, presetIndex: 2 }],
  })
  return result.created[0]
}

function syncIdOf(id: string): string | null {
  return findPageById(id)?.syncId ?? null
}

describe('page sync sets', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('syncs a two-page selection into one set', async () => {
    const a = createPage(0)
    const b = createPage(500)
    await settleSync()

    setSyncForSelection([a, b])
    await settleSync()

    const idA = syncIdOf(a)
    expect(idA).not.toBeNull()
    expect(syncIdOf(b)).toBe(idA)
    expect(isPageSynced(findPageById(a)!)).toBe(true)
  })

  it('toggle-merges a fully-synced selection back to unsynced', async () => {
    const a = createPage(0)
    const b = createPage(500)
    await settleSync()

    setSyncForSelection([a, b])
    await settleSync()
    setSyncForSelection([a, b])
    await settleSync()

    expect(syncIdOf(a)).toBeNull()
    expect(syncIdOf(b)).toBeNull()
  })

  it('is a no-op for a selection under two pages', async () => {
    const a = createPage(0)
    await settleSync()

    setSyncForSelection([a])
    await settleSync()

    expect(syncIdOf(a)).toBeNull()
  })

  it('auto-dissolves a set that drops below two members', async () => {
    const a = createPage(0)
    const b = createPage(500)
    const c = createPage(1000)
    await settleSync()

    setSyncForSelection([a, b])
    await settleSync()
    // Pull `a` into a new set with `c`; `b` is now alone and must dissolve.
    setSyncForSelection([a, c])
    await settleSync()

    expect(syncIdOf(b)).toBeNull()
    expect(syncIdOf(a)).toBe(syncIdOf(c))
    expect(syncIdOf(a)).not.toBeNull()
  })

  it('unsyncs a single page and dissolves its now-lone peer', async () => {
    const a = createPage(0)
    const b = createPage(500)
    await settleSync()

    setSyncForSelection([a, b])
    await settleSync()
    // Unsync `a` from the single-select popup; `b` is now alone and must dissolve.
    unsyncPage(a)
    await settleSync()

    expect(syncIdOf(a)).toBeNull()
    expect(syncIdOf(b)).toBeNull()
  })

  it('persists syncId to disk and round-trips through undo', async () => {
    const a = createPage(0)
    const b = createPage(500)
    await settleSync()

    setSyncForSelection([a, b])
    await settleSync()

    const disk = harness.diskDoc()
    const nodeA = disk!.nodes.find((n) => n.id === a) as { syncId?: string | null }
    const nodeB = disk!.nodes.find((n) => n.id === b) as { syncId?: string | null }
    expect(nodeA.syncId).toBeTruthy()
    expect(nodeB.syncId).toBe(nodeA.syncId)

    undo()
    expect(syncIdOf(a)).toBeNull()
    expect(syncIdOf(b)).toBeNull()
    expect((harness.doc.getMap(DOC_MAP_PAGES).get(a) as { get(k: string): unknown }).get('syncId')).toBeNull()

    redo()
    expect(syncIdOf(a)).not.toBeNull()
    expect(syncIdOf(b)).toBe(syncIdOf(a))
  })
})
