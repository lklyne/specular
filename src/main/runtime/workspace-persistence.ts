import { randomUUID } from 'crypto'
import { existsSync, mkdirSync, readFileSync, renameSync, unlinkSync, writeFileSync } from 'fs'
import { join } from 'path'
import type {
  Annotation,
  DevtoolsPanelTab,
  LegacyPersistedWorkspaceStore,
  PageColorScheme,
  PersistedCanvasEntity,
  PersistedPageEntity,
  PersistedWorkspaceRecord,
  PersistedWorkspaceStore,
  PersistedWorkspaceTab,
  WorkspaceEdge,
  WorkspacePageSource,
  WorkspaceGroup,
  WorkspacePageSnapshot,
  WorkspaceSnapshot,
  WorkspaceTabSummary,
  WorkspaceViewMode,
} from '../../shared/types'
import {
  cloneAnnotationsForPersistence,
  cloneWorkspaceSnapshot,
  pageDisplayLabel,
  normalizePresetIndex,
  viewportPresetForIndex,
} from './runtime-serialization'
import { pageCustomSizeFromMetadata } from './runtime-entities'

const WORKSPACE_STORE_FILE = 'workspace-store.json'
export const WORKSPACE_STORE_VERSION = 2 as const
export const DEFAULT_WORKSPACE_ID = 'default'
export const DEFAULT_WORKSPACE_NAME = 'Current Workspace'
export const DEFAULT_TAB_NAME = 'Canvas 1'
export const AUTOSAVE_DEBOUNCE_MS = 350

type PersistablePageSnapshotInput = {
  id: string
  name?: string
  title?: string
  url: string
  presetIndex: number
  canvasX: number
  canvasY: number
  syncId?: string | null
  source?: WorkspacePageSource
  parentGroupId?: string
  groupId?: string
  metadata?: Record<string, unknown>
  colorScheme?: PageColorScheme
}

type AutosaveSchedulerOptions = {
  autosaveTimer: NodeJS.Timeout | null
  shouldPersist: () => boolean
  setAutosaveTimer: (timer: NodeJS.Timeout | null) => void
  saveWorkspaceStore: () => void
}

export function workspaceStorePath(userDataPath: string): string {
  return join(userDataPath, WORKSPACE_STORE_FILE)
}

export function makeWorkspaceTabId(): string {
  return `tab_${randomUUID()}`
}

/** Canvas participants in a tab: entity-store entries plus any page carried
 *  only in the legacy `pages` array. Edges are not entities and don't count. */
function tabEntityCount(snapshot: WorkspaceSnapshot): number {
  const ids = new Set(Object.keys(snapshot.entities ?? {}))
  for (const page of snapshot.pages) {
    if (page.id) ids.add(page.id)
  }
  return ids.size
}

export function buildWorkspaceTabSummary(
  tab: PersistedWorkspaceTab,
  activeWorkspaceTabId: string | null,
): WorkspaceTabSummary {
  return {
    id: tab.id,
    name: tab.name,
    expanded: tab.expanded ?? true,
    isActive: tab.id === activeWorkspaceTabId,
    pageCount: tab.snapshot.pages.length,
    entityCount: tabEntityCount(tab.snapshot),
    pages: tab.snapshot.pages.map((page) => {
      const preset = viewportPresetForIndex(page.presetIndex)
      const customSize = pageCustomSizeFromMetadata(page.metadata)
      return {
        id: page.id ?? '',
        label: pageDisplayLabel(page),
        name: page.name?.trim() || undefined,
        url: page.url,
        presetIndex: normalizePresetIndex(page.presetIndex),
        faviconUrl: null,
        width: customSize?.width ?? preset?.width,
        height: customSize?.height ?? preset?.height,
      }
    }),
  }
}

export function buildPersistedWorkspaceRecord(params: {
  workspaceTabs: PersistedWorkspaceTab[]
  activeWorkspaceTabId: string
}): PersistedWorkspaceRecord {
  return {
    id: DEFAULT_WORKSPACE_ID,
    name: DEFAULT_WORKSPACE_NAME,
    updatedAt: new Date().toISOString(),
    activeTabId: params.activeWorkspaceTabId,
    tabs: params.workspaceTabs.map((tab) => ({
      ...tab,
      expanded: tab.expanded ?? true,
      snapshot: cloneWorkspaceSnapshot(tab.snapshot),
      annotations: cloneAnnotationsForPersistence(tab.annotations),
    })),
  }
}

