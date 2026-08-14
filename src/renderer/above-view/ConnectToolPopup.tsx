// ADR 0008 §1/§5 — connect tool popup. Writes the `connect` tool defaults,
// which BOTH edge-creation doors stamp, so an edge drawn with the tool and one
// dragged off an anchor come out identical.

import { ArrowRight } from 'lucide-react'
import { slotForStorage } from '../../shared/canvas-colors'
import type { LayoutUpdateData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { CanvasItemPopup } from './CanvasItemPopup'
import { RoutingDropdown } from './RoutingDropdown'
import { StrokeWidthSwatch } from './StrokeWidthSwatch'

const STROKE_WIDTHS: { width: number; variant: 'thin' | 'thick'; label: string }[] = [
  { width: 1.5, variant: 'thin', label: 'Thin' },
  { width: 3, variant: 'thick', label: 'Thick' },
]

export function ConnectToolPopup({
  api,
  isDark,
  layout,
}: {
  api: Pick<CanvasBgElectronAPI, 'setToolDefault'>
  isDark: boolean
  layout: LayoutUpdateData
}) {
  const defaults = layout.toolDefaults.connect
  return (
    <CanvasItemPopup.ViewportAnchor layout={layout} open offset={8}>
      <CanvasItemPopup.Frame isDark={isDark}>
        <RoutingDropdown
          isDark={isDark}
          routing={defaults.routing}
          noun="default"
          onPick={(routing) => api.setToolDefault({ scope: 'connect', key: 'routing', value: routing })}
        />
        <CanvasItemPopup.Divider isDark={isDark} />
        <CanvasItemPopup.Section>
          {STROKE_WIDTHS.map(({ width, variant, label }) => (
            <StrokeWidthSwatch
              key={width}
              isDark={isDark}
              active={defaults.strokeWidth === width}
              variant={variant}
              ariaLabel={`${label} connection`}
              onClick={() =>
                api.setToolDefault({ scope: 'connect', key: 'strokeWidth', value: width })
              }
            />
          ))}
        </CanvasItemPopup.Section>
        <CanvasItemPopup.Divider isDark={isDark} />
        <CanvasItemPopup.IconButton
          isDark={isDark}
          active={defaults.toEnd === 'arrow'}
          title="Toggle end arrowhead"
          ariaLabel="Toggle default end arrowhead"
          onClick={() =>
            api.setToolDefault({
              scope: 'connect',
              key: 'toEnd',
              value: defaults.toEnd === 'arrow' ? 'none' : 'arrow',
            })
          }
        >
          <ArrowRight size={14} />
        </CanvasItemPopup.IconButton>
        <CanvasItemPopup.Divider isDark={isDark} />
        <CanvasItemPopup.PaletteRow
          isDark={isDark}
          palette="vivid"
          role="ink"
          activeSlot={slotForStorage(defaults.color)}
          ariaLabel={(label) => `Set default connection color to ${label}`}
          onPick={(storage) => api.setToolDefault({ scope: 'connect', key: 'color', value: storage })}
        />
      </CanvasItemPopup.Frame>
    </CanvasItemPopup.ViewportAnchor>
  )
}
