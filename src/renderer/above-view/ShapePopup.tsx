// ADR 0008/0009 — shape selection popup. Variant morph per ADR 0009.

import type { ProjectedLayoutData } from '../../shared/scene-projection'
import { slotForStorage } from '../../shared/canvas-colors'
import type { CanvasSceneShapeEntity, ShapeKind } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { BorderDropdown } from './BorderDropdown'
import { CanvasItemPopup } from './CanvasItemPopup'
import { ColorDropdown } from './ColorDropdown'
import { ShapeDropdown } from './ShapeDropdown'
import { TEXT_SIZE_DEFAULT, TextSizeDropdown } from './TextSizeDropdown'
import { TextAlignDropdown } from './TextAlignDropdown'
import type { AnnotateHandler } from './annotationMath'
import {
  POPUP_OFFSET_Y,
  sharedValue,
  usePopupDelayedKey,
} from './usePopupDelayedKey'

export function ShapePopup({
  api,
  isDark,
  layout,
  selectedShapes,
  interactionIdle,
  onAnnotate,
}: {
  api: Pick<
    CanvasBgElectronAPI,
    'updateEntity' | 'focusSelection' | 'arrangeSelection'
  >
  isDark: boolean
  layout: ProjectedLayoutData
  selectedShapes: CanvasSceneShapeEntity[]
  interactionIdle: boolean
  onAnnotate: AnnotateHandler
}) {
  const count = selectedShapes.length
  const ids = selectedShapes.map((e) => e.id).join('|')
  const open = usePopupDelayedKey(ids, interactionIdle && count > 0)
  if (count === 0) return null

  const sharedShapeKind = sharedValue(selectedShapes.map((s) => s.shapeKind))
  const sharedColorRaw = sharedValue(selectedShapes.map((s) => s.color ?? null))
  const sharedFillStyle = sharedValue(
    selectedShapes.map((s) => s.fillStyle ?? 'solid'),
  )
  const activeSlot = slotForStorage(sharedColorRaw)
  const sharedBorderStyle = sharedValue(
    selectedShapes.map((s) => s.borderStyle ?? 'solid'),
  )
  const sharedBorderColorRaw = sharedValue(
    selectedShapes.map((s) => s.borderColor ?? null),
  )
  const borderColorSlot = slotForStorage(sharedBorderColorRaw)
  const sharedStrokeWidth = sharedValue(
    selectedShapes.map((s) => s.strokeWidth ?? 2),
  )
  const sharedTextSize = sharedValue(
    selectedShapes.map((s) => s.textSize ?? TEXT_SIZE_DEFAULT),
  )
  const sharedTextAlign = sharedValue(
    selectedShapes.map((s) => s.textAlign ?? 'center'),
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
        <ShapeDropdown
          isDark={isDark}
          activeKind={sharedShapeKind ?? null}
          noun={noun}
          onPick={(kind) => {
            const patch: { shapeKind: ShapeKind } = {
              shapeKind: kind as ShapeKind,
            }
            for (const s of selectedShapes)
              api.updateEntity('shape', s.id, patch)
          }}
        />
        <CanvasItemPopup.Divider isDark={isDark} />
        <CanvasItemPopup.Section>
          <TextSizeDropdown
            isDark={isDark}
            value={sharedTextSize ?? TEXT_SIZE_DEFAULT}
            ariaLabel={`Set ${noun} text size`}
            onPick={(size) => {
              for (const s of selectedShapes) {
                api.updateEntity('shape', s.id, { textSize: size })
              }
            }}
          />
        </CanvasItemPopup.Section>
        <CanvasItemPopup.Divider isDark={isDark} />
        <CanvasItemPopup.Section>
          <TextAlignDropdown
            isDark={isDark}
            alignment={sharedTextAlign}
            onPick={(textAlign) => {
              for (const s of selectedShapes)
                api.updateEntity('shape', s.id, { textAlign })
            }}
          />
        </CanvasItemPopup.Section>
        <CanvasItemPopup.Divider isDark={isDark} />
        <ColorDropdown
          isDark={isDark}
          palette="soft"
          activeSlot={activeSlot}
          role="fill"
          noun={noun}
          transparentActive={sharedFillStyle === 'none'}
          onPickTransparent={() => {
            for (const s of selectedShapes) {
              api.updateEntity('shape', s.id, { fillStyle: 'none' })
            }
          }}
          onPick={(storage) => {
            for (const s of selectedShapes) {
              api.updateEntity('shape', s.id, {
                color: storage,
                fillStyle: 'solid',
              })
            }
          }}
        />
        <CanvasItemPopup.Divider isDark={isDark} />
        <BorderDropdown
          isDark={isDark}
          borderStyle={sharedBorderStyle}
          strokeWidth={sharedStrokeWidth}
          activeColorSlot={borderColorSlot}
          palette="soft"
          noun={noun}
          onSetStyle={(style) => {
            for (const s of selectedShapes) {
              api.updateEntity('shape', s.id, { borderStyle: style })
            }
          }}
          onSetWidth={(width) => {
            for (const s of selectedShapes) {
              api.updateEntity('shape', s.id, { strokeWidth: width })
            }
          }}
          onSetColor={(storage) => {
            for (const s of selectedShapes) {
              api.updateEntity('shape', s.id, { borderColor: storage })
            }
          }}
        />
        <CanvasItemPopup.Divider isDark={isDark} />
        <CanvasItemPopup.EntityActions
          isDark={isDark}
          noun={noun}
          count={count}
          api={api}
          layout={layout}
          entityIds={entityIds}
          onAnnotate={onAnnotate}
        />
      </CanvasItemPopup.Frame>
    </CanvasItemPopup.Root>
  )
}
