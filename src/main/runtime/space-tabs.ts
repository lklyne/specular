import type {
  PersistedWorkspaceRecord,
  PersistedWorkspaceTab,
  WorkspaceSnapshot,
  SpaceTabSummary,
} from '../../shared/types'
import type { JsonCanvasTabIdentity } from '../../shared/json-canvas-types'
import {
  pages,
  zoom,
  pan,
} from './runtime-context'
import {
  activeSpaceTabId,
  spaceTabs,
  setActiveSpaceTabId,
  workspaceAnnotations,
  workspaceGroups,
  workspaceEdges,
} from './space-model'
import {
  devtoolsPanelTab as uiDevtoolsPanelTab,
  devtoolsWidth as uiDevtoolsWidth,
  leftSidebarOpen as uiLeftSidebarOpen,
  selectedPageIndex as uiSelectedPageIndex,
  selectedEntityIds as uiSelectedEntityIds,
  selectedGroupId as uiSelectedGroupId,
  devtoolsOpen as uiDevtoolsOpen,
} from '../ui-state'
import {
  buildSpaceTabSummary,
  makeSpaceTabId,
  DEFAULT_TAB_NAME,
  buildPersistedWorkspaceRecord as createPersistedWorkspaceRecord,
  makeEmptyWorkspaceSnapshot,
  buildWorkspaceSnapshot,
  buildPageSnapshot,
} from './space-persistence'
import {
  cloneAnnotationsForPersistence,
  cloneWorkspaceSnapshot,
} from './runtime-serialization'
import { allEntities, getEntityKind } from '../entities/contract'
import { DOC_ARRAY_ENTITY_ORDER, getActiveDoc } from './space-doc'

// fallow-ignore-next-line complexity
export function spaceSnapshot(): WorkspaceSnapshot {
  const pageIds = pages.map((p) => p.id)
  const selectedIndex = uiSelectedPageIndex(pageIds)
  const selectedPageId =
    selectedIndex !== null && selectedIndex >= 0 && selectedIndex < pages.length
      ? pages[selectedIndex].id
      : null

  const snapshot = buildWorkspaceSnapshot({
    zoom,
    pan,
    pages: pages.map((page) =>
      buildPageSnapshot({
        id: page.id,
        name: page.name,
        url: page.pageView.webContents.getURL() || 'about:blank',
        presetIndex: page.presetIndex,
        canvasX: page.canvasX,
        canvasY: page.canvasY,
        syncId: page.syncId,
        source: page.source,
        parentGroupId: page.parentGroupId ?? page.groupId,
        groupId: page.parentGroupId ?? page.groupId,
        metadata: page.metadata,
        colorScheme: page.colorScheme,
      }),
    ),
    selectedPageIndex: uiSelectedPageIndex(pageIds),
    selectedPageId,
    selectedPageIds: uiSelectedEntityIds(),
    selectedGroupId: uiSelectedGroupId(),
    leftSidebarOpen: uiLeftSidebarOpen(),
    devtoolsOpen: uiDevtoolsOpen(),
    devtoolsPanelTab: uiDevtoolsPanelTab(),
    devtoolsWidth: uiDevtoolsWidth(),
    groups: workspaceGroups,
    edges: workspaceEdges,
  })
  // Every registered kind but `page` (which mirrors to `snapshot.pages`
  // above) walked once through its own `persist` projection, replacing what
  // used to be five copy-pasted per-kind loops — one per map-backed kind
  // plus group. `allEntities()` walks registration order (text, file,
  // drawing, shape, group after page), matching the original loop order.
  for (const { kind, entity } of allEntities()) {
    if (kind === 'page') continue
    const persisted = getEntityKind(kind).persist!(entity)
    if (!snapshot.entities) snapshot.entities = {}
    if (!snapshot.entityOrder) snapshot.entityOrder = []
    snapshot.entities[persisted.id] = persisted
    snapshot.entityOrder.push(persisted.id)
  }
  if (snapshot.entities) {
    const currentIds = new Set([
      ...Object.keys(snapshot.entities),
      ...workspaceEdges.map((edge) => edge.id),
    ])
    const seen = new Set<string>()
    const ordered: string[] = []
    for (const id of getActiveDoc().getArray<string>(DOC_ARRAY_ENTITY_ORDER).toArray()) {
      if (!currentIds.has(id) || seen.has(id)) continue
      seen.add(id)
      ordered.push(id)
    }
    for (const id of snapshot.entityOrder ?? []) {
      if (seen.has(id)) continue
      seen.add(id)
      ordered.push(id)
    }
    for (const edge of workspaceEdges) {
      if (seen.has(edge.id)) continue
      seen.add(edge.id)
      ordered.push(edge.id)
    }
    snapshot.entityOrder = ordered
  }
  return snapshot
}

