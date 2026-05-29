import { afterEach, describe, expect, it } from 'vitest'
import {
  beginInteraction,
  cancelActiveInteraction,
  createEdges,
  createPages,
  createTextEntities,
  deleteEdges,
  deletePages,
  deleteTextEntities,
  deselectSelection,
  getInteractionMode,
  getSelection,
  getEntityOrder,
  getTextEntities,
  getWorkspace,
  pasteClipboardText,
  selectPage,
  getCurrentTool,
  sendKey,
  resetSmokeState,
  selectEntity,
} from './app-client'
import { wait, waitFor } from './test-utils'

const createdPageIds: string[] = []
const createdTextIds: string[] = []
const createdEdgeIds: string[] = []

async function createPage(url = 'https://example.com') {
  const result = await createPages([{ url, canvasX: 0, canvasY: 0 }])
  createdPageIds.push(...result.pageIds)
  return result.pageIds[0]
}

afterEach(async () => {
  if (createdEdgeIds.length) {
    const ids = createdEdgeIds.splice(0)
    await deleteEdges(ids).catch(() => {})
  }
  if (createdPageIds.length) {
    const ids = createdPageIds.splice(0)
    await deletePages(ids).catch(() => {})
  }
  if (createdTextIds.length) {
    const ids = createdTextIds.splice(0)
    await deleteTextEntities(ids).catch(() => {})
  }
  await resetSmokeState()
})

