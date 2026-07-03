import { contextBridge, ipcRenderer } from 'electron'
import type { OnboardingElectronAPI, OnboardingProgressEvent } from '../shared/types'
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
