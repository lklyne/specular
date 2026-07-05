import type { CanvasSceneFileEntity } from '../../../shared/types'
import type { CanvasBgElectronAPI } from '../../../shared/electron-api/canvas-bg'
import { DeviceViewportPopupControls } from '../DeviceViewportPopupControls'

export function WireframeDeviceControlsContribution({
  api,
  isDark,
  entity,
}: {
  api: Pick<CanvasBgElectronAPI, 'setFileDeviceOrientation' | 'toggleFileDeviceShell'>
  isDark: boolean
  entity: CanvasSceneFileEntity
}) {
  return (
    <DeviceViewportPopupControls
      isDark={isDark}
      showDeviceFrame={entity.showDeviceFrame ?? false}
      orientation={entity.deviceOrientation ?? 'portrait'}
      noun="wireframe file"
      onToggleDeviceFrame={() => api.toggleFileDeviceShell(entity.id)}
      onSetOrientation={(orientation) =>
        api.setFileDeviceOrientation(entity.id, orientation)
      }
    />
  )
}
