/**
 * Region select orchestrator — captures a composited screenshot of a canvas
 * region and extracts React component context, then creates an annotation.
 */

import type { RegionElementGroup, WorkspaceBounds } from '../../shared/types'
import type { Page } from './runtime-entities'
import { captureRegion } from './region-capture'
import { extractRegionComponents } from './region-components'
import { createAnnotation } from '../workspace-annotations'
import { queryElementsInRect } from './page-queries'
import { pageBodyCanvasBounds } from './runtime-geometry'
import { pageDisplayLabel } from './runtime-serialization'

/**
 * The page the region binds to, decided geometrically: the page body covering
 * the largest share of the region's area, provided that share is a majority.
 * A marquee that merely brushes a page edge stays canvas-anchored; one drawn
 * over page content binds even when the content is non-interactive (static
 * text grabs no elements but is still "about" the page).
 */
export function regionAnchorPageId(
  canvasRect: WorkspaceBounds,
  intersectingPages: Page[],
  bodyBounds: (page: Page) => WorkspaceBounds = pageBodyCanvasBounds,
): string | undefined {
  const area = canvasRect.width * canvasRect.height
  if (area <= 0) return undefined
  let bestId: string | undefined
  let bestOverlap = 0
  for (const page of intersectingPages) {
    const bounds = bodyBounds(page)
    const width =
      Math.min(canvasRect.x + canvasRect.width, bounds.x + bounds.width) -
      Math.max(canvasRect.x, bounds.x)
    const height =
      Math.min(canvasRect.y + canvasRect.height, bounds.y + bounds.height) -
      Math.max(canvasRect.y, bounds.y)
    const overlap = Math.max(0, width) * Math.max(0, height)
    if (overlap > bestOverlap) {
      bestOverlap = overlap
      bestId = page.id
    }
  }
  return bestOverlap / area > 0.5 ? bestId : undefined
}

/**
 * Execute a region select: capture screenshot, extract components, create annotation.
 */
export async function executeRegionSelect(canvasRect: WorkspaceBounds, text?: string): Promise<void> {
  const { base64, intersectingPages } = await captureRegion(canvasRect, { includeBgView: true })
  const regionComponents = extractRegionComponents(intersectingPages)

  // Query DOM elements within the region for each intersecting page.
  const regionElements: RegionElementGroup[] = []
  for (const page of intersectingPages) {
    const pageBounds = pageBodyCanvasBounds(page)
    // Convert canvas rect to page-local viewport coordinates.
    const viewportRect = {
      x: Math.max(0, canvasRect.x - pageBounds.x),
      y: Math.max(0, canvasRect.y - pageBounds.y),
      width:
        Math.min(canvasRect.x + canvasRect.width, pageBounds.x + pageBounds.width) -
        Math.max(canvasRect.x, pageBounds.x),
      height:
        Math.min(canvasRect.y + canvasRect.height, pageBounds.y + pageBounds.height) -
        Math.max(canvasRect.y, pageBounds.y),
    }
    try {
      const elements = await queryElementsInRect(page.id, viewportRect, 15)
      regionElements.push({
        pageId: page.id,
        pageName: pageDisplayLabel(page),
        elements,
      })
    } catch {
      // Page may be navigating or destroyed — skip.
    }
  }

  const anchorPageId = regionAnchorPageId(canvasRect, intersectingPages)

  createAnnotation({
    anchor: { type: 'region', canvasRect },
    author: 'user',
    text: text ?? '',
    ...(anchorPageId ? { anchorPageId } : {}),
    metadata: {
      regionScreenshot: base64,
      regionComponents,
      regionElements,
    },
  })
}
