// fallow-ignore-file circular-dependencies
// Suppressed: see #141. preferences imports debug-window for UI coordination
import { app, BrowserWindow } from 'electron'
import type { WebContents } from 'electron'
import { loadRenderer, preloadPath } from './runtime/load-renderer'
import { isDark } from './runtime/preferences'

let debugWindow: BrowserWindow | null = null

const WINDOW_WIDTH = 1024
const WINDOW_HEIGHT = 720

export function isDebugWindowOpen(): boolean {
  return debugWindow !== null && !debugWindow.isDestroyed()
}

export function focusDebugWindow(): void {
  if (isDebugWindowOpen()) debugWindow!.focus()
}

function createDebugWindow(): BrowserWindow {
  const win = new BrowserWindow({
    width: WINDOW_WIDTH,
    height: WINDOW_HEIGHT,
    resizable: true,
    minimizable: true,
    maximizable: true,
    fullscreenable: true,
    title: 'Specular Debug',
    titleBarStyle: 'hiddenInset',
    show: false,
    backgroundColor: isDark() ? '#18181b' : '#fafafa',
    ...(process.platform === 'darwin'
      ? { trafficLightPosition: { x: 14, y: 13 } }
      : {}),
    webPreferences: {
      preload: preloadPath('debug'),
      contextIsolation: true,
      nodeIntegration: false,
    },
  })

  loadRenderer(win, 'debug')
  win.once('ready-to-show', () => win.show())
  win.on('closed', () => {
    debugWindow = null
  })
  return win
}

export function showDebugWindow(): void {
  // Dev-only tool — never open it in packaged builds.
  if (app.isPackaged) return
  if (isDebugWindowOpen()) {
    debugWindow!.focus()
    return
  }
  debugWindow = createDebugWindow()
}

export function getDebugWebContents(): WebContents | null {
  if (!isDebugWindowOpen()) return null
  return debugWindow!.webContents
}

export function closeDebugWindow(): void {
  if (isDebugWindowOpen()) debugWindow!.close()
}
