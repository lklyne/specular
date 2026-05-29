import type {
  PersistedWorkspaceRecord,
  PersistedWorkspaceTab,
  WorkspaceSnapshot,
  WorkspaceTabSummary,
} from '../../shared/types'
import {
  pages,
  zoom,
  pan,
} from './runtime-context'
import {
  activeWorkspaceTabId,
  workspaceTabs,
  setActiveWorkspaceTabId,
  workspaceAnnotations,
  workspaceGroups,
  workspaceEdges,
} from './workspace-model'
import {
  devtoolsPanelTab as uiDevtoolsPanelTab,
  devtoolsWidth as uiDevtoolsWidth,
  leftSidebarOpen as uiLeftSidebarOpen,
  selectedPageIndex as uiSelectedPageIndex,
  selectedEntityIds as uiSelectedEntityIds,
  selectedGroupId as uiSelectedGroupId,
  devtoolsOpen as uiDevtoolsOpen,
  workspaceViewMode as uiWorkspaceViewMode,
} from '../ui-state'
import {
  buildWorkspaceTabSummary,
  makeWorkspaceTabId,
  DEFAULT_TAB_NAME,
  buildPersistedWorkspaceRecord as createPersistedWorkspaceRecord,
  makeEmptyWorkspaceSnapshot,
  buildWorkspaceSnapshot,
  buildPageSnapshot,
} from './workspace-persistence'
import {
  cloneAnnotationsForPersistence,
  cloneWorkspaceSnapshot,
} from './runtime-serialization'
import {
  textEntities,
  persistTextEntity,
} from './text-entity-state'
import {
  fileEntities,
  persistFileEntity,
} from './file-entity-state'
import {
  drawingEntities,
  persistDrawingEntity,
} from './drawing-entity-state'
import {
  shapeEntities,
  persistShapeEntity,
} from './shape-entity-state'
import { persistGroupEntity } from './group-entity-state'
import { DOC_ARRAY_ENTITY_ORDER, getActiveDoc } from './workspace-doc'

export function workspaceSnapshot(): WorkspaceSnapshot {
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
        linked: page.linked,
        source: page.source,
        parentGroupId: page.parentGroupId ?? page.groupId,
        groupId: page.parentGroupId ?? page.groupId,
        metadata: page.metadata,
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
    browserTabMode: 'page',
    groups: workspaceGroups,
    edges: workspaceEdges,
  })
  // Add text entities to the entity store
  for (const te of textEntities) {
    const entity = persistTextEntity(te)
    if (!snapshot.entities) snapshot.entities = {}
    if (!snapshot.entityOrder) snapshot.entityOrder = []
    snapshot.entities[entity.id] = entity
    snapshot.entityOrder.push(entity.id)
  }
  // Add file entities to the entity store
  for (const fe of fileEntities) {
    const entity = persistFileEntity(fe)
    if (!snapshot.entities) snapshot.entities = {}
    if (!snapshot.entityOrder) snapshot.entityOrder = []
    snapshot.entities[entity.id] = entity
    snapshot.entityOrder.push(entity.id)
  }
  for (const de of drawingEntities) {
    const entity = persistDrawingEntity(de)
    if (!snapshot.entities) snapshot.entities = {}
    if (!snapshot.entityOrder) snapshot.entityOrder = []
    snapshot.entities[entity.id] = entity
    snapshot.entityOrder.push(entity.id)
  }
  for (const se of shapeEntities) {
    const entity = persistShapeEntity(se)
    if (!snapshot.entities) snapshot.entities = {}
    if (!snapshot.entityOrder) snapshot.entityOrder = []
    snapshot.entities[entity.id] = entity
    snapshot.entityOrder.push(entity.id)
  }
  for (const group of workspaceGroups) {
    const entity = persistGroupEntity(group)
    if (!snapshot.entities) snapshot.entities = {}
    if (!snapshot.entityOrder) snapshot.entityOrder = []
    snapshot.entities[entity.id] = entity
    snapshot.entityOrder.push(entity.id)
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
  if (!activeWorkspaceTabId || !workspaceTabs.length) return
  const tab = workspaceTabs.find((candidate) => candidate.id === activeWorkspaceTabId)
  if (!tab) return
  tab.updatedAt = new Date().toISOString()
  tab.snapshot = cloneWorkspaceSnapshot(workspaceSnapshot())
  tab.annotations = cloneAnnotationsForPersistence(workspaceAnnotations)
}

function buildTabSummary(tab: PersistedWorkspaceTab): WorkspaceTabSummary {
  return buildWorkspaceTabSummary(tab, activeWorkspaceTabId)
}

export function ensureWorkspaceTabsInitialized(): void {
  if (workspaceTabs.length) return
  const now = new Date().toISOString()
  const id = makeWorkspaceTabId()
  workspaceTabs.push({
    id,
    name: DEFAULT_TAB_NAME,
    updatedAt: now,
    snapshot: makeEmptyTabSnapshot(),
    annotations: [],
    expanded: true,
  })
  setActiveWorkspaceTabId(id)
}

export function workspaceTabSummaries(): WorkspaceTabSummary[] {
  syncActiveTabRecord()
  return workspaceTabs.map(buildTabSummary)
}

export function activeWorkspaceTabSummary(): WorkspaceTabSummary | null {
  ensureWorkspaceTabsInitialized()
  const active = workspaceTabs.find((tab) => tab.id === activeWorkspaceTabId) ?? workspaceTabs[0]
  return active ? buildTabSummary(active) : null
}

export function buildPersistedWorkspaceRecord(): PersistedWorkspaceRecord {
  syncActiveTabRecord()
  if (!workspaceTabs.length) {
    const now = new Date().toISOString()
    workspaceTabs.push({
      id: makeWorkspaceTabId(),
      name: DEFAULT_TAB_NAME,
      updatedAt: now,
      snapshot: cloneWorkspaceSnapshot(workspaceSnapshot()),
      annotations: cloneAnnotationsForPersistence(workspaceAnnotations),
      expanded: true,
    })
  }
  if (!activeWorkspaceTabId || !workspaceTabs.some((tab) => tab.id === activeWorkspaceTabId)) {
    setActiveWorkspaceTabId(workspaceTabs[0]?.id ?? null)
  }
  return createPersistedWorkspaceRecord({
    workspaceTabs,
    activeWorkspaceTabId: activeWorkspaceTabId ?? workspaceTabs[0]!.id,
    viewMode: uiWorkspaceViewMode(),
  })
}

export function currentPersistedWorkspaceRecord(): PersistedWorkspaceRecord {
  return buildPersistedWorkspaceRecord()
}
