import { BrowserWindow } from 'electron'
import { loadRenderer, preloadPath } from '../runtime/load-renderer'
import { isDark } from '../runtime/preferences'
import { attachOsrLabTarget, teardownOsrLab } from './osr-lab-pages'

let labWindow: BrowserWindow | null = null

export function isOsrLabOpen(): boolean {
  return labWindow !== null && !labWindow.isDestroyed()
}

export function showOsrLabWindow(): void {
  if (isOsrLabOpen()) {
    labWindow!.focus()
    return
  }
  const win = new BrowserWindow({
    width: 1400,
    height: 900,
    title: 'Specular Offscreen Rendering Lab',
    backgroundColor: isDark() ? '#18181b' : '#fafafa',
    show: false,
    webPreferences: {
      preload: preloadPath('osr-lab'),
      contextIsolation: true,
      nodeIntegration: false,
      // The preload receives GPU textures through the `sharedTexture` module,
      // which the sandboxed renderer API exposes, and forwards them to the
      // page world as transferred ImageBitmaps.
      sandbox: true,
    },
  })
  attachOsrLabTarget(win.webContents)
  loadRenderer(win, 'osr-lab')
  win.once('ready-to-show', () => win.show())
  win.on('closed', () => {
    labWindow = null
    attachOsrLabTarget(null)
    teardownOsrLab()
  })
  labWindow = win
}
