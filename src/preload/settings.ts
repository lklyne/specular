import { contextBridge, ipcRenderer } from 'electron'
import type {
  ConnectedRepo,
  FixConfig,
  OnboardingProgressEvent,
  SettingsElectronAPI,
} from '../shared/types'
import { makeThemeSubscriber, sub } from './ipc-subscribe'

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
  onSkillProgress: sub<OnboardingProgressEvent>('settings:skill-progress'),
  onFixConfigChanged: sub<FixConfig>('settings:fix-config-changed'),
  onConnectedReposChanged: sub<ConnectedRepo[]>('repo-changed'),
  onThemeChanged: makeThemeSubscriber(),
}

contextBridge.exposeInMainWorld('electronAPI', api)
