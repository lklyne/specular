// ADR 0008 — group selection popup. Replaces canvas-bg GroupInlineMenu.

import { slotForStorage } from '../../shared/canvas-colors'
import type { CanvasSceneGroupEntity, LayoutUpdateData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { CanvasItemPopup } from './CanvasItemPopup'
import { ColorDropdown } from './ColorDropdown'
import { POPUP_OFFSET_Y, usePopupDelayedKey } from './usePopupDelayedKey'

export function GroupPopup({
  api,
  isDark,
  layout,
  selectedGroup,
  interactionIdle,
}: {
  api: Pick<CanvasBgElectronAPI, 'updateEntity' | 'focusSelection'>
  isDark: boolean
  layout: LayoutUpdateData
  selectedGroup: CanvasSceneGroupEntity | null
  interactionIdle: boolean
}) {
  const open = usePopupDelayedKey(
    selectedGroup?.id ?? '',
    interactionIdle && selectedGroup !== null,
  )
  if (!selectedGroup) return null
  const activeSlot = slotForStorage(selectedGroup.color ?? null)
  return (
    <CanvasItemPopup.Root
      entityId={selectedGroup.id}
      layout={layout}
      open={open}
      placement="above"
      offset={POPUP_OFFSET_Y}
    >
      <CanvasItemPopup.Frame isDark={isDark}>
        <ColorDropdown
          isDark={isDark}
          palette="vivid"
          activeSlot={activeSlot}
          role="fill"
          noun="group"
          onPick={(storage) =>
            api.updateEntity('group', selectedGroup.id, { color: storage })
          }
        />
        <CanvasItemPopup.EntityActions
          isDark={isDark}
          noun="group"
          count={1}
          api={api}
        />
      </CanvasItemPopup.Frame>
    </CanvasItemPopup.Root>
  )
}
