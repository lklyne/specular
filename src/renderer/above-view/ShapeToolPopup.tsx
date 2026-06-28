// ADR 0008 §1/§5, ADR 0009 — add-shape tool popup; persists via tool defaults.

import {
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
import { SHAPE_VARIANT_OPTIONS } from './popupVariantOptions'
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
        <CanvasItemPopup.Section>
          {SHAPE_VARIANT_OPTIONS.map(({ kind, label, Icon }) => (
            <CanvasItemPopup.IconButton
              key={kind}
              isDark={isDark}
              active={defaults.shapeKind === kind}
              title={label}
              ariaLabel={`Set default shape to ${label}`}
              onClick={() => {
                const patch: ToolDefaultPatch = {
                  scope: 'add-shape',
                  key: 'shapeKind',
                  value: kind,
                }
                api.setToolDefault(patch)
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
        <CanvasItemPopup.Section>
          {paletteSlots('soft').map((slot) => {
            const swatch =
              slot.hex ?? resolveCanvasColor(slot.storage, { role: 'fill', isDark })
            return (
              <CanvasItemPopup.ColorSwatch
                key={slot.id}
                active={activeSlot === slot.id}
                color={swatch}
                ariaLabel={`Set default shape color to ${slot.label}`}
                onClick={() => {
                  const patch: ToolDefaultPatch = {
                    scope: 'add-shape',
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
