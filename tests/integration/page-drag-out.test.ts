/**
 * Drag-out from a page onto the canvas (ADR 0038). Covers the `text` payload
 * path only — `link` and `image` route through page/asset creation already
 * covered elsewhere (open-entity-link, clipboard-paste); this guards the
 * arm/consume state machine itself: a payload armed for one page id is only
 * consumable by a matching drop, and consuming it clears it.
 *
 * Mutation-verified by removing the `pendingDrag = null` reset in
 * `takePendingDrag` (src/main/runtime/page-drag-out.ts) — "consuming a drag
 * clears it, so a second drop for the same page finds nothing" then fails
 * because the second drop still finds (and re-creates from) the payload.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import { armPageDrag, dropPageDragOnCanvas } from '../../src/main/runtime/page-drag-out'
import { getTextEntities } from '../../src/main/runtime/document-commands'

let harness: WorkspaceHarness

describe('page drag-out (ADR 0038)', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('drops an armed text payload as a sticky text entity at the release point', async () => {
    armPageDrag('page-1', { kind: 'text', text: 'dragged out' })

    const result = await dropPageDragOnCanvas({ pageId: 'page-1', canvasX: 240, canvasY: 320 })

    expect(result).not.toBeNull()
    const match = getTextEntities().find((e) => e.id === result!.createdId)
    expect(match).toBeDefined()
    expect(match!.text).toBe('dragged out')
    expect(match!.textStyle).toBe('sticky')
    expect(match!.canvasX).toBe(240)
    expect(match!.canvasY).toBe(320)
  })

  it('does not drop for a page id that does not match the armed drag', async () => {
    armPageDrag('page-1', { kind: 'text', text: 'dragged out' })

    const result = await dropPageDragOnCanvas({ pageId: 'page-2', canvasX: 0, canvasY: 0 })

    expect(result).toBeNull()
  })

  it('consumes the drag so a second drop for the same page finds nothing', async () => {
    armPageDrag('page-1', { kind: 'text', text: 'dragged out' })

    const first = await dropPageDragOnCanvas({ pageId: 'page-1', canvasX: 0, canvasY: 0 })
    const second = await dropPageDragOnCanvas({ pageId: 'page-1', canvasX: 0, canvasY: 0 })

    expect(first).not.toBeNull()
    expect(second).toBeNull()
  })
})
