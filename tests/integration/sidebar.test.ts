/**
 * Left-sidebar hierarchy + stack-order reorder against the real runtime,
 * in-process.
 *
 * Guards `getLeftSidebarData` (what GET /sidebar serves): nested user groups
 * serialize as nested sidebar items and persist to disk / round-trip undo,
 * and `reorderSidebarStackOrder`: page rows reorder within the Pages section
 * (undo/redo restores the stack) while note-row reorders never move page or
 * edge stack slots.
 *
 * Mutation-verified by: deleting the `setEntityParentGroupId(entityId,
 * group.id)` loop in `createUserGroup` (src/main/workspace-groups.ts) — the
 * hierarchy, disk-persistence, and undo/redo cases all fail.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import type { JsonCanvasLinkNode } from '../../src/shared/json-canvas-types'
import { createTextEntity } from '../../src/main/runtime/document-commands'
import { createEdges } from '../../src/main/workspace-edges'
import { createUserGroup } from '../../src/main/workspace-groups'
import { getLeftSidebarData } from '../../src/main/runtime/canvas-layout-data'
import {
  currentEntityOrder,
  reorderSidebarStackOrder,
} from '../../src/main/runtime/entity-order-state'
import { selectNone } from '../../src/main/runtime/selection-controller'
import { pages } from '../../src/main/runtime/runtime-context'
import { workspaceGroups } from '../../src/main/runtime/workspace-model'
import { undo, redo } from '../../src/main/runtime/workspace-undo'
import {
  createAnnotation,
  updateAnnotationStatus,
} from '../../src/main/workspace-annotations'

let harness: WorkspaceHarness

function linkNode(id: string, x: number): JsonCanvasLinkNode {
  return {
    id,
    type: 'link',
    x,
    y: 120,
    width: 375,
    height: 667,
    url: `https://example.com/${id}`,
    presetIndex: 0,
  }
}

function idsInOrder(order: string[], ids: string[]): string[] {
  const wanted = new Set(ids)
  return order.filter((id) => wanted.has(id))
}

describe('left sidebar', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    selectNone()
  })

  afterAll(() => harness?.dispose())

  it('serializes nested user groups as nested sidebar items', async () => {
    const innerLeft = createTextEntity({ canvasX: 0, canvasY: 0, text: 'inner left' })
    const innerRight = createTextEntity({ canvasX: 240, canvasY: 0, text: 'inner right' })
    const outerOnly = createTextEntity({ canvasX: 640, canvasY: 0, text: 'outer only' })
    await settleSync()

    const innerGroup = createUserGroup([innerLeft.id, innerRight.id], 'Inner group')
    const outerGroup = createUserGroup([innerLeft.id, innerRight.id, outerOnly.id], 'Outer group')
    await settleSync()

    const sidebar = getLeftSidebarData()
    const outerItem = sidebar.items.find((item) => item.id === outerGroup.id)
    expect(outerItem).toMatchObject({ kind: 'group', label: 'Outer group', entityCount: 3 })
    expect(sidebar.items.some((item) => item.id === innerGroup.id)).toBe(false)

    const outerChildren =
      outerItem && 'children' in outerItem && Array.isArray(outerItem.children)
        ? outerItem.children
        : []
    expect(outerChildren).toHaveLength(2)
    expect(outerChildren.find((item) => item.id === innerGroup.id)).toMatchObject({
      kind: 'group',
      label: 'Inner group',
      entityCount: 2,
    })
    expect(outerChildren.find((item) => item.id === outerOnly.id)).toMatchObject({
      kind: 'text',
    })
  })

  it('persists nested groups to disk', async () => {
    harness.loadFixture({
      name: 'Nested persist',
      doc: {
        nodes: [linkNode('page-a', 120), linkNode('page-b', 620)],
        edges: [],
        appState: { zoom: 1, pan: { x: 0, y: 0 } },
      },
    })
    const inner = createUserGroup(['page-a', 'page-b'], 'Inner persist')
    await settleSync()
    const outer = createUserGroup(['page-a', 'page-b'], 'Outer persist')
    await settleSync()

    const disk = harness.diskDoc()
    expect(disk?.nodes.find((n) => n.id === outer.id)).toMatchObject({
      type: 'group',
      label: 'Outer persist',
    })
    expect(disk?.nodes.find((n) => n.id === inner.id)).toMatchObject({
      type: 'group',
      label: 'Inner persist',
      parentGroupId: outer.id,
    })
    for (const pageId of ['page-a', 'page-b']) {
      expect(disk?.nodes.find((n) => n.id === pageId)).toMatchObject({
        parentGroupId: inner.id,
      })
    }
  })

  it('round-trips nested group creation through undo/redo', async () => {
    const a = createTextEntity({ canvasX: 0, canvasY: 400, text: 'nested a' })
    const b = createTextEntity({ canvasX: 300, canvasY: 400, text: 'nested b' })
    await settleSync()
    const inner = createUserGroup([a.id, b.id], 'Inner undo')
    await settleSync()
    const outer = createUserGroup([a.id, b.id], 'Outer undo')
    await settleSync()

    const innerGroup = () => workspaceGroups.find((g) => g.id === inner.id)
    expect(innerGroup()?.parentGroupId).toBe(outer.id)

    undo()
    expect(workspaceGroups.some((g) => g.id === outer.id)).toBe(false)
    expect(innerGroup()?.parentGroupId).toBeUndefined()

    redo()
    expect(workspaceGroups.some((g) => g.id === outer.id)).toBe(true)
    expect(innerGroup()?.parentGroupId).toBe(outer.id)
  })

  it('reorders page rows within the Pages section and undo/redo restores the stack', async () => {
    harness.loadFixture({
      name: 'Sidebar pages',
      doc: {
        nodes: [linkNode('page-a', 120), linkNode('page-b', 620), linkNode('page-c', 1120)],
        edges: [],
        appState: { zoom: 1, pan: { x: 0, y: 0 } },
      },
    })
    const ids = ['page-a', 'page-b', 'page-c']

    // The sidebar renders the stack top-first, i.e. the reverse of entityOrder.
    const pagesSection = () =>
      idsInOrder(getLeftSidebarData().sections.pages.map((item) => item.id), ids)
    expect(pagesSection()).toEqual(['page-c', 'page-b', 'page-a'])

    expect(
      reorderSidebarStackOrder({
        section: 'pages',
        draggedId: 'page-c',
        anchorId: 'page-a',
        position: 'before',
        parentId: null,
      }),
    ).toBe(true)
    await settleSync()

    expect(pagesSection()).toEqual(['page-b', 'page-a', 'page-c'])
    expect(idsInOrder(currentEntityOrder(), ids)).toEqual(['page-c', 'page-a', 'page-b'])

    undo()
    expect(idsInOrder(currentEntityOrder(), ids)).toEqual(['page-a', 'page-b', 'page-c'])

    redo()
    expect(idsInOrder(currentEntityOrder(), ids)).toEqual(['page-c', 'page-a', 'page-b'])
  })

  // Mutation-verified by: removing the `...(children.length ? { children } : {})`
  // spread in `describeSidebarLeaf` (sidebar-builder.ts) — the nesting and
  // resolution cases fail; hard-coding `onCurrentPage: true` in the annotation
  // projection of `sidebarPageChildren` — the navigation case fails.
  it('nests page-anchored annotations under their page and dims them after navigation', async () => {
    harness.loadFixture({
      name: 'Sidebar annotations',
      doc: {
        nodes: [linkNode('page-a', 120)],
        edges: [],
        appState: { zoom: 1, pan: { x: 0, y: 0 } },
      },
    })

    const created = createAnnotation({
      anchor: { type: 'page', pageId: 'page-a', offsetX: 0.5, offsetY: 0.4 },
      text: 'Tighten the hero spacing before shipping',
    })
    await settleSync()

    const pageItem = () => {
      const item = getLeftSidebarData().sections.pages.find((entry) => entry.id === 'page-a')
      return item && item.kind === 'page' ? item : null
    }

    expect(pageItem()?.children).toEqual([
      {
        kind: 'annotation',
        id: created.id,
        label: 'Tighten the hero spacing before shipping',
        messageCount: 1,
        onCurrentPage: true,
      },
    ])

    // did-navigate updates page.url in place; the sidebar must flag the
    // annotation as no longer on the page's current document.
    const page = pages.find((candidate) => candidate.id === 'page-a')
    expect(page).toBeDefined()
    page!.url = 'https://example.com/elsewhere'
    expect(pageItem()?.children?.[0]?.onCurrentPage).toBe(false)

    // Resolving the thread removes the child row entirely.
    updateAnnotationStatus(created.id, 'resolved', undefined, 'user')
    await settleSync()
    expect(pageItem()?.children).toBeUndefined()
  })

  it('reorders note rows without moving page or edge stack slots', async () => {
    harness.loadFixture({
      name: 'Sidebar notes',
      doc: {
        nodes: [linkNode('page-slot', 120)],
        edges: [],
        appState: { zoom: 1, pan: { x: 0, y: 0 } },
      },
    })
    const alpha = createTextEntity({ canvasX: 120, canvasY: 900, text: 'Alpha' })
    const beta = createTextEntity({ canvasX: 360, canvasY: 900, text: 'Beta' })
    const gamma = createTextEntity({ canvasX: 600, canvasY: 900, text: 'Gamma' })
    const { edgeIds } = createEdges({
      edges: [{ fromEntityId: alpha.id, toEntityId: beta.id, kind: 'connection' }],
    })
    await settleSync()

    const beforeOrder = currentEntityOrder()
    const pageIndex = beforeOrder.indexOf('page-slot')
    const edgeIndex = beforeOrder.indexOf(edgeIds[0])
    expect(pageIndex).toBeGreaterThanOrEqual(0)
    expect(edgeIndex).toBeGreaterThanOrEqual(0)

    expect(
      reorderSidebarStackOrder({
        section: 'notes',
        draggedId: gamma.id,
        anchorId: alpha.id,
        position: 'before',
        parentId: null,
      }),
    ).toBe(true)
    await settleSync()

    const afterOrder = currentEntityOrder()
    expect(idsInOrder(afterOrder, [alpha.id, beta.id, gamma.id])).toEqual([
      gamma.id,
      alpha.id,
      beta.id,
    ])
    expect(afterOrder.indexOf('page-slot')).toBe(pageIndex)
    expect(afterOrder.indexOf(edgeIds[0])).toBe(edgeIndex)
  })
})
