/**
 * Wireframe content smoke tests (plan 3.0b).
 *
 * Wireframe content is a projection of Y.Doc state: each op runs as a single
 * Y.Doc transaction (undoable), then projects to `.wireframe.json` on the
 * autosave debounce. These tests drive the runtime via the HTTP test routes the
 * CLI/IPC will share and assert on both runtime content and on-disk projection.
 *
 * Mutation-verified by:
 *   - removing the `doc.getMap(DOC_MAP_WIREFRAMES)` entry from the UndoManager's
 *     tracked types in `createCanvasUndoManager()` and confirming the undo/redo
 *     case fails (the content never enters the undo stack).
 *   - dropping `projectWireframeContentsToDisk()` from `saveWorkspaceStore()`
 *     and confirming the disk-projection assertions fail.
 */

import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import {
  applyWireframeOp,
  createWireframeEntity,
  deleteFileEntities,
  externalWriteWireframe,
  flushWorkspaceAutosave,
  getUndoState,
  getWireframeContent,
  insertWireframeNode,
  listFileEntities,
  redoWorkspace,
  reloadWorkspace,
  resetSmokeState,
  startTransactionCounter,
  stopTransactionCounter,
  undoWorkspace,
} from './app-client'
import type { WireframeOpInput } from './app-client'
import { observeYDocTransactions, wait, waitFor } from './test-utils'

function sampleContent(): string {
  return JSON.stringify(
    {
      version: '1.0',
      root: {
        id: 'root',
        type: 'frame',
        direction: 'vertical',
        children: [
          { id: 'title', type: 'text', text: 'Hello', level: 'h1' },
          { id: 'agree', type: 'checkbox', label: 'Agree', checked: false },
        ],
      },
    },
    null,
    2,
  )
}

async function cleanupFileEntities(): Promise<void> {
  const { fileEntities } = await listFileEntities()
  if (fileEntities.length) {
    await deleteFileEntities(fileEntities.map((e) => e.id))
  }
}

async function drainUndoStack(): Promise<void> {
  for (let i = 0; i < 200; i++) {
    const state = await getUndoState()
    if (!state.canUndo) return
    await undoWorkspace()
  }
}

describe('wireframe content', () => {
  beforeEach(async () => {
    await resetSmokeState()
    await cleanupFileEntities()
    await drainUndoStack()
  })

  afterEach(async () => {
    await cleanupFileEntities()
  })

  it('applies an op and projects the result to disk', async () => {
    const { id } = await createWireframeEntity({ content: sampleContent() })
    const result = await applyWireframeOp(id, {
      kind: 'setText',
      nodeId: 'title',
      value: 'Updated',
    })
    expect(result.ok).toBe(true)
    expect(result.content).toContain('"text": "Updated"')

    await flushWorkspaceAutosave()
    const after = await getWireframeContent(id)
    expect(after.disk).toBe(result.content)
    expect(after.disk).toContain('Updated')
  })

  // 3.3 — per-node property editing. The panel editors send a `setProps` patch
  // for the selected node; changing a frame's layout direction must project to
  // disk (the observable surface the canvas re-renders from).
  it('setProps: changes a frame direction and projects it to disk', async () => {
    const { id } = await createWireframeEntity({ content: sampleContent() })
    const result = await applyWireframeOp(id, {
      kind: 'setProps',
      nodeId: 'root',
      patch: { direction: 'horizontal' },
    })
    expect(result.ok).toBe(true)
    expect(result.content).toContain('"direction": "horizontal"')

    await flushWorkspaceAutosave()
    const after = await getWireframeContent(id)
    expect(after.disk).toBe(result.content)
    expect(after.disk).toContain('"direction": "horizontal"')
  })

  it('runs each op type and validates illegal props as a 4xx', async () => {
    const { id } = await createWireframeEntity({ content: sampleContent() })

    // setProps on a legal key.
    const setProps = await applyWireframeOp(id, {
      kind: 'setProps',
      nodeId: 'agree',
      patch: { label: 'I agree' },
    })
    expect(setProps.ok).toBe(true)

    // duplicate + delete + toggle all succeed.
    expect((await applyWireframeOp(id, { kind: 'toggle', nodeId: 'agree' })).ok).toBe(true)
    expect((await applyWireframeOp(id, { kind: 'duplicate', nodeId: 'title' })).ok).toBe(true)
    expect((await applyWireframeOp(id, { kind: 'delete', nodeId: 'title' })).ok).toBe(true)

    // An illegal prop for the node type is rejected.
    const illegal = await applyWireframeOp(id, {
      kind: 'setProps',
      nodeId: 'agree',
      patch: { variant: 'primary' },
    })
    expect(illegal.ok).toBe(false)
    expect(illegal.error).toMatch(/not valid for node type/)
  })

  it('applies an op as exactly one Y.Doc transaction', async () => {
    const { id } = await createWireframeEntity({ content: sampleContent() })
    // Baseline is seeded at create; the op itself is the only tracked change.
    const count = await observeYDocTransactions(async () => {
      await applyWireframeOp(id, { kind: 'setText', nodeId: 'title', value: 'Once' })
    })
    expect(count).toBe(1)
  })

  it('undo restores the prior tree and redo reapplies (disk round-trips)', async () => {
    const original = sampleContent()
    const { id } = await createWireframeEntity({ content: original })

    const edited = (await applyWireframeOp(id, {
      kind: 'setText',
      nodeId: 'title',
      value: 'Edited',
    })).content!
    await wait(50) // let the forward-sync microtask reach the UndoManager
    let snap = await flushWorkspaceAutosave().then(() => getWireframeContent(id))
    expect(snap.disk).toBe(edited)

    await undoWorkspace()
    snap = await flushWorkspaceAutosave().then(() => getWireframeContent(id))
    expect(snap.runtime).toBe(original)
    expect(snap.disk).toBe(original)

    await redoWorkspace()
    snap = await flushWorkspaceAutosave().then(() => getWireframeContent(id))
    expect(snap.runtime).toBe(edited)
    expect(snap.disk).toBe(edited)
  })

  it('round-trips through a workspace reload from disk', async () => {
    const { id } = await createWireframeEntity({ content: sampleContent() })
    const edited = (await applyWireframeOp(id, {
      kind: 'setText',
      nodeId: 'title',
      value: 'Persisted',
    })).content!
    await flushWorkspaceAutosave()

    await reloadWorkspace()

    const after = await getWireframeContent(id)
    expect(after.disk).toBe(edited)
    expect(after.runtime).toBe(edited)
  })
})

