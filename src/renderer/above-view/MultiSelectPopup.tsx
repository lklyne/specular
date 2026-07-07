// Mixed-kind multi-select popup. The per-kind popups (ADR 0008 §4) only mount
// for a uniform selection, so a selection spanning kinds (a page + a shape + a
// sticky) has no surface. This is that surface: the cross-kind actions —
// focus and arrange — off the combined bounding box.

import type { LayoutUpdateData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { CanvasItemPopup } from './CanvasItemPopup'
import { POPUP_OFFSET_Y } from './usePopupDelayedKey'

export function MultiSelectPopup({
  api,
  isDark,
  layout,
  mixed,
}: {
  api: Pick<
    CanvasBgElectronAPI,
    'focusSelection' | 'arrangeSelection'
  >
  isDark: boolean
  layout: LayoutUpdateData
  /** True only when the selection spans more than one kind — same-kind
   *  selections already get their per-kind popup, which carries these actions. */
  mixed: boolean
}) {
  const entityIds = layout.selectedEntityIds
  if (!mixed || entityIds.length < 2) return null

  const noun = `${entityIds.length} items`
  return (
    <CanvasItemPopup.Root
      entityIds={entityIds}
      layout={layout}
      open
      placement="above"
      offset={POPUP_OFFSET_Y}
    >
      <CanvasItemPopup.Frame isDark={isDark}>
        <CanvasItemPopup.EntityActions
          isDark={isDark}
          noun={noun}
          count={entityIds.length}
          api={api}
        />
      </CanvasItemPopup.Frame>
    </CanvasItemPopup.Root>
  )
}