export function buildWorkspaceStore(
  record: PersistedWorkspaceRecord,
): PersistedWorkspaceStore {
  return {
    version: WORKSPACE_STORE_VERSION,
    activeWorkspaceId: DEFAULT_WORKSPACE_ID,
    workspaces: [record],
  }
}

export function writeWorkspaceStoreSync(
  file: string,
  store: PersistedWorkspaceStore,
): void {
  const tmpFile = `${file}.tmp`
  writeFileSync(tmpFile, JSON.stringify(store, null, 2), 'utf8')
  renameSync(tmpFile, file)
}

export function loadWorkspaceStore(file: string): PersistedWorkspaceStore | null {
  if (!existsSync(file)) return null
  const parsed = JSON.parse(readFileSync(file, 'utf8')) as
    | PersistedWorkspaceStore
    | LegacyPersistedWorkspaceStore
  if (parsed.version === WORKSPACE_STORE_VERSION) {
    if (!Array.isArray(parsed.workspaces) || !parsed.workspaces.length) {
      return null
    }
    return parsed
  }
  if (parsed.version === 1) {
    return migrateLegacyWorkspaceStore(parsed)
  }
  console.warn(
    `Ignoring workspace store with unsupported version: ${String(
      (parsed as { version?: number }).version,
    )}`,
  )
  return null
}

export function migrateLegacyWorkspaceStore(
  legacy: LegacyPersistedWorkspaceStore,
): PersistedWorkspaceStore {
  return {
    version: WORKSPACE_STORE_VERSION,
    activeWorkspaceId: legacy.activeWorkspaceId,
    workspaces: legacy.workspaces.map((workspace) => {
      const tabId = makeWorkspaceTabId()
      return {
        id: workspace.id,
        name: workspace.name,
        updatedAt: workspace.updatedAt,
        activeTabId: tabId,
        tabs: [
          {
            id: tabId,
            name: DEFAULT_TAB_NAME,
            updatedAt: workspace.updatedAt,
            snapshot: cloneWorkspaceSnapshot(workspace.snapshot),
            annotations: cloneAnnotationsForPersistence(workspace.annotations),
            expanded: true,
          },
        ],
      }
    }),
  }
}

export function activePersistedWorkspace(
  store: PersistedWorkspaceStore,
): PersistedWorkspaceRecord | null {
  const byId = store.workspaces.find(
    (workspace) => workspace.id === store.activeWorkspaceId,
  )
  return byId ?? store.workspaces[0] ?? null
}

export function clonePersistedWorkspaceTabs(
  tabs: PersistedWorkspaceTab[],
): PersistedWorkspaceTab[] {
  return tabs.map((tab) => ({
    ...tab,
    snapshot: cloneWorkspaceSnapshot(tab.snapshot),
    annotations: cloneAnnotationsForPersistence(tab.annotations),
    expanded: tab.expanded ?? true,
  }))
}

export function makeEmptyWorkspaceSnapshot(params: {
  leftSidebarOpen: boolean
  devtoolsPanelTab: DevtoolsPanelTab
  devtoolsWidth: number
}): WorkspaceSnapshot {
  return {
    zoom: 1,
    pan: { x: 0, y: 0 },
    pages: [],
    entities: {},
    entityOrder: [],
    selectedPageIndex: null,
    selectedPageIds: [],
    selectedPageId: null,
    selectedGroupId: null,
    leftSidebarOpen: params.leftSidebarOpen,
    devtoolsOpen: false,
    devtoolsPanelTab: params.devtoolsPanelTab,
    devtoolsWidth: params.devtoolsWidth,
    groups: [],
    edges: [],
  }
}

export function buildPageSnapshot(
  page: PersistablePageSnapshotInput,
): WorkspacePageSnapshot {
  return {
    id: page.id,
    name: page.name?.trim() || undefined,
    url: page.url,
    presetIndex: page.presetIndex,
    canvasX: page.canvasX,
    canvasY: page.canvasY,
    syncId: page.syncId ?? null,
    source: page.source,
    parentGroupId: page.parentGroupId ?? page.groupId,
    groupId: page.parentGroupId ?? page.groupId,
    metadata: page.metadata,
    colorScheme: page.colorScheme,
  }
}

