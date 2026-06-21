import type { PanelFileEntityDetail } from '../../../shared/types'
import { rightDetailsPanelApi } from '../rightDetailsPanelApi'
import { DeviceSection } from './DeviceSection'

export function FileDeviceSection({ fileEntity }: { fileEntity: PanelFileEntityDetail }) {
  return (
    <DeviceSection
      deviceId={fileEntity.deviceId ?? null}
      orientation={fileEntity.deviceOrientation ?? 'portrait'}
      showShell={fileEntity.showDeviceFrame ?? false}
      width={fileEntity.width}
      height={fileEntity.height}
      presetIndex={fileEntity.presetIndex ?? null}
      onSelectPreset={(index) => rightDetailsPanelApi.setFilePreset(fileEntity.id, index)}
      onSelectCustom={() => rightDetailsPanelApi.setFileCustom(fileEntity.id)}
      onSetOrientation={(o) => rightDetailsPanelApi.setFileDeviceOrientation(fileEntity.id, o)}
      onToggleShell={() => rightDetailsPanelApi.toggleFileDeviceShell(fileEntity.id)}
    />
  )
}
