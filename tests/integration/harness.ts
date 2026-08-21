/**
 * In-process workspace harness.
 *
 * Boots the REAL main-process runtime — workspace model, Y.Doc, undo manager,
 * doc observers, autosave — inside plain Node, bound to a per-run temp dir.
 * No Electron process, no window, no HTTP: tests import the same mutators the
 * IPC handlers and CLI routes call, and assert on three production surfaces:
 *
 *   1. runtime arrays (what the renderer would be shown)
 *   2. the Y.Doc (what undo operates on)
 *   3. .canvas files on disk (what survives a relaunch)
 *
 * Mirrors the boot sequence in src/main/index.ts and the fixture loader the
 * retired Electron smoke suite used (routes/test.ts /test/workspace/
 * load-canvas-fixture), so coverage here exercises the same code paths.
 */

import { existsSync, mkdtempSync, readdirSync, rmSync } from 'fs'
import { join } from 'path'
import { tmpdir } from 'os'
import type * as Y from 'yjs'
import { __setUserDataPath, __broadcasts, WebContentsView, BaseWindow, type BroadcastRecord } from './electron-stub'
import type { JsonCanvasDocument } from '../../src/shared/json-canvas-types'

import {
  setWin,
  setToolbarView,
  setAboveView,
  setBgView,
} from '../../src/main/runtime/view-refs'
import { pages, zoom, pan } from '../../src/main/runtime/runtime-context'
import {
  workspaceAnnotations,
  workspaceEdges,
  workspaceGroups,
  spaceTabs,
  activeSpaceTabId,
  setActiveSpaceTabId,
} from '../../src/main/runtime/space-model'
import { textEntities } from '../../src/main/runtime/text-entity-state'
import { fileEntities } from '../../src/main/runtime/file-entity-state'
import { drawingEntities } from '../../src/main/runtime/drawing-entity-state'
import { shapeEntities } from '../../src/main/runtime/shape-entity-state'
import { getActiveDoc } from '../../src/main/runtime/space-doc'
import {
  createCanvasUndoManager,
  setUndoSelectionHooks,
  clearUndoHistory,
} from '../../src/main/runtime/space-undo'
import { initializeDocObservers } from '../../src/main/runtime/space-observers'
import { createPage, removePageById } from '../../src/main/runtime/page-runtime'
import { destroyActivePages, restorePersistedSpace } from '../../src/main/runtime/space-restore'
import { cancelActive as cancelActiveInteraction } from '../../src/main/runtime/interaction-controller'
import { sendInteractiveState } from '../../src/main/runtime/overlay-manager'
import { getUiState, setSelection } from '../../src/main/ui-state'
import { loadSpace, flushSpaceAutosaveSync } from '../../src/main/runtime/space-autosave'
import {
  canvasFilePath,
  readCanvasFile,
  readSpaceMeta,
  writeCanvasFileSync,
  writeSpaceMetaSync,
} from '../../src/main/runtime/space-persistence'
import { spaceDir } from '../../src/main/runtime/space-dir'
import { registerBuiltInEntityKinds } from '../../src/main/entities'

export interface WorkspaceHarness {
  userDataPath: string
  doc: Y.Doc
  /** Renderer-bound sends captured at the webContents.send seam. */
  broadcasts: BroadcastRecord[]
  clearBroadcasts(): void
  /** Force the debounced autosave to write now, like app quit does. */
  flush(): void
  /** Flush, then parse the active tab's .canvas file from disk. */
  diskDoc(tabName?: string): JsonCanvasDocument | null
  /** A tab's .canvas path. Keyed by id — pass a record for a tab that a
   *  pending undo may have removed from the runtime array. */
  diskPath(tab?: string | { id: string; name: string }): string
  /** Read workspace-meta.json from disk. */
  diskMeta(): { activeTabId?: string | null; tabs?: Array<{ id: string; name: string }> } | null
  /** Replace the whole workspace with a fixture, as if the app relaunched on it. */
  loadFixture(fixture: CanvasFixture): void
  /** Reset to a single blank tab. */
  reset(): void
  dispose(): void
}

export interface CanvasFixture {
  name?: string
  doc: JsonCanvasDocument
}

export const BLANK_FIXTURE: CanvasFixture = {
  name: 'Blank',
  doc: { nodes: [], edges: [], appState: { zoom: 1, pan: { x: 0, y: 0 } } },
}

let activeHarness: WorkspaceHarness | null = null

/**
 * Boot the runtime against a fresh temp dir. Call once per test file
 * (beforeAll); use `reset()`/`loadFixture()` between tests. Runtime state is
 * module-global in src/main, so only one harness can exist per process —
 * vitest gives each test file its own process, which is the isolation model.
 */
