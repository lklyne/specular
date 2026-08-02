// fallow-ignore-file circular-dependencies
// Suppressed: see #141. settings-window imports preferences creating a mutual dependency
/**
 * User preferences — devtools panel width/tab persistence + onboarding state.
 */

import { app, nativeTheme } from 'electron'
import { join } from 'path'
import { readFileSync, writeFileSync } from 'fs'
import type {
  AppThemeMode,
  DevtoolsPanelTab,
  FixConfig,
  OnboardingState,
} from '../../shared/types'
import { ipcChannels } from '../../shared/ipc-contract'
import {
  DEFAULT_TOOL_DEFAULTS,
  normalizeToolDefaults,
  type ToolDefaults,
} from '../../shared/tool-defaults'
import type { LegacyOriginBindings } from './dev-server-manager'
import {
  DEFAULT_CURSOR_TUNING,
  normalizeCursorTuning,
  type CursorTuningParams,
} from '../../shared/cursor-tuning'
import {
  devtoolsBackgroundView,
  win,
} from './view-refs'
import { broadcast } from './view-broadcast'
import {
  hoverTarget,
  pages,
} from './runtime-context'
import {
  devtoolsPanelTab as uiDevtoolsPanelTab,
  devtoolsWidth as uiDevtoolsWidth,
  selectedEntityIds as uiSelectedEntityIds,
  setDevtoolsPanelTab as setUiDevtoolsPanelTab,
  setDevtoolsWidth as setUiDevtoolsWidth,
} from '../ui-state'

import {
  DEVTOOLS_MAX_WIDTH,
  DEVTOOLS_MIN_WIDTH,
  PREFERENCES_FILE,
} from './runtime-constants'

function preferencesPath(): string {
  return join(app.getPath('userData'), PREFERENCES_FILE)
}

type PreferencesFile = {
  devtoolsWidth?: number
  devtoolsPanelTab?: DevtoolsPanelTab | 'elements' | 'devtools'
  onboarding?: OnboardingState
  /** The user-chosen space folder. Unset resolves to the legacy
   *  userData/workspaces/default location (ADR 0033). */
  spacePath?: string
  /** Legacy: bindings now live in repos.json under ConnectedRepo.boundOrigins.
   *  Read once at startup for migration, then stripped from this file. */
  originBindings?: LegacyOriginBindings
  fixConfig?: Omit<FixConfig, 'configured'>
  toolDefaults?: ToolDefaults
  themeMode?: AppThemeMode
  debug?: {
    cursorSplineViz?: boolean
    cursorTuning?: CursorTuningParams
  }
}

let currentCursorSplineViz = false
let currentCursorTuning: CursorTuningParams = { ...DEFAULT_CURSOR_TUNING }
let currentToolDefaults: ToolDefaults = normalizeToolDefaults(DEFAULT_TOOL_DEFAULTS)
let currentThemeMode: AppThemeMode = 'system'
let currentSpacePath: string | undefined

function readPreferencesFile(): PreferencesFile {
  try {
    return JSON.parse(readFileSync(preferencesPath(), 'utf8')) as PreferencesFile
  } catch (error) {
    if ((error as NodeJS.ErrnoException)?.code === 'ENOENT') return {}
    console.error('Failed to read preferences:', error)
    return {}
  }
}

function writePreferencesFile(next: PreferencesFile): void {
  try {
    writeFileSync(preferencesPath(), JSON.stringify(next, null, 2), 'utf8')
  } catch (error) {
    console.error('Failed to write preferences:', error)
  }
}

export function loadOnboardingState(): OnboardingState {
  const parsed = readPreferencesFile()
  return parsed.onboarding ?? { completed: false }
}

export function saveOnboardingState(next: OnboardingState): void {
  const parsed = readPreferencesFile()
  writePreferencesFile({ ...parsed, onboarding: next })
}

export function normalizeDevtoolsPanelTab(
  tab: DevtoolsPanelTab | 'elements' | 'devtools' | undefined,
): DevtoolsPanelTab | null {
  if (tab === 'elements') return 'inspect'
  if (tab === 'devtools') return 'browser-devtools'
  if (
    tab === 'comments' ||
    tab === 'inspect' ||
    tab === 'browser-devtools' ||
    tab === 'settings'
  ) {
    return tab
  }
  return null
}

export function clampDevtoolsWidth(value: number): number {
  const maxByWindow = win
    ? Math.floor(win.getBounds().width * 0.8)
    : DEVTOOLS_MAX_WIDTH
  return Math.max(
    DEVTOOLS_MIN_WIDTH,
    Math.min(DEVTOOLS_MAX_WIDTH, maxByWindow, Math.round(value)),
  )
}

let pendingLegacyOriginBindings: LegacyOriginBindings | null = null

const DEFAULT_FIX_CONFIG: FixConfig = { model: 'opus', permissions: 'acceptEdits', configured: false }
let fixConfig: FixConfig = { ...DEFAULT_FIX_CONFIG }

