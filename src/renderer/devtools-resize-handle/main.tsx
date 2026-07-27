import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import './styles.css'
import { initRendererSentry } from '../shared/sentry-init'
import { installFocusModality } from '../shared/focusModality'
import type { DevtoolsResizeHandleElectronAPI } from '../../shared/electron-api/devtools-resize-handle'

initRendererSentry()
installFocusModality()

const api =
  (window as unknown as { electronAPI: DevtoolsResizeHandleElectronAPI }).electronAPI

async function bootstrap() {
  const initialData = await api.getInitialData()
  document.documentElement.classList.toggle('dark', initialData.theme.isDark)

  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <App api={api} initialTheme={initialData.theme} />
    </StrictMode>,
  )
}

void bootstrap()