export function makeEmptyTabSnapshot(): WorkspaceSnapshot {
  return makeEmptyWorkspaceSnapshot({
    leftSidebarOpen: uiLeftSidebarOpen(),
    devtoolsPanelTab: uiDevtoolsPanelTab(),
    devtoolsWidth: uiDevtoolsWidth(),
  })
}

export function syncActiveTabRecord(): void {
  if (!activeSpaceTabId || !spaceTabs.length) return
  const tab = spaceTabs.find((candidate) => candidate.id === activeSpaceTabId)
  if (!tab) return
  tab.updatedAt = new Date().toISOString()
  tab.snapshot = cloneWorkspaceSnapshot(spaceSnapshot())
  tab.annotations = cloneAnnotationsForPersistence(workspaceAnnotations)
}

function buildTabSummary(tab: PersistedWorkspaceTab): SpaceTabSummary {
  return buildSpaceTabSummary(tab, activeSpaceTabId)
}

export function ensureSpaceTabsInitialized(): void {
  if (spaceTabs.length) return
  const now = new Date().toISOString()
  const id = makeSpaceTabId()
  spaceTabs.push({
    id,
    name: DEFAULT_TAB_NAME,
    updatedAt: now,
    snapshot: makeEmptyTabSnapshot(),
    annotations: [],
    expanded: true,
  })
  setActiveSpaceTabId(id)
}

export function spaceTabSummaries(): SpaceTabSummary[] {
  syncActiveTabRecord()
  return spaceTabs.map(buildTabSummary)
}

export interface SpaceTabIdentity {
  activeTab: { id: string; name: string } | null
  tabs: JsonCanvasTabIdentity[]
}

/** Which canvas a read answered from, and what else is open. `spaceTabSummaries()`
 *  syncs the active tab record first, so its `entityCount` reflects live runtime
 *  state rather than the last persisted snapshot. */
export function spaceTabIdentity(): SpaceTabIdentity {
  ensureSpaceTabsInitialized()
  const summaries = spaceTabSummaries()
  const active = summaries.find((tab) => tab.isActive) ?? summaries[0]
  return {
    activeTab: active ? { id: active.id, name: active.name } : null,
    tabs: summaries.map(({ id, name, entityCount }) => ({ id, name, entityCount })),
  }
}

export function activeSpaceTabSummary(): SpaceTabSummary | null {
  ensureSpaceTabsInitialized()
  const active = spaceTabs.find((tab) => tab.id === activeSpaceTabId) ?? spaceTabs[0]
  return active ? buildTabSummary(active) : null
}

export function buildPersistedWorkspaceRecord(): PersistedWorkspaceRecord {
  syncActiveTabRecord()
  if (!spaceTabs.length) {
    const now = new Date().toISOString()
    spaceTabs.push({
      id: makeSpaceTabId(),
      name: DEFAULT_TAB_NAME,
      updatedAt: now,
      snapshot: cloneWorkspaceSnapshot(spaceSnapshot()),
      annotations: cloneAnnotationsForPersistence(workspaceAnnotations),
      expanded: true,
    })
  }
  if (!activeSpaceTabId || !spaceTabs.some((tab) => tab.id === activeSpaceTabId)) {
    setActiveSpaceTabId(spaceTabs[0]?.id ?? null)
  }
  return createPersistedWorkspaceRecord({
    spaceTabs,
    activeSpaceTabId: activeSpaceTabId ?? spaceTabs[0]!.id,
  })
}

export function currentPersistedWorkspaceRecord(): PersistedWorkspaceRecord {
  return buildPersistedWorkspaceRecord()
}
