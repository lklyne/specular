/**
 * Note Content State (issue #262)
 *
 * Runtime mirror of markdown note content, keyed by file-entity id. The
 * `.md` file on disk stays the source of truth; this mirror (and its Y.Map
 * projection, `DOC_MAP_NOTES`) exists only so note edits participate in the
 * unified Yjs UndoManager. See docs/adr/0023.
 *
 * Lifecycle:
 * - `ensureNoteBaseline` seeds the mirror + Y.Map from disk on first touch,
 *   using an untracked origin so seeding existing content is not itself an
 *   undo step.
 * - `setNoteContent` records a new edit in the mirror; the caller is
 *   responsible for triggering the forward diff-sync (`scheduleWorkspaceAutosave`).
 * - `applyNoteContentsFromDoc` is the reverse direction: after Y.Doc reverts
 *   (undo/redo), pull the notes map back into the mirror.
 */

import type * as Y from 'yjs'
import { fileEntities } from './file-entity-state'
import { MARKDOWN_EXTENSIONS } from '../../shared/file-extensions'
import { readNoteFile, writeNoteFile } from './note-assets'
import { getActiveDoc, DOC_MAP_NOTES } from './workspace-doc'

const noteContentMirror = new Map<string, string>()

function backingNoteFilePath(entityId: string): string | null {
  const entity = fileEntities.find((e) => e.id === entityId)
  if (!entity || !MARKDOWN_EXTENSIONS.test(entity.file)) return null
  return entity.file
}

/**
 * Entries to forward-sync into the `notes` Y.Map. Prunes mirror entries
 * whose backing markdown file entity no longer exists (deleted, or morphed
 * back to a text entity), so the diff-sync deletes the stale Y.Map key too.
 */
export function noteContentEntries(): ReadonlyMap<string, string> {
  for (const id of [...noteContentMirror.keys()]) {
    if (!backingNoteFilePath(id)) noteContentMirror.delete(id)
  }
  return noteContentMirror
}

export function getNoteContent(entityId: string): string | undefined {
  return noteContentMirror.get(entityId)
}

/** Seed the mirror + Y.Map from the current on-disk content, if not already tracked. */
export function ensureNoteBaseline(entityId: string, filePath: string): void {
  if (noteContentMirror.has(entityId)) return
  const disk = readNoteFile(filePath) ?? ''
  noteContentMirror.set(entityId, disk)
  const doc = getActiveDoc()
  doc.transact(() => {
    ;(doc.getMap(DOC_MAP_NOTES) as Y.Map<string>).set(entityId, disk)
  }, 'note-seed')
}

export function setNoteContent(entityId: string, content: string): void {
  noteContentMirror.set(entityId, content)
  // Write the Y.Map immediately in its own tracked transaction rather than
  // waiting for the deferred diff-sync: if a gesture batch window is open
  // (blur commit racing a drag start), the deferred sync would fold this
  // edit into the gesture's transaction — one undo step for edit + move.
  // The diff-sync later compares equal and no-ops.
  const doc = getActiveDoc()
  doc.transact(() => {
    const yNotes = doc.getMap(DOC_MAP_NOTES) as Y.Map<string>
    if (yNotes.get(entityId) !== content) yNotes.set(entityId, content)
  }, 'user')
}

export function projectNoteContentToDisk(entityId: string): void {
  const filePath = backingNoteFilePath(entityId)
  const content = noteContentMirror.get(entityId)
  if (!filePath || content === undefined) return
  writeNoteFile(filePath, content)
}

export function projectAllNoteContentToDisk(): void {
  for (const id of noteContentMirror.keys()) projectNoteContentToDisk(id)
}

/**
 * Reverse-sync: pull the `notes` Y.Map (already reverted by UndoManager)
 * back into the mirror. Returns the ids whose content actually changed, so
 * the caller can project just those to disk.
 */
export function applyNoteContentsFromDoc(entries: ReadonlyMap<string, string>): string[] {
  const changed: string[] = []
  for (const [id, content] of entries) {
    if (noteContentMirror.get(id) !== content) {
      noteContentMirror.set(id, content)
      changed.push(id)
    }
  }
  for (const id of [...noteContentMirror.keys()]) {
    if (!entries.has(id)) {
      noteContentMirror.delete(id)
      changed.push(id)
    }
  }
  return changed
}

/** Clear the in-memory mirror (tab switch, workspace reload). */
export function clearNoteContentState(): void {
  noteContentMirror.clear()
}
