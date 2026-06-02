// ADR 0008 §4 — text selection popup. Plain and sticky count as same kind
// for color so color edits apply uniformly across both in multi-select.
// ADR 0013 §3 — for a single plain-text selection, clicking the inactive
// half of the leading short/long toggle morphs the entity into a markdown
// file at the same rect.

import { Copy, Trash2 } from 'lucide-react'
import {
  paletteSlots,
  resolveCanvasColor,
  slotForStorage,
} from '../../shared/canvas-colors'
import type {
  CanvasBgElectronAPI,
  CanvasSceneTextEntity,
  LayoutUpdateData,
} from '../../shared/types'
import { CanvasItemPopup } from './CanvasItemPopup'
import { DistributeAction } from './DistributeAction'
import { TextKindToggle } from './TextKindToggle'
import { TEXT_SIZE_DEFAULT, TextSizeDropdown } from './TextSizeDropdown'
import { POPUP_OFFSET_Y, sharedValue, usePopupDelayedKey } from './usePopupDelayedKey'

export function StickyNotePopover({
  api,
  isDark,
  layout,
  selectedTextEntities,
  popupReady,
}: {
  api: Pick<
    CanvasBgElectronAPI,
    | 'duplicateTextEntity'
    | 'deleteTextEntity'
    | 'updateTextEntity'
    | 'morphTextFile'
    | 'distributeSelection'
  >
  isDark: boolean
  layout: LayoutUpdateData
  selectedTextEntities: CanvasSceneTextEntity[]
  popupReady: boolean
}) {
  const count = selectedTextEntities.length
  const ids = selectedTextEntities.map((e) => e.id).join('|')
  const open = usePopupDelayedKey(ids, popupReady && count > 0)
  if (count === 0) return null

  const sharedColor = sharedValue(selectedTextEntities.map((e) => e.color))
  const activeSlot = slotForStorage(sharedColor)
  const sharedTextSize = sharedValue(
    selectedTextEntities.map((e) => e.textSize ?? TEXT_SIZE_DEFAULT),
  )

  const entityIds = selectedTextEntities.map((e) => e.id)
  const noun = count === 1 ? 'sticky note' : `${count} text entities`
  const singlePlainText =
    count === 1 && selectedTextEntities[0].textStyle === 'plain'
      ? selectedTextEntities[0]
      : null

  return (
    <CanvasItemPopup.Root
      entityIds={entityIds}
      layout={layout}
      open={open}
      placement="above"
      offset={POPUP_OFFSET_Y}
    >
      <CanvasItemPopup.Frame isDark={isDark}>
        {singlePlainText ? (
          <>
            <TextKindToggle
              isDark={isDark}
              active="short"
              onPick={(kind) => {
                if (kind === 'long') {
                  void api.morphTextFile(singlePlainText.id, 'text-to-file')
                }
              }}
            />
            <CanvasItemPopup.Divider isDark={isDark} />
          </>
        ) : null}
        <CanvasItemPopup.Section>
          <TextSizeDropdown
            isDark={isDark}
            value={sharedTextSize ?? TEXT_SIZE_DEFAULT}
            ariaLabel={`Set ${noun} text size`}
            onPick={(size) => {
              for (const e of selectedTextEntities) {
                api.updateTextEntity(e.id, { textSize: size })
              }
            }}
          />
        </CanvasItemPopup.Section>
        <CanvasItemPopup.Divider isDark={isDark} />
        <CanvasItemPopup.Section>
          {paletteSlots('soft').map((slot) => {
            const swatch =
              slot.hex ?? resolveCanvasColor(slot.storage, { role: 'fill', isDark })
            return (
              <CanvasItemPopup.ColorSwatch
                key={slot.id}
                isDark={isDark}
                active={activeSlot === slot.id}
                color={swatch}
                ariaLabel={`Set color to ${slot.label}`}
                onClick={() => {
                  for (const e of selectedTextEntities) {
                    api.updateTextEntity(e.id, { color: slot.storage })
                  }
                }}
              />
            )
          })}
        </CanvasItemPopup.Section>
        <CanvasItemPopup.Divider isDark={isDark} />
        <CanvasItemPopup.Section>
          <DistributeAction api={api} isDark={isDark} count={count} />
          <CanvasItemPopup.IconButton
            isDark={isDark}
            title={`Duplicate ${noun}`}
            ariaLabel={`Duplicate ${noun}`}
            onClick={() => {
              for (const e of selectedTextEntities) api.duplicateTextEntity(e.id)
            }}
          >
            <Copy size={14} />
          </CanvasItemPopup.IconButton>
          <CanvasItemPopup.IconButton
            isDark={isDark}
            title={`Delete ${noun}`}
            ariaLabel={`Delete ${noun}`}
            onClick={() => {
              for (const e of selectedTextEntities) api.deleteTextEntity(e.id)
            }}
          >
            <Trash2 size={14} />
          </CanvasItemPopup.IconButton>
        </CanvasItemPopup.Section>
      </CanvasItemPopup.Frame>
    </CanvasItemPopup.Root>
  )
}
