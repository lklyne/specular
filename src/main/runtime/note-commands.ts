/**
 * Note content commands (issue #262).
 *
 * `commitNoteContent` is the single mutation seam for markdown note edits:
 * seed the Y.Doc baseline on first touch, update the runtime mirror,
 * project to disk, then schedule the diff-sync that turns the mirror change
 * into an undo-tracked Y.Doc write.
 */

import { fileEntities } from './file-entity-state'
import { ensureNoteBaseline, setNoteContent } from './note-content-state'
import { writeNoteFile } from './note-assets'
import { scheduleWorkspaceAutosave } from './workspace-session'

export function commitNoteContent(entityId: string, content: string): boolean {
  const entity = fileEntities.find((e) => e.id === entityId)
  if (!entity) return false
  ensureNoteBaseline(entityId, entity.file)
  setNoteContent(entityId, content)
  writeNoteFile(entity.file, content)
  scheduleWorkspaceAutosave()
  return true
}
