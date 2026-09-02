/**
 * Live page scroll: the offset every page is at right now, as opposed to the
 * offset the last layout pass projected the scene from (which rides each page
 * scene entity). Overlays anchored into a page's document render from the
 * latter and shift by the delta to the former, so they track the page's native
 * compositor scroll instead of trailing the layout pass.
 */

import type { PageScrollOffsets } from '../../shared/types'
import { pages } from './runtime-context'
import { anchorableEntities } from './anchorable-entity-store'
import { workspaceAnnotations } from './space-model'

export function livePageScrollOffsets(): PageScrollOffsets {
  const offsets: PageScrollOffsets = {}
  for (const page of pages) {
    offsets[page.id] = { scrollX: page.scrollX ?? 0, scrollY: page.scrollY ?? 0 }
  }
  return offsets
}

/**
 * Whether anything in the scene is bound to this page's document, and so moves
 * when the page scrolls: anchored entities shift main-side by the scroll delta,
 * and annotations are stored in document space and projected against the page
 * entity's offset. When nothing is, the only thing a layout pass would do with
 * a new offset is copy it into a page entity no consumer reads — the patch
 * already carries it, so the pass is pure waste and is skipped.
 */
export function pageScrollMovesScene(pageId: string): boolean {
  for (const entity of anchorableEntities()) {
    if (entity.pageAnchor?.pageId === pageId) return true
  }
  for (const annotation of workspaceAnnotations) {
    if (annotation.pageAnchor?.pageId === pageId) return true
  }
  return false
}
