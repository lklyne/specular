import { afterEach, describe, expect, it } from 'vitest'
import {
  createPages,
  createGroup,
  deleteGroups,
  deletePages,
  getSidebar,
  getWorkspace,
} from './app-client'
import { assertPersists, assertUndoable } from './test-utils'

const createdPageIds: string[] = []

async function createPage(input: { url: string; canvasX: number; canvasY: number; presetIndex?: number }) {
  const result = await createPages([input])
  createdPageIds.push(...result.pageIds)
  return result.pageIds[0]
}

async function cleanupPages() {
  if (!createdPageIds.length) return
  const pageIds = createdPageIds.splice(0, createdPageIds.length)
  await deletePages(pageIds)
}

describe('left sidebar hierarchy', () => {
  afterEach(async () => {
    await cleanupPages()
  })

  it('serializes nested user groups as nested sidebar items', async () => {
    const innerLeft = await createPage({
      url: 'https://example.com',
      canvasX: 160,
      canvasY: 120,
    })
    const innerRight = await createPage({
      url: 'https://example.org',
      canvasX: 520,
      canvasY: 120,
    })
    const outerOnly = await createPage({
      url: 'https://example.net',
      canvasX: 920,
      canvasY: 120,
    })

    const innerGroup = await createGroup([innerLeft, innerRight], 'Inner group')
    const outerGroup = await createGroup([innerLeft, innerRight, outerOnly], 'Outer group')

    const sidebar = await getSidebar()
    const outerItem = sidebar.items.find((item) => item.id === outerGroup.id)
    expect(outerItem).toMatchObject({
      kind: 'group',
      label: 'Outer group',
      entityCount: 3,
    })

    expect(sidebar.items.some((item) => item.id === innerGroup.id)).toBe(false)

    const outerChildren = Array.isArray(outerItem?.children) ? outerItem.children : []
    expect(outerChildren).toHaveLength(2)
    expect(outerChildren.find((item) => item.id === innerGroup.id)).toMatchObject({
      kind: 'group',
      id: innerGroup.id,
      label: 'Inner group',
      entityCount: 2,
    })
    expect(outerChildren.find((item) => item.id === outerOnly)).toMatchObject({
      kind: 'page',
      id: outerOnly,
    })
  })
})

describe('left sidebar hierarchy — lifecycle', () => {
  afterEach(async () => {
    const graph = await getWorkspace()
    const groupIds = graph.entities.filter((e) => e.kind === 'group').map((e) => e.id)
    if (groupIds.length) await deleteGroups(groupIds)
    if (createdPageIds.length) {
      await deletePages(createdPageIds.splice(0))
    }
  })

  it('persists nested groups to disk', async () => {
    const innerLeft = await createPage({
      url: 'https://example.com',
      canvasX: 180,
      canvasY: 320,
    })
    const innerRight = await createPage({
      url: 'https://example.org',
      canvasX: 540,
      canvasY: 320,
    })
    const inner = await createGroup([innerLeft, innerRight], 'Inner persist')
    await assertPersists(async () => {
      await createGroup([innerLeft, innerRight], 'Outer persist')
    })
    expect(inner).toBeDefined()
  })

  it('round-trips a nested group creation through undo/redo', async () => {
    const innerLeft = await createPage({
      url: 'https://example.com',
      canvasX: 180,
      canvasY: 520,
    })
    const innerRight = await createPage({
      url: 'https://example.org',
      canvasX: 540,
      canvasY: 520,
    })
    await createGroup([innerLeft, innerRight], 'Inner undo')
    await assertUndoable(async () => {
      await createGroup([innerLeft, innerRight], 'Outer undo')
    })
  })
})
