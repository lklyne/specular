import * as Y from 'yjs'
import type {
  WorkspaceEdge,
  WorkspaceGroup,
  WorkspacePageSnapshot,
  WorkspaceSnapshot,
} from '../../shared/types'

// ---------------------------------------------------------------------------
// Y.Doc map names (single source of truth for all map/array keys)
// ---------------------------------------------------------------------------

export const DOC_MAP_VIEWPORT = 'viewport'
export const DOC_MAP_PAGES = 'pages'
export const DOC_MAP_GROUPS = 'groups'
export const DOC_MAP_EDGES = 'edges'
export const DOC_MAP_ANNOTATIONS = 'annotations'
export const DOC_MAP_ENTITIES = 'entities'
export const DOC_MAP_WORKSPACE = 'workspace'
export const DOC_ARRAY_ENTITY_ORDER = 'entityOrder'

/** All entity-related Y.Map names (excludes entityOrder, viewport, workspace) */
export const DOC_ENTITY_MAP_NAMES = [
  DOC_MAP_PAGES,
  DOC_MAP_GROUPS,
  DOC_MAP_EDGES,
  DOC_MAP_ANNOTATIONS,
  DOC_MAP_ENTITIES,
] as const

/** All Y.Map names */
export const DOC_ALL_MAP_NAMES = [
  DOC_MAP_VIEWPORT,
  ...DOC_ENTITY_MAP_NAMES,
  DOC_MAP_WORKSPACE,
] as const

// ---------------------------------------------------------------------------
// Y.Doc lifecycle
// ---------------------------------------------------------------------------

let activeDoc: Y.Doc | null = null

export function createWorkspaceDoc(): Y.Doc {
  const doc = new Y.Doc()
  for (const name of DOC_ALL_MAP_NAMES) doc.getMap(name)
  doc.getArray(DOC_ARRAY_ENTITY_ORDER)
  return doc
}

export function getActiveDoc(): Y.Doc {
  if (!activeDoc) {
    activeDoc = createWorkspaceDoc()
  }
  return activeDoc
}

export function setActiveDoc(doc: Y.Doc): void {
  activeDoc = doc
}

// ---------------------------------------------------------------------------
// Typed accessors — workspace metadata (activeTabId, tab list)
// ---------------------------------------------------------------------------

export interface DocTabEntry {
  id: string
  name: string
}

export function getDocActiveTabId(doc: Y.Doc): string | null {
  return (doc.getMap(DOC_MAP_WORKSPACE).get('activeTabId') as string) ?? null
}

export function setDocActiveTabId(doc: Y.Doc, tabId: string): void {
  doc.getMap(DOC_MAP_WORKSPACE).set('activeTabId', tabId)
}

export function getDocTabList(doc: Y.Doc): DocTabEntry[] {
  const arr = doc.getMap(DOC_MAP_WORKSPACE).get('tabs') as unknown[] | undefined
  return (arr as DocTabEntry[]) ?? []
}

export function setDocTabList(doc: Y.Doc, tabs: DocTabEntry[]): void {
  doc.getMap(DOC_MAP_WORKSPACE).set('tabs', tabs)
}

// ---------------------------------------------------------------------------
// Y.Map construction helpers
// ---------------------------------------------------------------------------

function objectToYMap(obj: Record<string, unknown>): Y.Map<unknown> {
  const ymap = new Y.Map<unknown>()
  for (const [k, v] of Object.entries(obj)) {
    if (v !== undefined) ymap.set(k, v)
  }
  return ymap
}

// ---------------------------------------------------------------------------
// Snapshot → Y.Doc (hydrate from existing JSON persistence)
// ---------------------------------------------------------------------------

