// ---------------------------------------------------------------------------
// Scoped tab context
// ---------------------------------------------------------------------------
// Background-tab writes reuse the single-active-tab machinery instead of
// growing a second mutation seam (ADR 0025): swap the target tab's data into
// the runtime arrays, run the normal spine, sync the result back into the
// tab's record and its `.canvas` file, then put the user's tab back.
//
// The whole swap is ONE synchronous JS turn. That is the invariant everything
// else rests on: the 350ms autosave debounce, IPC handlers, and
// `syncActiveTabRecord()` cannot interleave with it, so a transiently-wrong
// set of module arrays is never observable. `applyCanvasPatch` is synchronous
// today — keep any `fn` passed here synchronous too.
//
// Three things the swap deliberately does NOT do, all for the same reason
// (the user is looking at another canvas and must not notice):
//
//   - It never destroys or instantiates a `WebContentsView`. The active tab's
//     pages are lifted out of the `pages` array by reference and put back;
//     the target tab's pages ride through as snapshot data only. Creating a
//     page on a background tab is rejected at the patch door — see
//     `isBackgroundTabContextActive`.
//   - It never touches focus, viewport, selection, or panel state. The
//     target's snapshot keeps its own copies of those fields verbatim.
//   - It never writes the active tab's `.canvas` file. Persistence is
//     suspended for the whole swap and only the target tab is written.
//
// Undo is deliberately absent: the write happens in a fresh detached Y.Doc
// with no UndoManager attached, so the user's undo stack is untouched and
// Cmd+Z after an agent write is a no-op with respect to it (issue #360 §6).

import type * as Y from 'yjs'
import type { CanvasEntityKind, PersistedWorkspaceTab, WorkspaceSnapshot } from '../../shared/types'
import { forEachEntityKind, type RuntimeEntity } from '../entities/contract'
import { pages } from './runtime-context'
import {
  activeWorkspaceTabId,
  workspaceAnnotations,
  workspaceEdges,
  workspaceTabs,
} from './workspace-model'
import {
  createWorkspaceDoc,
  getActiveDoc,
  hydrateDocFromSnapshot,
  setActiveDoc,
  withSuppressedDocSync,
} from './workspace-doc'
import { getActiveUndoManager, setActiveUndoManager } from './workspace-undo'
import { withWorkspacePersistenceSuspended } from './workspace-autosave'
import { writeTabAsCanvasFile } from './workspace-persistence'
import { spaceDir } from './space-dir'
import {
  cloneAnnotationsForPersistence,
  cloneWorkspaceSnapshot,
} from './runtime-serialization'
import { syncActiveTabRecord, workspaceSnapshot } from './workspace-tabs'

// ---------------------------------------------------------------------------
// Background-context probe (read by the patch door to reject page creates)
// ---------------------------------------------------------------------------

export interface BackgroundTabContext {
  tabId: string
  /**
   * The target's page ids. They exist in its snapshot but not in the `pages`
   * runtime array, so kind lookups can't see them — the patch door needs this
   * set to tell "a page on the background canvas" from "an unknown id".
   */
  pageIds: ReadonlySet<string>
}

let backgroundContext: BackgroundTabContext | null = null

/**
 * The background-tab swap currently in effect, if any. `applyCanvasPatch`
 * consults this to refuse page work: pages are WebContentsView-backed, and
 * the swap carries them as data only (issue #360 §5).
 */
export function activeBackgroundTabContext(): BackgroundTabContext | null {
  return backgroundContext
}

function pageIdsIn(snapshot: WorkspaceSnapshot): Set<string> {
  const ids = new Set<string>()
  for (const page of snapshot.pages) if (page.id) ids.add(page.id)
  for (const [id, entity] of Object.entries(snapshot.entities ?? {})) {
    if (entity?.kind === 'page') ids.add(id)
  }
  return ids
}

// ---------------------------------------------------------------------------
// Runtime-array save / restore
// ---------------------------------------------------------------------------

interface SwappedRuntimeState {
  entitiesByKind: Map<CanvasEntityKind, RuntimeEntity[]>
  pages: unknown[]
  edges: unknown[]
  annotations: unknown[]
  doc: Y.Doc
  undoManager: ReturnType<typeof getActiveUndoManager>
}

