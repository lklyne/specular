// ADR 0008 §8 — drawing selection popup. Edits fan out across selected
// drawings; legacy multi-stroke drawings accept uniform writes per stroke.

import { Copy, Trash2 } from 'lucide-react'
import {
  paletteForBrushType,
  paletteSlots,
  resolveCanvasColor,
  slotForStorage,
} from '../../shared/canvas-colors'
import type {
  AnnotationDrawingStroke,
  CanvasBgElectronAPI,
  CanvasSceneDrawingEntity,
  LayoutUpdateData,
} from '../../shared/types'
import { CanvasItemPopup } from './CanvasItemPopup'
import { DistributeAction } from './DistributeAction'
import { drawingBounds } from './annotationMath'
import {
  BRUSH_VARIANT_OPTIONS,
  nearestStrokeWidthPreset,
  strokeWidthPresetsFor,
} from './popupVariantOptions'
import { StrokeWidthSwatch } from './StrokeWidthSwatch'
import { POPUP_OFFSET_Y, sharedValue, usePopupDelayedKey } from './usePopupDelayedKey'

export function DrawingPopup({
  api,
  isDark,
  layout,
  selectedDrawings,
  interactionIdle,
}: {
  api: Pick<
    CanvasBgElectronAPI,
    | 'duplicateDrawingEntity'
    | 'deleteDrawingEntity'
    | 'updateDrawingEntity'
    | 'distributeSelection'
  >
  isDark: boolean
  layout: LayoutUpdateData
  selectedDrawings: CanvasSceneDrawingEntity[]
  interactionIdle: boolean
}) {
  const count = selectedDrawings.length
  const ids = selectedDrawings.map((e) => e.id).join('|')
  const open = usePopupDelayedKey(ids, interactionIdle && count > 0)
  if (count === 0) return null

  const allStrokes = selectedDrawings.flatMap((d) => d.strokes)
  const brush = sharedValue(allStrokes.map((s) => s.brushType ?? 'pen'))
  const swatchPalette = paletteForBrushType(brush ?? 'pen')
  const colorRaw = sharedValue(allStrokes.map((s) => s.color))
  const iconInk =
    colorRaw === null
      ? null
      : resolveCanvasColor(colorRaw, { role: 'ink', isDark, palette: 'vivid' })
  const activeSlot = slotForStorage(colorRaw)
  const widthRaw = sharedValue(allStrokes.map((s) => s.width))
  const widthPresets = strokeWidthPresetsFor(brush ?? undefined)
  const activeStrokeWidth =
    widthRaw === null ? null : nearestStrokeWidthPreset(widthRaw, widthPresets)

  const writeStrokes = (
    rewrite: (stroke: AnnotationDrawingStroke) => AnnotationDrawingStroke,
  ) => {
    for (const drawing of selectedDrawings) {
      const next = drawing.strokes.map(rewrite)
      const bbox = drawingBounds(next)
      api.updateDrawingEntity(drawing.id, {
        strokes: next,
        canvasX: bbox.x,
        canvasY: bbox.y,
        width: bbox.width,
        height: bbox.height,
      })
    }
  }

  const entityIds = selectedDrawings.map((d) => d.id)
  const noun = count === 1 ? 'drawing' : `${count} drawings`

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
          {BRUSH_VARIANT_OPTIONS.map(({ kind, label, Icon }) => (
            <CanvasItemPopup.IconButton
              key={kind}
              isDark={isDark}
              active={brush === kind}
              title={label}
              ariaLabel={`Switch ${noun} brush to ${label}`}
              onClick={() => {
                const targetPresets = strokeWidthPresetsFor(kind)
                writeStrokes((stroke) => ({
                  ...stroke,
                  brushType: kind,
                  width: nearestStrokeWidthPreset(stroke.width, targetPresets),
                }))
              }}
            >
              <Icon
                size={14}
                ink={iconInk ?? undefined}
                selected={brush === kind}
              />
            </CanvasItemPopup.IconButton>
          ))}
        </CanvasItemPopup.Section>
        <CanvasItemPopup.Divider isDark={isDark} />
        <CanvasItemPopup.Section>
          {widthPresets.map((width, index) => (
            <StrokeWidthSwatch
              key={width}
              isDark={isDark}
              active={activeStrokeWidth === width}
              variant={index === 0 ? 'thin' : 'thick'}
              ariaLabel={`Set ${noun} stroke width to ${width}px`}
              onClick={() => writeStrokes((stroke) => ({ ...stroke, width }))}
            />
          ))}
        </CanvasItemPopup.Section>
        <CanvasItemPopup.Divider isDark={isDark} />
        <CanvasItemPopup.Section>
          {paletteSlots(swatchPalette).map((slot) => {
            const swatch =
              slot.hex ?? resolveCanvasColor(slot.storage, { role: 'ink', isDark })
            return (
              <CanvasItemPopup.ColorSwatch
                key={slot.id}
                isDark={isDark}
                active={activeSlot === slot.id}
                color={swatch}
                ariaLabel={`Set ${noun} color to ${slot.label}`}
                onClick={() =>
                  writeStrokes((stroke) => ({ ...stroke, color: slot.storage }))
                }
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
              for (const d of selectedDrawings) api.duplicateDrawingEntity(d.id)
            }}
          >
            <Copy size={14} />
          </CanvasItemPopup.IconButton>
          <CanvasItemPopup.IconButton
            isDark={isDark}
            title={`Delete ${noun}`}
            ariaLabel={`Delete ${noun}`}
            onClick={() => {
              for (const d of selectedDrawings) api.deleteDrawingEntity(d.id)
            }}
          >
            <Trash2 size={14} />
          </CanvasItemPopup.IconButton>
        </CanvasItemPopup.Section>
      </CanvasItemPopup.Frame>
    </CanvasItemPopup.Root>
  )
}
