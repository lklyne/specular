/**
 * Tab identity for agents (issue #360, phase 1) against the real runtime.
 *
 * Guards the three things an agent needs to stop being coupled to the user's
 * focus: the `GET /canvas` read names the canvas that answered and its
 * siblings (`readCanvasDocument`); `tab new` creates a canvas without moving
 * focus and refuses a duplicate name (`createBackgroundWorkspaceTab`); and a
 * tab ref resolves by id or exact name, erroring with candidates rather than
 * guessing (`resolveWorkspaceTabRef`).
 *
 * Mutation-verified by: (a) restoring `setActiveWorkspaceTab(nextTab.id)` in
 * `createBackgroundWorkspaceTab` — the "does not activate" case fails; (b)
 * dropping the duplicate-name guard there — the duplicate case fails; (c)
 * making `resolveWorkspaceTabRef` fall back to the first name match instead
 * of erroring — the ambiguous case fails; (d) deleting the `doc.appState.tabs`
 * assignment in `readCanvasDocument` — the read case fails; (e) building the
 * identity from unsynced tab records instead of `workspaceTabSummaries()` —
 * the active tab's entityCount reads back as 0 instead of 1.
 *
 * Deletion is the other half of the create surface: removing a canvas the user
 * is not looking at must not move them (`deleteWorkspaceTab`). Mutation-verified
 * by: (h) dropping the `tabId !== activeWorkspaceTabId` early return there —
 * the background-delete case re-activates a neighbour and fails. The scenario
 * needs three tabs; with two, the fallback coincides with the active tab and
 * the regression is invisible.
 *
 * Also guards the side-effect half of a tab switch (`applyTabState`), which
 * data assertions alone leave unprotected. Mutation-verified by: (f) dropping
 * the `resetUiStateForTabSwitch()` call in `applyTabState` — the selection
 * survives the switch; (g) dropping the `applyEmptyTabViewState(...)` call —
 * the outgoing tab's zoom and pan survive the switch to an empty canvas.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { createTextEntity } from '../../src/main/runtime/document-commands'
import { readCanvasDocument } from '../../src/main/routes/canvas'
import {
  createBackgroundWorkspaceTab,
  createWorkspaceTab,
  deleteWorkspaceTab,
  setActiveWorkspaceTab,
} from '../../src/main/runtime/workspace-tab-operations'
import { resolveWorkspaceTabRef } from '../../src/main/runtime/workspace-tab-refs'
import { activeWorkspaceTabId, workspaceTabs } from '../../src/main/runtime/workspace-model'
import { selectEntity } from '../../src/main/runtime/selection-controller'
import { getSelectionState } from '../../src/main/workspace-entities'
import { setPan, setZoom } from '../../src/main/runtime/viewport-control'
import { getZoom } from '../../src/main/runtime/runtime-context'
import { workspaceSnapshot } from '../../src/main/runtime/workspace-tabs'
import { getUiState } from '../../src/main/ui-state'

let harness: WorkspaceHarness

describe('tab identity', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('reports the active tab and every tab in the canvas read', async () => {
    const created = createBackgroundWorkspaceTab('scratch')
    expect(created.ok).toBe(true)
    // Created after the last tab-record sync: the active tab's entityCount has
    // to come from live runtime state, not its persisted snapshot.
    createTextEntity({ canvasX: 0, canvasY: 0, text: 'live' })
    await settleSync()

    const appState = readCanvasDocument().appState
    expect(appState?.activeTab).toEqual({ id: activeWorkspaceTabId, name: 'Blank' })
    expect(appState?.tabs).toEqual([
      { id: activeWorkspaceTabId, name: 'Blank', entityCount: 1 },
      { id: created.ok ? created.id : '', name: 'scratch', entityCount: 0 },
    ])
  })

  it('creates a tab without activating it', () => {
    const before = activeWorkspaceTabId
    const created = createBackgroundWorkspaceTab('scratch')

    expect(created).toEqual({ ok: true, id: expect.stringMatching(/^tab_/) })
    expect(activeWorkspaceTabId).toBe(before)
    expect(workspaceTabs.map((tab) => tab.name)).toEqual(['Blank', 'scratch'])
  })

  it('refuses a duplicate tab name', () => {
    createBackgroundWorkspaceTab('scratch')
    const again = createBackgroundWorkspaceTab('  scratch  ')

    expect(again).toEqual({ ok: false, error: "a tab named 'scratch' already exists" })
    expect(workspaceTabs).toHaveLength(2)
  })

  it('refuses an empty tab name', () => {
    expect(createBackgroundWorkspaceTab('   ')).toEqual({
      ok: false,
      error: 'tab name is required',
    })
    expect(workspaceTabs).toHaveLength(1)
  })

  it('switches to a tab by id and by name', () => {
    const created = createBackgroundWorkspaceTab('scratch')
    const id = created.ok ? created.id : ''

    const byId = resolveWorkspaceTabRef(id)
    expect(byId.ok && setActiveWorkspaceTab(byId.tab.id)).toBe(true)
    expect(activeWorkspaceTabId).toBe(id)

    const byName = resolveWorkspaceTabRef('Blank')
    expect(byName.ok && setActiveWorkspaceTab(byName.tab.id)).toBe(true)
    expect(activeWorkspaceTabId).not.toBe(id)
  })

  it('clears selection and resets the viewport when switching to an empty tab', async () => {
    const note = createTextEntity({ canvasX: 120, canvasY: 40, text: 'selected' })
    selectEntity(note.id, 'text')
    setZoom(2)
    setPan(400, 300)
    await settleSync()
    expect(getSelectionState().selectedEntityIds).toEqual([note.id])

    const created = createBackgroundWorkspaceTab('empty')
    expect(setActiveWorkspaceTab(created.ok ? created.id : '')).toBe(true)

    expect(getSelectionState().selectedEntityIds ?? []).toEqual([])
    expect(getUiState().selection).toEqual({ kind: 'none' })
    expect(getZoom()).toBe(1)
    expect(workspaceSnapshot().pan).toEqual({ x: 0, y: 0 })
  })

  it('errors with candidate ids for an ambiguous name', () => {
    // Duplicate names are reachable through the UI create path, which does not
    // refuse them; the CLI's refusal only stops new ones.
    const first = createWorkspaceTab('scratch')
    const second = createWorkspaceTab('scratch')

    expect(resolveWorkspaceTabRef('scratch')).toEqual({
      ok: false,
      error: `tab name 'scratch' matches 2 tabs: ${first}, ${second} — use an id`,
    })
  })

  it('deletes a background tab without moving the user', () => {
    // Three tabs, with the doomed one last and the user on the first: the
    // neighbour the old fallback would jump to is neither, so a regression
    // that re-activates on every delete is visible here (with two tabs the
    // fallback coincides with the active tab and the bug hides).
    const middle = createBackgroundWorkspaceTab('middle')
    const doomed = createBackgroundWorkspaceTab('doomed')
    const doomedId = doomed.ok ? doomed.id : ''
    const activeBefore = activeWorkspaceTabId
    expect(workspaceTabs).toHaveLength(3)
    expect(middle.ok && middle.id).not.toBe(activeBefore)

    expect(deleteWorkspaceTab(doomedId)).toBe(true)

    expect(workspaceTabs.map((tab) => tab.id)).not.toContain(doomedId)
    expect(activeWorkspaceTabId).toBe(activeBefore)
  })

  it('falls back to a neighbour when the active tab is deleted', () => {
    const created = createBackgroundWorkspaceTab('scratch')
    const scratchId = created.ok ? created.id : ''
    const blankId = activeWorkspaceTabId
    expect(setActiveWorkspaceTab(scratchId)).toBe(true)

    expect(deleteWorkspaceTab(scratchId)).toBe(true)

    expect(workspaceTabs.map((tab) => tab.id)).toEqual([blankId])
    expect(activeWorkspaceTabId).toBe(blankId)
  })

  it('resets the last tab to an empty default rather than removing it', () => {
    const note = createTextEntity({ canvasX: 0, canvasY: 0, text: 'gone' })
    expect(workspaceTabs).toHaveLength(1)

    expect(deleteWorkspaceTab(activeWorkspaceTabId)).toBe(true)

    expect(workspaceTabs).toHaveLength(1)
    expect(Object.keys(workspaceSnapshot().entities ?? {})).not.toContain(note.id)
  })

  it('errors with the available tabs for an unknown ref', () => {
    const resolved = resolveWorkspaceTabRef('nope')
    expect(resolved.ok).toBe(false)
    expect(!resolved.ok && resolved.error).toBe(
      `unknown tab 'nope' — available: ${activeWorkspaceTabId} (Blank)`,
    )
  })
})
