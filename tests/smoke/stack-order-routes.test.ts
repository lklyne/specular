import { afterEach, describe, expect, it } from 'vitest'
import {
  createGroup,
  createTextEntities,
  deleteGroups,
  deleteTextEntities,
  getEntityOrder,
  getSidebar,
  reorderStackOrder,
} from './app-client'

const createdTextIds: string[] = []
const createdGroupIds: string[] = []

async function createText(label: string, x: number) {
  const result = await createTextEntities([{ text: label, canvasX: x, canvasY: 200 }])
  createdTextIds.push(...result.ids)
  return result.ids[0]
}

function idsInOrder(order: string[], ids: string[]) {
  const wanted = new Set(ids)
  return order.filter((id) => wanted.has(id))
}

describe('stack-order HTTP routes', () => {
  afterEach(async () => {
    if (createdGroupIds.length) await deleteGroups(createdGroupIds.splice(0))
    if (createdTextIds.length) await deleteTextEntities(createdTextIds.splice(0))
  })

  it('drives the four stack-order mutations for a single id', async () => {
    const first = await createText('HTTP stack A', 120)
    const second = await createText('HTTP stack B', 360)
    const third = await createText('HTTP stack C', 600)
    const ids = [first, second, third]

    expect(idsInOrder((await getEntityOrder()).entityOrder, ids)).toEqual([first, second, third])

    await reorderStackOrder('send-backward', { id: third })
    expect(idsInOrder((await getEntityOrder()).entityOrder, ids)).toEqual([first, third, second])

    await reorderStackOrder('bring-forward', { id: third })
    expect(idsInOrder((await getEntityOrder()).entityOrder, ids)).toEqual([first, second, third])

    await reorderStackOrder('send-to-back', { id: third })
    expect(idsInOrder((await getEntityOrder()).entityOrder, ids)).toEqual([third, first, second])

    await reorderStackOrder('bring-to-front', { id: third })
    expect(idsInOrder((await getEntityOrder()).entityOrder, ids)).toEqual([first, second, third])
  })

  it('moves an ids array as one block and updates the sidebar ordering', async () => {
    const first = await createText('HTTP block A', 120)
    const second = await createText('HTTP block B', 360)
    const third = await createText('HTTP block C', 600)
    const fourth = await createText('HTTP block D', 840)
    const ids = [first, second, third, fourth]

    await reorderStackOrder('send-backward', { ids: [second, third] })

    const expected = [second, third, first, fourth]
    expect(idsInOrder((await getEntityOrder()).entityOrder, ids)).toEqual(expected)
    expect(idsInOrder((await getSidebar()).sections.notes.map((item) => item.id), ids)).toEqual(expected)
  })

  it('preserves group contiguity when a member is reordered through HTTP', async () => {
    const first = await createText('HTTP group A', 120)
    const second = await createText('HTTP group B', 360)
    const third = await createText('HTTP group C', 600)
    const group = await createGroup([first, second], 'HTTP stack group')
    createdGroupIds.push(group.id)
    const ids = [first, second, group.id, third]

    await reorderStackOrder('send-to-back', { id: first })

    const ordered = idsInOrder((await getEntityOrder()).entityOrder, ids)
    const groupRun = ordered.filter((id) => id === first || id === second || id === group.id)
    const runStart = ordered.indexOf(groupRun[0])
    expect(ordered.slice(runStart, runStart + groupRun.length)).toEqual(groupRun)
    expect(groupRun[groupRun.length - 1]).toBe(group.id)
  })
})
