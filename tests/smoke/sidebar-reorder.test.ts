import { afterEach, describe, expect, it } from 'vitest'
import {
  createPages,
  createEdges,
  createTextEntities,
  deleteEdges,
  deletePages,
  deleteTextEntities,
  getEntityOrder,
  getSidebar,
  redoWorkspace,
  reorderSidebarItem,
  undoWorkspace,
} from './app-client'

const createdPageIds: string[] = []
const createdTextIds: string[] = []
const createdEdgeIds: string[] = []

async function createPage(url: string, x: number) {
  const result = await createPages([{ url, canvasX: x, canvasY: 120 }])
  createdPageIds.push(...result.pageIds)
  return result.pageIds[0]
}

async function createText(label: string, x: number) {
  const result = await createTextEntities([{ text: label, canvasX: x, canvasY: 360 }])
  createdTextIds.push(...result.ids)
  return result.ids[0]
}

function idsInOrder(order: string[], ids: string[]) {
  const wanted = new Set(ids)
  return order.filter((id) => wanted.has(id))
}

describe('left sidebar stack-order drag', () => {
  afterEach(async () => {
    if (createdEdgeIds.length) await deleteEdges(createdEdgeIds.splice(0))
    if (createdTextIds.length) await deleteTextEntities(createdTextIds.splice(0))
    if (createdPageIds.length) await deletePages(createdPageIds.splice(0))
  })

  it('reorders page rows within the Pages section and undo/redo restores the stack', async () => {
    const first = await createPage('https://example.com/stack-a', 120)
    const second = await createPage('https://example.com/stack-b', 420)
    const third = await createPage('https://example.com/stack-c', 720)

    expect(idsInOrder((await getSidebar()).sections.pages.map((item) => item.id), [
      first,
      second,
      third,
    ])).toEqual([
      third,
      second,
      first,
    ])

    await reorderSidebarItem({
      section: 'pages',
      draggedId: third,
      anchorId: first,
      position: 'before',
      parentId: null,
    })

    expect(idsInOrder((await getSidebar()).sections.pages.map((item) => item.id), [
      first,
      second,
      third,
    ])).toEqual([
      second,
      first,
      third,
    ])
    expect(idsInOrder((await getEntityOrder()).entityOrder, [first, second, third])).toEqual([
      third,
      first,
      second,
    ])

    await undoWorkspace()
    expect(idsInOrder((await getEntityOrder()).entityOrder, [first, second, third])).toEqual([
      first,
      second,
      third,
    ])

    await redoWorkspace()
    expect(idsInOrder((await getEntityOrder()).entityOrder, [first, second, third])).toEqual([
      third,
      first,
      second,
    ])
  })

  it('reorders note rows without moving page stack slots', async () => {
    const page = await createPage('https://example.com/stack-page', 120)
    const first = await createText('Alpha', 120)
    const second = await createText('Beta', 360)
    const third = await createText('Gamma', 600)
    const beforeOrder = (await getEntityOrder()).entityOrder
    const pageIndex = beforeOrder.indexOf(page)

    await reorderSidebarItem({
      section: 'notes',
      draggedId: third,
      anchorId: first,
      position: 'before',
      parentId: null,
    })

    const afterOrder = (await getEntityOrder()).entityOrder
    expect(idsInOrder(afterOrder, [first, second, third])).toEqual([third, first, second])
    expect(afterOrder.indexOf(page)).toBe(pageIndex)
  })

  it('reorders note rows without moving edge stack slots', async () => {
    const first = await createText('Alpha', 120)
    const second = await createText('Beta', 360)
    const edgeResult = await createEdges([
      { fromEntityId: first, toEntityId: second, kind: 'connection' },
    ])
    const edge = edgeResult.edgeIds[0]
    createdEdgeIds.push(edge)
    const beforeOrder = (await getEntityOrder()).entityOrder
    const edgeIndex = beforeOrder.indexOf(edge)

    await reorderSidebarItem({
      section: 'notes',
      draggedId: second,
      anchorId: first,
      position: 'before',
      parentId: null,
    })

    const afterOrder = (await getEntityOrder()).entityOrder
    expect(idsInOrder(afterOrder, [first, second])).toEqual([second, first])
    expect(afterOrder.indexOf(edge)).toBe(edgeIndex)
  })
})
