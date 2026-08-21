import { BrowserWindow } from 'electron'
import type { WebContents } from 'electron'
import { loadRenderer, preloadPath } from './runtime/load-renderer'
import { isDark } from './runtime/preferences'
import { wireRendererLogging } from './crash-log'

let settingsWindow: BrowserWindow | null = null

const WINDOW_WIDTH = 760
const WINDOW_HEIGHT = 560

export function isSettingsWindowOpen(): boolean {
  return settingsWindow !== null && !settingsWindow.isDestroyed()
}

export function focusSettingsWindow(): void {
  if (isSettingsWindowOpen()) settingsWindow!.focus()
}

export function getSettingsWebContents(): WebContents | null {
  if (!isSettingsWindowOpen()) return null
  return settingsWindow!.webContents
}

export function closeSettingsWindow(): void {
  if (isSettingsWindowOpen()) settingsWindow!.close()
}

function createSettingsWindow(): BrowserWindow {
  const win = new BrowserWindow({
    width: WINDOW_WIDTH,
    height: WINDOW_HEIGHT,
    resizable: false,
    minimizable: false,
    maximizable: false,
    fullscreenable: false,
    title: 'Specular Settings',
    titleBarStyle: 'hiddenInset',
    show: false,
    backgroundColor: isDark() ? '#292524' : '#f5f5f4',
    ...(process.platform === 'darwin'
      ? { trafficLightPosition: { x: 14, y: 13 } }
      : {}),
    webPreferences: {
      preload: preloadPath('settings'),
      contextIsolation: true,
      nodeIntegration: false,
    },
  })

  wireRendererLogging(win.webContents, 'settings')
  loadRenderer(win, 'settings')
  win.once('ready-to-show', () => win.show())
  win.on('closed', () => {
    settingsWindow = null
  })
  return win
}

export function showSettingsWindow(): void {
  if (isSettingsWindowOpen()) {
    settingsWindow!.focus()
    return
  }
  settingsWindow = createSettingsWindow()
}
