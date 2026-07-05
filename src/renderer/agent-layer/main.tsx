import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import '../above-view/styles.css'
import { initRendererSentry } from '../shared/sentry-init'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'

initRendererSentry()

const api = (window as unknown as { electronAPI: CanvasBgElectronAPI }).electronAPI

async function bootstrap() {
  const initialData = await api.getInitialData()
  document.documentElement.classList.toggle('dark', initialData.theme.isDark)

  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <App initialLayoutData={initialData.layoutData} />
    </StrictMode>,
  )
}

void bootstrap()
