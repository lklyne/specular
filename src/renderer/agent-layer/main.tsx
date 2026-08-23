import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import '../above-view/styles.css'
import { initRendererSentry } from '../shared/sentry-init'
import { installFocusModality } from '../shared/focusModality'
import { runtimeStore } from '../shared/runtime-store'
import { connectRuntimeStore } from '../shared/runtime-store-feed'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'

initRendererSentry()
installFocusModality()

const api = (window as unknown as { electronAPI: CanvasBgElectronAPI }).electronAPI

connectRuntimeStore(api)

async function bootstrap() {
  const initialData = await api.getInitialData()
  runtimeStore.applySnapshot(initialData.layoutData)
  document.documentElement.classList.toggle('dark', initialData.theme.isDark)

  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <App />
    </StrictMode>,
  )
}

void bootstrap()
