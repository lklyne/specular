import { ChevronDown, Monitor, Smartphone, Tablet } from 'lucide-react'
import { DEVICE_CATALOG } from '../../../shared/device-catalog'
import { VIEWPORT_PRESETS } from '../../../shared/constants'
import { PagePresetDropdown } from '../../shared/PagePresetDropdown'
import { dividerClass } from '../rightDetailsPanelHelpers'
import { usePaneTheme } from '../PaneContext'

function OrientationIcon({
  category,
  orientation,
  size,
  className,
}: {
  category: string
  orientation: 'portrait' | 'landscape'
  size: number
  className?: string
}) {
  const iconIsLandscapeNative = category === 'laptop' || category === 'desktop'
  const shouldRotate = iconIsLandscapeNative ? orientation === 'portrait' : orientation === 'landscape'
  const combined = [className, shouldRotate && 'rotate-90'].filter(Boolean).join(' ')
  const isMobile = category === 'iphone'
  const isTablet = category === 'ipad'
  if (isMobile) return <Smartphone size={size} className={combined} />
  if (isTablet) return <Tablet size={size} className={combined} />
  return <Monitor size={size} className={combined} />
}

export function DeviceSection({
  deviceId,
  orientation,
  showShell,
  width,
  height,
  presetIndex,
  onSelectPreset,
  onSelectCustom,
  onSetOrientation,
  onToggleShell,
}: {
  deviceId: string | null
  orientation: 'portrait' | 'landscape'
  showShell: boolean
  width: number | undefined
  height: number | undefined
  presetIndex: number | null
  onSelectPreset: (index: number) => void
  onSelectCustom: () => void
  onSetOrientation: (o: 'portrait' | 'landscape') => void
  onToggleShell: () => void
}) {
  const isDark = usePaneTheme()
  const divider = dividerClass(isDark)
  const dev = deviceId ? DEVICE_CATALOG.get(deviceId) : null
  const supportsOrientation = !!dev

  const preset = presetIndex != null ? VIEWPORT_PRESETS[presetIndex] : null
  const isCustom = !preset || width !== preset.width || height !== preset.height
  const triggerLabel = isCustom ? 'Custom' : `${preset.label} (${preset.width}×${preset.height})`

  const triggerClassName =
    'flex h-7 min-w-0 flex-1 items-center justify-between gap-1 rounded-md border border-[var(--surface-input-border)] bg-[var(--surface-input)] px-2 text-[11px] hover:border-[var(--surface-toolbar-border)]'
  const tabBg = 'bg-[var(--surface-interactive)] border border-[var(--surface-input-border)]'
  const tabActive = isDark
    ? 'bg-[var(--surface-toolbar)] text-[var(--surface-foreground)]'
    : 'bg-[var(--surface-input)] text-[var(--surface-foreground)] shadow-sm'
  const tabInactive = 'text-[var(--surface-foreground-muted)]'

  return (
    <section className={`border-t ${divider}`}>
      <div className="flex items-center gap-2 px-2 py-2">
        <PagePresetDropdown
          align="start"
          isDark={isDark}
          side="bottom"
          sideOffset={4}
          onSelectPreset={onSelectPreset}
          onSelectCustom={onSelectCustom}
          trigger={
            <button type="button" className={triggerClassName}>
              <span className="min-w-0 truncate">{triggerLabel}</span>
              <ChevronDown size={10} className="shrink-0 text-[var(--surface-toolbar-foreground)] opacity-50" />
            </button>
          }
        />

        {supportsOrientation ? (
          <div className={`flex shrink-0 rounded-md ${tabBg} p-0.5`}>
            <button
              type="button"
              className={`rounded px-1.5 py-1 transition-colors ${orientation === 'portrait' ? tabActive : tabInactive}`}
              title="Portrait"
              onClick={() => onSetOrientation('portrait')}
            >
              <OrientationIcon category={dev!.category} orientation="portrait" size={14} />
            </button>
            <button
              type="button"
              className={`rounded px-1.5 py-1 transition-colors ${orientation === 'landscape' ? tabActive : tabInactive}`}
              title="Landscape"
              onClick={() => onSetOrientation('landscape')}
            >
              <OrientationIcon category={dev!.category} orientation="landscape" size={14} />
            </button>
          </div>
        ) : null}
      </div>

      <div className="flex flex-col gap-1 px-2 pb-2">
        <label className="flex items-center gap-1.5 text-[11px]">
          <input
            type="checkbox"
            checked={showShell}
            onChange={onToggleShell}
            className="accent-blue-500"
          />
          Show device page
        </label>
      </div>
    </section>
  )
}