/** Write snapshot state into Y.Doc maps. Caller must wrap in doc.transact(). */
export function hydrateDocFromSnapshot(doc: Y.Doc, snapshot: WorkspaceSnapshot): void {
  const viewport = doc.getMap(DOC_MAP_VIEWPORT)
  viewport.set('zoom', snapshot.zoom)
  viewport.set('pan', snapshot.pan)

  const yPages = doc.getMap(DOC_MAP_PAGES) as Y.Map<Y.Map<unknown>>
  const entityOrder = doc.getArray<string>(DOC_ARRAY_ENTITY_ORDER)
  const yEntities = doc.getMap(DOC_MAP_ENTITIES) as Y.Map<Y.Map<unknown>>

  for (const page of snapshot.pages) {
    if (!page.id) continue
    const pageData: Record<string, unknown> = {
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
    }
    yPages.set(page.id, objectToYMap(pageData))
  }

  if (snapshot.entities) {
    for (const [id, entity] of Object.entries(snapshot.entities)) {
      if (entity.kind === 'page') {
        if (!yPages.has(id)) {
          yPages.set(id, objectToYMap(entity as unknown as Record<string, unknown>))
        }
      } else {
        yEntities.set(id, objectToYMap(entity as unknown as Record<string, unknown>))
      }
    }
  }

  if (snapshot.entityOrder) {
    entityOrder.push(snapshot.entityOrder)
  }

  const yGroups = doc.getMap(DOC_MAP_GROUPS) as Y.Map<Y.Map<unknown>>
  if (snapshot.groups) {
    for (const group of snapshot.groups) {
      yGroups.set(group.id, objectToYMap(group as unknown as Record<string, unknown>))
    }
  }

  const yEdges = doc.getMap(DOC_MAP_EDGES) as Y.Map<Y.Map<unknown>>
  if (snapshot.edges) {
    for (const edge of snapshot.edges) {
      yEdges.set(edge.id, objectToYMap(edge as unknown as Record<string, unknown>))
    }
  }
}

// ---------------------------------------------------------------------------
// Suppress flag — prevents doc sync during restore or undo observer
// ---------------------------------------------------------------------------

let _suppressDocSync = false

export function isDocSyncSuppressed(): boolean {
  return _suppressDocSync
}

export function withSuppressedDocSync<T>(fn: () => T): T {
  _suppressDocSync = true
  try {
    return fn()
  } finally {
    _suppressDocSync = false
  }
}

// ---------------------------------------------------------------------------
// Diff-sync: runtime state → Y.Doc (called after each mutation)
// ---------------------------------------------------------------------------

export function syncRuntimeToDoc(
  doc: Y.Doc,
  runtime: {
    pages: ReadonlyArray<{ id: string }>
    textEntities: ReadonlyArray<{ id: string; kind?: string }>
    fileEntities: ReadonlyArray<{ id: string; kind?: string }>
    drawingEntities: ReadonlyArray<{ id: string; kind?: string }>
    shapeEntities: ReadonlyArray<{ id: string; kind?: string }>
    workspaceGroups: ReadonlyArray<{ id: string }>
    workspaceEdges: ReadonlyArray<{ id: string }>
    workspaceAnnotations: ReadonlyArray<{ id: string }>
    zoom: number
    pan: { x: number; y: number }
    activeTabId?: string | null
    workspaceTabs?: ReadonlyArray<{ id: string; name: string }>
  },
  serializePage: (page: { id: string }) => Record<string, unknown>,
): void {
  if (_suppressDocSync) return

  doc.transact(() => {
    const viewport = doc.getMap(DOC_MAP_VIEWPORT)
    if (viewport.get('zoom') !== runtime.zoom) viewport.set('zoom', runtime.zoom)
    const currentPan = viewport.get('pan') as { x: number; y: number } | undefined
    if (!currentPan || currentPan.x !== runtime.pan.x || currentPan.y !== runtime.pan.y) {
      viewport.set('pan', { x: runtime.pan.x, y: runtime.pan.y })
    }

    syncMapFromArray(
      doc.getMap(DOC_MAP_PAGES) as Y.Map<Y.Map<unknown>>,
      runtime.pages,
      serializePage,
    )

    const allEntities = [
      ...runtime.textEntities.map((e) => ({ ...e, kind: 'text' as const })),
      ...runtime.fileEntities.map((e) => ({ ...e, kind: 'file' as const })),
      ...runtime.drawingEntities.map((e) => ({ ...e, kind: 'drawing' as const })),
      ...runtime.shapeEntities.map((e) => ({ ...e, kind: 'shape' as const })),
    ]
    syncMapFromArray(
      doc.getMap(DOC_MAP_ENTITIES) as Y.Map<Y.Map<unknown>>,
      allEntities,
      (e) => e as Record<string, unknown>,
    )

    syncMapFromArray(
      doc.getMap(DOC_MAP_GROUPS) as Y.Map<Y.Map<unknown>>,
      runtime.workspaceGroups,
      (g) => g as Record<string, unknown>,
    )

    syncMapFromArray(
      doc.getMap(DOC_MAP_EDGES) as Y.Map<Y.Map<unknown>>,
      runtime.workspaceEdges,
      (e) => e as Record<string, unknown>,
    )

    syncMapFromArray(
      doc.getMap(DOC_MAP_ANNOTATIONS) as Y.Map<Y.Map<unknown>>,
      runtime.workspaceAnnotations,
      (a) => a as Record<string, unknown>,
    )

    syncEntityOrder(doc, runtime)

    if (runtime.activeTabId) {
      const workspace = doc.getMap(DOC_MAP_WORKSPACE)
      if (workspace.get('activeTabId') !== runtime.activeTabId) {
        workspace.set('activeTabId', runtime.activeTabId)
      }
      if (runtime.workspaceTabs) {
        const tabs = runtime.workspaceTabs.map((tab) => ({ id: tab.id, name: tab.name }))
        if (JSON.stringify(workspace.get('tabs') ?? []) !== JSON.stringify(tabs)) {
          workspace.set('tabs', tabs)
        }
      }
    }
  }, 'user')
}