/**
 * Lift every runtime array out by reference. Each kind's `restore` replaces
 * its store wholesale, so handing back the exact objects taken out round-trips
 * the live entities untouched — no persist/rehydrate projection in between.
 * `pages` is spliced rather than restored so live `WebContentsView`s are never
 * destroyed and rebuilt.
 */
function liftRuntimeState(): SwappedRuntimeState {
  const entitiesByKind = new Map<CanvasEntityKind, RuntimeEntity[]>()
  forEachEntityKind((def) => {
    if (def.kind === 'page') return
    entitiesByKind.set(def.kind, [...def.entities()])
  })
  return {
    entitiesByKind,
    pages: pages.splice(0, pages.length),
    edges: workspaceEdges.splice(0, workspaceEdges.length),
    annotations: workspaceAnnotations.splice(0, workspaceAnnotations.length),
    doc: getActiveDoc(),
    undoManager: getActiveUndoManager(),
  }
}

function restoreRuntimeState(saved: SwappedRuntimeState): void {
  forEachEntityKind((def) => {
    if (def.kind === 'page') return
    def.restore((saved.entitiesByKind.get(def.kind) ?? []) as unknown as Record<string, unknown>[])
  })
  pages.push(...(saved.pages as never[]))
  workspaceEdges.length = 0
  workspaceEdges.push(...(saved.edges as never[]))
  workspaceAnnotations.length = 0
  workspaceAnnotations.push(...(saved.annotations as never[]))
  setActiveDoc(saved.doc)
  setActiveUndoManager(saved.undoManager)
}

// ---------------------------------------------------------------------------
// Target hydration
// ---------------------------------------------------------------------------

/** Legacy snapshots persisted sticky notes under their own kind. */
function normalizeKind(kind: unknown): CanvasEntityKind | null {
  if (kind === 'sticky-note') return 'text'
  return typeof kind === 'string' ? (kind as CanvasEntityKind) : null
}

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T
}

/**
 * Bucket a tab's persisted entities by kind, in `entityOrder` so each kind's
 * store rebuilds in its persisted stack order. Pages are skipped: they are
 * carried through the swap as snapshot data and never instantiated.
 */
function entityBucketsFor(snapshot: WorkspaceSnapshot): Map<CanvasEntityKind, Record<string, unknown>[]> {
  const buckets = new Map<CanvasEntityKind, Record<string, unknown>[]>()
  const entities = snapshot.entities ?? {}
  const seen = new Set<string>()
  const push = (id: string, entity: unknown): void => {
    if (seen.has(id)) return
    seen.add(id)
    const kind = normalizeKind((entity as { kind?: unknown }).kind)
    if (!kind || kind === 'page') return
    const bucket = buckets.get(kind)
    const data = clone(entity) as Record<string, unknown>
    data.kind = kind
    if (bucket) bucket.push(data)
    else buckets.set(kind, [data])
  }
  for (const id of snapshot.entityOrder ?? []) {
    if (entities[id]) push(id, entities[id])
  }
  for (const [id, entity] of Object.entries(entities)) push(id, entity)
  // Older snapshots carry groups only in the dedicated `groups` array.
  for (const group of snapshot.groups ?? []) push(group.id, { ...group, kind: 'group' })
  return buckets
}

function hydrateTargetTab(tab: PersistedWorkspaceTab, doc: Y.Doc): void {
  const buckets = entityBucketsFor(tab.snapshot)
  forEachEntityKind((def) => {
    if (def.kind === 'page') return
    def.restore(buckets.get(def.kind) ?? [])
  })
  workspaceEdges.push(...clone(tab.snapshot.edges ?? []))
  workspaceAnnotations.push(...cloneAnnotationsForPersistence(tab.annotations))
  doc.transact(() => hydrateDocFromSnapshot(doc, tab.snapshot), 'background-tab-hydrate')
}

// ---------------------------------------------------------------------------
// Sync back
// ---------------------------------------------------------------------------