// --- Entity <-> Page Conversion ---

export function pageSnapshotToEntity(page: WorkspacePageSnapshot): PersistedPageEntity {
  return {
    kind: 'page',
    id: page.id ?? '',
    name: page.name,
    url: page.url,
    presetIndex: page.presetIndex,
    canvasX: page.canvasX,
    canvasY: page.canvasY,
    syncId: page.syncId ?? null,
    source: page.source,
    parentGroupId: page.parentGroupId ?? page.groupId,
    groupId: page.parentGroupId ?? page.groupId,
    metadata: page.metadata,
    colorScheme: page.colorScheme,
  }
}

export function entityToPageSnapshot(entity: PersistedCanvasEntity): WorkspacePageSnapshot | null {
  if (entity.kind !== 'page') return null
  return {
    id: entity.id,
    name: entity.name,
    url: entity.url,
    presetIndex: entity.presetIndex,
    canvasX: entity.canvasX,
    canvasY: entity.canvasY,
    syncId: entity.syncId ?? null,
    source: entity.source,
    parentGroupId: entity.parentGroupId ?? entity.groupId,
    groupId: entity.parentGroupId ?? entity.groupId,
    metadata: entity.metadata,
    colorScheme: entity.colorScheme,
  }
}

export function buildEntitiesFromPages(
  pages: WorkspacePageSnapshot[],
): { entities: Record<string, PersistedCanvasEntity>; entityOrder: string[] } {
  const entities: Record<string, PersistedCanvasEntity> = {}
  const entityOrder: string[] = []
  for (const page of pages) {
    const entity = pageSnapshotToEntity(page)
    if (entity.id) {
      entities[entity.id] = entity
      entityOrder.push(entity.id)
    }
  }
  return { entities, entityOrder }
}

export function buildPagesFromEntities(
  entities: Record<string, PersistedCanvasEntity>,
  entityOrder?: string[],
): WorkspacePageSnapshot[] {
  const orderedIds = entityOrder ?? Object.keys(entities)
  const pages: WorkspacePageSnapshot[] = []
  for (const id of orderedIds) {
    const entity = entities[id]
    if (!entity) continue
    const page = entityToPageSnapshot(entity)
    if (page) pages.push(page)
  }
  return pages
}

export function cloneWorkspaceGroupsForSnapshot(
  groups: WorkspaceGroup[],
): WorkspaceGroup[] {
  return groups.map((group) => ({
    ...group,
    pageIds: group.pageIds ? [...group.pageIds] : undefined,
    entityIds: group.entityIds ? [...group.entityIds] : undefined,
    metadata: group.metadata ? { ...group.metadata } : undefined,
  }))
}

export function cloneWorkspaceEdgesForSnapshot(
  edges: WorkspaceEdge[],
): WorkspaceEdge[] {
  return edges.map((edge) => ({
    ...edge,
    metadata: edge.metadata ? { ...edge.metadata } : undefined,
  }))
}

export function buildWorkspaceSnapshot(params: {
  zoom: number
  pan: { x: number; y: number }
  pages: WorkspacePageSnapshot[]
  selectedPageIndex: number | null
  selectedPageId: string | null
  selectedPageIds: string[]
  selectedGroupId: string | null
  leftSidebarOpen: boolean
  devtoolsOpen: boolean
  devtoolsPanelTab: DevtoolsPanelTab
  devtoolsWidth: number
  groups: WorkspaceGroup[]
  edges: WorkspaceEdge[]
}): WorkspaceSnapshot {
  const { entities, entityOrder } = buildEntitiesFromPages(params.pages)
  return {
    zoom: params.zoom,
    pan: params.pan,
    pages: params.pages,
    entities,
    entityOrder,
    selectedPageIndex: params.selectedPageIndex,
    selectedPageId: params.selectedPageId,
    selectedPageIds: params.selectedPageIds,
    selectedGroupId: params.selectedGroupId,
    leftSidebarOpen: params.leftSidebarOpen,
    devtoolsOpen: params.devtoolsOpen,
    devtoolsPanelTab: params.devtoolsPanelTab,
    devtoolsWidth: params.devtoolsWidth,
    groups: cloneWorkspaceGroupsForSnapshot(params.groups),
    edges: cloneWorkspaceEdgesForSnapshot(params.edges),
  }
}

