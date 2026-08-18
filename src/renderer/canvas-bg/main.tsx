import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import './styles.css'
import { installFocusModality } from '../shared/focusModality'
import { installRendererErrorReporter } from '../shared/install-error-reporter'
import { RendererErrorBoundary } from '../shared/RendererErrorBoundary'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'

installFocusModality()
installRendererErrorReporter('canvas-bg')

const api = (window as unknown as { electronAPI: CanvasBgElectronAPI }).electronAPI

async function bootstrap() {
  const initialData = await api.getInitialData()
  document.documentElement.classList.toggle('dark', initialData.theme.isDark)

  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <RendererErrorBoundary label="canvas-bg">
        <App
          initialLayoutData={initialData.layoutData}
          initialTheme={initialData.theme}
        />
      </RendererErrorBoundary>
    </StrictMode>,
  )
}

void bootstrap()