// 3.2 — the canvas/panel structural edits (insert / duplicate / delete) drive the
// same op→Y.Doc apply path. Each must be one undoable transaction that projects
// to disk, exactly like the setText case above.
describe('wireframe structural ops (insert / duplicate / delete)', () => {
  beforeEach(async () => {
    await resetSmokeState()
    await cleanupFileEntities()
    await drainUndoStack()
  })

  afterEach(async () => {
    await cleanupFileEntities()
  })

  // Apply `op`, assert it is exactly one transaction + reflected on disk, that
  // undo restores the original tree and redo reapplies — the 3.2 contract.
  async function expectOpRoundTrips(op: WireframeOpInput, expectDisk: (disk: string) => void) {
    const original = sampleContent()
    const { id } = await createWireframeEntity({ content: original })

    let edited = ''
    const count = await observeYDocTransactions(async () => {
      const result = await applyWireframeOp(id, op)
      expect(result.ok).toBe(true)
      edited = result.content!
    })
    expect(count).toBe(1)

    let snap = await flushWorkspaceAutosave().then(() => getWireframeContent(id))
    expect(snap.disk).toBe(edited)
    expectDisk(snap.disk!)

    await undoWorkspace()
    snap = await flushWorkspaceAutosave().then(() => getWireframeContent(id))
    expect(snap.runtime).toBe(original)
    expect(snap.disk).toBe(original)

    await redoWorkspace()
    snap = await flushWorkspaceAutosave().then(() => getWireframeContent(id))
    expect(snap.runtime).toBe(edited)
    expect(snap.disk).toBe(edited)
  }

  it('insert: one transaction, undo/redo round-trips, projects to disk', async () => {
    await expectOpRoundTrips(
      { kind: 'insert', parentId: 'root', index: 1, node: { id: 'mid', type: 'divider' } },
      (disk) => expect(disk).toContain('"id": "mid"'),
    )
  })

  it('duplicate: one transaction, undo/redo round-trips, projects to disk', async () => {
    // The headline "another card like this": the clone lands after the source.
    await expectOpRoundTrips({ kind: 'duplicate', nodeId: 'title' }, (disk) => {
      // Two text nodes carrying "Hello" now (the original + the clone).
      expect(disk.match(/"text": "Hello"/g)?.length).toBe(2)
    })
  })

  it('delete: one transaction, undo/redo round-trips, projects to disk', async () => {
    await expectOpRoundTrips({ kind: 'delete', nodeId: 'agree' }, (disk) => {
      expect(disk).not.toContain('"id": "agree"')
    })
  })

  it('panel insert palette: inserts a default node into the root, undoably', async () => {
    const original = sampleContent()
    const { id } = await createWireframeEntity({ content: original })

    let edited = ''
    const count = await observeYDocTransactions(async () => {
      const result = await insertWireframeNode(id, 'button')
      expect(result.ok).toBe(true)
      edited = result.content!
    })
    expect(count).toBe(1)
    // A default button landed at the end of the root frame.
    expect(edited).toContain('"type": "button"')
    expect(edited).toContain('"text": "Button"')

    let snap = await flushWorkspaceAutosave().then(() => getWireframeContent(id))
    expect(snap.disk).toBe(edited)

    await undoWorkspace()
    snap = await flushWorkspaceAutosave().then(() => getWireframeContent(id))
    expect(snap.runtime).toBe(original)
    expect(snap.disk).toBe(original)
  })
})

