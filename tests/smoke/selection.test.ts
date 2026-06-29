import { afterEach, describe, expect, it } from 'vitest'
import {
  createFocusedPage,
  createPages,
  createGroup,
  deleteGroups,
  deletePages,
  deselectSelection,
  enterGroup,
  getSelection,
  getSelectionOverlayState,
  getWorkspace,
  selectGroup,
  selectPage,
  ungroup,
} from './app-client'
import { assertPersists, assertUndoable, waitFor } from './test-utils'

const createdPageIds: string[] = []

async function createPage(input: { url: string; canvasX: number; canvasY: number; presetIndex?: number }) {
  const result = await createPages([input])
  createdPageIds.push(...result.pageIds)
  return result.pageIds[0]
}

async function createPagePair(aY: number) {
  const firstId = await createPage({
    url: 'https://example.com',
    canvasX: 120,
    canvasY: aY,
  })
  const secondId = await createPage({
    url: 'https://example.org',
    canvasX: 620,
    canvasY: aY,
  })
  return [firstId, secondId]
}

async function cleanupPages() {
  if (!createdPageIds.length) return
  const pageIds = createdPageIds.splice(0, createdPageIds.length)
  await deletePages(pageIds)
}

describe('selection', () => {
  afterEach(async () => {
    await cleanupPages()
  })

  it('selecting a group clears stale child-page interactivity and deselect works', async () => {
    const pageIds = await createPagePair(120)

    await selectPage(pageIds[0])
    const group = await createGroup(pageIds, 'Smoke group')
    await selectGroup(group.id)

    const selection = await waitFor(
      () => getSelection(),
      (value) => value.selectedGroupId === group.id,
      'Timed out waiting for group selection',
    )
    expect(selection.selectedEntityIds ?? []).toEqual([])
    expect(selection.selectedGroupId).toBe(group.id)

    const overlay = await waitFor(
      () => getSelectionOverlayState(),
      (value) => {
        const states = value.pages.filter((page) => pageIds.includes(page.pageId))
        return states.length === 2 && states.every((page) => !page.interactive && page.multiSelected)
      },
      'Timed out waiting for group overlay state (non-interactive, multiSelected)',
    )
    const groupStates = overlay.pages.filter((page) => pageIds.includes(page.pageId))
    expect(groupStates).toHaveLength(2)
    expect(groupStates.every((page) => page.interactive === false)).toBe(true)
    expect(groupStates.every((page) => page.multiSelected === true)).toBe(true)

    await deselectSelection()

    const cleared = await getSelection()
    expect(cleared.selectedEntityIds ?? []).toEqual([])
    expect(cleared.selectedGroupId).toBeUndefined()

    const clearedOverlay = await getSelectionOverlayState()
    const clearedGroupStates = clearedOverlay.pages.filter((page) => pageIds.includes(page.pageId))
    expect(clearedGroupStates).toHaveLength(2)
    expect(clearedGroupStates.every((page) => !page.interactive && !page.multiSelected)).toBe(true)
  })

  it('ungrouping selects freed entities and preserves non-interactive overlay state until deselect', async () => {
    const pageIds = await createPagePair(260)

    const group = await createGroup(pageIds, 'Ungroup me')
    await selectGroup(group.id)
    const ungrouped = await ungroup(group.id)

    expect(ungrouped.entityIds.sort()).toEqual([...pageIds].sort())

    const selection = await waitFor(
      () => getSelection(),
      (value) => (value.selectedEntityIds ?? []).sort().join(',') === [...pageIds].sort().join(','),
      'Timed out waiting for ungrouped pages to become selected',
    )
    expect((selection.selectedEntityIds ?? []).sort()).toEqual([...pageIds].sort())
    expect(selection.selectedGroupId).toBeUndefined()

    const overlay = await getSelectionOverlayState()
    const selectedStates = overlay.pages.filter((page) => pageIds.includes(page.pageId))
    expect(selectedStates.every((page) => page.interactive === false)).toBe(true)
    expect(selectedStates.every((page) => page.multiSelected === true)).toBe(true)

    await deselectSelection()

    const cleared = await getSelection()
    expect(cleared.selectedEntityIds ?? []).toEqual([])
    expect(cleared.selectedGroupId).toBeUndefined()
  })

  it('creating a page yields a selected-but-not-interactive page (select-first, #124) that can be deselected immediately', async () => {
    const result = await createFocusedPage({
      canvasX: 280,
      canvasY: 420,
      presetIndex: 0,
    })
    createdPageIds.push(result.pageId)

    const selection = await waitFor(
      () => getSelection(),
      (value) => value.selectedEntityId === result.pageId,
      'Timed out waiting for focused page selection',
    )
    expect(selection.selectedEntityId).toBe(result.pageId)
    expect(selection.selectedEntityIds).toEqual([result.pageId])

    // Select-first / interact-second: a freshly selected page is NOT
    // interactive (blocker on, keyboard stays on the canvas so Delete removes
    // the frame). It becomes interactive only once the user enters it.
    const overlay = await getSelectionOverlayState()
    const pageState = overlay.pages.find((page) => page.pageId === result.pageId)
    expect(pageState).toMatchObject({ interactive: false, multiSelected: false })

    await deselectSelection()

    const cleared = await getSelection()
    expect(cleared.selectedEntityIds ?? []).toEqual([])
    expect(cleared.selectedGroupId).toBeUndefined()
  })

  it('entering a group after group selection selects only its children', async () => {
    const pageIds = await createPagePair(560)

    const group = await createGroup(pageIds, 'Enter me')
    await selectPage(pageIds[0])
    await selectGroup(group.id)
    await enterGroup(group.id)

    const selection = await waitFor(
      () => getSelection(),
      (value) => (value.selectedEntityIds ?? []).sort().join(',') === [...pageIds].sort().join(','),
      'Timed out waiting for group children to become selected after entering group',
    )
    expect((selection.selectedEntityIds ?? []).sort()).toEqual([...pageIds].sort())
    expect(selection.selectedGroupId).toBeUndefined()
  })
})

describe('selection — group lifecycle', () => {
  // Group creation is the load-bearing persisted/undoable mutation in this
  // surface. Selection itself is ephemeral runtime state and isn't included in
  // the persistence/undo round-trip.
  afterEach(async () => {
    const graph = await getWorkspace()
    const groupIds = graph.entities.filter((e) => e.kind === 'group').map((e) => e.id)
    if (groupIds.length) await deleteGroups(groupIds)
    if (createdPageIds.length) {
      await deletePages(createdPageIds.splice(0))
    }
  })

  it('persists a created group to disk', async () => {
    const pageIds = await createPagePair(720)
    await assertPersists(async () => {
      await createGroup(pageIds, 'Persisted group')
    })
  })

  it('round-trips a created group through undo/redo', async () => {
    const pageIds = await createPagePair(860)
    await assertUndoable(async () => {
      await createGroup(pageIds, 'Undoable group')
    })
  })
})
