import { contextBridge, ipcRenderer } from 'electron'
import type { OnboardingElectronAPI, OnboardingProgressEvent } from '../shared/types'
import { makeThemeSubscriber, sub } from './ipc-subscribe'

const api: OnboardingElectronAPI = {
  getInitialData: () => ipcRenderer.invoke('onboarding:get-initial-data'),
  install: (selections) => ipcRenderer.invoke('onboarding:install', selections),
  complete: () => ipcRenderer.send('onboarding:complete'),
  dismiss: () => ipcRenderer.send('onboarding:dismiss'),
  onProgress: sub<OnboardingProgressEvent>('onboarding:progress'),
  onThemeChanged: makeThemeSubscriber(),
}

contextBridge.exposeInMainWorld('electronAPI', api)