// 3.5 — external on-disk edits (agent `Write`, git checkout) are watched, parsed,
// validated, and imported into the Y.Doc as one undoable transaction. Self-writes
// from the projection step are ignored by content hash so the write→watch→write
// loop never closes.
//
// Mutation-verified by:
//   - removing the `scheduleWorkspaceAutosave()` call from
//     `importExternalWireframeEdit` and confirming the import never reaches the
//     Y.Doc (the transaction-count / undo assertions fail).
//   - dropping the `isWireframeSelfWrite` guard in `importWireframeFileEdit` and
//     confirming the self-write test re-imports (count > 0).
describe('wireframe external-edit import (3.5)', () => {
  beforeEach(async () => {
    await resetSmokeState()
    await cleanupFileEntities()
    await drainUndoStack()
  })

  afterEach(async () => {
    await cleanupFileEntities()
  })

  // An external edit, in canonical form so the projection is itself a no-op
  // (disk already matches) — keeps the transaction count deterministic.
  function externalContent(title: string): string {
    return JSON.stringify(
      JSON.parse(
        JSON.stringify({
          version: '1.0',
          root: {
            id: 'root',
            type: 'frame',
            direction: 'vertical',
            children: [
              { id: 'title', type: 'text', text: title, level: 'h1' },
              { id: 'agree', type: 'checkbox', label: 'Agree', checked: false },
            ],
          },
        }),
      ),
      null,
      2,
    )
  }

  it('imports an external edit as one undoable transaction, undo reverts', async () => {
    const original = sampleContent()
    const { id } = await createWireframeEntity({ content: original })
    // Let the create's own file write drain through the watcher (an unchanged
    // no-op) before we measure.
    await wait(200)

    const expected = externalContent('FromDisk')

    await startTransactionCounter()
    await externalWriteWireframe(id, expected)
    // Wait for the watcher (debounce + fs latency) to fold the edit into the
    // runtime mirror, then let the forward-sync microtask reach the UndoManager.
    await waitFor(
      () => getWireframeContent(id),
      (c) => c.runtime === expected,
      'external edit imported into the runtime mirror',
      { maxAttempts: 40, intervalMs: 100 },
    )
    await wait(100)
    const count = (await stopTransactionCounter()).count
    expect(count).toBe(1)

    // The import projects back to disk (a self-write — no second import).
    const snap = await flushWorkspaceAutosave().then(() => getWireframeContent(id))
    expect(snap.runtime).toBe(expected)
    expect(snap.disk).toBe(expected)

    // The import is one undo step back to the pre-import tree.
    await undoWorkspace()
    const undone = await getWireframeContent(id)
    expect(undone.runtime).toBe(original)
  })

  it('does not re-import its own projection (self-write suppressed by hash)', async () => {
    const { id } = await createWireframeEntity({ content: sampleContent() })
    // A normal in-app edit projects to disk — that write must NOT loop back in.
    const edited = (
      await applyWireframeOp(id, { kind: 'setText', nodeId: 'title', value: 'InApp' })
    ).content!
    await wait(50)
    await flushWorkspaceAutosave()

    // Give the watcher ample time to (not) re-import the projection's write.
    await startTransactionCounter()
    await wait(400)
    const count = (await stopTransactionCounter()).count
    expect(count).toBe(0)

    // Content is intact — not reverted, duplicated, or churned by a loop.
    const snap = await getWireframeContent(id)
    expect(snap.runtime).toBe(edited)
    expect(snap.disk).toBe(edited)
  })
})
