/**
 * Page ↔ Y.Doc projection.
 *
 * Pages are hybrid entities: their serializable fields mirror to the doc's
 * pages map while the WebContentsView stays in runtime state, so undo patches
 * live page objects instead of rebuilding the array. `PAGE_DOC_FIELD_SET` is
 * the single declaration both sync directions derive from: `persistPage` must
 * write every field and `PAGE_RESTORE_PATCHERS` must decide per field how
 * undo applies it — both compile-enforced, so a new persisted field cannot
 * silently skip either direction.
 */

import type { PersistedPageEntity } from '../../shared/types'
import type { Page } from './runtime-entities'

// `kind` is implicit in the pages map (the map itself is the kind); `groupId`
// is the legacy spelling folded into `parentGroupId` at write time.
const PAGE_DOC_FIELD_SET = {
  id: true,
  name: true,
  url: true,
  presetIndex: true,
  canvasX: true,
  canvasY: true,
  linked: true,
  source: true,
  parentGroupId: true,
  metadata: true,
} as const satisfies Record<Exclude<keyof PersistedPageEntity, 'kind' | 'groupId'>, true>

type PageDocField = keyof typeof PAGE_DOC_FIELD_SET

export const PAGE_PERSISTED_FIELDS = Object.keys(PAGE_DOC_FIELD_SET) as readonly PageDocField[]

/** Project a live page to its doc pages-map record (the forward-sync shape). */
export function persistPage(page: Page): Record<string, unknown> {
  return {
    id: page.id,
    name: page.name,
    url: page.url,
    presetIndex: page.presetIndex,
    canvasX: page.canvasX,
    canvasY: page.canvasY,
    linked: page.linked,
    source: page.source,
    parentGroupId: page.parentGroupId ?? page.groupId,
    metadata: page.metadata,
  } satisfies Record<PageDocField, unknown>
}

type PagePatcher = (page: Page, value: unknown) => void

/**
 * Per-field undo behavior for live pages. `null` marks fields that persist
 * but are deliberately NOT patched on undo:
 * - `id` is the reconciliation key — add/remove, never patched in place
 * - `url` — patching would navigate the live WebContents mid-undo
 * - `source` is create-time provenance
 * Geometry, preset, and linked keep their current value when the doc lacks
 * the key; name and group membership overwrite so undo can clear them;
 * metadata applies only when present.
 */
const PAGE_RESTORE_PATCHERS = {
  id: null,
  url: null,
  source: null,
  name: (page, value) => {
    page.name = value as string | undefined
  },
  presetIndex: (page, value) => {
    page.presetIndex = (value as number) ?? page.presetIndex
  },
  canvasX: (page, value) => {
    page.canvasX = (value as number) ?? page.canvasX
  },
  canvasY: (page, value) => {
    page.canvasY = (value as number) ?? page.canvasY
  },
  linked: (page, value) => {
    page.linked = (value as boolean) ?? page.linked
  },
  parentGroupId: (page, value) => {
    page.parentGroupId = value as string | undefined
  },
  metadata: (page, value) => {
    if (value !== undefined) {
      page.metadata = value as Record<string, unknown> | undefined
    }
  },
} satisfies Record<PageDocField, PagePatcher | null>

/**
 * The live page store `restorePagesFromDoc` reconciles against. `pages` must
 * be the live array (created pages are patched in the same pass).
 */
export interface PageRestoreStore {
  pages: readonly Page[]
  createPage: (data: Record<string, unknown>) => void
  removePageById: (id: string) => void
}

/**
 * Reconcile live pages against the doc's pages-map snapshots after undo:
 * remove deleted, recreate restored, then patch surviving pages field by
 * field through `PAGE_RESTORE_PATCHERS`.
 */
export function restorePagesFromDoc(
  snapshots: readonly Record<string, unknown>[],
  store: PageRestoreStore,
): void {
  const docPages = new Map(snapshots.map((snapshot) => [snapshot.id as string, snapshot]))
  const runtimePageIds = new Set(store.pages.map((page) => page.id))

  for (const page of [...store.pages]) {
    if (!docPages.has(page.id)) {
      store.removePageById(page.id)
    }
  }

  for (const [id, data] of docPages) {
    if (!runtimePageIds.has(id)) {
      store.createPage(data)
    }
  }

  for (const page of store.pages) {
    const data = docPages.get(page.id)
    if (!data) continue
    for (const field of PAGE_PERSISTED_FIELDS) {
      PAGE_RESTORE_PATCHERS[field]?.(page, data[field])
    }
  }
}