export function scheduleWorkspaceAutosave(options: AutosaveSchedulerOptions): void {
  if (!options.shouldPersist()) return
  if (options.autosaveTimer) clearTimeout(options.autosaveTimer)
  options.setAutosaveTimer(
    setTimeout(() => {
      options.setAutosaveTimer(null)
      options.saveWorkspaceStore()
    }, AUTOSAVE_DEBOUNCE_MS),
  )
}

export function flushWorkspaceAutosaveSync(options: {
  autosaveTimer: NodeJS.Timeout | null
  setAutosaveTimer: (timer: NodeJS.Timeout | null) => void
  saveWorkspaceStore: () => void
}): void {
  if (options.autosaveTimer) {
    clearTimeout(options.autosaveTimer)
    options.setAutosaveTimer(null)
  }
  options.saveWorkspaceStore()
}

// --- JSON Canvas File I/O ---

import {
  serializeToJsonCanvas,
  deserializeFromJsonCanvas,
} from './json-canvas-serializer'
import type { JsonCanvasDocument } from '../../shared/json-canvas-types'

const SPACE_META_DIR = '.specular'
const WORKSPACE_META_FILE = 'workspace-meta.json'

function ensureSpaceDir(spacePath: string): string {
  if (!existsSync(spacePath)) mkdirSync(spacePath, { recursive: true })
  return spacePath
}

function ensureSpaceMetaDir(spacePath: string): string {
  const dir = join(spacePath, SPACE_META_DIR)
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true })
  return dir
}

