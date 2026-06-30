import { contextBridge, ipcRenderer } from 'electron'
import type { OnboardingElectronAPI } from '../shared/types'

const api: OnboardingElectronAPI = {
  getInitialData: () => ipcRenderer.invoke('onboarding:get-initial-data'),
  install: (selections) => ipcRenderer.invoke('onboarding:install', selections),
  complete: () => ipcRenderer.send('onboarding:complete'),
  dismiss: () => ipcRenderer.send('onboarding:dismiss'),
  onProgress: (callback) => {
    const handler = (_event: Electron.IpcRendererEvent, payload: unknown) => {
      callback(payload as Parameters<typeof callback>[0])
    }
    ipcRenderer.on('onboarding:progress', handler)
    return () => ipcRenderer.removeListener('onboarding:progress', handler)
  },
  onThemeChanged: (callback) => {
    const handler = (_event: Electron.IpcRendererEvent, data: { isDark: boolean }) =>
      callback(data)
    ipcRenderer.on('theme-changed', handler)
    return () => ipcRenderer.removeListener('theme-changed', handler)
  },
}

contextBridge.exposeInMainWorld('electronAPI', api)
