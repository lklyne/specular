/**
 * Markdown note content smoke tests (issue #262).
 *
 * Covers `commitNoteContent` / `note-content-state.ts`: the Y.Doc mirror
 * that backs markdown notes so edits participate in the unified
 * UndoManager, while the `.md` file on disk stays the source of truth.
 *
 * Mutation-verified by:
 *   - removing `doc.getMap(DOC_MAP_NOTES)` from `undoableTypes` in
 *     `workspace-undo.ts` and confirming "undo/redo round-trips note
 *     content" fails (undo has nothing to revert).
 *   - making `ensureNoteBaseline` skip the disk read (always seed `''`)
 *     and confirming "note content persists to disk and is correctly
 *     re-seeded after a workspace reload" fails (undo lands on `''`
 *     instead of the persisted content).
 *   - having `ensureNoteBaseline` write into the Y.Map under the tracked
 *     `'user'` origin instead of `'note-seed'` and confirming "baseline
 *     seed is not an undo step" fails (one undo empties the note instead
 *     of leaving it at the original content).
 */

import { readFileSync } from 'fs'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import {
  applyCanvas,
  applyNoteContent,
  getFileEntities,
  getNoteContentMirror,
  getUndoState,
  redoWorkspace,
  resetNoteContentMirror,
  resetSmokeState,
  undoWorkspace,
} from './app-client'
import { observeYDocTransactions, wait } from './test-utils'

const NOTE_SEED_TEXT = 'Original note body, unmodified.'

async function createNote(): Promise<{ id: string; filePath: string }> {
  const { created } = await applyCanvas({
    entities: [{ kind: 'note', text: NOTE_SEED_TEXT, _forceFile: true, canvasX: 0, canvasY: 0 }],
  })
  const id = created[0]
  const { fileEntities } = await getFileEntities()
  const entity = fileEntities.find((f) => f.id === id)
  if (!entity) throw new Error(`note file entity ${id} not found after create`)
  return { id, filePath: entity.file }
}

async function drainUndoStack(): Promise<void> {
  for (let i = 0; i < 200; i++) {
    const state = await getUndoState()
    if (!state.canUndo) return
    await undoWorkspace()
  }
}

async function cleanupNotes(): Promise<void> {
  const { fileEntities } = await getFileEntities()
  if (fileEntities.length) {
    await applyCanvas({ delete: fileEntities.map((f) => f.id) })
  }
}

describe('note content', () => {
  beforeEach(async () => {
    await resetSmokeState()
    await resetNoteContentMirror()
    await cleanupNotes()
    await drainUndoStack()
  })

  afterEach(async () => {
    await cleanupNotes()
  })

  it('commitNoteContent updates the runtime mirror and writes the .md file', async () => {
    const { id, filePath } = await createNote()

    await applyNoteContent(id, 'hello from the mirror')

    expect((await getNoteContentMirror(id)).content).toBe('hello from the mirror')
    expect(readFileSync(filePath, 'utf8')).toBe('hello from the mirror')
  })

  it('produces exactly one Y.Doc transaction per edit once baseline is seeded', async () => {
    const { id } = await createNote()

    // First edit seeds the baseline (untracked 'note-seed' transaction) in
    // addition to the tracked edit transaction — don't count this one.
    await applyNoteContent(id, 'edit one')
    await wait(50)

    const count = await observeYDocTransactions(async () => {
      await applyNoteContent(id, 'edit two')
    })
    expect(count).toBe(1)
  })

  it('undo/redo round-trips note content on disk and in the mirror', async () => {
    const { id, filePath } = await createNote()
    const original = readFileSync(filePath, 'utf8')

    await applyNoteContent(id, 'edited once')
    await wait(50)
    expect(readFileSync(filePath, 'utf8')).toBe('edited once')

    await undoWorkspace()
    await wait(50)
    expect(readFileSync(filePath, 'utf8')).toBe(original)
    expect((await getNoteContentMirror(id)).content).toBe(original)

    await redoWorkspace()
    await wait(50)
    expect(readFileSync(filePath, 'utf8')).toBe('edited once')
    expect((await getNoteContentMirror(id)).content).toBe('edited once')
  })

  it('note content persists to disk and is correctly re-seeded after a workspace reload', async () => {
    const { id, filePath } = await createNote()

    await applyNoteContent(id, 'v2 content')
    await wait(50)
    expect(readFileSync(filePath, 'utf8')).toBe('v2 content')

    // Simulate a relaunch: the in-memory Y.Doc mirror starts empty again,
    // same as a fresh process would — the .md file is the only thing that
    // actually persisted.
    await resetNoteContentMirror()
    expect((await getNoteContentMirror(id)).content).toBeNull()

    // Touching the note again must re-seed the undo baseline from the
    // persisted disk content ('v2 content'), not stale/empty in-memory state.
    await applyNoteContent(id, 'v3 content')
    await wait(50)
    expect(readFileSync(filePath, 'utf8')).toBe('v3 content')

    await undoWorkspace()
    await wait(50)
    expect(readFileSync(filePath, 'utf8')).toBe('v2 content')
    expect((await getNoteContentMirror(id)).content).toBe('v2 content')
  })

  it('baseline seed is not an undo step', async () => {
    const { id, filePath } = await createNote()
    const original = readFileSync(filePath, 'utf8')

    await applyNoteContent(id, 'first edit')
    await wait(50)

    await undoWorkspace()
    await wait(50)

    // One undo after the first-ever edit must land on the original content
    // (the pre-edit baseline), not an empty note — the baseline seed itself
    // must not have consumed an undo step.
    expect(readFileSync(filePath, 'utf8')).toBe(original)
    expect((await getNoteContentMirror(id)).content).toBe(original)
  })
})
