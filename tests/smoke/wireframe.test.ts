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
  flushWorkspaceAutosave,
  getUndoState,
  getWireframeContent,
  listFileEntities,
  redoWorkspace,
  reloadWorkspace,
  resetSmokeState,
  undoWorkspace,
} from './app-client'
import { observeYDocTransactions, wait } from './test-utils'

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
