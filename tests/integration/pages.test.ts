/**
 * Page lifecycle against the real runtime, in-process.
 *
 * Guards the production page path end to end: `applyCanvasPatch` with a
 * `page` entity kind (the same door POST /canvas/apply and every CLI verb
 * use) must land the page in the `pages` runtime array, persist it to disk
 * as a JSON Canvas `link` node, delete cleanly from both surfaces, survive an
 * undo/redo round-trip (including destroying/recreating its views), and
 * restore from a fixture `.canvas` file with `link` nodes on relaunch.
 *
 * Mutation-verified by: replacing `nodes.push(serializePageToLinkNode(entity))`
 * for the `kind === 'page'` branch in
 * src/main/runtime/json-canvas-serializer.ts with a no-op — "persists the
 * created page to disk as a link node" fails (page never reaches the .canvas
 * file).
 *
 * The `page colorScheme` block below is mutation-verified by turning the
 * `colorScheme` patcher in `PAGE_RESTORE_PATCHERS`
 * (src/main/runtime/page-doc-projection.ts) into a no-op — "undo/redo
 * round-trips colorScheme, including the absent case" fails (undo no longer
 * clears the field back to absent).
 *
 * The carry-through cases (duplicate / copy-paste) are mutation-verified by
 * deleting `colorScheme: sourcePage.colorScheme` from `duplicatePageInternal`
 * (src/main/workspace-pages.ts) and `colorScheme: page.colorScheme` from the
 * page branch of `copyableSelectionPayload` (src/main/workspace-clipboard.ts)
 * respectively — each fix's test fails on its own with that one line reverted.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import type * as Y from 'yjs'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { applyCanvasPatch } from '../../src/main/canvas-apply'
import { pages, findPageById } from '../../src/main/runtime/runtime-context'
import { undo, redo, canUndo, canRedo } from '../../src/main/runtime/workspace-undo'
import { DOC_MAP_PAGES } from '../../src/main/runtime/workspace-doc'
import { setPageColorScheme } from '../../src/main/runtime/document-commands'
import { duplicatePageFromSource } from '../../src/main/workspace-pages'
import { copyableSelectionPayload, pasteEntitiesFromClipboard } from '../../src/main/workspace-clipboard'
import { selectPageById } from '../../src/main/runtime/ui-actions'

let harness: WorkspaceHarness

/**
 * Count Y.Doc afterTransaction events during `fn`, including the
 * microtask-scheduled forward sync it triggers. A single user mutation must
 * produce exactly one transaction; more implies an echo loop (see
 * tests/integration/sync.test.ts for the pattern this mirrors).
 */
async function observeTransactions(fn: () => void | Promise<void>): Promise<number> {
  let count = 0
  const handler = () => {
    count += 1
  }
  harness.doc.on('afterTransaction', handler)
  try {
    await fn()
    await settleSync()
  } finally {
    harness.doc.off('afterTransaction', handler)
  }
  return count
}

function docColorScheme(pageId: string): unknown {
  return harness.doc.getMap<Y.Map<unknown>>(DOC_MAP_PAGES).get(pageId)?.get('colorScheme')
}

function createPageViaPatch(overrides: Record<string, unknown> = {}): string {
  const result = applyCanvasPatch({
    entities: [
      {
        kind: 'page',
        url: 'https://example.com',
        canvasX: 120,
        canvasY: 240,
        presetIndex: 2,
        ...overrides,
      },
    ],
  })
  expect(result.created).toHaveLength(1)
  return result.created[0]
}

describe('pages', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('creating a page via applyCanvasPatch adds it to the pages runtime array', async () => {
    const pageId = createPageViaPatch()
    await settleSync()

    const page = findPageById(pageId)
    expect(page).toBeDefined()
    expect(page?.url).toBe('https://example.com/')
    expect(page?.canvasX).toBe(120)
    expect(page?.canvasY).toBe(240)
    expect(page?.presetIndex).toBe(2)
    expect(pages.map((p) => p.id)).toContain(pageId)
    expect(harness.doc.getMap(DOC_MAP_PAGES).has(pageId)).toBe(true)
  })

  it('persists the created page to disk as a link node with url and position', async () => {
    const pageId = createPageViaPatch({ canvasX: 800, canvasY: 100 })
    await settleSync()

    const disk = harness.diskDoc()
    expect(disk).not.toBeNull()
    const node = disk!.nodes.find((n) => n.id === pageId)
    expect(node).toBeDefined()
    expect(node!.type).toBe('link')
    expect((node as { url?: string }).url).toBe('https://example.com/')
    expect(node!.x).toBe(800)
    expect(node!.y).toBe(100)
  })

  it('deleting the page removes it from runtime and disk', async () => {
    const pageId = createPageViaPatch()
    await settleSync()

    const result = applyCanvasPatch({ delete: [pageId] })
    expect(result.deleted).toContain(pageId)
    await settleSync()

    expect(pages.map((p) => p.id)).not.toContain(pageId)
    expect(harness.doc.getMap(DOC_MAP_PAGES).has(pageId)).toBe(false)
    const disk = harness.diskDoc()
    expect(disk!.nodes.some((n) => n.id === pageId)).toBe(false)
  })

  it('page creation round-trips through undo/redo', async () => {
    const pageId = createPageViaPatch()
    await settleSync()
    expect(canUndo()).toBe(true)

    // Undo removes the page from runtime + doc; the reverse-sync observer
    // destroys the page's (fake) views via removePageById.
    undo()
    expect(pages.map((p) => p.id)).not.toContain(pageId)
    expect(harness.doc.getMap(DOC_MAP_PAGES).has(pageId)).toBe(false)
    expect(canRedo()).toBe(true)

    // Redo recreates it through the observer's createPage callback.
    redo()
    const restored = findPageById(pageId)
    expect(restored).toBeDefined()
    expect(restored?.url).toBe('https://example.com/')
    expect(restored?.canvasX).toBe(120)
    expect(restored?.canvasY).toBe(240)
  })

  it('loading a fixture with link nodes restores pages into the runtime array', () => {
    harness.loadFixture({
      name: 'Linked',
      doc: {
        nodes: [
          {
            id: 'page_fixture-1',
            type: 'link',
            x: 40,
            y: 60,
            width: 1280,
            height: 800,
            url: 'https://fixture.example.com',
            presetIndex: 2,
          } as never,
        ],
        edges: [],
        appState: { zoom: 1, pan: { x: 0, y: 0 } },
      },
    })

    expect(pages.map((p) => p.id)).toContain('page_fixture-1')
    const page = findPageById('page_fixture-1')
    expect(page?.url).toBe('https://fixture.example.com')
    expect(page?.canvasX).toBe(40)
    expect(page?.canvasY).toBe(60)
  })
})

