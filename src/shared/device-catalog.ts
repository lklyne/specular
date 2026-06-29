// ---------------------------------------------------------------------------
// Device catalog — single source of truth for device dimensions, shell
// geometry, and viewport presets.
//
// Sources:
//   Screen corner radii: github.com/kylebshr/ScreenCorners
//   Viewport sizes:      ios-resolution.com, screensizechecker.com
//   Shell proportions:   Figma community vector mockups, Apple HIG
// ---------------------------------------------------------------------------

import type { ViewportPreset } from './types'

export type DeviceOrientation = 'portrait' | 'landscape'

export interface ShellInsets {
  top: number
  right: number
  bottom: number
  left: number
}

export interface DeviceDef {
  id: string
  label: string
  category: 'iphone' | 'ipad' | 'laptop' | 'desktop'
  /** Default viewport dimensions in CSS logical pixels (portrait for mobile, landscape for desktop). */
  viewport: { width: number; height: number }
  /** Whether the viewport uses mobile emulation. */
  mobile: boolean
  /** Shell insets in portrait orientation (px around the content area). */
  shellInsets: ShellInsets
  /** Outer shell corner radius. */
  cornerRadius: number
  /** Screen/content area corner radius (0 for rectangular screens). */
  screenCornerRadius: number
  /**
   * Safe area insets in portrait orientation (CSS px).
   * On real devices these push content below the status bar / Dynamic Island
   * and above the home indicator. Null means no safe area.
   */
  safeAreaInsets: ShellInsets | null
  /**
   * Safe area insets in landscape orientation (CSS px).
   * When omitted, landscape insets default to all zeros (no safe area).
   * Only iPhones with a Dynamic Island need non-zero landscape insets.
   */
  landscapeSafeAreaInsets?: ShellInsets | null
  /** Preset index into VIEWPORT_PRESETS (always portrait / default orientation). */
  presetIndex: number
}

// --- Catalog entries ---
// Outer corner radius = screenCornerRadius + min(shellInsets) for concentric curves.
// iPhone SE has a rectangular screen (home button) so screenCornerRadius = 0.