export function loadPreferences(): void {
  const parsed = readPreferencesFile()
  if (typeof parsed.devtoolsWidth === 'number') {
    setUiDevtoolsWidth(clampDevtoolsWidth(parsed.devtoolsWidth))
  }
  const normalizedTab = normalizeDevtoolsPanelTab(parsed.devtoolsPanelTab)
  if (normalizedTab) {
    setUiDevtoolsPanelTab(normalizedTab)
  }
  if (parsed.originBindings && typeof parsed.originBindings === 'object') {
    pendingLegacyOriginBindings = parsed.originBindings
  }
  if (parsed.fixConfig && typeof parsed.fixConfig === 'object') {
    fixConfig = { ...DEFAULT_FIX_CONFIG, ...parsed.fixConfig, configured: true }
  }
  currentCursorSplineViz = parsed.debug?.cursorSplineViz === true
  currentCursorTuning = normalizeCursorTuning(parsed.debug?.cursorTuning)
  currentToolDefaults = normalizeToolDefaults(parsed.toolDefaults)
  currentThemeMode = normalizeThemeMode(parsed.themeMode)
  currentSpacePath = typeof parsed.spacePath === 'string' ? parsed.spacePath : undefined
  nativeTheme.themeSource = currentThemeMode
}

function normalizeThemeMode(mode: unknown): AppThemeMode {
  return mode === 'light' || mode === 'dark' ? mode : 'system'
}

export function getToolDefaults(): ToolDefaults {
  return currentToolDefaults
}

export function saveToolDefaults(next: ToolDefaults): void {
  currentToolDefaults = normalizeToolDefaults(next)
  const parsed = readPreferencesFile()
  writePreferencesFile({ ...parsed, toolDefaults: currentToolDefaults })
}

/** The user-chosen space folder, or undefined when unset (spaceDir() resolves
 *  the legacy default in that case). */
export function getSpacePath(): string | undefined {
  return currentSpacePath
}

export function setSpacePath(path: string): void {
  currentSpacePath = path
  const parsed = readPreferencesFile()
  writePreferencesFile({ ...parsed, spacePath: currentSpacePath })
}

export function getCursorSplineViz(): boolean {
  return currentCursorSplineViz
}

export function saveCursorSplineViz(next: boolean): void {
  currentCursorSplineViz = next === true
  const parsed = readPreferencesFile()
  writePreferencesFile({
    ...parsed,
    debug: { ...parsed.debug, cursorSplineViz: currentCursorSplineViz },
  })
}

export function getCursorTuning(): CursorTuningParams {
  return currentCursorTuning
}

export function saveCursorTuning(next: CursorTuningParams): void {
  currentCursorTuning = normalizeCursorTuning(next)
  const parsed = readPreferencesFile()
  writePreferencesFile({
    ...parsed,
    debug: { ...parsed.debug, cursorTuning: currentCursorTuning },
  })
}

export function savePreferences(): void {
  const parsed = readPreferencesFile()
  // Drop the legacy originBindings key on every write — bindings now live in
  // repos.json.
  const { originBindings: _drop, ...rest } = parsed
  writePreferencesFile({
    ...rest,
    devtoolsWidth: uiDevtoolsWidth(),
    devtoolsPanelTab: uiDevtoolsPanelTab(),
    fixConfig: { model: fixConfig.model, permissions: fixConfig.permissions },
    toolDefaults: currentToolDefaults,
    themeMode: currentThemeMode,
  })
}

/** One-shot: returns and clears any legacy `originBindings` read during
 *  `loadPreferences()`. The caller folds them into `dev-server-manager` and
 *  `savePreferences()` then strips the key from disk. */
export function consumeLegacyOriginBindings(): LegacyOriginBindings | null {
  const next = pendingLegacyOriginBindings
  pendingLegacyOriginBindings = null
  return next
}

export function getFixConfig(): FixConfig {
  return fixConfig
}

export function setFixConfig(patch: { model?: FixConfig['model']; permissions?: FixConfig['permissions'] }): void {
  fixConfig = { ...fixConfig, ...patch, configured: true }
  savePreferences()
}

export function getThemeMode(): AppThemeMode {
  return currentThemeMode
}

export function setThemeMode(mode: AppThemeMode): void {
  currentThemeMode = mode
  nativeTheme.themeSource = mode
  savePreferences()
  // nativeTheme's 'updated' event only fires when shouldUseDarkColors flips;
  // broadcast directly so themeMode reaches renderers even when it doesn't.
  broadcastTheme()
}

export function isDark(): boolean {
  return nativeTheme.shouldUseDarkColors
}

export function frameColor(): string {
  // Match --surface-device-border token (stone-400 light, stone-600 dark)
  return isDark() ? '#57534e' : '#a8a29e'
}

export function broadcastTheme(): void {
  if (win) win.contentView.setBackgroundColor(isDark() ? '#44403c' : '#f5f5f4')
  if (devtoolsBackgroundView) {
    devtoolsBackgroundView.setBackgroundColor(isDark() ? '#18181b' : '#fafafa')
  }
  broadcast(ipcChannels.themeChanged, { isDark: isDark(), themeMode: currentThemeMode })
  for (let i = 0; i < pages.length; i++) {
    const page = pages[i]
    page.frameView.setBackgroundColor(frameColor())
  }
}

export function broadcastCursorSplineViz(): void {
  broadcast(ipcChannels.cursorSplineVizChanged, currentCursorSplineViz, 'debug')
}
