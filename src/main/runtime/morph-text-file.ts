/**
 * Morph text ↔ markdown file (ADR 0013 §3 / Phase 7).
 *
 * Both directions replace the selected entity with a new entity of the
 * opposite kind, anchored at the same canvas rect. The file-system half
 * (writing or deleting the `.md` note) is registered as a paired undo/redo
 * side-effect against the upcoming Y.Doc stack item via `workspace-undo`,
 * so a single Ctrl+Z reverses both the entity replacement and the disk
 * mutation.
 *
 * The renderer surfaces both directions from the popup-row variant toggle:
 *   - `morphTextEntityToMarkdownFile` (short → long): clicked from a plain
 *      text selection.
 *   - `morphMarkdownFileToTextEntity` (long → short): clicked from a
 *      markdown file selection.
 */

import { existsSync, unlinkSync, writeFileSync } from 'fs'
import { basename } from 'path'
import { MARKDOWN_EXTENSIONS } from '../../shared/file-extensions'
import {
  createFileEntity as createFileEntityInState,
  fileEntities,
  deleteFileEntity as deleteFileEntityInState,
} from './file-entity-state'
import {
  createTextEntity as createTextEntityInState,
  deleteTextEntity as deleteTextEntityInState,
  textEntities,
} from './text-entity-state'
import { createNoteFile, readNoteFile } from './note-assets'
import { mutateWorkspace } from './mutate-workspace'
import { selectEntity } from './ui-actions'
import { pushPendingUndoSideEffect } from './workspace-undo'

export type MorphResult =
  | { kind: 'morphed'; newEntityId: string }
  | { kind: 'noop'; reason: 'not-found' | 'wrong-kind' }

/**
 * short → long. The selected plain text becomes a markdown file entity at
 * the same rect. Color/size fields are dropped — markdown content owns
 * its own formatting (ADR 0013 §3).
 */
export function morphTextEntityToMarkdownFile(entityId: string): MorphResult {
  const text = textEntities.find((e) => e.id === entityId)
  if (!text) return { kind: 'noop', reason: 'not-found' }

  return mutateWorkspace(() => {
    const noteName = text.label?.trim() || firstNonEmptyLine(text.text) || 'Untitled Note'
    const filePath = createNoteFile(noteName, text.text)

    deleteTextEntityInState(entityId)
    const file = createFileEntityInState({
      canvasX: text.canvasX,
      canvasY: text.canvasY,
      width: text.width,
      height: text.height,
      file: filePath,
      parentGroupId: text.parentGroupId,
    })

    pushPendingUndoSideEffect({
      // Undo restores the text entity through Y.Doc — clean the newly written
      // file so we don't leave it dangling on disk.
      undo: () => safeUnlink(filePath),
      // Redo re-creates the file entity through Y.Doc — make sure the .md
      // file exists again with the original content.
      redo: () => safeWrite(filePath, text.text),
    })

    selectEntity(file.id, 'file')
    return { kind: 'morphed', newEntityId: file.id }
  })
}

/**
 * long → short. The selected markdown file entity becomes a plain text
 * entity at the same rect, carrying the file's flattened content (we use
 * the raw markdown source as the body — no formatter for v1).
 */
export function morphMarkdownFileToTextEntity(entityId: string): MorphResult {
  const file = fileEntities.find((e) => e.id === entityId)
  if (!file) return { kind: 'noop', reason: 'not-found' }
  if (!MARKDOWN_EXTENSIONS.test(file.file)) return { kind: 'noop', reason: 'wrong-kind' }

  return mutateWorkspace(() => {
    const filePath = file.file
    const body = readNoteFile(filePath) ?? ''

    deleteFileEntityInState(entityId)
    safeUnlink(filePath)

    const noteName = stripMarkdownExt(basename(filePath))
    const text = createTextEntityInState({
      canvasX: file.canvasX,
      canvasY: file.canvasY,
      width: file.width,
      height: file.height,
      text: body,
      textStyle: 'plain',
      // Preserve the markdown file's existing bounds so the morph doesn't reflow.
      widthMode: 'fixed',
      parentGroupId: file.parentGroupId,
      label: noteName,
    })

    pushPendingUndoSideEffect({
      // Undo restores the file entity — recreate the deleted file with the
      // captured content so the renderer can read it back.
      undo: () => safeWrite(filePath, body),
      // Redo removes the file entity again — drop the file too.
      redo: () => safeUnlink(filePath),
    })

    selectEntity(text.id, 'text')
    return { kind: 'morphed', newEntityId: text.id }
  })
}

function firstNonEmptyLine(text: string): string {
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim().replace(/^#+\s*/, '')
    if (trimmed) return trimmed.slice(0, 80)
  }
  return ''
}

function stripMarkdownExt(name: string): string {
  return name.replace(/\.md$/i, '')
}

function safeUnlink(filePath: string): void {
  try {
    if (existsSync(filePath)) unlinkSync(filePath)
  } catch {
    /* best-effort */
  }
}

function safeWrite(filePath: string, content: string): void {
  try {
    writeFileSync(filePath, content, 'utf8')
  } catch {
    /* best-effort */
  }
}
