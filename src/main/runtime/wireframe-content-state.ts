/**
 * Wireframe content under the Y.Doc (plan 3.0b).
 *
 * Wireframe file content is a *projection* of Y.Doc state: every edit runs as a
 * Y.Doc transaction and inherits undo/redo from the existing UndoManager. This
 * module owns the runtime mirror (one canonical JSON string per file-entity id),
 * the disk projection back to `.wireframe.json`, and the seed/diff helpers the
 * forward/reverse sync calls.
 *
 * Granularity A1 (per plan 2.1): the Y value is a JSON string, one entry per
 * file-entity, one transaction per op. The renderer keeps *reading* the
 * projected `.wireframe.json` for now (read-path repoint is the 3.5b cleanup);
 * only the write path is severed here.
 *
 * Layering: this module holds state + pure-ish projection only. The command
 * seam that schedules autosave / forward sync lives in `wireframe-commands.ts`
 * so the autosave module can depend on the projection without a cycle.
 */

import { createHash } from 'crypto'
import {
  createNodeIdGenerator,
  deleteNode,
  duplicateNode,
  findNodeById,
  insertNode,
  reorderNode,
  toggleNodeState,
  updateNodeProps,
  updateNodeText,
} from '../../shared/wireframe/wireframe-ops'
import type { WireframeFile, WireframeNode } from '../../shared/wireframe/wireframe-types'
import {
  parseWireframeFile,
  seedWireframeContent,
  serializeWireframeFile,
} from '../../shared/wireframe/wireframe-codec'
import { DOC_MAP_WIREFRAMES, getActiveDoc } from './workspace-doc'
import { readNoteFile, writeNoteFile } from './note-assets'
import { fileEntities } from './file-entity-state'

// ---------------------------------------------------------------------------
// Op descriptor — one shape for canvas gestures, IPC, and the agent CLI (3.4)
// ---------------------------------------------------------------------------

export type WireframeOp =
  | { kind: 'insert'; parentId: string; index: number; node: WireframeNode }
  | { kind: 'delete'; nodeId: string }
  | { kind: 'duplicate'; nodeId: string }
  | { kind: 'reorder'; nodeId: string; targetParentId: string; targetIndex: number }
  | { kind: 'setProps'; nodeId: string; patch: Record<string, unknown> }
  | { kind: 'setText'; nodeId: string; value: string }
  | { kind: 'toggle'; nodeId: string }
  | { kind: 'replace'; content: string }

// ---------------------------------------------------------------------------
// Runtime mirror — entity id → canonical JSON string
// ---------------------------------------------------------------------------

const wireframeContents = new Map<string, string>()

/** A workspace-managed wireframe file (the kind whose content we project). */
export function isWireframeFilePath(filePath: string): boolean {
  return /\.wireframe\.json$/i.test(filePath)
}

function fileEntityById(entityId: string): { id: string; file: string } | undefined {
  return fileEntities.find((e) => e.id === entityId)
}

let _dupCounter = 0

// ---------------------------------------------------------------------------
// Seeding — baseline content lives in BOTH the store and the Y.Doc map
// ---------------------------------------------------------------------------

/**
 * Ensure `entityId` has a baseline in the store and the Y.Doc `wireframes` map,
 * seeded from disk. The Y.Doc baseline is written in an *untracked* transaction
 * (`'wireframe-seed'` origin) so it is not itself an undo step — only the edit
 * that follows is. Idempotent: a no-op once a baseline exists.
 *
 * Without a baseline, undoing the *first* edit of an entity would remove the
 * map entry entirely (revert to "absent") and lose the original tree, which
 * survives only on disk.
 */
export function ensureWireframeBaseline(entityId: string): void {
  if (wireframeContents.has(entityId)) return
  const entity = fileEntityById(entityId)
  if (!entity || !isWireframeFilePath(entity.file)) return
  const fromDisk = readNoteFile(entity.file)
  // Canonicalize when the file is valid; otherwise keep the raw bytes so a
  // mid-edit malformed file still round-trips through the store.
  let baseline: string
  try {
    baseline = fromDisk == null ? '' : seedWireframeContent(fromDisk)
  } catch {
    baseline = fromDisk ?? ''
  }
  wireframeContents.set(entityId, baseline)
  const doc = getActiveDoc()
  const map = doc.getMap<string>(DOC_MAP_WIREFRAMES)
  if (map.get(entityId) !== baseline) {
    doc.transact(() => {
      map.set(entityId, baseline)
    }, 'wireframe-seed')
  }
}

/**
 * Seed a Y.Doc baseline for every current wireframe file-entity. Called when the
 * external-edit watcher (3.5) starts so an existing entity holds its at-rest tree
 * in the store + Y.Doc *before* any external edit can race in — without it, the
 * first thing seen for an untouched entity would be the already-edited file, and
 * the import couldn't diff back to the pre-edit tree. Idempotent (per-entity
 * no-op once seeded).
 */
export function seedAllWireframeBaselines(): void {
  for (const entity of fileEntities) {
    if (isWireframeFilePath(entity.file)) ensureWireframeBaseline(entity.id)
  }
}