describe('page colorScheme', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('is absent by default and follows the system scheme', async () => {
    const pageId = createPageViaPatch()
    await settleSync()

    expect(findPageById(pageId)?.colorScheme).toBeUndefined()
    expect(docColorScheme(pageId)).toBeUndefined()
  })

  it('setPageColorScheme sets the runtime page, the Y.Doc pages map, and disk', async () => {
    const pageId = createPageViaPatch()
    await settleSync()

    setPageColorScheme(pageId, 'dark')
    await settleSync()

    expect(findPageById(pageId)?.colorScheme).toBe('dark')
    expect(docColorScheme(pageId)).toBe('dark')

    const disk = harness.diskDoc()
    const node = disk!.nodes.find((n) => n.id === pageId)
    expect((node as { colorScheme?: string }).colorScheme).toBe('dark')
  })

  it('one setPageColorScheme call produces exactly one Y.Doc transaction', async () => {
    const pageId = createPageViaPatch()
    await settleSync()

    const count = await observeTransactions(() => {
      setPageColorScheme(pageId, 'dark')
    })
    expect(count).toBe(1)
  })

  it('setPageColorScheme(id, null) clears the field back to absent (system) on disk and in the doc', async () => {
    const pageId = createPageViaPatch()
    await settleSync()
    setPageColorScheme(pageId, 'dark')
    await settleSync()

    setPageColorScheme(pageId, null)
    await settleSync()

    expect(findPageById(pageId)?.colorScheme).toBeUndefined()
    expect(docColorScheme(pageId)).toBeUndefined()

    const disk = harness.diskDoc()
    const node = disk!.nodes.find((n) => n.id === pageId)
    expect((node as { colorScheme?: string }).colorScheme).toBeUndefined()
  })

  it('undo/redo round-trips colorScheme, including the absent case', async () => {
    const pageId = createPageViaPatch()
    await settleSync()
    expect(findPageById(pageId)?.colorScheme).toBeUndefined()

    // absent -> 'dark'
    setPageColorScheme(pageId, 'dark')
    await settleSync()
    expect(findPageById(pageId)?.colorScheme).toBe('dark')

    // undo: 'dark' -> absent again (not merely "unchanged" — the field must
    // be cleared, since presetIndex-style "keep current value" semantics
    // would leave it stuck at 'dark').
    undo()
    expect(findPageById(pageId)?.colorScheme).toBeUndefined()
    expect(docColorScheme(pageId)).toBeUndefined()

    // redo: absent -> 'dark'
    redo()
    expect(findPageById(pageId)?.colorScheme).toBe('dark')
    expect(docColorScheme(pageId)).toBe('dark')

    // Clearing is itself undoable: 'dark' -> null -> undo -> back to 'dark'.
    setPageColorScheme(pageId, null)
    await settleSync()
    expect(findPageById(pageId)?.colorScheme).toBeUndefined()

    undo()
    expect(findPageById(pageId)?.colorScheme).toBe('dark')
    expect(docColorScheme(pageId)).toBe('dark')
  })

  it('duplicating a page carries its colorScheme override to the copy', async () => {
    const pageId = createPageViaPatch()
    await settleSync()
    setPageColorScheme(pageId, 'dark')
    await settleSync()

    const { pageId: dupId } = duplicatePageFromSource({ sourcePageId: pageId })
    await settleSync()

    expect(dupId).not.toBe(pageId)
    expect(findPageById(dupId)?.colorScheme).toBe('dark')
  })

  it('copy-paste of a page (canvas-internal clipboard) carries its colorScheme override', async () => {
    const pageId = createPageViaPatch()
    await settleSync()
    setPageColorScheme(pageId, 'dark')
    await settleSync()

    selectPageById(pageId)
    const payload = copyableSelectionPayload()
    expect(payload).not.toBeNull()

    const { entityIds } = pasteEntitiesFromClipboard({
      payload: payload!,
      canvasX: 500,
      canvasY: 500,
    })
    await settleSync()

    expect(entityIds).toHaveLength(1)
    expect(findPageById(entityIds[0])?.colorScheme).toBe('dark')
  })
})
