// ADR 0008/0009 — shape selection popup. Variant morph per ADR 0009.

import { slotForStorage } from '../../shared/canvas-colors'
import type {
  CanvasBgElectronAPI,
  CanvasSceneShapeEntity,
  LayoutUpdateData,
  ShapeKind,
} from '../../shared/types'
import { CanvasItemPopup } from './CanvasItemPopup'
import { SHAPE_VARIANT_OPTIONS } from './popupVariantOptions'
import { TEXT_SIZE_DEFAULT, TextSizeDropdown } from './TextSizeDropdown'
import { POPUP_OFFSET_Y, sharedValue, usePopupDelayedKey } from './usePopupDelayedKey'

export function ShapePopup({
  api,
  isDark,
  layout,
  selectedShapes,
  interactionIdle,
}: {
  api: Pick<
    CanvasBgElectronAPI,
    | 'updateShapeEntity'
    | 'focusSelection'
    | 'distributeSelection'
  >
  isDark: boolean
  layout: LayoutUpdateData
  selectedShapes: CanvasSceneShapeEntity[]
  interactionIdle: boolean
}) {
  const count = selectedShapes.length
  const ids = selectedShapes.map((e) => e.id).join('|')
  const open = usePopupDelayedKey(ids, interactionIdle && count > 0)
  if (count === 0) return null

  const sharedShapeKind = sharedValue(selectedShapes.map((s) => s.shapeKind))
  const sharedColorRaw = sharedValue(selectedShapes.map((s) => s.color ?? null))
  const activeSlot = slotForStorage(sharedColorRaw)
  const sharedTextSize = sharedValue(
    selectedShapes.map((s) => s.textSize ?? TEXT_SIZE_DEFAULT),
  )

  const entityIds = selectedShapes.map((s) => s.id)
  const noun = count === 1 ? 'shape' : `${count} shapes`

  return (
    <CanvasItemPopup.Root
      entityIds={entityIds}
      layout={layout}
      open={open}
      placement="above"
      offset={POPUP_OFFSET_Y}
    >
      <CanvasItemPopup.Frame isDark={isDark}>
        <CanvasItemPopup.Section>
          {SHAPE_VARIANT_OPTIONS.map(({ kind, label, Icon }) => (
            <CanvasItemPopup.IconButton
              key={kind}
              isDark={isDark}
              active={sharedShapeKind === kind}
              title={label}
              ariaLabel={`Morph ${noun} to ${label}`}
              onClick={() => {
                const patch: { shapeKind: ShapeKind } = { shapeKind: kind }
                for (const s of selectedShapes) api.updateShapeEntity(s.id, patch)
              }}
            >
              <Icon size={14} />
            </CanvasItemPopup.IconButton>
          ))}
        </CanvasItemPopup.Section>
        <CanvasItemPopup.Divider isDark={isDark} />
        <CanvasItemPopup.Section>
          <TextSizeDropdown
            isDark={isDark}
            value={sharedTextSize ?? TEXT_SIZE_DEFAULT}
            ariaLabel={`Set ${noun} text size`}
            onPick={(size) => {
              for (const s of selectedShapes) {
                api.updateShapeEntity(s.id, { textSize: size })
              }
            }}
          />
        </CanvasItemPopup.Section>
        <CanvasItemPopup.Divider isDark={isDark} />
        <CanvasItemPopup.PaletteSection
          isDark={isDark}
          palette="soft"
          activeSlot={activeSlot}
          role="fill"
          noun={noun}
          onPick={(storage) => {
            for (const s of selectedShapes) {
              api.updateShapeEntity(s.id, { color: storage })
            }
          }}
        />
        <CanvasItemPopup.Divider isDark={isDark} />
        <CanvasItemPopup.EntityActions
          isDark={isDark}
          noun={noun}
          count={count}
          api={api}
        />
      </CanvasItemPopup.Frame>
    </CanvasItemPopup.Root>
  )
}