const entries: DeviceDef[] = [
  // ── iPhones ────────────────────────────────────────────────────────────
  {
    id: 'iphone-se',
    label: 'iPhone SE',
    category: 'iphone',
    viewport: { width: 375, height: 667 },
    mobile: true,
    presetIndex: 0,
    // Home-button phone: thick top/bottom bezels (~16% of height), moderate sides (~7.5% of width)
    shellInsets: { top: 96, right: 28, bottom: 96, left: 28 },
    cornerRadius: 30,
    screenCornerRadius: 0,
    // iPhone SE has a status bar but no notch/island; physical bezels already contain the safe area
    safeAreaInsets: { top: 20, right: 0, bottom: 0, left: 0 },
  },
  {
    id: 'iphone-14-pro',
    label: 'iPhone 14 Pro',
    category: 'iphone',
    viewport: { width: 393, height: 852 },
    mobile: true,
    presetIndex: 1,
    // Edge-to-edge: nearly uniform thin bezels, bottom slightly thicker
    shellInsets: { top: 22, right: 22, bottom: 26, left: 22 },
    cornerRadius: 77, // 55 screen + 22 bezel
    screenCornerRadius: 55,
    // Status bar (47px) + Dynamic Island area → 59px top; home indicator → 34px bottom
    safeAreaInsets: { top: 59, right: 0, bottom: 34, left: 0 },
    // In landscape the Dynamic Island shifts to the left side; home indicator stays at bottom
    landscapeSafeAreaInsets: { top: 0, right: 0, bottom: 21, left: 59 },
  },
  {
    id: 'iphone-14-pro-max',
    label: 'iPhone 14 Pro Max',
    category: 'iphone',
    viewport: { width: 430, height: 932 },
    mobile: true,
    presetIndex: 2,
    shellInsets: { top: 22, right: 22, bottom: 26, left: 22 },
    cornerRadius: 77,
    screenCornerRadius: 55,
    safeAreaInsets: { top: 59, right: 0, bottom: 34, left: 0 },
    landscapeSafeAreaInsets: { top: 0, right: 0, bottom: 21, left: 59 },
  },
  // ── iPads ──────────────────────────────────────────────────────────────
  {
    id: 'ipad-mini',
    label: 'iPad Mini',
    category: 'ipad',
    viewport: { width: 744, height: 1133 },
    mobile: false,
    presetIndex: 3,
    // Modern edge-to-edge iPad (6th gen, no home button)
    shellInsets: { top: 24, right: 24, bottom: 24, left: 24 },
    cornerRadius: 42, // 18 screen + 24 bezel
    screenCornerRadius: 18,
    safeAreaInsets: { top: 24, right: 0, bottom: 20, left: 0 },
  },
  {
    id: 'ipad-pro-11',
    label: 'iPad Pro 11',
    category: 'ipad',
    viewport: { width: 834, height: 1194 },
    mobile: false,
    presetIndex: 4,
    shellInsets: { top: 24, right: 24, bottom: 24, left: 24 },
    cornerRadius: 42,
    screenCornerRadius: 18,
    safeAreaInsets: { top: 24, right: 0, bottom: 20, left: 0 },
  },
  {
    id: 'ipad-pro-129',
    label: 'iPad Pro 12.9',
    category: 'ipad',
    viewport: { width: 1024, height: 1366 },
    mobile: false,
    presetIndex: 5,
    shellInsets: { top: 24, right: 24, bottom: 24, left: 24 },
    cornerRadius: 42,
    screenCornerRadius: 18,
    safeAreaInsets: { top: 24, right: 0, bottom: 20, left: 0 },
  },
  // ── Laptops / Desktops ─────────────────────────────────────────────────
  {
    id: 'laptop',
    label: 'Laptop',
    category: 'laptop',
    viewport: { width: 1280, height: 800 },
    mobile: false,
    presetIndex: 6,
    shellInsets: { top: 12, right: 12, bottom: 12, left: 12 },
    cornerRadius: 20,
    screenCornerRadius: 8,
    safeAreaInsets: null,
  },
  {
    id: 'desktop',
    label: 'Desktop',
    category: 'desktop',
    viewport: { width: 1440, height: 900 },
    mobile: false,
    presetIndex: 7,
    shellInsets: { top: 12, right: 12, bottom: 12, left: 12 },
    cornerRadius: 20,
    screenCornerRadius: 8,
    safeAreaInsets: null,
  },
  {
    id: 'desktop-xl',
    label: 'Desktop XL',
    category: 'desktop',
    viewport: { width: 1920, height: 1080 },
    mobile: false,
    presetIndex: 8,
    shellInsets: { top: 12, right: 12, bottom: 12, left: 12 },
    cornerRadius: 20,
    screenCornerRadius: 8,
    safeAreaInsets: null,
  },
]

// --- Derived VIEWPORT_PRESETS (backward-compatible array) ---

function buildViewportPresets(): ViewportPreset[] {
  const presets: ViewportPreset[] = []
  for (const dev of entries) {
    presets[dev.presetIndex] = {
      label: dev.label,
      width: dev.viewport.width,
      height: dev.viewport.height,
      mobile: dev.mobile,
    }
  }
  return presets
}

export const VIEWPORT_PRESETS: ViewportPreset[] = buildViewportPresets()
export const LAPTOP_PRESET_INDEX = entries.find((d) => d.id === 'laptop')!.presetIndex
export const DESKTOP_PRESET_INDEX = entries.find((d) => d.id === 'desktop')!.presetIndex

// --- Custom (no-device) shell constants — uniform balanced bezel ---

export const CUSTOM_SHELL_INSETS: ShellInsets = { top: 12, right: 12, bottom: 12, left: 12 }
export const CUSTOM_SHELL_CORNER_RADIUS = 10
export const CUSTOM_SHELL_SCREEN_CORNER_RADIUS = 6

