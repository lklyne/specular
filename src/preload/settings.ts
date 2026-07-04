import { contextBridge, ipcRenderer } from 'electron'
import type { ConnectedRepo, FixConfig, OnboardingProgressEvent } from '../shared/types'
import type { SettingsElectronAPI } from '../shared/electron-api/settings'
import { ipcChannels } from '../shared/ipc-contract'
import { on } from './ipc-helpers'

const api: SettingsElectronAPI = {
  getInitialData: () => ipcRenderer.invoke(ipcChannels.settingsGetInitialData),
  refreshStatus: () => ipcRenderer.invoke(ipcChannels.settingsRefreshStatus),
  installSkills: (selections) => ipcRenderer.invoke(ipcChannels.settingsInstallSkills, selections),
  setComponentInstalled: (component, installed) =>
    ipcRenderer.invoke(ipcChannels.settingsSetComponentInstalled, { component, installed }),
  setFixConfig: (config) => ipcRenderer.send(ipcChannels.settingsSetFixConfig, config),
  removeOriginBinding: (origin) => ipcRenderer.send(ipcChannels.settingsRemoveOriginBinding, origin),
  repoConnectViaPicker: () => ipcRenderer.invoke(ipcChannels.repoConnectViaPicker),
  repoDisconnect: (id) => ipcRenderer.invoke(ipcChannels.repoDisconnect, { id }),
  repoBindOrigin: (repoId, origin) =>
    ipcRenderer.invoke(ipcChannels.repoBindOrigin, { repoId, origin }),
  close: () => ipcRenderer.send(ipcChannels.settingsClose),
  onSkillProgress: on<OnboardingProgressEvent>(ipcChannels.settingsSkillProgress),
  onFixConfigChanged: on<FixConfig>(ipcChannels.settingsFixConfigChanged),
  onConnectedReposChanged: on<ConnectedRepo[]>(ipcChannels.repoChanged),
  onThemeChanged: on(ipcChannels.themeChanged),
}

contextBridge.exposeInMainWorld('electronAPI', api)
