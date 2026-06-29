import { Smartphone } from 'lucide-react'
import type { DeviceOrientation } from '../../shared/device-catalog'
import { RotateIcon } from '../shared/CustomIcons'
import { CanvasItemPopup } from './CanvasItemPopup'

export function DeviceViewportPopupControls({
  isDark,
  showDeviceFrame,
  orientation,
  noun,
  disabled = false,
  onToggleDeviceFrame,
  onSetOrientation,
}: {
  isDark: boolean
  showDeviceFrame: boolean
  orientation: DeviceOrientation
  noun: string
  disabled?: boolean
  onToggleDeviceFrame: () => void
  onSetOrientation: (orientation: DeviceOrientation) => void
}) {
  const nextOrientation: DeviceOrientation =
    orientation === 'portrait' ? 'landscape' : 'portrait'

  return (
    <CanvasItemPopup.Section>
      <CanvasItemPopup.IconButton
        isDark={isDark}
        active={showDeviceFrame}
        disabled={disabled}
        title="Device frame"
        ariaLabel={`Toggle device frame for ${noun}`}
        onClick={onToggleDeviceFrame}
      >
        <Smartphone size={14} />
      </CanvasItemPopup.IconButton>
      <CanvasItemPopup.IconButton
        isDark={isDark}
        disabled={disabled}
        title="Rotate viewport"
        ariaLabel={`Rotate viewport for ${noun}`}
        onClick={() => onSetOrientation(nextOrientation)}
      >
        <RotateIcon size={14} />
      </CanvasItemPopup.IconButton>
    </CanvasItemPopup.Section>
  )
}
