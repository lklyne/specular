/**
 * Reverse-sync (undo path) round-trip unit tests.
 *
 * Drives the real observer wiring — `initializeDocObservers()` plus a real
 * Y.UndoManager on a real Y.Doc, no Electron — so the code under test is the
 * production path: `syncDocToRuntime`'s registry reconciliation and
 * `restorePagesFromDoc`'s remove/create/patch pass. Registered kinds are test
 * stubs over in-memory stores (the WebContentsView-backed page store cannot
 * exist here), registered the same way the serializer tests stub kinds.
 *
 * Mutation-verified two ways: dropping the `entitySnapshots.get(def.kind) ??
 * []` fallback in `syncDocToRuntime` fails "undo of a create clears the
 * store"; dropping `parentGroupId` from `PAGE_RESTORE_PATCHERS` (to `null`)
 * fails "undo patches live pages in place".
 */

import * as Y from 'yjs'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('../../src/main/runtime/viewport-control', () => ({
  requestLayout: vi.fn(),
}))
vi.mock('../../src/main/runtime/workspace-tabs', () => ({
  makeEmptyTabSnapshot: vi.fn(() => ({})),
}))

import {
  beginBatch,
  endBatch,
  initializeDocObservers,
} from '../../src/main/runtime/workspace-observers'
import {
  createWorkspaceDoc,
  setActiveDoc,
  DOC_MAP_ENTITIES,
  DOC_MAP_PAGES,
} from '../../src/main/runtime/workspace-doc'
import {
  clearUndoHistory,
  createCanvasUndoManager,
  markUndoBoundary,
  setActiveUndoManager,
  undo,
} from '../../src/main/runtime/workspace-undo'
import {
  registerEntityKind,
  __resetEntityRegistryForTests,
} from '../../src/main/entities/contract'
import {
  restorePagesFromDoc,
  PAGE_PERSISTED_FIELDS,
} from '../../src/main/runtime/page-doc-projection'
import type { Page } from '../../src/main/runtime/runtime-entities'
import type { PersistedCanvasEntity } from '../../src/shared/types'

// ---------------------------------------------------------------------------
// Fake kind: an entity-map kind with a declared field list over an in-memory
// store, registered under the `text` kind name.
// ---------------------------------------------------------------------------

interface FakeNote {
  kind: 'text'
  id: string
  canvasX: number
  canvasY: number
  width: number
  height: number
  body?: string
  parentGroupId?: string
}

const FAKE_NOTE_FIELDS = [
  'kind',
  'id',
  'canvasX',
  'canvasY',
  'width',
  'height',
  'body',
  'parentGroupId',
] as const

const noteStore: FakeNote[] = []
const pageStore: Page[] = []
const createPageCalls: string[] = []
const removePageCalls: string[] = []

function registerFakeKinds(): void {
  registerEntityKind({
    kind: 'text',
    fields: FAKE_NOTE_FIELDS,
    create: () => { throw new Error('create unused in round-trip tests') },
    update: () => {},
    delete: () => false,
    serialize: () => { throw new Error('serialize unused in round-trip tests') },
    deserialize: () => { throw new Error('deserialize unused in round-trip tests') },
    defaultSize: () => ({ width: 0, height: 0 }),
    entities: () => noteStore,
    persist: (entity) => {
      const note = entity as FakeNote
      const data: Record<string, unknown> = {}
      for (const field of FAKE_NOTE_FIELDS) data[field] = note[field]
      return data as unknown as PersistedCanvasEntity
    },
    restore: (snapshots) => {
      noteStore.length = 0
      for (const snapshot of snapshots) noteStore.push(snapshot as unknown as FakeNote)
    },
  })

  registerEntityKind({
    kind: 'page',
    fields: PAGE_PERSISTED_FIELDS,
    create: () => { throw new Error('create unused in round-trip tests') },
    update: () => {},
    delete: () => false,
    serialize: () => { throw new Error('serialize unused in round-trip tests') },
    deserialize: () => { throw new Error('deserialize unused in round-trip tests') },
    defaultSize: () => ({ width: 0, height: 0 }),
    entities: () => pageStore,
    restore: (snapshots) => {
      restorePagesFromDoc(snapshots, {
        pages: pageStore,
        createPage: (data) => {
          createPageCalls.push(data.id as string)
          pageStore.push({ ...data } as unknown as Page)
        },
        removePageById: (id) => {
          removePageCalls.push(id)
          const index = pageStore.findIndex((page) => page.id === id)
          if (index !== -1) pageStore.splice(index, 1)
        },
      })
    },
  })
}

function makeFakePage(overrides: Partial<Record<string, unknown>> = {}): Page {
  return {
    id: 'p1',
    name: 'Home',
    url: 'http://a.test/',
    presetIndex: 1,
    canvasX: 0,
    canvasY: 0,
    syncId: null,
    source: 'user',
    parentGroupId: 'g1',
    metadata: { deviceId: 'laptop' },
    ...overrides,
  } as unknown as Page
}

