/**
 * Cloud-sync desktop-seam integration tests (ADR 0018 §1/§4, spike step 4).
 *
 * Exercises the four production surfaces the sync seam touches: remote updates
 * arriving through `applyRemoteUpdate`, the generalized Y.Doc→runtime observer,
 * the UndoManager's exclusion of remote origins, CRDT merge convergence, and
 * the `specular.server` binding round-tripping to disk with the fork guard.
 *
 * Mutation-verified by:
 *   - reverting the observer guard in workspace-observers.ts to
 *     `if (!undoManager || transaction.origin !== undoManager) return`
 *     (dropping the REMOTE_SYNC_ORIGIN branch) — "remote edit reaches the
 *     runtime" fails because the reverse sync never fires for remote origins.
 *   - making applyRemoteUpdate stamp origin `'user'` instead of
 *     REMOTE_SYNC_ORIGIN — "local undo does not revert the remote edit" fails
 *     because the UndoManager captures the remote change and undo reverts it.
 *   - dropping the `server` argument in writeAllTabsAsCanvasFiles (so the block
 *     is never written) — "publish writes the specular.server block" fails.
 *   - making resolveBindingOnLoad ignore the registry (always return the
 *     binding) — "a second path claiming the docId is forked" fails.
 */

import { mkdtempSync, rmSync } from 'fs'
import { join } from 'path'
import { tmpdir } from 'os'
import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import * as Y from 'yjs'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { createTextEntity, getTextEntities } from '../../src/main/runtime/document-commands'
import { createPage } from '../../src/main/runtime/page-runtime'
import { undo } from '../../src/main/runtime/workspace-undo'
import { DOC_MAP_ENTITIES } from '../../src/main/runtime/workspace-doc'
import { loadWorkspace } from '../../src/main/runtime/workspace-autosave'
import {
  REMOTE_SYNC_ORIGIN,
  applyRemoteUpdate,
  publishBinding,
  getSyncBinding,
  resetSyncState,
  resolveBindingOnLoad,
  registerDoc,
  readSyncRegistry,
} from '../../src/main/runtime/workspace-sync'

let harness: WorkspaceHarness

/** Replay the harness doc's whole state into a fresh headless peer. */
function replayPeer(): Y.Doc {
  const peer = new Y.Doc()
  Y.applyUpdate(peer, Y.encodeStateAsUpdate(harness.doc))
  return peer
}

/** Encode the peer's changes as a diff the harness doc hasn't seen yet. */
function diffFor(peer: Y.Doc): Uint8Array {
  return Y.encodeStateAsUpdate(peer, Y.encodeStateVector(harness.doc))
}

/** Minimal text-entity Y.Map, mirroring the runtime's persisted shape. */
function makeTextEntityYMap(id: string, text: string): Y.Map<unknown> {
  const m = new Y.Map<unknown>()
  m.set('kind', 'text')
  m.set('id', id)
  m.set('text', text)
  m.set('color', '3')
  m.set('textStyle', 'sticky')
  m.set('widthMode', 'fixed')
  m.set('canvasX', 0)
  m.set('canvasY', 0)
  m.set('width', 200)
  m.set('height', 200)
  return m
}