// ---------------------------------------------------------------------------
// Reads
// ---------------------------------------------------------------------------

/** Current content for an entity: the store value, falling back to disk. */
export function getWireframeContent(entityId: string): string | null {
  const stored = wireframeContents.get(entityId)
  if (stored !== undefined) return stored
  const entity = fileEntityById(entityId)
  if (!entity) return null
  return readNoteFile(entity.file)
}

// ---------------------------------------------------------------------------
// Writes (store only — schedule autosave from wireframe-commands.ts)
// ---------------------------------------------------------------------------

/** Replace an entity's content verbatim (renderer write-path / `replace` op). */
export function setWireframeContent(entityId: string, content: string): void {
  wireframeContents.set(entityId, content)
}

/**
 * Apply a structural op to an entity's content via the shared pure ops (3.0).
 * Returns the new canonical content, or an error for an unknown entity / illegal
 * op (the validating boundary the CLI route surfaces as a 4xx).
 */
export function applyWireframeOp(
  entityId: string,
  op: WireframeOp,
): { ok: true; content: string } | { ok: false; error: string } {
  if (op.kind === 'replace') {
    setWireframeContent(entityId, op.content)
    return { ok: true, content: op.content }
  }

  const current = getWireframeContent(entityId)
  if (current == null) return { ok: false, error: `Unknown wireframe entity: ${entityId}` }

  let file: WireframeFile
  try {
    file = parseWireframeFile(current)
  } catch (err) {
    return { ok: false, error: (err as Error).message }
  }

  let next: WireframeFile
  try {
    next = applyOp(file, op)
  } catch (err) {
    return { ok: false, error: (err as Error).message }
  }

  const content = serializeWireframeFile(next)
  wireframeContents.set(entityId, content)
  return { ok: true, content }
}

/**
 * Validate an op's node references against `file`. Returns a legible error for an
 * unknown / wrong-typed reference, or `null` when the op is applicable.
 *
 * The shared pure ops (3.0) deliberately *no-op* on bad references — a safety net
 * for canvas gestures that race against a stale tree. The agent CLI route (3.4)
 * runs this first so a bad id surfaces as a 4xx instead of a silent no-op. Prop
 * legality is still enforced in `updateNodeProps` (it throws), so `setProps` only
 * checks node existence here.
 */
export function findWireframeOpError(file: WireframeFile, op: WireframeOp): string | null {
  switch (op.kind) {
    case 'replace':
      return null
    case 'insert': {
      const parent = findNodeById(file.root, op.parentId)
      if (!parent) return `Unknown parent node: ${op.parentId}`
      if (parent.type !== 'frame') return `Cannot insert into a non-frame node: ${op.parentId}`
      return null
    }
    case 'delete':
      if (file.root.id === op.nodeId) return 'Cannot delete the root node'
      if (!findNodeById(file.root, op.nodeId)) return `Unknown node: ${op.nodeId}`
      return null
    case 'duplicate':
      if (file.root.id === op.nodeId) return 'Cannot duplicate the root node'
      if (!findNodeById(file.root, op.nodeId)) return `Unknown node: ${op.nodeId}`
      return null
    case 'reorder': {
      if (!findNodeById(file.root, op.nodeId)) return `Unknown node: ${op.nodeId}`
      const target = findNodeById(file.root, op.targetParentId)
      if (!target) return `Unknown target parent: ${op.targetParentId}`
      if (target.type !== 'frame') return `Cannot reorder into a non-frame node: ${op.targetParentId}`
      return null
    }
    case 'setProps':
    case 'setText':
    case 'toggle':
      if (!findNodeById(file.root, op.nodeId)) return `Unknown node: ${op.nodeId}`
      return null
  }
}

function applyOp(file: WireframeFile, op: Exclude<WireframeOp, { kind: 'replace' }>): WireframeFile {
  switch (op.kind) {
    case 'insert':
      return insertNode(file, op.parentId, op.index, op.node)
    case 'delete':
      return deleteNode(file, op.nodeId)
    case 'duplicate':
      _dupCounter += 1
      return duplicateNode(file, op.nodeId, createNodeIdGenerator(`dup${_dupCounter}`))
    case 'reorder':
      return reorderNode(file, op.nodeId, op.targetParentId, op.targetIndex)
    case 'setProps':
      return updateNodeProps(file, op.nodeId, op.patch)
    case 'setText':
      return updateNodeText(file, op.nodeId, op.value)
    case 'toggle':
      return toggleNodeState(file, op.nodeId)
  }
}

// ---------------------------------------------------------------------------
// Forward sync — runtime mirror → Y.Doc (called inside syncRuntimeToDoc)
// ---------------------------------------------------------------------------

/**
 * Content entries to sync to the Y.Doc, pruned to entities that still exist and
 * are wireframes. Mutates the store to drop stale entries so a deleted entity's
 * content is GC'd from both the store and (via the diff) the Y.Doc map.
 */
export function getWireframeContentEntries(): Array<{ id: string; content: string }> {
  const entries: Array<{ id: string; content: string }> = []
  for (const [id, content] of wireframeContents) {
    const entity = fileEntityById(id)
    if (!entity || !isWireframeFilePath(entity.file)) {
      wireframeContents.delete(id)
      continue
    }
    entries.push({ id, content })
  }
  return entries
}

