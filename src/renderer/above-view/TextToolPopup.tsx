// ADR 0008 §1/§5 — add-text tool popup; writes to per-style tool defaults.

import type { ProjectedLayoutData } from '../../shared/scene-projection'
import { paletteForTextStyle, slotForStorage } from '../../shared/canvas-colors'
import type { TextEntityStyle, ToolDefaultPatch } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { CanvasItemPopup } from './CanvasItemPopup'
import { TextSizeDropdown } from './TextSizeDropdown'
import { TextFontDropdown } from './TextFontDropdown'

export function TextToolPopup({
  api,
  isDark,
  layout,
  style,
}: {
  api: Pick<CanvasBgElectronAPI, 'setToolDefault'>
  isDark: boolean
  layout: ProjectedLayoutData
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
  const currentTextFont =
    style === 'sticky'
      ? layout.toolDefaults['add-sticky'].textFont
      : layout.toolDefaults['add-text'].textFont
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
          <TextFontDropdown
            isDark={isDark}
            value={currentTextFont}
            ariaLabel={`Set default ${style} text font`}
            onPick={(font) => {
              const patch: ToolDefaultPatch =
                style === 'sticky'
                  ? { scope: 'add-sticky', key: 'textFont', value: font }
                  : { scope: 'add-text', key: 'textFont', value: font }
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
