import { contextBridge, ipcRenderer } from 'electron'
import type {
  ConnectedRepo,
  FixConfig,
  OnboardingProgressEvent,
  SettingsElectronAPI,
} from '../shared/types'
import { ipcChannels } from '../shared/ipc-contract'
import { on } from './ipc-helpers'

const api: SettingsElectronAPI = {
  getInitialData: () => ipcRenderer.invoke('settings:get-initial-data'),
  refreshStatus: () => ipcRenderer.invoke('settings:refresh-status'),
  installSkills: (selections) => ipcRenderer.invoke('settings:install-skills', selections),
  setComponentInstalled: (component, installed) =>
    ipcRenderer.invoke('settings:set-component-installed', { component, installed }),
  setFixConfig: (config) => ipcRenderer.send('settings:set-fix-config', config),
  removeOriginBinding: (origin) => ipcRenderer.send('settings:remove-origin-binding', origin),
  repoConnectViaPicker: () => ipcRenderer.invoke('repo-connect-via-picker'),
  repoDisconnect: (id) => ipcRenderer.invoke('repo-disconnect', { id }),
  repoBindOrigin: (repoId, origin) =>
    ipcRenderer.invoke('repo-bind-origin', { repoId, origin }),
  close: () => ipcRenderer.send('settings:close'),
  onSkillProgress: on<OnboardingProgressEvent>('settings:skill-progress'),
  onFixConfigChanged: on<FixConfig>('settings:fix-config-changed'),
  onConnectedReposChanged: on<ConnectedRepo[]>('repo-changed'),
  onThemeChanged: on(ipcChannels.themeChanged),
}

contextBridge.exposeInMainWorld('electronAPI', api)