/**
 * Fold the swapped runtime arrays back into the target's snapshot. Only the
 * entity data is taken from runtime — viewport, selection, panel state, and
 * the page list come from the tab's own snapshot, because those runtime
 * variables still describe the canvas the user is looking at.
 */
function mergeIntoTabSnapshot(original: WorkspaceSnapshot): WorkspaceSnapshot {
  const live = workspaceSnapshot()
  const pageEntities = Object.fromEntries(
    Object.entries(original.entities ?? {}).filter(([, entity]) => entity?.kind === 'page'),
  )
  const entities = { ...pageEntities, ...(live.entities ?? {}) }
  const known = new Set([...Object.keys(entities), ...(live.edges ?? []).map((edge) => edge.id)])
  const entityOrder: string[] = []
  const seen = new Set<string>()
  for (const id of [...(original.entityOrder ?? []), ...(live.entityOrder ?? [])]) {
    if (!known.has(id) || seen.has(id)) continue
    seen.add(id)
    entityOrder.push(id)
  }
  for (const id of known) {
    if (seen.has(id)) continue
    seen.add(id)
    entityOrder.push(id)
  }
  return {
    ...cloneWorkspaceSnapshot(original),
    groups: clone(live.groups ?? []),
    edges: clone(live.edges ?? []),
    entities: clone(entities),
    entityOrder,
  }
}

// ---------------------------------------------------------------------------
// The context
// ---------------------------------------------------------------------------

export interface TabContextOptions {
  /**
   * Whether to fold the result back into the tab's record and rewrite its
   * `.canvas` file. Reads (placement math, serialization) pass `false` so a
   * lookup never bumps the tab on disk.
   */
  commit?: boolean
}

/**
 * Run `fn` with `tabId`'s canvas swapped into the runtime arrays.
 *
 * `fn` MUST be synchronous — see the module header. Targeting the active tab
 * is a no-op passthrough, so callers can hand a resolved ref straight in.
 */
export function withTabContext<T>(tabId: string, fn: () => T, options?: TabContextOptions): T {
  const tab = workspaceTabs.find((candidate) => candidate.id === tabId)
  if (!tab) throw new Error(`unknown tab '${tabId}'`)
  if (tab.id === activeWorkspaceTabId) return fn()

  const commit = options?.commit !== false

  // Bring the active tab's record up to date before anything else moves. The
  // runtime arrays themselves are lifted out by reference and put back, so
  // this is not what makes the swap reversible — it keeps the record from
  // being the stale one if the target write is followed by a flush that
  // rewrites every tab from its record.
  syncActiveTabRecord()

  return withWorkspacePersistenceSuspended(() => {
    const saved = liftRuntimeState()
    const detached = createWorkspaceDoc()
    const restoreContext = backgroundContext
    backgroundContext = { tabId: tab.id, pageIds: pageIdsIn(tab.snapshot) }
    try {
      // Sync stays suppressed across the swap in and the swap out — the
      // rebuild is not a mutation. It is deliberately live while `fn` runs so
      // the write lands as one real transaction in the detached doc.
      withSuppressedDocSync(() => {
        setActiveDoc(detached)
        setActiveUndoManager(null)
        hydrateTargetTab(tab, detached)
      })

      const result = fn()

      if (commit) {
        // Disk first, record second. If the write throws (full disk, bad
        // permissions) the tab record still describes what is on disk, so the
        // next autosave has nothing phantom to preserve.
        const snapshot = mergeIntoTabSnapshot(tab.snapshot)
        const annotations = cloneAnnotationsForPersistence(workspaceAnnotations)
        const updatedAt = new Date().toISOString()
        writeTabAsCanvasFile(spaceDir(), {
          ...tab,
          snapshot,
          annotations,
          updatedAt,
        })
        tab.snapshot = snapshot
        tab.annotations = annotations
        tab.updatedAt = updatedAt
      }
      return result
    } finally {
      backgroundContext = restoreContext
      withSuppressedDocSync(() => restoreRuntimeState(saved))
      // The detached doc carries observers; an AFK run does hundreds of these.
      detached.destroy()
    }
  })
}
