import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import {
  applyMultiResize,
  beginInteraction,
  beginMultiResize,
  cancelActiveInteraction,
  cancelInteraction,
  commitInteraction,
  createPages,
  createTextEntities,
  deletePages,
  deleteTextEntities,
  endMultiResize,
  getInteractionMode,
  getTextEntities,
  getUndoState,
  resetInteraction,
  resetSmokeState,
  undoWorkspace,
  type CancelReason,
  type InteractionToken,
  type TryEnterInput,
} from './app-client'
import { wait } from './test-utils'

/**
 * Per-mode begin/commit/cancel matrix for the InteractionController.
 * Spec docs/interaction-layer.md §9.
 *
 * Tests validate the controller's state machine via the test HTTP routes.
 */

const createdPageIds: string[] = []

async function createPage(): Promise<string> {
  const result = await createPages([{ url: 'https://example.com', canvasX: 120, canvasY: 120 }])
  createdPageIds.push(...result.pageIds)
  return result.pageIds[0]
}

async function cleanupPages() {
  if (!createdPageIds.length) return
  await deletePages(createdPageIds.splice(0))
}

beforeEach(async () => {
  await resetInteraction()
})

afterEach(async () => {
  await resetInteraction()
  await cleanupPages()
})

const modes: Array<{ label: string; build: () => Promise<TryEnterInput> | TryEnterInput }> = [
  { label: 'panning', build: () => ({ kind: 'panning' }) },
  { label: 'marquee', build: () => ({ kind: 'marquee' }) },
  {
    label: 'dragging-entities',
    build: async () => ({ kind: 'dragging-entities', entityIds: [await createPage()] }),
  },
  {
    label: 'resizing-entity',
    build: async () => ({ kind: 'resizing-entity', target: { kind: 'page', id: await createPage() } }),
  },
  {
    label: 'resizing-multi-selection',
    build: () => ({ kind: 'resizing-multi-selection' as const }),
  },
  {
    label: 'editing-entity',
    build: async () => ({ kind: 'editing-entity', entityId: await createPage() }),
  },
  {
    label: 'dragging-edge',
    build: async () => ({
      kind: 'dragging-edge',
      from: { kind: 'page', id: await createPage() },
      fromSide: 'right',
    }),
  },
]

describe('InteractionController state machine', () => {
  it('starts idle', async () => {
    const { mode } = await getInteractionMode()
    expect(mode.kind).toBe('idle')
  })

  for (const { label, build } of modes) {
    describe(`mode: ${label}`, () => {
      it('tryEnter → commit returns to idle', async () => {
        const input = await build()
        const token = await beginInteraction(input)
        expect('refused' in token).toBe(false)
        const t = token as InteractionToken
        const { mode } = await getInteractionMode()
        expect(mode.kind).not.toBe('idle')
        await commitInteraction(t)
        const after = await getInteractionMode()
        expect(after.mode.kind).toBe('idle')
      })

      const reasons: CancelReason[] = ['blur', 'escape', 'undo', 'tab-switch', 'external']
      for (const reason of reasons) {
        it(`tryEnter → cancel(${reason}) returns to idle`, async () => {
          const input = await build()
          const token = await beginInteraction(input)
          const t = token as InteractionToken
          await cancelInteraction(t, reason)
          const after = await getInteractionMode()
          expect(after.mode.kind).toBe('idle')
        })
      }

      it('tryEnter → cancelActive returns to idle', async () => {
        const input = await build()
        await beginInteraction(input)
        await cancelActiveInteraction('external')
        const after = await getInteractionMode()
        expect(after.mode.kind).toBe('idle')
      })
    })
  }

  it('refuses concurrent tryEnter while a gesture is active', async () => {
    await beginInteraction({ kind: 'panning' })
    const second = await beginInteraction({ kind: 'marquee' })
    expect('refused' in second).toBe(true)
  })

  it('cancel with stale token is a no-op', async () => {
    const a = (await beginInteraction({ kind: 'panning' })) as InteractionToken
    await commitInteraction(a)
    // Cancel the now-stale token; controller should not throw or affect state.
    await cancelInteraction(a, 'external')
    const after = await getInteractionMode()
    expect(after.mode.kind).toBe('idle')
  })

})

describe('I3 violation fix: conflicting gesture-begin does not corrupt batch/undo', () => {
  const createdTextIds: string[] = []

  beforeEach(async () => {
    await resetSmokeState()
    await resetInteraction()
  })

  afterEach(async () => {
    await resetInteraction()
    if (createdTextIds.length) {
      await deleteTextEntities(createdTextIds.splice(0))
    }
  })

  it('second concurrent multi-resize-begin is refused and does not open a dangling batch', async () => {
    const { ids } = await createTextEntities([
      { canvasX: 0, canvasY: 0, text: 'resize-me', width: 100, height: 50 },
    ])
    createdTextIds.push(...ids)
    await wait(50)

    const before = await getTextEntities()
    const init = before.textEntities.find((t) => t.id === ids[0])!

    // Begin the first gesture.
    const first = await beginMultiResize()
    expect(first.ok).toBe(true)

    // A second begin while the first is active must be refused.
    const second = await beginMultiResize()
    expect(second.ok).toBe(false)
    expect(second.refused).toBe(true)

    // Apply a resize tick on the first gesture.
    await applyMultiResize([
      { id: ids[0], kind: 'text', width: 150, height: 50, canvasX: init.canvasX, canvasY: init.canvasY },
    ])

    // End the first gesture — batch count should be balanced (one begin, one end).
    await endMultiResize()
    await wait(50)

    const afterResize = await getTextEntities()
    const resized = afterResize.textEntities.find((t) => t.id === ids[0])!
    expect(resized.width).toBe(150)

    // Undo must round-trip cleanly — if the batch were mismatched, undo would
    // either no-op or corrupt a prior step.
    const undoState = await getUndoState()
    expect(undoState.canUndo).toBe(true)

    await undoWorkspace()
    await wait(50)

    const afterUndo = await getTextEntities()
    const undone = afterUndo.textEntities.find((t) => t.id === ids[0])!
    expect(undone.width).toBe(init.width)
  })
})
