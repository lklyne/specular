// ADR 0008 §1/§5 — add-text tool popup; writes to per-style tool defaults.

import { paletteForTextStyle, slotForStorage } from '../../shared/canvas-colors'
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
        <CanvasItemPopup.PaletteRow
          isDark={isDark}
          palette={swatchPalette}
          role={swatchRole}
          activeSlot={activeSlot}
          ariaLabel={(label) => `Set default ${style} text color to ${label}`}
          onPick={(storage) =>
            api.setToolDefault(
              style === 'sticky'
                ? { scope: 'add-sticky', key: 'color', value: storage }
                : { scope: 'add-text', key: 'color', value: storage },
            )
          }
        />
      </CanvasItemPopup.Frame>
    </CanvasItemPopup.ViewportAnchor>
  )
}
