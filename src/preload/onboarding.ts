import { contextBridge, ipcRenderer } from 'electron'
import type { OnboardingProgressEvent } from '../shared/types'
import type { OnboardingElectronAPI } from '../shared/electron-api/onboarding'
import { ipcChannels } from '../shared/ipc-contract'
import { on } from './ipc-helpers'

const api: OnboardingElectronAPI = {
  getInitialData: () => ipcRenderer.invoke(ipcChannels.onboardingGetInitialData),
  install: (selections) => ipcRenderer.invoke(ipcChannels.onboardingInstall, selections),
  complete: () => ipcRenderer.send(ipcChannels.onboardingComplete),
  dismiss: () => ipcRenderer.send(ipcChannels.onboardingDismiss),
  onProgress: on<OnboardingProgressEvent>(ipcChannels.onboardingProgress),
  onThemeChanged: on(ipcChannels.themeChanged),
}

contextBridge.exposeInMainWorld('electronAPI', api)