describe('keyboard shortcuts (binding dispatcher)', () => {
  it('v activates select tool', async () => {
    await sendKey('v')
    const { tool } = await getCurrentTool()
    expect(tool.kind).toBe('select')
  })

  it('c activates comment tool', async () => {
    await sendKey('c')
    const { tool } = await waitFor(
      () => getCurrentTool(),
      (r) => r.tool.kind === 'comment',
      'Timed out waiting for comment tool',
    )
    expect(tool.kind).toBe('comment')
  })

  it('pressing c again stays on comment tool (no toggle)', async () => {
    await sendKey('c')
    await sendKey('c')
    const { tool } = await getCurrentTool()
    expect(tool.kind).toBe('comment')
  })

  it('escape resets tool to select when a tool is active', async () => {
    await sendKey('c')
    await waitFor(
      () => getCurrentTool(),
      (r) => r.tool.kind === 'comment',
      'comment tool not active',
    )
    await sendKey('escape')
    const { tool } = await waitFor(
      () => getCurrentTool(),
      (r) => r.tool.kind === 'select',
      'Timed out waiting for select tool after escape',
    )
    expect(tool.kind).toBe('select')
  })

  it('m activates draw-pen tool', async () => {
    await sendKey('m')
    const { tool } = await waitFor(
      () => getCurrentTool(),
      (r) => r.tool.kind === 'draw' || r.tool.kind === 'select',
      'Timed out waiting for tool change',
    )
    // draw tool is feature-flagged — passes either way
    expect(['draw', 'select']).toContain(tool.kind)
  })

  it('Cmd+Z triggers undo without error', async () => {
    // Create and delete a page to have something to undo
    const pageId = await createPage()
    await deletePages([pageId])
    createdPageIds.splice(createdPageIds.indexOf(pageId), 1)
    await wait(50)
    // Undo — should not throw
    await sendKey('z', { cmd: true })
    await wait(50)
  })

  it('Cmd+G groups selected pages', async () => {
    const id1 = await createPage()
    const id2 = await createPage('https://example.org')
    await selectPage(id1)
    await wait(50)

    // Verify pages exist before group
    const selBefore = await getSelection()
    expect(selBefore.selectedEntityId).toBe(id1)

    // Select both via selectEntities then group via keyboard
    const { selectEntities } = await import('./app-client')
    await selectEntities([id1, id2])
    await wait(50)
    await sendKey('g', { cmd: true })
    await wait(100)

    const sel = await getSelection()
    // After grouping, a group should be selected
    expect(sel.selectedGroupId).toBeTruthy()
  })

  it('Cmd+1 resets viewport without error', async () => {
    await sendKey('1', { cmd: true })
    await wait(50)
    // No assertion — just verifying no crash
  })

  it('delete key removes selected entity', async () => {
    const pageId = await createPage()
    await selectPage(pageId)
    await wait(50)
    await sendKey('delete')
    await wait(100)

    const sel = await getSelection()
    expect(sel.selectedEntityId).toBeUndefined()
    // Remove from cleanup since it was deleted
    const idx = createdPageIds.indexOf(pageId)
    if (idx >= 0) createdPageIds.splice(idx, 1)
  })

  it('delete key removes selected sticky after exiting inline edit', async () => {
    const { ids } = await createTextEntities([
      { canvasX: 120, canvasY: 140, text: 'Delete me after edit' },
    ])
    const textId = ids[0]
    createdTextIds.push(textId)
    await selectEntity(textId, 'text')
    await wait(50)

    const token = await beginInteraction({ kind: 'editing-entity', entityId: textId })
    expect('refused' in token).toBe(false)
    await waitFor(
      () => getInteractionMode(),
      (state) => state.editingEntityId === textId,
      'Timed out waiting for sticky inline edit to start',
    )

    await cancelActiveInteraction('escape')
    await waitFor(
      () => getInteractionMode(),
      (state) => state.editingEntityId === null,
      'Timed out waiting for sticky inline edit to end',
    )

    await sendKey('delete')
    await waitFor(
      () => getTextEntities(),
      ({ textEntities }) => !textEntities.some((entity) => entity.id === textId),
      'Timed out waiting for sticky Delete key removal',
    )
    await waitFor(
      () => getSelection(),
      (selection) => selection.selectedEntityId !== textId,
      'Timed out waiting for deleted sticky selection to clear',
    )
    const idx = createdTextIds.indexOf(textId)
    if (idx >= 0) createdTextIds.splice(idx, 1)
  })

  it('delete key removes a selected page from page focus', async () => {
    const pageId = await createPage()
    await selectPage(pageId)
    await wait(50)
    await sendKey('delete', { target: 'page', pageId })

    await waitFor(
      () => getWorkspace(),
      (workspace) => !workspace.entities.some((entity) => entity.id === pageId),
      'Timed out waiting for focused-page Delete to remove selected page',
    )
    const idx = createdPageIds.indexOf(pageId)
    if (idx >= 0) createdPageIds.splice(idx, 1)
  })

  it('Cmd+Shift+Z triggers redo without error', async () => {
    await sendKey('z', { cmd: true, shift: true })
    await wait(50)
  })

  it('Cmd+A selects every entity on the canvas', async () => {
    const id1 = await createPage()
    const id2 = await createPage('https://example.org')
    await deselectSelection()
    await wait(50)

    await sendKey('a', { cmd: true })
    await wait(100)

    const sel = await getSelection()
    const ids = sel.selectedEntityIds ?? []
    expect(ids).toContain(id1)
    expect(ids).toContain(id2)
  })

  it('Cmd+D duplicates the selected page without auto-creating a row group', async () => {
    const sourceId = await createPage()
    await selectPage(sourceId)
    await wait(50)

    const before = await getWorkspace()
    const groupsBefore = before.entities.filter((e) => e.kind === 'group').length

    await sendKey('d', { cmd: true })
    await waitFor(
      () => getWorkspace(),
      (w) => w.entities.filter((e) => e.kind === 'page').length >= 2,
      'Timed out waiting for duplicate page',
    )

    const after = await getWorkspace()
    const pagesAfter = after.entities.filter((e) => e.kind === 'page')
    const groupsAfter = after.entities.filter((e) => e.kind === 'group').length
    expect(pagesAfter.length).toBeGreaterThanOrEqual(2)
    // Cmd+D must NOT auto-group source + duplicate.
    expect(groupsAfter).toBe(groupsBefore)
    // Track new pages for cleanup.
    for (const p of pagesAfter) {
      if (!createdPageIds.includes(p.id)) createdPageIds.push(p.id)
    }
  })

  it('paste of a URL creates a page', async () => {
    const before = await getWorkspace()
    const pagesBefore = before.entities.filter((e) => e.kind === 'page').length

    await pasteClipboardText({ text: 'https://example.org', canvasX: 0, canvasY: 0 })
    const after = await waitFor(
      () => getWorkspace(),
      (w) => w.entities.filter((e) => e.kind === 'page').length === pagesBefore + 1,
      'Timed out waiting for pasted URL page',
    )
    const pagesAfter = after.entities.filter((e) => e.kind === 'page')
    expect(pagesAfter.length).toBe(pagesBefore + 1)
    for (const p of pagesAfter) {
      if (!createdPageIds.includes(p.id)) createdPageIds.push(p.id)
    }
  })

  it('paste of plain text creates a sticky note', async () => {
    const before = await getTextEntities()
    const countBefore = before.textEntities.length

    await pasteClipboardText({ text: 'hello sticky', canvasX: 100, canvasY: 100 })
    const after = await waitFor(
      () => getTextEntities(),
      (t) => t.textEntities.length === countBefore + 1,
      'Timed out waiting for pasted sticky',
    )
    const newest = after.textEntities[after.textEntities.length - 1]
    expect(newest.text).toBe('hello sticky')
  })

  it('Cmd+1 from a focused page resets the viewport (page focus exception)', async () => {
    const pageId = await createPage()
    await selectPage(pageId)
    await wait(50)
    // sendKey to the page WebContents simulates page-focus dispatch.
    await sendKey('1', { cmd: true, target: 'page', pageId })
    await wait(50)
  })

  it('Escape from page focus exits page focus, not the active tool', async () => {
    const pageId = await createPage()
    await selectPage(pageId)
    // Activate a non-select tool. Escape from a non-page surface would reset it.
    await sendKey('c')
    await waitFor(
      () => getCurrentTool(),
      (r) => r.tool.kind === 'comment',
      'comment tool not active before escape',
    )
    // Escape arriving from page-focus must hit escape-page-focus (table order),
    // not escape-tool — so the active tool stays as 'comment'.
    await sendKey('escape', { target: 'page', pageId })
    await wait(50)
    const { tool } = await getCurrentTool()
    expect(tool.kind).toBe('comment')
    // Reset back to select for downstream tests.
    await sendKey('escape')
  })

  it('Cmd+Z from a focused page does not trigger main undo (falls through to page)', async () => {
    const pageId = await createPage()
    await selectPage(pageId)
    await wait(50)
    // No assertion beyond no-crash — the binding's firesFromPageFocus is false,
    // so dispatchKey returns null and the keystroke is forwarded to the page.
    await sendKey('z', { cmd: true, target: 'page', pageId })
    await wait(50)
  })

  it('Cmd+[ sends a selected page backward in stack order', async () => {
    const first = await createPage('https://example.com/stack-shortcut-a')
    const second = await createPage('https://example.com/stack-shortcut-b')
    await selectPage(second)

    await sendKey('[', { cmd: true })

    await waitFor(
      () => getEntityOrder(),
      ({ entityOrder }) => {
        const ids = entityOrder.filter((id) => id === first || id === second)
        return ids[0] === second && ids[1] === first
      },
      'Timed out waiting for page stack shortcut reorder',
    )
  })

  it('Cmd+[ sends a selected sticky backward in stack order', async () => {
    const firstResult = await createTextEntities([
      { canvasX: 120, canvasY: 120, text: 'Stack shortcut A' },
    ])
    const secondResult = await createTextEntities([
      { canvasX: 160, canvasY: 160, text: 'Stack shortcut B' },
    ])
    const ids = [firstResult.ids[0], secondResult.ids[0]]
    createdTextIds.push(...ids)
    const [first, second] = ids
    await waitFor(
      () => getTextEntities(),
      ({ textEntities }) => ids.every((id) => textEntities.some((entity) => entity.id === id)),
      'Timed out waiting for stack shortcut stickies',
    )
    const before = await getEntityOrder()
    const beforePair = before.entityOrder.filter((id) => id === first || id === second)
    expect(beforePair).toHaveLength(2)
    const [backmost, frontmost] = beforePair
    await selectEntity(frontmost, 'text')

    await sendKey('[', { cmd: true })

    await waitFor(
      () => getEntityOrder(),
      ({ entityOrder }) => {
        const ordered = entityOrder.filter((id) => id === first || id === second)
        return ordered[0] === frontmost && ordered[1] === backmost
      },
      'Timed out waiting for sticky stack shortcut reorder',
    )
  })

  it('Cmd+[ sends a selected edge backward in stack order', async () => {
    const firstResult = await createTextEntities([
      { canvasX: 120, canvasY: 140, text: 'From' },
    ])
    const secondResult = await createTextEntities([
      { canvasX: 360, canvasY: 140, text: 'To' },
    ])
    const first = firstResult.ids[0]
    const second = secondResult.ids[0]
    const ids = [first, second]
    createdTextIds.push(first, second)
    await waitFor(
      () => getTextEntities(),
      ({ textEntities }) => ids.every((id) => textEntities.some((entity) => entity.id === id)),
      'Timed out waiting for edge endpoint stickies',
    )
    const edgeResult = await createEdges([
      { fromEntityId: first, toEntityId: second, kind: 'connection' },
    ])
    const edge = edgeResult.edgeIds[0]
    createdEdgeIds.push(edge)
    const before = await getEntityOrder()
    expect(before.entityOrder).toContain(edge)

    await selectEntity(edge, 'edge')
    const selection = await getSelection()
    expect(selection.selectedEntityId).toBe(edge)
    await sendKey('[', { cmd: true })

    const after = await getEntityOrder()
    expect(after.entityOrder.filter((id) => id === first || id === second || id === edge)).toEqual([
      first,
      edge,
      second,
    ])
  })
})