function sanitizeTabName(name: string): string {
  return name.replace(/[/\\:*?"<>|]/g, '_').trim() || 'Untitled'
}

/**
 * A tab's `.canvas` file. The id suffix is what makes the path unique — two
 * tabs sharing a name would otherwise share one file, so saving, renaming, or
 * deleting either one would clobber the other.
 */
export function canvasFilePath(
  spacePath: string,
  tab: { id: string; name: string },
): string {
  const suffix = tab.id.replace(/^tab_/, '').slice(0, 4)
  return join(
    ensureSpaceDir(spacePath),
    `${sanitizeTabName(tab.name)}-${suffix}.canvas`,
  )
}

/** Pre-id-suffix filename. Read-only — kept so existing workspaces load. */
function legacyCanvasFilePath(spacePath: string, tabName: string): string {
  return join(ensureSpaceDir(spacePath), `${sanitizeTabName(tabName)}.canvas`)
}

export function writeCanvasFileSync(
  filePath: string,
  doc: JsonCanvasDocument,
): void {
  const tmpFile = `${filePath}.tmp`
  writeFileSync(tmpFile, JSON.stringify(doc, null, 2), 'utf8')
  renameSync(tmpFile, filePath)
}

export function readCanvasFile(filePath: string): JsonCanvasDocument | null {
  if (!existsSync(filePath)) return null
  try {
    return JSON.parse(readFileSync(filePath, 'utf8')) as JsonCanvasDocument
  } catch {
    return null
  }
}

export function writeTabAsCanvasFile(
  spacePath: string,
  tab: PersistedWorkspaceTab,
): void {
  const doc = serializeToJsonCanvas(tab.snapshot, tab.annotations)
  const filePath = canvasFilePath(spacePath, tab)
  writeCanvasFileSync(filePath, doc)
}

export function writeAllTabsAsCanvasFiles(
  spacePath: string,
  tabs: PersistedWorkspaceTab[],
): void {
  for (const tab of tabs) {
    writeTabAsCanvasFile(spacePath, tab)
  }
}

export function writeWorkspaceMetaSync(
  spacePath: string,
  meta: {
    activeTabId: string
    viewMode?: string
    tabs: Array<{ id: string; name: string; updatedAt: string; expanded?: boolean }>
  },
): void {
  // Writes always land in .specular/, even when the meta was last read from
  // the legacy root location — the next load picks up the new copy and the
  // old file is harmless litter, not a second source of truth.
  const filePath = join(ensureSpaceMetaDir(spacePath), WORKSPACE_META_FILE)
  const tmpFile = `${filePath}.tmp`
  writeFileSync(tmpFile, JSON.stringify(meta, null, 2), 'utf8')
  renameSync(tmpFile, filePath)
}

export function readWorkspaceMeta(
  spacePath: string,
): { activeTabId: string; viewMode?: string; tabs: Array<{ id: string; name: string; updatedAt: string; expanded?: boolean }> } | null {
  const filePath = join(spacePath, SPACE_META_DIR, WORKSPACE_META_FILE)
  const legacyFilePath = join(spacePath, WORKSPACE_META_FILE)
  const target = existsSync(filePath) ? filePath : legacyFilePath
  if (!existsSync(target)) return null
  try {
    return JSON.parse(readFileSync(target, 'utf8'))
  } catch {
    return null
  }
}

/**
 * Migrate an existing workspace-store.json to .canvas files.
 * Called once on first launch with the new format.
 */
export function migrateWorkspaceStoreToCanvasFiles(
  spacePath: string,
  store: PersistedWorkspaceStore,
): void {
  for (const workspace of store.workspaces) {
    // Write each tab as a .canvas file
    writeAllTabsAsCanvasFiles(spacePath, workspace.tabs)

    // Write workspace metadata
    writeWorkspaceMetaSync(spacePath, {
      activeTabId: workspace.activeTabId,
      viewMode: workspace.viewMode,
      tabs: workspace.tabs.map((t) => ({
        id: t.id,
        name: t.name,
        updatedAt: t.updatedAt,
        expanded: t.expanded,
      })),
    })
  }
}

// --- Load workspace from .canvas files ---

/**
 * Load a workspace from individual .canvas files + workspace-meta.json.
 * This is the primary load path — .canvas files are the source of truth.
 */
export function loadWorkspaceFromCanvasFiles(
  spacePath: string,
): PersistedWorkspaceRecord | null {
  const meta = readWorkspaceMeta(spacePath)
  if (!meta || !meta.tabs.length) return null

  const tabs: PersistedWorkspaceTab[] = []
  const legacyPaths = new Set<string>()
  for (const tabMeta of meta.tabs) {
    const filePath = canvasFilePath(spacePath, tabMeta)
    let doc = readCanvasFile(filePath)
    if (!doc) {
      // Workspace written before filenames carried an id. Read the old file and
      // retire it, so the next autosave doesn't leave a stale twin on disk.
      // Retired after the loop, not here: same-named tabs share one legacy file
      // and every one of them still has to read it.
      const legacyPath = legacyCanvasFilePath(spacePath, tabMeta.name)
      doc = readCanvasFile(legacyPath)
      if (doc) legacyPaths.add(legacyPath)
    }
    if (!doc) continue
    const { snapshot, annotations } = deserializeFromJsonCanvas(doc)
    // Populate legacy pages array from entities for backward compat
    snapshot.pages = buildPagesFromEntities(snapshot.entities ?? {}, snapshot.entityOrder)
    tabs.push({
      id: tabMeta.id,
      name: tabMeta.name,
      updatedAt: tabMeta.updatedAt,
      expanded: tabMeta.expanded ?? true,
      snapshot,
      annotations,
    })
  }

  if (!tabs.length) return null

  for (const legacyPath of legacyPaths) {
    try {
      unlinkSync(legacyPath)
    } catch {
      // Ignore — the suffixed files are authoritative from here on
    }
  }

  return {
    id: DEFAULT_WORKSPACE_ID,
    name: DEFAULT_WORKSPACE_NAME,
    updatedAt: new Date().toISOString(),
    activeTabId: meta.activeTabId,
    viewMode: meta.viewMode as WorkspaceViewMode | undefined,
    tabs,
  }
}

/**
 * Delete a .canvas file for a tab. Used when renaming or deleting tabs.
 */
export function deleteCanvasFile(
  spacePath: string,
  tab: { id: string; name: string },
): void {
  const filePath = canvasFilePath(spacePath, tab)
  try {
    if (existsSync(filePath)) unlinkSync(filePath)
  } catch {
    // Ignore — file may already be gone
  }
}
