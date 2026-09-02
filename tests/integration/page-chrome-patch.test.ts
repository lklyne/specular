/**
 * Page chrome leaves the geometry pass (camera-local projection, phase 4).
 *
 * A page's title, favicon, URL, load state, and back/forward availability were
 * read out of `webContents` inside `buildCanvasLayoutData`, so every pass —
 * one per drag tick — paid a `navigationHistory` walk and an `isLoading()` call
 * per page to observe fields that move a handful of times per page load. The
 * lifecycle hooks that already know when they change now mirror them onto the
 * `Page` record and push one entity patch for the page they moved.
 *
 * Chrome and membership are separate questions, and the cases below are the
 * split. A title, a favicon, or a load beginning moves chrome only, so it
 * patches and arms no pass. A settled load re-opens the document-binding gate
 * (`offPageDocument` reads `page.isLoading`), which adds or removes
 * page-anchored entities — a membership change, so it keeps the pass.
 *
 * The patch and the snapshot come from one builder (`buildPageSceneEntity`),
 * which is what stops them from being two descriptions of the same page.
 *
 * Mutation-verified by:
 * - dropping `broadcastPageChrome(page)` from the `page-title-updated` hook
 *   (page-factory.ts) — the title case's patch assertions fail;
 * - restoring `requestLayout()` in place of it — the "arms no pass" assertion
 *   fails;
 * - dropping `broadcastPageChrome(page)` from the `page-favicon-updated` hook,
 *   and `refreshPageChrome(page)` from `did-start-loading` — the favicon and
 *   load-starting cases fail;
 * - restoring `page.pageView.webContents.navigationHistory.canGoBack()` in
 *   `buildPageSceneEntity` — the mirror case fails, because the stub's history
 *   reports false where the mirror says true;
 * - dropping `markDirty('canvas')` from the `did-stop-loading` hook — the
 *   settled-load case's dirty assertion fails.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import { ipcChannels } from '../../src/shared/ipc-contract'
import type { RuntimePatch, RuntimePatchBatch } from '../../src/shared/runtime-patch'
import type { CanvasScenePageEntity } from '../../src/shared/types'
import { createPage } from '../../src/main/runtime/page-runtime'
import { getCanvasLayoutData } from '../../src/main/runtime/canvas-layout-data'
import { broadcastSceneSnapshot } from '../../src/main/runtime/runtime-patch-broadcast'
import { clearAllDirty, isDirty } from '../../src/main/runtime/layout-dirty'
import { layoutCache } from '../../src/main/runtime/layout-cache'
import { bgView } from '../../src/main/runtime/view-refs'

let harness: WorkspaceHarness

describe('page chrome ships as an entity patch, not a scene rebuild', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  /** Seat the bus baseline on current truth, then clear what the setup
   *  dirtied, so the lifecycle event under test is the only thing observable. */
  function armWatch(): void {
    broadcastSceneSnapshot(getCanvasLayoutData())
    harness.clearBroadcasts()
    clearAllDirty()
    if (layoutCache.layoutTimer) {
      clearTimeout(layoutCache.layoutTimer)
      layoutCache.layoutTimer = null
    }
  }

  function patchesToCanvas(): RuntimePatch[] {
    return harness.broadcasts
      .filter(
        (b) =>
          b.channel === ipcChannels.runtimePatch &&
          b.webContentsId === bgView!.webContents.id,
      )
      .flatMap((send) => (send.args[0] as RuntimePatchBatch).patches)
  }

  it('ships a title change as one patch for that page, arming no pass', () => {
    const page = createPage('https://example.com/titled')
    const wc = page.pageView.webContents as unknown as {
      getTitle(): string
      emit(event: string): void
    }
    wc.getTitle = () => 'Renamed'
    armWatch()

    wc.emit('page-title-updated')

    const patches = patchesToCanvas()
    expect(patches).toHaveLength(1)
    expect(patches[0].kind).toBe('entity')
    expect(patches[0].kind === 'entity' && patches[0].id).toBe(page.id)
    expect(patches[0].kind === 'entity' && patches[0].entity).toMatchObject({
      kind: 'page',
      label: 'Renamed',
    })
    expect(isDirty('canvas')).toBe(false)
    expect(layoutCache.layoutTimer).toBeNull()
  })

  it('ships a load starting as one patch, arming no pass', () => {
    const page = createPage('https://example.com/loading')
    const wc = page.pageView.webContents as unknown as { emit(event: string): void }
    armWatch()

    wc.emit('did-start-loading')

    const patches = patchesToCanvas()
    expect(patches).toHaveLength(1)
    expect(patches[0].kind === 'entity' && patches[0].id).toBe(page.id)
    expect(patches[0].kind === 'entity' && patches[0].entity).toMatchObject({
      isLoading: true,
    })
    expect(isDirty('canvas')).toBe(false)
    expect(layoutCache.layoutTimer).toBeNull()
  })

  it('reads chrome from the runtime mirror, and the snapshot agrees', () => {
    const page = createPage('https://example.com/history')
    const wc = page.pageView.webContents as unknown as {
      emit(event: string, ...args: unknown[]): void
    }
    // The mirror says there is somewhere to go back to; the stub's own
    // navigation history says there isn't. Only one of the two can be what the
    // scene reports, and it has to be the mirror — otherwise a pass is still
    // walking into the renderer process per page.
    page.canGoBack = true
    armWatch()

    wc.emit('page-favicon-updated', {}, ['https://example.com/icon.png'])

    const patches = patchesToCanvas()
    expect(patches).toHaveLength(1)
    const patched = patches[0].kind === 'entity' ? patches[0].entity : null
    expect(patched).toMatchObject({
      faviconUrl: 'https://example.com/icon.png',
      canGoBack: true,
    })

    // The snapshot is the reconcile baseline, so it has to describe the page
    // the same way the patch just did.
    const snapshotEntity = getCanvasLayoutData().entities.find(
      (entity): entity is CanvasScenePageEntity => entity.id === page.id,
    )
    expect(snapshotEntity).toEqual(patched)
  })

  it('keeps the pass for a settled load, which re-opens the document gate', () => {
    const page = createPage('https://example.com/settled')
    const wc = page.pageView.webContents as unknown as { emit(event: string): void }
    wc.emit('did-start-loading')
    armWatch()

    wc.emit('did-stop-loading')

    expect(isDirty('canvas')).toBe(true)
    expect(isDirty('sidebar')).toBe(true)
    expect(layoutCache.layoutTimer).not.toBeNull()
  })
})