function syncMapFromArray<T extends { id: string }>(
  ymap: Y.Map<Y.Map<unknown>>,
  runtimeArray: ReadonlyArray<T>,
  serialize: (item: T) => Record<string, unknown>,
): void {
  const runtimeIds = new Set<string>()
  for (const item of runtimeArray) {
    runtimeIds.add(item.id)
    const data = serialize(item)
    const existing = ymap.get(item.id)
    if (!existing) {
      ymap.set(item.id, objectToYMap(data))
    } else {
      for (const [k, v] of Object.entries(data)) {
        if (v === undefined) continue
        const current = existing.get(k)
        if (typeof v === 'object' && v !== null) {
          if (JSON.stringify(current) !== JSON.stringify(v)) {
            existing.set(k, v)
          }
        } else if (current !== v) {
          existing.set(k, v)
        }
      }
    }
  }
  for (const id of ymap.keys()) {
    if (!runtimeIds.has(id)) {
      ymap.delete(id)
    }
  }
}

function syncEntityOrder(
  doc: Y.Doc,
  runtime: {
    pages: ReadonlyArray<{ id: string }>
    textEntities: ReadonlyArray<{ id: string }>
    fileEntities: ReadonlyArray<{ id: string }>
    drawingEntities: ReadonlyArray<{ id: string }>
    shapeEntities: ReadonlyArray<{ id: string }>
    workspaceGroups: ReadonlyArray<{ id: string }>
    workspaceEdges: ReadonlyArray<{ id: string }>
  },
): void {
  const order = doc.getArray<string>(DOC_ARRAY_ENTITY_ORDER)
  const defaultOrder = [
    ...runtime.pages.map((p) => p.id),
    ...runtime.textEntities.map((e) => e.id),
    ...runtime.fileEntities.map((e) => e.id),
    ...runtime.drawingEntities.map((e) => e.id),
    ...runtime.shapeEntities.map((e) => e.id),
    ...runtime.workspaceGroups.map((g) => g.id),
    ...runtime.workspaceEdges.map((e) => e.id),
  ]
  const currentIds = new Set(defaultOrder)
  const currentOrder = order.toArray()
  const seen = new Set<string>()
  const desiredOrder: string[] = []
  for (const id of currentOrder) {
    if (!currentIds.has(id) || seen.has(id)) continue
    seen.add(id)
    desiredOrder.push(id)
  }
  for (const id of defaultOrder) {
    if (seen.has(id)) continue
    seen.add(id)
    desiredOrder.push(id)
  }
  if (JSON.stringify(currentOrder) !== JSON.stringify(desiredOrder)) {
    order.delete(0, order.length)
    if (desiredOrder.length) order.push(desiredOrder)
  }
}
