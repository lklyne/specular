import type { WebContents } from 'electron'
import type { ElementAttachmentPositionsUpdate } from '../../shared/types'
import { findPageByPageView } from './page-runtime'

type LivePosition = { docX: number; docY: number; viewportPositioned?: boolean }
type PositionUpdate = ElementAttachmentPositionsUpdate['positions'][number]

function applyPosition(
  map: Map<string, LivePosition>,
  position: PositionUpdate,
): boolean {
  if (typeof position?.selector !== 'string') return false
  if (position.resolved === false) return map.delete(position.selector)
  if (typeof position.docX !== 'number' || typeof position.docY !== 'number') return false

  const next: LivePosition = {
    docX: position.docX,
    docY: position.docY,
    ...(position.viewportPositioned === true ? { viewportPositioned: true } : {}),
  }
  const previous = map.get(position.selector)
  const unchanged =
    previous?.docX === next.docX &&
    previous.docY === next.docY &&
    previous.viewportPositioned === next.viewportPositioned
  if (unchanged) return false
  map.set(position.selector, next)
  return true
}

export function applyElementAttachmentPositions(
  sender: WebContents,
  data: ElementAttachmentPositionsUpdate | undefined,
): boolean {
  const page = findPageByPageView(sender)
  if (!page) return false
  const positions = Array.isArray(data?.positions) ? data.positions : []
  if (!positions.length) return false

  const map = page.elementPositions ?? new Map<string, LivePosition>()
  let changed = false
  for (const position of positions) {
    changed = applyPosition(map, position) || changed
  }
  if (changed) page.elementPositions = map.size ? map : undefined
  return changed
}