describe('cloud sync — desktop seam', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    resetSyncState()
  })

  afterAll(() => harness?.dispose())

  it('remote edit round-trips into runtime without echoing back', async () => {
    // A page makes the reverse-sync side effects broadcast (sendInteractiveState
    // iterates pages), giving an observable "a layout/broadcast occurred" signal.
    createPage({ url: 'https://example.com/', canvasX: 0, canvasY: 0, presetIndex: 0 })
    const text = createTextEntity({ canvasX: 10, canvasY: 10, text: 'local' })
    await settleSync()

    const peer = replayPeer()
    peer.transact(() => {
      const entity = (peer.getMap(DOC_MAP_ENTITIES) as Y.Map<Y.Map<unknown>>).get(text.id)!
      entity.set('text', 'remote-edit')
    }, 'remote-peer')
    const update = diffFor(peer)

    harness.clearBroadcasts()
    const origins: unknown[] = []
    const spy = (tx: { origin: unknown }) => origins.push(tx.origin)
    harness.doc.on('afterTransaction', spy)
    applyRemoteUpdate(update)
    await settleSync()
    harness.doc.off('afterTransaction', spy)

    // Runtime reflects the remote change.
    expect(getTextEntities().find((t) => t.id === text.id)?.text).toBe('remote-edit')
    // A broadcast fired from the reverse-sync side effects.
    expect(harness.broadcasts.length).toBeGreaterThan(0)
    // The remote transaction landed under its own origin...
    expect(origins).toContain(REMOTE_SYNC_ORIGIN)
    // ...and forward sync did NOT echo it back as a local 'user' transaction.
    expect(origins.some((o) => o === 'user')).toBe(false)
  })

  it('local undo does not revert a remote edit; local edits still undo cleanly', async () => {
    // Local edit BEFORE the remote edit.
    const a = createTextEntity({ canvasX: 0, canvasY: 0, text: 'A0' })
    await settleSync()

    const peer = replayPeer()
    peer.transact(() => {
      const entity = (peer.getMap(DOC_MAP_ENTITIES) as Y.Map<Y.Map<unknown>>).get(a.id)!
      entity.set('text', 'A-remote')
    }, 'remote-peer')
    applyRemoteUpdate(diffFor(peer))
    await settleSync()
    expect(getTextEntities().find((t) => t.id === a.id)?.text).toBe('A-remote')

    // Local edit AFTER the remote edit.
    const b = createTextEntity({ canvasX: 50, canvasY: 50, text: 'B0' })
    await settleSync()

    // Undo pops the local create of B; the remote edit to A is untouched
    // because it never entered the (null/'user'-only) undo stack.
    undo()
    expect(getTextEntities().some((t) => t.id === b.id)).toBe(false)
    expect(getTextEntities().find((t) => t.id === a.id)?.text).toBe('A-remote')

    // The pre-remote local edit still undoes cleanly.
    undo()
    expect(getTextEntities().some((t) => t.id === a.id)).toBe(false)
  })

  it('divergent offline edits merge to the union; runtime picks up the merge', async () => {
    const base = createTextEntity({ canvasX: 0, canvasY: 0, text: 'shared' })
    await settleSync()

    const snapshot = Y.encodeStateAsUpdate(harness.doc)
    const docA = new Y.Doc()
    Y.applyUpdate(docA, snapshot)
    const docB = new Y.Doc()
    Y.applyUpdate(docB, snapshot)

    docA.transact(() => {
      ;(docA.getMap(DOC_MAP_ENTITIES) as Y.Map<Y.Map<unknown>>).set('a1', makeTextEntityYMap('a1', 'from A'))
    })
    docB.transact(() => {
      ;(docB.getMap(DOC_MAP_ENTITIES) as Y.Map<Y.Map<unknown>>).set('b1', makeTextEntityYMap('b1', 'from B'))
    })

    // Exchange diffs both directions.
    Y.applyUpdate(docB, Y.encodeStateAsUpdate(docA, Y.encodeStateVector(docB)))
    Y.applyUpdate(docA, Y.encodeStateAsUpdate(docB, Y.encodeStateVector(docA)))

    // Both peers converged to the union.
    for (const doc of [docA, docB]) {
      const entities = doc.getMap(DOC_MAP_ENTITIES) as Y.Map<Y.Map<unknown>>
      expect(entities.has(base.id)).toBe(true)
      expect(entities.has('a1')).toBe(true)
      expect(entities.has('b1')).toBe(true)
    }

    // Feed the merged result into the runtime.
    applyRemoteUpdate(Y.encodeStateAsUpdate(docA, Y.encodeStateVector(harness.doc)))
    await settleSync()

    const ids = getTextEntities().map((t) => t.id)
    expect(ids).toContain(base.id)
    expect(ids).toContain('a1')
    expect(ids).toContain('b1')
  })

  it('publish writes the specular.server block; it survives reload', () => {
    const binding = { docId: 'doc-abc', url: 'ws://localhost:1234/doc-abc' }
    publishBinding(binding)

    // The block lands in the .canvas file on disk (asserted via diskDoc).
    const disk = harness.diskDoc()
    expect(disk?.specular?.server).toEqual(binding)

    // Simulate a fresh process: drop the in-memory binding, reload from disk.
    resetSyncState()
    expect(getSyncBinding()).toBeNull()
    loadWorkspace()
    // Same workspace path owns the docId in the registry, so the binding is kept.
    expect(getSyncBinding()).toEqual(binding)
  })

  it('fork guard: a second path claiming a known docId gets its binding cleared', () => {
    // Unit-tested at the registry-function level: the single-global harness
    // cannot boot a second workspace at a distinct path in the same process, so
    // the fork branch is verified directly against a scratch registry file.
    const dir = mkdtempSync(join(tmpdir(), 'specular-forkguard-'))
    try {
      const binding = { docId: 'doc-x', url: 'ws://host/doc-x' }
      registerDoc(dir, binding.docId, '/workspaces/A')

      // Same path keeps the binding.
      expect(resolveBindingOnLoad(dir, '/workspaces/A', binding)).toEqual(binding)
      // A different path is forked — binding dropped.
      expect(resolveBindingOnLoad(dir, '/workspaces/B', binding)).toBeNull()
      // The fork did not steal ownership in the registry.
      expect(readSyncRegistry(dir)['doc-x']).toBe('/workspaces/A')
    } finally {
      rmSync(dir, { recursive: true, force: true })
    }
  })
})
