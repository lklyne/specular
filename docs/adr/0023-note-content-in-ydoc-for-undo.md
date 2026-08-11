# ADR 0023 — Markdown note content mirrored into the Y.Doc for undo

**Status:** Accepted
**Date:** 2026-07-01
**Implements:** issue #262

## Context

Editing a markdown **note** (`file` entity with a `.md` backing file, the
"Document" affordance in `CONTEXT.md`) was not undoable, and worse, Cmd+Z
destroyed the whole note instead of reverting an edit. Note text is written
only to the `.md` file on disk — never to the Y.Doc — so the two text-editing
sessions in the repro recorded zero undo steps. Cmd+Z then skipped past the
(unrecorded) edits straight to the note's creation and deleted it.

Three facts collided: CodeMirror's local undo is deliberately disabled
(undo is meant to be owned by the shared Yjs `UndoManager`); Cmd+Z is
intercepted before CodeMirror sees it and routed to `UndoManager`; but
`UndoManager` had nothing to revert, because note text never entered the
Y.Doc. Plain **text** entities don't have this bug because their text lives
in the Y.Doc already — notes were the lone exception.

## Decision

Add a `notes` Y.Map (`DOC_MAP_NOTES`, `src/main/runtime/space-doc.ts`),
keyed by file-entity id, value = the note's full markdown text, and track it
in the `UndoManager` (`space-undo.ts`).

**The `.md` file on disk remains the source of truth.** The `notes` Y.Map is
a transient, undo-tracked *mirror* — it is not the canonical store and its
content is **not** duplicated into `.canvas` / JSON Canvas files. Entities
still reference the file by path only; the JSON Canvas serializer never
reads or writes `notes` map content.

Lifecycle (`src/main/runtime/note-content-state.ts`, `note-commands.ts`):

- **Baseline seed** — before a note's first tracked edit, its current
  on-disk content is copied into the runtime mirror and the Y.Map, using an
  untracked `'note-seed'` Y.Doc transaction origin (not in
  `UndoManager`'s `trackedOrigins`). Seeding existing content is therefore
  never itself an undo step — otherwise the first Cmd+Z on an untouched note
  would blank it instead of reverting nothing.
- **Edit** — `commitNoteContent(entityId, content)` is the single mutation
  seam: ensure baseline → update the runtime mirror → write the `.md` file
  → `scheduleSpaceAutosave()`. The renderer calls it via
  `applyNoteContent` (IPC), replacing the old direct `writeNoteFile` write
  for markdown notes specifically.
- **Forward sync** — the existing diff-sync engine (`syncRuntimeToDoc`)
  gained a `noteContent` parameter; changed entries are written into the
  `notes` Y.Map under the normal `'user'` origin on the existing
  autosave/`requestDocSync` microtask, so each edit becomes exactly one
  `UndoManager` step, same as every other tracked mutation.
- **Undo/redo** — the undo observer (`space-observers.ts`) reads the
  reverted `notes` Y.Map back into the runtime mirror
  (`applyNoteContentsFromDoc`) and projects only the changed ids back to the
  `.md` file. No file watcher exists in `main`, so there's no self-write
  loop to guard against.
- **Scene reflection** — `CanvasSceneFileEntity.noteContent` carries the
  mirror value once a note has been touched (undefined beforehand). The
  renderer (`MarkdownInlineRenderer`) falls back to its original
  `fetch()`-from-disk load for untouched notes, and switches to trusting
  scene broadcasts — which include undo/redo — once the field is defined.
  This mirrors the echo-suppression pattern already used by plain text
  entities in `StickyBodyLayer`.

Non-markdown note-backed renderers (wireframe JSON) are **not** covered by
this change — `writeNoteFile` stays as a raw disk-write IPC handler for
them. `note-content-state.ts` is written as a reusable template for doing
the same to wireframes in a follow-up (an earlier, never-merged branch for
issue #197 attempted this for wireframes directly).

## Consequences

**Fixes:** Cmd+Z on a note now reverts the last text edit instead of
deleting the whole note; redo restores it; behavior matches plain text
entities.

**Scope:** only markdown (`file` entities matching `MARKDOWN_EXTENSIONS`)
are covered. Wireframe JSON content is unchanged (still raw disk writes,
no undo).

**Not persisted structurally:** note text is not part of the `.canvas` /
JSON Canvas document — only the file reference is. The Y.Doc mirror is
cleared on tab switch (`resetDocSync`) and reseeded lazily from disk on
next touch, so it never needs its own migration or backward-compat story.

**Out of scope:** full CRDT character-level merge of note text (full-string
replace per edit matches how text entities already behave); changing the
unified-undo design or the Electron `{ role: 'undo' }` menu.