/** One forward sync = one tracked Y.Doc transaction = one undo step. */
function commit(mutate: () => void): void {
  beginBatch()
  mutate()
  endBatch()
  markUndoBoundary()
}

let doc: Y.Doc
let manager: Y.UndoManager

beforeEach(() => {
  noteStore.length = 0
  pageStore.length = 0
  createPageCalls.length = 0
  removePageCalls.length = 0
  registerFakeKinds()

  doc = createWorkspaceDoc()
  setActiveDoc(doc)
  manager = createCanvasUndoManager(doc)
  initializeDocObservers({
    pages: pageStore,
    workspaceGroups: [],
    workspaceEdges: [],
    workspaceAnnotations: [],
    getZoom: () => 1,
    getPan: () => ({ x: 0, y: 0 }),
    cancelActiveInteraction: vi.fn(),
    sendInteractiveState: vi.fn(),
    destroyActivePages: vi.fn(),
    getActiveTabId: () => null,
    setActiveTabId: vi.fn(),
    workspaceTabs: [],
  })
  clearUndoHistory()
})

afterEach(() => {
  clearUndoHistory()
  manager.destroy()
  setActiveUndoManager(null)
  __resetEntityRegistryForTests()
})

describe('doc → runtime restore on undo', () => {
  it('round-trips every declared field of a registered kind through undo', () => {
    const original: FakeNote = {
      kind: 'text',
      id: 'n1',
      canvasX: 10,
      canvasY: 20,
      width: 100,
      height: 50,
      body: 'first',
      parentGroupId: 'g1',
    }
    commit(() => noteStore.push({ ...original }))
    expect(manager.undoStack.length).toBe(1)

    commit(() => {
      noteStore[0].body = 'second'
      noteStore[0].canvasX = 999
      noteStore[0].width = 1
    })
    expect(manager.undoStack.length).toBe(2)

    undo()

    expect(noteStore).toHaveLength(1)
    for (const field of FAKE_NOTE_FIELDS) {
      expect(noteStore[0][field], field).toEqual(original[field])
    }
  })

  it('undo of a create clears the store (empty bucket still restores)', () => {
    commit(() => noteStore.push({
      kind: 'text', id: 'n1', canvasX: 0, canvasY: 0, width: 10, height: 10,
    }))
    expect((doc.getMap(DOC_MAP_ENTITIES) as Y.Map<unknown>).has('n1')).toBe(true)

    undo()

    expect((doc.getMap(DOC_MAP_ENTITIES) as Y.Map<unknown>).has('n1')).toBe(false)
    expect(noteStore).toHaveLength(0)
  })

  it('undo applies without re-triggering forward sync', async () => {
    commit(() => noteStore.push({
      kind: 'text', id: 'n1', canvasX: 0, canvasY: 0, width: 10, height: 10,
    }))
    commit(() => { noteStore[0].canvasX = 50 })
    expect(manager.undoStack.length).toBe(2)

    undo()
    await Promise.resolve() // flush any microtask-scheduled sync

    expect(manager.undoStack.length).toBe(1)
    expect(manager.redoStack.length).toBe(1)
    expect(noteStore[0].canvasX).toBe(0)
  })

  it('undo patches live pages in place and skips the excluded fields', () => {
    const home = makeFakePage()
    commit(() => pageStore.push(home))
    commit(() => {
      home.canvasX = 500
      home.name = 'Renamed'
      home.url = 'http://b.test/'
      home.parentGroupId = 'g2'
      home.metadata = { deviceId: 'phone' }
    })

    undo()

    // Same live object, patched — never rebuilt.
    expect(pageStore[0]).toBe(home)
    expect(home.canvasX).toBe(0)
    expect(home.name).toBe('Home')
    expect(home.parentGroupId).toBe('g1')
    expect(home.metadata).toEqual({ deviceId: 'laptop' })
    // `url` persists to the doc but is deliberately not patched on undo —
    // patching it would navigate the live WebContents.
    expect((doc.getMap(DOC_MAP_PAGES).get('p1') as Y.Map<unknown>).get('url')).toBe('http://a.test/')
    expect(home.url).toBe('http://b.test/')
  })

  it('undo recreates deleted pages with their persisted fields', () => {
    commit(() => pageStore.push(makeFakePage()))
    commit(() => pageStore.splice(0, 1))
    expect((doc.getMap(DOC_MAP_PAGES) as Y.Map<unknown>).has('p1')).toBe(false)

    undo()

    expect(createPageCalls).toEqual(['p1'])
    expect(pageStore).toHaveLength(1)
    expect(pageStore[0]).toMatchObject({
      id: 'p1',
      name: 'Home',
      url: 'http://a.test/',
      presetIndex: 1,
      syncId: null,
      parentGroupId: 'g1',
      metadata: { deviceId: 'laptop' },
    })
  })

  it('undo of a page create removes the live page', () => {
    commit(() => pageStore.push(makeFakePage()))
    expect((doc.getMap(DOC_MAP_PAGES) as Y.Map<unknown>).has('p1')).toBe(true)

    undo()

    expect(removePageCalls).toEqual(['p1'])
    expect(pageStore).toHaveLength(0)
  })
})