// ---------------------------------------------------------------------------
// Reverse sync — Y.Doc → runtime mirror (called on undo/redo)
// ---------------------------------------------------------------------------

/** Rebuild the runtime mirror from the Y.Doc `wireframes` map (undo/redo). */
export function applyWireframeContentsFromDoc(entries: Array<{ id: string; content: string }>): void {
  wireframeContents.clear()
  for (const { id, content } of entries) wireframeContents.set(id, content)
}

// ---------------------------------------------------------------------------
// Disk projection — runtime mirror → .wireframe.json (autosave debounce)
// ---------------------------------------------------------------------------

// Self-write registry: last content hash written per path. The external-edit
// watcher (3.5) hashes a changed file and consults `isWireframeSelfWrite` to
// ignore writes this process just made.
const selfWriteHashes = new Map<string, string>()

function hashContent(content: string): string {
  return createHash('sha1').update(content).digest('hex')
}

export function isWireframeSelfWrite(filePath: string, hash: string): boolean {
  return selfWriteHashes.get(filePath) === hash
}

// ---------------------------------------------------------------------------
// External-edit import — disk → runtime mirror (plan 3.5)
// ---------------------------------------------------------------------------

/** Why an external-edit import was skipped (for logging / tests). */
export type WireframeImportSkip =
  | 'untracked' // not a tracked `.wireframe.json` file-entity
  | 'unreadable' // file vanished / unreadable between event and read
  | 'self-write' // our own projection wrote these bytes — break the loop
  | 'unchanged' // canonicalizes to the tree we already hold (echoed event)
  | 'invalid' // malformed JSON / invalid tree — refuse to import

/**
 * Import a genuine external on-disk edit of a `.wireframe.json` into the runtime
 * mirror (plan 3.5). The watcher calls this when a file changes; the bytes are
 * validated and folded into the store, from where the shared forward-sync path
 * writes them to the Y.Doc as one undoable transaction and re-projects them (the
 * caller schedules that autosave).
 *
 * Loop safety: a write this process made (the projection step) is recognized by
 * content hash and skipped, and the imported bytes are themselves registered as a
 * self-write so the echo of the same change does not re-import. A content that
 * canonicalizes to what we already hold is skipped too, so no empty transaction
 * runs.
 */
export function importWireframeFileEdit(
  filePath: string,
): { ok: true; entityId: string } | { ok: false; reason: WireframeImportSkip; error?: string } {
  if (!isWireframeFilePath(filePath)) return { ok: false, reason: 'untracked' }
  const entity = fileEntities.find((e) => e.file === filePath)
  if (!entity) return { ok: false, reason: 'untracked' }

  const raw = readNoteFile(filePath)
  if (raw == null) return { ok: false, reason: 'unreadable' }

  // Our own projection wrote these exact bytes — ignore to break the
  // write → watch → write loop.
  if (isWireframeSelfWrite(filePath, hashContent(raw))) {
    return { ok: false, reason: 'self-write' }
  }

  // Canonicalize + validate. A malformed mid-edit file is refused, not imported.
  let canonical: string
  try {
    canonical = seedWireframeContent(raw)
  } catch (err) {
    return { ok: false, reason: 'invalid', error: (err as Error).message }
  }

  // Nothing new relative to the tree we already hold (an echoed event, or content
  // that canonicalizes to the current value). Resolve the current tree from the
  // store, falling back to the last-synced Y.Doc value when the entity has not
  // been mirrored yet.
  const current =
    wireframeContents.get(entity.id) ??
    getActiveDoc().getMap<string>(DOC_MAP_WIREFRAMES).get(entity.id)
  if (current === canonical) return { ok: false, reason: 'unchanged' }

  wireframeContents.set(entity.id, canonical)
  // Register the imported bytes as a self-write so the echo of this same change
  // re-firing the watcher is suppressed.
  selfWriteHashes.set(filePath, hashContent(raw))
  return { ok: true, entityId: entity.id }
}

function projectOne(entityFile: string, content: string): void {
  const hash = hashContent(content)
  if (selfWriteHashes.get(entityFile) === hash) return
  writeNoteFile(entityFile, content)
  selfWriteHashes.set(entityFile, hash)
}

/** Project a single entity's content to disk now (immediate, post-edit freshness). */
export function projectWireframeEntityToDisk(entityId: string): void {
  const content = wireframeContents.get(entityId)
  if (content === undefined) return
  const entity = fileEntityById(entityId)
  if (!entity || !isWireframeFilePath(entity.file)) return
  projectOne(entity.file, content)
}

/** Project all in-session wireframe content to disk (called from saveWorkspaceStore). */
export function projectWireframeContentsToDisk(): void {
  for (const { id, content } of getWireframeContentEntries()) {
    const entity = fileEntityById(id)
    if (entity) projectOne(entity.file, content)
  }
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

/** Drop all in-session wireframe content (workspace load / tab reset). */
export function clearWireframeContents(): void {
  wireframeContents.clear()
  selfWriteHashes.clear()
}