// --- Maps and lookups ---

export const DEVICE_CATALOG: ReadonlyMap<string, DeviceDef> = new Map(
  entries.map((d) => [d.id, d]),
)

/** All devices in catalog order, for dropdown rendering. */
const DEVICE_LIST: readonly DeviceDef[] = entries

const presetToDevice = new Map<number, DeviceDef>()
for (const d of entries) {
  if (!presetToDevice.has(d.presetIndex)) {
    presetToDevice.set(d.presetIndex, d)
  }
}

/** Find the default device for a given preset index, or null. */
export function deviceForPresetIndex(presetIndex: number): DeviceDef | null {
  return presetToDevice.get(presetIndex) ?? null
}

// --- Geometry helpers ---

/** Shell insets for a device, accounting for orientation (swaps top/bottom <-> left/right). */
export function shellInsetsForDevice(deviceId: string, orientation: DeviceOrientation): ShellInsets {
  const dev = DEVICE_CATALOG.get(deviceId)
  if (!dev) return { top: 0, right: 0, bottom: 0, left: 0 }
  const insets = dev.shellInsets
  if (orientation === 'landscape') {
    return { top: insets.left, right: insets.top, bottom: insets.right, left: insets.bottom }
  }
  return insets
}

/** Content area corner radius for a device. */
export function contentCornerRadiusForDevice(deviceId: string, _orientation: DeviceOrientation): number {
  const dev = DEVICE_CATALOG.get(deviceId)
  if (!dev) return 0
  return dev.screenCornerRadius
}

/** Default orientation for a device — laptops/desktops default to landscape, everything else portrait. */
export function defaultOrientationForDevice(device: DeviceDef | null): DeviceOrientation {
  return device && (device.category === 'laptop' || device.category === 'desktop') ? 'landscape' : 'portrait'
}

/**
 * Swap width/height when the requested orientation disagrees with the preset's
 * natural shape. A landscape-native preset stays as-is when landscape is asked,
 * and flips when portrait is asked; portrait-native presets do the reverse.
 */
export function sizeForOrientation(
  baseWidth: number,
  baseHeight: number,
  orientation: DeviceOrientation,
): { width: number; height: number } {
  const isPresetLandscape = baseWidth > baseHeight
  const wantsLandscape = orientation === 'landscape'
  if (isPresetLandscape !== wantsLandscape) {
    return { width: baseHeight, height: baseWidth }
  }
  return { width: baseWidth, height: baseHeight }
}

/** Safe area insets for a device, accounting for orientation. Null if none. */
function safeAreaInsetsForDevice(deviceId: string, orientation: DeviceOrientation): ShellInsets | null {
  const dev = DEVICE_CATALOG.get(deviceId)
  if (!dev?.safeAreaInsets) return null
  if (orientation === 'landscape') {
    // Use explicit landscape insets when defined; otherwise no safe area in landscape
    return dev.landscapeSafeAreaInsets ?? null
  }
  return dev.safeAreaInsets
}

/**
 * Build CSS to inject safe-area padding into a page for a given device.
 * Returns null if the device has no safe area insets.
 */
export function safeAreaCssForDevice(deviceId: string, orientation: DeviceOrientation): string | null {
  const sa = safeAreaInsetsForDevice(deviceId, orientation)
  if (!sa || (sa.top === 0 && sa.right === 0 && sa.bottom === 0 && sa.left === 0)) return null
  const parts: string[] = []
  if (sa.top) parts.push(`padding-top: ${sa.top}px !important`)
  if (sa.right) parts.push(`padding-right: ${sa.right}px !important`)
  if (sa.bottom) parts.push(`padding-bottom: ${sa.bottom}px !important`)
  if (sa.left) parts.push(`padding-left: ${sa.left}px !important`)
  return `html { ${parts.join('; ')}; }`
}
