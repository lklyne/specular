/**
 * Markdown note content ↔ Y.Doc mirror (issue #262, ADR 0023).
 *
 * Note text lives in a `.md` file on disk (source of truth) with an
 * undo-tracked mirror in the `notes` Y.Map. These tests drive the same
 * mutator the `apply-note-content` IPC handler calls and assert on the
 * runtime mirror, the Y.Doc, and the `.md` file.
 *
 * Mutation-verified by:
 *   - removing `DOC_MAP_NOTES` from `undoableTypes` in workspace-undo.ts —
 *     "undo reverts the last edit" fails (undo no-ops on note content).
 *   - dropping the `'note-seed'` origin (transacting the baseline under
 *     'user') — "baseline seed is not an undo step" fails (undo blanks to '').
 *   - removing `projectNoteContentToDisk` from the undo observer in
 *     workspace-observers.ts — the on-disk asserts after undo/redo fail.
 */

import { readFileSync } from 'fs'
import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import {
  createFileEntity,
  createTextEntity,
  deleteFileEntity,
  getFileEntities,
  updateFileEntity,
} from '../../src/main/runtime/document-commands'
import { createNoteFile } from '../../src/main/runtime/note-assets'
import { commitNoteContent } from '../../src/main/runtime/note-commands'
import { getNoteContent } from '../../src/main/runtime/note-content-state'
import { undo, redo, canUndo } from '../../src/main/runtime/workspace-undo'
import { beginGestureSession } from '../../src/main/runtime/workspace-gesture-session'
import { DOC_MAP_NOTES } from '../../src/main/runtime/workspace-doc'

let harness: WorkspaceHarness

/** Create a note file on disk plus its file entity, like the note tool does. */
function makeNote(initial = ''): { id: string; file: string } {
  const file = createNoteFile('Test Note', initial)
  const entity = createFileEntity({ file, canvasX: 0, canvasY: 0, width: 200, height: 200 })
  return { id: entity.id, file }
}

const diskText = (file: string) => readFileSync(file, 'utf8')

describe('markdown note content', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('commitNoteContent updates the mirror, the Y.Map, and the .md on disk', async () => {
    const note = makeNote()
    await settleSync()

    expect(commitNoteContent(note.id, 'hello')).toBe(true)
    await settleSync()

    expect(getNoteContent(note.id)).toBe('hello')
    expect(harness.doc.getMap(DOC_MAP_NOTES).get(note.id)).toBe('hello')
    expect(diskText(note.file)).toBe('hello')
  })

  it('returns false for an unknown entity id', () => {
    expect(commitNoteContent('nope', 'x')).toBe(false)
  })

  it('an edit after the baseline seed produces exactly one Y.Doc transaction', async () => {
    const note = makeNote('seeded')
    await settleSync()
    commitNoteContent(note.id, 'first edit') // seeds baseline (+1 untracked tx)
    await settleSync()

    let count = 0
    const handler = () => (count += 1)
    harness.doc.on('afterTransaction', handler)
    try {
      commitNoteContent(note.id, 'second edit')
      await settleSync()
    } finally {
      harness.doc.off('afterTransaction', handler)
    }
    expect(count).toBe(1)
  })

  it('undo reverts the last edit in the mirror and on disk; redo reapplies it', async () => {
    const note = makeNote('original')
    await settleSync()

    commitNoteContent(note.id, 'edit one')
    await settleSync()
    commitNoteContent(note.id, 'edit two')
    await settleSync()

    undo()
    expect(getNoteContent(note.id)).toBe('edit one')
    expect(diskText(note.file)).toBe('edit one')

    redo()
    expect(getNoteContent(note.id)).toBe('edit two')
    expect(diskText(note.file)).toBe('edit two')
  })

  it('baseline seed is not an undo step — undo of the first edit restores the original disk content', async () => {
    const note = makeNote('pre-existing text')
    await settleSync()

    commitNoteContent(note.id, 'pre-existing text plus more')
    await settleSync()

    undo()
    // Lands on the seeded original, not empty and not the note's creation.
    expect(getNoteContent(note.id)).toBe('pre-existing text')
    expect(diskText(note.file)).toBe('pre-existing text')
  })

  it('undoing past all edits removes the note entity, and the edits were distinct steps', async () => {
    const note = makeNote()
    await settleSync()
    commitNoteContent(note.id, 'a')
    await settleSync()
    commitNoteContent(note.id, 'ab')
    await settleSync()

    undo() // 'ab' -> 'a'
    expect(getNoteContent(note.id)).toBe('a')
    undo() // 'a' -> '' (seed baseline)
    expect(getNoteContent(note.id)).toBe('')
    expect(canUndo()).toBe(true)
    undo() // entity creation
    expect(getFileEntities().some((e) => e.id === note.id)).toBe(false)
    // The notes Y.Map entry is pruned lazily by the next forward sync
    // (noteContentEntries drops ids with no backing entity), not inside the
    // undo transaction itself.
    createTextEntity({ canvasX: 0, canvasY: 0, text: 'trigger sync' })
    await settleSync()
    expect(harness.doc.getMap(DOC_MAP_NOTES).has(note.id)).toBe(false)
  })

  it('a note edit and a subsequent move are distinct undo steps', async () => {
    const note = makeNote('base')
    await settleSync()
    commitNoteContent(note.id, 'edited')
    await settleSync()
    updateFileEntity(note.id, { canvasX: 500, canvasY: 500 })
    await settleSync()

    undo() // move reverts, edit stays
    const moved = getFileEntities().find((e) => e.id === note.id)
    expect(moved?.canvasX).toBe(0)
    expect(getNoteContent(note.id)).toBe('edited')

    undo() // now the edit reverts
    expect(getNoteContent(note.id)).toBe('base')
  })

  it('a commit landing inside a drag gesture stays a separate undo step', async () => {
    // Repro: pointerdown on a focused note starts the drag gesture before
    // CodeMirror's blur commit reaches main, so the commit lands inside the
    // gesture's batch window. It must not fold into the move's undo step.
    const note = makeNote('base')
    await settleSync()
    commitNoteContent(note.id, 'edit one')
    await settleSync()

    const session = beginGestureSession()
    commitNoteContent(note.id, 'edit two') // blur commit racing the drag
    updateFileEntity(note.id, { canvasX: 500, canvasY: 500 })
    session.finalize()
    await settleSync()

    undo() // move reverts, edit two stays
    const moved = getFileEntities().find((e) => e.id === note.id)
    expect(moved?.canvasX).toBe(0)
    expect(getNoteContent(note.id)).toBe('edit two')

    undo() // edit two reverts
    expect(getNoteContent(note.id)).toBe('edit one')
  })

  it('deleting the note entity prunes its mirror entry from the Y.Map', async () => {
    const note = makeNote()
    await settleSync()
    commitNoteContent(note.id, 'doomed')
    await settleSync()
    expect(harness.doc.getMap(DOC_MAP_NOTES).has(note.id)).toBe(true)

    deleteFileEntity(note.id)
    await settleSync()
    expect(harness.doc.getMap(DOC_MAP_NOTES).has(note.id)).toBe(false)
  })
})
