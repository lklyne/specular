/**
 * Scroll-to-comment targeting (ADR 0029 scroll amendment) against
 * the real runtime, in-process. Clicking a comment smooth-scrolls its page to
 * reveal the commented content; the *where* is `computeAnnotationScrollTarget`,
 * a pure function per anchor type. The IPC dispatch itself is not observable
 * under the electron stub, so these assert the pure target — the one new piece
 * of logic the amendment adds.
 *
 * Guards:
 * - page-anchored region → `{ pageId, documentY: docRect.y }`
 *   (annotation-scroll-target.ts `case 'region'`).
 * - canvas-anchored region → null (same case, the `'docRect' in anchor` gate).
 * - canvas point → null (`case 'canvas'`).
 * - page anchor → `offsetY × page.scrollHeight` (`case 'page'`; guards the
 *   phase-1 broadcast extension that stores `page.scrollHeight`).
 * - element anchor, unresolvable selector → falls back to the stored bbox
 *   (`boundingBox.y + page.scrollY`), never throws (`case 'element'`); no bbox
 *   → null.
 *
 * Mutation-verified by:
 * - returning `null` from `computeAnnotationScrollTarget`'s `case 'region'`
 *   page-anchored branch → the page-anchored-region case fails;
 * - returning a non-null target from the canvas-anchored branch (e.g.
 *   `{ pageId: '', documentY: 0 }` before the `'docRect' in anchor` gate) →
 *   the canvas-anchored case fails.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import type { JsonCanvasLinkNode } from '../../src/shared/json-canvas-types'
import type { Annotation } from '../../src/shared/types'
import { createAnnotation } from '../../src/main/workspace-annotations'
import { computeAnnotationScrollTarget } from '../../src/main/runtime/annotation-scroll-target'
import { pages } from '../../src/main/runtime/runtime-context'
import { selectNone } from '../../src/main/runtime/selection-controller'

let harness: WorkspaceHarness

const PAGE_ID = 'page-scroll-host'
const PAGE_URL = 'https://example.com/pricing'

function hostPageNode(): JsonCanvasLinkNode {
  return {
    id: PAGE_ID,
    type: 'link',
    x: 120,
    y: 120,
    width: 375,
    height: 667,
    url: PAGE_URL,
    presetIndex: 0,
  }
}

function loadHostPage(): void {
  harness.loadFixture({
    name: 'Scroll-target host',
    doc: {
      nodes: [hostPageNode()],
      edges: [],
      appState: { zoom: 1, pan: { x: 0, y: 0 } },
    },
  })
}

function hostPage() {
  const page = pages.find((candidate) => candidate.id === PAGE_ID)
  if (!page) throw new Error('host page missing')
  return page
}

/** A region whose marquee grabbed host-page content → page-anchored `docRect`.
 *  Marquee canvas x140/y340; page body at x120/y120, scroll 0 → docRect y220. */
function createGrabbedRegion(): Annotation {
  return createAnnotation({
    anchor: { type: 'region', canvasRect: { x: 140, y: 340, width: 80, height: 60 } },
    text: 'grabbed region',
    metadata: {
      regionComponents: [{ pageId: PAGE_ID, pageName: 'Host', components: [] }],
      regionElements: [{ pageId: PAGE_ID, pageName: 'Host', elements: [{}] }],
    },
  })
}

/** A region whose marquee grabbed nothing → canvas-anchored `canvasRect`. */
function createGrablessRegion(): Annotation {
  return createAnnotation({
    anchor: { type: 'region', canvasRect: { x: 900, y: 900, width: 50, height: 50 } },
    text: 'canvas region',
    metadata: { regionComponents: [], regionElements: [] },
  })
}

describe('computeAnnotationScrollTarget', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    selectNone()
    loadHostPage()
  })

  afterAll(() => harness?.dispose())

  it('targets a page-anchored region at its stored docRect.y', async () => {
    const region = createGrabbedRegion()
    await settleSync()
    // docRect.y = marquee 340 − page body 120 + scroll 0 = 220.
    expect(computeAnnotationScrollTarget(region)).toEqual({ pageId: PAGE_ID, documentY: 220 })
  })

  it('does not track page scroll into docRect — the docRect is scroll-independent', async () => {
    const region = createGrabbedRegion()
    await settleSync()
    hostPage().scrollY = 500
    // docRect.y stays 220; the ramp converts to a delta against live scroll at
    // dispatch time, not here.
    expect(computeAnnotationScrollTarget(region)).toEqual({ pageId: PAGE_ID, documentY: 220 })
  })

  it('returns null for a canvas-anchored region — nothing to reveal', async () => {
    const region = createGrablessRegion()
    await settleSync()
    expect(computeAnnotationScrollTarget(region)).toBeNull()
  })

  it('returns null for a canvas-point anchor', () => {
    const annotation = createAnnotation({
      anchor: { type: 'canvas', canvasX: 300, canvasY: 400 },
      text: 'canvas point',
    })
    expect(computeAnnotationScrollTarget(annotation)).toBeNull()
  })

  it('targets a page anchor at offsetY × scrollHeight', () => {
    hostPage().scrollHeight = 4000
    const annotation = createAnnotation({
      anchor: { type: 'page', pageId: PAGE_ID, offsetX: 0.5, offsetY: 0.25 },
      text: 'page anchor',
    })
    expect(computeAnnotationScrollTarget(annotation)).toEqual({ pageId: PAGE_ID, documentY: 1000 })
  })

  it('falls back to the stored bbox for an element anchor, never throws', () => {
    hostPage().scrollY = 60
    const annotation = createAnnotation({
      anchor: {
        type: 'element',
        pageId: PAGE_ID,
        selector: '#no-longer-in-the-dom',
        boundingBox: { x: 10, y: 500, width: 120, height: 40 },
      },
      text: 'element anchor',
    })
    // The stub page can't resolve the selector; the pure function reads the
    // stored bbox top plus the page's current scroll → document position.
    expect(() => computeAnnotationScrollTarget(annotation)).not.toThrow()
    expect(computeAnnotationScrollTarget(annotation)).toEqual({ pageId: PAGE_ID, documentY: 560 })
  })

  it('returns null for an element anchor with no stored bbox', () => {
    const annotation = createAnnotation({
      anchor: { type: 'element', pageId: PAGE_ID, selector: '#anything' },
      text: 'element anchor, no bbox',
    })
    expect(computeAnnotationScrollTarget(annotation)).toBeNull()
  })

  it('returns null when the anchor names a page that is gone', () => {
    const annotation = createAnnotation({
      anchor: { type: 'page', pageId: 'not-a-real-page', offsetX: 0.5, offsetY: 0.5 },
      text: 'orphan page anchor',
    })
    expect(computeAnnotationScrollTarget(annotation)).toBeNull()
  })
})
