// ADR 0008 — group selection popup. Replaces canvas-bg GroupInlineMenu.

import { slotForStorage } from '../../shared/canvas-colors'
import type {
  CanvasBgElectronAPI,
  CanvasSceneGroupEntity,
  LayoutUpdateData,
} from '../../shared/types'
import { CanvasItemPopup } from './CanvasItemPopup'
import { POPUP_OFFSET_Y, usePopupDelayedKey } from './usePopupDelayedKey'

export function GroupPopup({
  api,
  isDark,
  layout,
  selectedGroup,
  interactionIdle,
}: {
  api: Pick<
    CanvasBgElectronAPI,
    'duplicateGroup' | 'deleteGroup' | 'updateGroupEntity'
  >
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
        <CanvasItemPopup.PaletteSection
          isDark={isDark}
          palette="vivid"
          activeSlot={activeSlot}
          role="fill"
          noun="group"
          onPick={(storage) =>
            api.updateGroupEntity(selectedGroup.id, { color: storage })
          }
        />
        <CanvasItemPopup.EntityActions
          isDark={isDark}
          noun="group"
          count={1}
          onDuplicate={() => api.duplicateGroup(selectedGroup.id)}
          onDelete={() => api.deleteGroup(selectedGroup.id)}
        />
      </CanvasItemPopup.Frame>
    </CanvasItemPopup.Root>
  )
}
