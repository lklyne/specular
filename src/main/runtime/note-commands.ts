/**
 * Note content commands (issue #262).
 *
 * `commitNoteContent` is the single mutation seam for markdown note edits:
 * seed the Y.Doc baseline on first touch, update the runtime mirror (which
 * writes the `notes` Y.Map in its own undo-tracked transaction), and project
 * to disk. No autosave is scheduled: a note edit touches nothing in the
 * `.canvas` file — the `.md` and the Y.Map are both written here directly —
 * and the deferred diff-sync would fold the edit into an open gesture batch
 * (blur commit racing a drag start), merging edit + move into one undo step.
 */

import { fileEntities } from './file-entity-state'
import { ensureNoteBaseline, setNoteContent } from './note-content-state'
import { writeNoteFile } from './note-assets'

export function commitNoteContent(entityId: string, content: string): boolean {
  const entity = fileEntities.find((e) => e.id === entityId)
  if (!entity) return false
  ensureNoteBaseline(entityId, entity.file)
  setNoteContent(entityId, content)
  writeNoteFile(entity.file, content)
  return true
}
