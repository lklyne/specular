// ADR 0008 §1 — add-page tool popup. Unlike the other tool popups it carries no
// tool defaults: the chosen preset rides on the active `add-page` Tool object
// (tool.presetIndex / tool.customSize), so picking here just re-arms the tool.

import type { LayoutUpdateData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { PresetList } from '../shared/PresetList'
import { CanvasItemPopup } from './CanvasItemPopup'

export function PageToolPopup({
  api,
  isDark,
  layout,
}: {
  api: Pick<CanvasBgElectronAPI, 'setTool'>
  isDark: boolean
  layout: LayoutUpdateData
}) {
  const tool = layout.activeTool
  const customActive = tool.kind === 'add-page' && tool.customSize === true
  const activePreset =
    tool.kind === 'add-page' && !tool.customSize ? tool.presetIndex ?? 0 : null

  return (
    <CanvasItemPopup.ViewportAnchor layout={layout} open offset={8}>
      <CanvasItemPopup.Frame isDark={isDark}>
        <PresetList
          isDark={isDark}
          activePreset={activePreset}
          customActive={customActive}
          ariaVerb="Add"
          onSelectPreset={(index) => api.setTool({ kind: 'add-page', presetIndex: index })}
          onSelectCustom={() => api.setTool({ kind: 'add-page', customSize: true })}
        />
      </CanvasItemPopup.Frame>
    </CanvasItemPopup.ViewportAnchor>
  )
}
