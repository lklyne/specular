// ADR 0015 D7 — distribute action, shown in the selection popovers' right-actions
// group for a loose 3+ same-kind selection. Evens edge-to-edge gaps along the
// dominant axis, endpoints fixed; the result is reorder-eligible by construction.
// Styled as a sibling of the Copy/Delete icon buttons (see EntityChrome compound).

import { AlignHorizontalDistributeCenter } from 'lucide-react'
import type { CanvasBgElectronAPI } from '../../shared/types'
import { CanvasItemPopup } from './CanvasItemPopup'

export function DistributeAction({
  api,
  isDark,
  count,
}: {
  api: Pick<CanvasBgElectronAPI, 'distributeSelection'>
  isDark: boolean
  /** Number of selected same-kind entities; distribute needs at least 3. */
  count: number
}) {
  if (count < 3) return null
  return (
    <CanvasItemPopup.IconButton
      isDark={isDark}
      title="Distribute spacing"
      ariaLabel="Distribute spacing"
      onClick={() => api.distributeSelection()}
    >
      <AlignHorizontalDistributeCenter size={14} />
    </CanvasItemPopup.IconButton>
  )
}
