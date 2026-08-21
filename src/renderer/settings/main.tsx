import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import './styles.css'
import { initRendererSentry } from '../shared/sentry-init'
import { installFocusModality } from '../shared/focusModality'
import type { SettingsElectronAPI } from '../../shared/electron-api/settings'

initRendererSentry()
installFocusModality()

const api =
  (window as unknown as { electronAPI: SettingsElectronAPI }).electronAPI

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App api={api} />
  </StrictMode>,
)
