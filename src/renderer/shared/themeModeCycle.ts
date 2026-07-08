import type { AppThemeMode } from '../../shared/types'
import { MoonToolIcon, SunMoonToolIcon, SunToolIcon } from './CustomIcons'

const CYCLE: AppThemeMode[] = ['system', 'light', 'dark']

export const THEME_MODE_ICON: Record<AppThemeMode, typeof SunToolIcon> = {
  system: SunMoonToolIcon,
  light: SunToolIcon,
  dark: MoonToolIcon,
}

export const THEME_MODE_LABEL: Record<AppThemeMode, string> = {
  system: 'System',
  light: 'Light',
  dark: 'Dark',
}

export function nextThemeMode(mode: AppThemeMode): AppThemeMode {
  return CYCLE[(CYCLE.indexOf(mode) + 1) % CYCLE.length]
}
