import type { DevtoolsPanelElectronAPI } from '../../shared/electron-api/right-details-panel'

export const rightDetailsPanelApi = (
  window as unknown as { electronAPI: DevtoolsPanelElectronAPI }
).electronAPI
