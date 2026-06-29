// ADR 0008 §1/§5, ADR 0009 — draw tool popup; persists via tool defaults.

import {
  paletteForBrushType,
  paletteSlots,
  resolveCanvasColor,
  slotForStorage,
} from '../../shared/canvas-colors'
import type {
  CanvasBgElectronAPI,
  LayoutUpdateData,
  ToolDefaultPatch,
} from '../../shared/types'
import { CanvasItemPopup } from './CanvasItemPopup'
import {
  BRUSH_VARIANT_OPTIONS,
  nearestStrokeWidthPreset,
  strokeWidthPresetsFor,
} from './popupVariantOptions'
import { StrokeWidthSwatch } from './StrokeWidthSwatch'

export function DrawToolPopup({
  api,
  isDark,
  layout,
}: {
  api: Pick<CanvasBgElectronAPI, 'setToolDefault'>
  isDark: boolean
  layout: LayoutUpdateData
}) {
  const defaults = layout.toolDefaults.draw
  const swatchPalette = paletteForBrushType(defaults.brushType)
  const iconInk = resolveCanvasColor(defaults.color, {
    role: 'ink',
    isDark,
    palette: 'vivid',
  })
  const activeSlot = slotForStorage(defaults.color)
  const widthPresets = strokeWidthPresetsFor(defaults.brushType)
  const activeStrokeWidth = nearestStrokeWidthPreset(defaults.strokeWidth, widthPresets)
  return (
    <CanvasItemPopup.ViewportAnchor layout={layout} open offset={8}>
      <CanvasItemPopup.Frame isDark={isDark}>
        <CanvasItemPopup.Section>
          {BRUSH_VARIANT_OPTIONS.map(({ kind, label, Icon }) => (
            <CanvasItemPopup.IconButton
              key={kind}
              isDark={isDark}
              active={defaults.brushType === kind}
              title={label}
              ariaLabel={`Set default brush to ${label}`}
              onClick={() => {
                api.setToolDefault({ scope: 'draw', key: 'brushType', value: kind })
                // Snap stroke width into the new brush's preset range so the
                // next stroke has a sensible default (pen's 2px would be
                // invisible as a highlight; highlight's 16px would be a slab
                // as a pen).
                const snapped = nearestStrokeWidthPreset(
                  defaults.strokeWidth,
                  strokeWidthPresetsFor(kind),
                )
                if (snapped !== defaults.strokeWidth) {
                  api.setToolDefault({ scope: 'draw', key: 'strokeWidth', value: snapped })
                }
              }}
            >
              <Icon
                size={14}
                ink={iconInk}
                selected={defaults.brushType === kind}
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
              ariaLabel={`Set default brush width to ${width}px`}
              onClick={() => {
                const patch: ToolDefaultPatch = {
                  scope: 'draw',
                  key: 'strokeWidth',
                  value: width,
                }
                api.setToolDefault(patch)
              }}
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
                active={activeSlot === slot.id}
                color={swatch}
                ariaLabel={`Set default brush color to ${slot.label}`}
                onClick={() => {
                  const patch: ToolDefaultPatch = {
                    scope: 'draw',
                    key: 'color',
                    value: slot.storage,
                  }
                  api.setToolDefault(patch)
                }}
              />
            )
          })}
        </CanvasItemPopup.Section>
      </CanvasItemPopup.Frame>
    </CanvasItemPopup.ViewportAnchor>
  )
}
