// ADR 0008 §1/§5 — add-text tool popup; writes to per-style tool defaults.

import {
  paletteForTextStyle,
  paletteSlots,
  resolveCanvasColor,
  slotForStorage,
} from '../../shared/canvas-colors'
import type { LayoutUpdateData, TextEntityStyle, ToolDefaultPatch } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { CanvasItemPopup } from './CanvasItemPopup'
import { TextSizeDropdown } from './TextSizeDropdown'

export function TextToolPopup({
  api,
  isDark,
  layout,
  style,
}: {
  api: Pick<CanvasBgElectronAPI, 'setToolDefault'>
  isDark: boolean
  layout: LayoutUpdateData
  style: TextEntityStyle
}) {
  const currentRaw =
    style === 'sticky'
      ? layout.toolDefaults['add-sticky'].color
      : layout.toolDefaults['add-text'].color
  const activeSlot = slotForStorage(currentRaw)
  const swatchRole = style === 'sticky' ? 'fill' : 'ink'
  const swatchPalette = paletteForTextStyle(style)
  const currentTextSize =
    style === 'sticky'
      ? layout.toolDefaults['add-sticky'].textSize
      : layout.toolDefaults['add-text'].textSize
  return (
    <CanvasItemPopup.ViewportAnchor layout={layout} open offset={8}>
      <CanvasItemPopup.Frame isDark={isDark}>
        <CanvasItemPopup.Section>
          <TextSizeDropdown
            isDark={isDark}
            value={currentTextSize}
            ariaLabel={`Set default ${style} text size`}
            onPick={(size) => {
              const patch: ToolDefaultPatch =
                style === 'sticky'
                  ? { scope: 'add-sticky', key: 'textSize', value: size }
                  : { scope: 'add-text', key: 'textSize', value: size }
              api.setToolDefault(patch)
            }}
          />
        </CanvasItemPopup.Section>
        <CanvasItemPopup.Divider isDark={isDark} />
        <CanvasItemPopup.Section>
          {paletteSlots(swatchPalette).map((slot) => {
            const swatch =
              slot.hex ?? resolveCanvasColor(slot.storage, { role: swatchRole, isDark })
            return (
              <CanvasItemPopup.ColorSwatch
                key={slot.id}
                isDark={isDark}
                active={activeSlot === slot.id}
                color={swatch}
                ariaLabel={`Set default ${style} text color to ${slot.label}`}
                onClick={() => {
                  const patch: ToolDefaultPatch =
                    style === 'sticky'
                      ? { scope: 'add-sticky', key: 'color', value: slot.storage }
                      : { scope: 'add-text', key: 'color', value: slot.storage }
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