export function bootWorkspaceHarness(fixture: CanvasFixture = BLANK_FIXTURE): WorkspaceHarness {
  if (activeHarness) throw new Error('harness already booted in this process — use reset()/loadFixture()')

  const userDataPath = mkdtempSync(join(tmpdir(), 'specular-itest-'))
  __setUserDataPath(userDataPath)

  // App boot registers the entity-kind handlers before anything touches the
  // registry (src/main/index.ts); idempotent, so per-file re-boots are safe.
  registerBuiltInEntityKinds()

  // Minimal view topology: createPage() requires win + toolbarView to exist;
  // the fake window reports isDestroyed() so the layout engine never runs.
  setWin(new BaseWindow() as never)
  setToolbarView(new WebContentsView() as never)
  setAboveView(new WebContentsView() as never)
  setBgView(new WebContentsView() as never)

  writeFixtureToDisk(fixture)
  const record = loadSpace()
  if (!record || !restorePersistedSpace(record)) {
    throw new Error('harness failed to load initial workspace fixture')
  }

  const doc = getActiveDoc()
  createCanvasUndoManager(doc)
  setUndoSelectionHooks(
    () => getUiState().selection,
    (selection) => setSelection(selection as never),
  )
  wireDocObservers()
  clearUndoHistory()
  __broadcasts.length = 0

  const harness: WorkspaceHarness = {
    userDataPath,
    doc,
    broadcasts: __broadcasts,
    clearBroadcasts: () => {
      __broadcasts.length = 0
    },
    flush: () => flushSpaceAutosaveSync(),
    diskDoc: (tabName?: string) => {
      flushSpaceAutosaveSync()
      return readCanvasFile(harness.diskPath(tabName))
    },
    diskPath: (ref?: string | { id: string; name: string }) => {
      const tab =
        typeof ref === 'string'
          ? spaceTabs.find((candidate) => candidate.name === ref)
          : (ref ?? activeTab())
      if (!tab) throw new Error(`no tab named ${String(ref)}`)
      return canvasFilePath(spaceDir(), tab)
    },
    diskMeta: () => readSpaceMeta(spaceDir()),
    loadFixture: (next: CanvasFixture) => {
      writeFixtureToDisk(next)
      const nextRecord = loadSpace()
      if (!nextRecord || !restorePersistedSpace(nextRecord)) {
        throw new Error('harness failed to load fixture workspace')
      }
      // Restore replaces the runtime arrays and rehydrates the doc; the
      // observer wiring and undo stack are rebuilt exactly like app boot.
      wireDocObservers()
      clearUndoHistory()
      __broadcasts.length = 0
    },
    reset: () => harness.loadFixture(BLANK_FIXTURE),
    dispose: () => {
      destroyActivePages()
      rmSync(userDataPath, { recursive: true, force: true })
      activeHarness = null
    },
  }
  activeHarness = harness
  return harness
}

function activeTab() {
  return spaceTabs.find((t) => t.id === activeSpaceTabId) ?? spaceTabs[0]
}

function writeFixtureToDisk(fixture: CanvasFixture): void {
  const name = fixture.name?.trim() || 'Fixture'
  const tabId = 'fixture-tab'
  // The fixture's meta replaces the whole index, so the previous test's
  // .canvas files have to go with it: loading a space adopts every .canvas
  // file it finds there, so leaving them behind makes each test inherit its
  // predecessors' canvases as extra tabs.
  if (existsSync(spaceDir())) {
    for (const entry of readdirSync(spaceDir())) {
      if (entry.endsWith('.canvas')) rmSync(join(spaceDir(), entry), { force: true })
    }
  }
  writeCanvasFileSync(canvasFilePath(spaceDir(), { id: tabId, name }), fixture.doc)
  writeSpaceMetaSync(spaceDir(), {
    activeTabId: tabId,
    tabs: [{ id: tabId, name, updatedAt: new Date().toISOString(), expanded: true }],
  })
}

function wireDocObservers(): void {
  initializeDocObservers({
    pages,
    textEntities,
    fileEntities,
    drawingEntities,
    shapeEntities,
    workspaceGroups,
    workspaceEdges,
    workspaceAnnotations,
    getZoom: () => zoom,
    getPan: () => pan,
    serializePage: (page) => ({
      id: page.id,
      name: page.name,
      url: page.url,
      presetIndex: page.presetIndex,
      canvasX: page.canvasX,
      canvasY: page.canvasY,
      syncId: page.syncId,
      source: (page as { source?: unknown }).source,
      parentGroupId: page.parentGroupId ?? (page as { groupId?: string }).groupId,
      metadata: page.metadata,
      colorScheme: (page as { colorScheme?: unknown }).colorScheme,
    }),
    cancelActiveInteraction: () => cancelActiveInteraction('undo'),
    sendInteractiveState,
    createPage: (data) => createPage(data as never),
    removePageById,
    destroyActivePages,
    getActiveTabId: () => activeSpaceTabId,
    setActiveTabId: setActiveSpaceTabId,
    spaceTabs,
  })
}

/**
 * The forward sync (runtime → Y.Doc) is scheduled on a microtask by
 * requestDocSync(). Await this after a mutation before asserting on the doc,
 * the undo stack, or disk.
 */
export function settleSync(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0))
}
