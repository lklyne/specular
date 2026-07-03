import { contextBridge, ipcRenderer } from 'electron'
import type { OnboardingElectronAPI, OnboardingProgressEvent } from '../shared/types'
import { on } from './ipc-helpers'

const api: OnboardingElectronAPI = {
  getInitialData: () => ipcRenderer.invoke('onboarding:get-initial-data'),
  install: (selections) => ipcRenderer.invoke('onboarding:install', selections),
  complete: () => ipcRenderer.send('onboarding:complete'),
  dismiss: () => ipcRenderer.send('onboarding:dismiss'),
  onProgress: on<OnboardingProgressEvent>('onboarding:progress'),
  onThemeChanged: on<{ isDark: boolean }>('theme-changed'),
}

contextBridge.exposeInMainWorld('electronAPI', api)
