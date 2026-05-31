// Distribute popup — shown for 3+ loose entities (no managed-row group selected).
// Clicking "Distribute" evens edge-to-edge gaps along the dominant axis, making
// the selection reorder-eligible. ADR 0015 D7.

import { AlignJustify } from 'lucide-react'
import type { CanvasBgElectronAPI, LayoutUpdateData } from '../../shared/types'
import { CanvasItemPopup } from './CanvasItemPopup'
import { POPUP_OFFSET_Y, usePopupDelayedKey } from './usePopupDelayedKey'

export function DistributePopup({
  api,
  isDark,
  layout,
  looseEntityIds,
  interactionIdle,
}: {
  api: Pick<CanvasBgElectronAPI, 'distributeSelection'>
  isDark: boolean
  layout: LayoutUpdateData
  /** IDs of the currently selected loose entities (not a managed-row group). */
  looseEntityIds: readonly string[]
  interactionIdle: boolean
}) {
  const ids = looseEntityIds.join('|')
  const eligible = looseEntityIds.length >= 3
  const open = usePopupDelayedKey(ids, interactionIdle && eligible)

  if (!eligible) return null

  return (
    <CanvasItemPopup.Root
      entityIds={looseEntityIds as string[]}
      layout={layout}
      open={open}
      placement="above"
      offset={POPUP_OFFSET_Y}
    >
      <CanvasItemPopup.Frame isDark={isDark}>
        <CanvasItemPopup.Section>
          <CanvasItemPopup.IconButton
            isDark={isDark}
            title="Distribute"
            ariaLabel="Distribute spacing"
            onClick={() => api.distributeSelection()}
          >
            <AlignJustify size={14} />
          </CanvasItemPopup.IconButton>
        </CanvasItemPopup.Section>
      </CanvasItemPopup.Frame>
    </CanvasItemPopup.Root>
  )
}
