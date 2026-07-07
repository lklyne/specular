// ADR 0008 §1/§5, ADR 0009 — add-shape tool popup; persists via tool defaults.

import { slotForStorage } from '../../shared/canvas-colors'
import type { LayoutUpdateData, ShapeKind, ToolDefaultPatch } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { CanvasItemPopup } from './CanvasItemPopup'
import { ShapeDropdown } from './ShapeDropdown'
import { TextSizeDropdown } from './TextSizeDropdown'

export function ShapeToolPopup({
  api,
  isDark,
  layout,
}: {
  api: Pick<CanvasBgElectronAPI, 'setToolDefault'>
  isDark: boolean
  layout: LayoutUpdateData
}) {
  const defaults = layout.toolDefaults['add-shape']
  const activeSlot = slotForStorage(defaults.color)
  return (
    <CanvasItemPopup.ViewportAnchor layout={layout} open offset={8}>
      <CanvasItemPopup.Frame isDark={isDark}>
        <ShapeDropdown
          isDark={isDark}
          activeKind={defaults.shapeKind}
          noun="default"
          onPick={(kind) => {
            const patch: ToolDefaultPatch = {
              scope: 'add-shape',
              key: 'shapeKind',
              value: kind as ShapeKind,
            }
            api.setToolDefault(patch)
          }}
        />
        <CanvasItemPopup.Divider isDark={isDark} />
        <CanvasItemPopup.Section>
          <TextSizeDropdown
            isDark={isDark}
            value={defaults.textSize}
            ariaLabel="Set default shape text size"
            onPick={(size) => {
              const patch: ToolDefaultPatch = {
                scope: 'add-shape',
                key: 'textSize',
                value: size,
              }
              api.setToolDefault(patch)
            }}
          />
        </CanvasItemPopup.Section>
        <CanvasItemPopup.Divider isDark={isDark} />
        <CanvasItemPopup.PaletteRow
          isDark={isDark}
          palette="soft"
          role="fill"
          activeSlot={activeSlot}
          ariaLabel={(label) => `Set default shape color to ${label}`}
          onPick={(storage) =>
            api.setToolDefault({ scope: 'add-shape', key: 'color', value: storage })
          }
        />
      </CanvasItemPopup.Frame>
    </CanvasItemPopup.ViewportAnchor>
  )
}
