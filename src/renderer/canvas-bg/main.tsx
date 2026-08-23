import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import './styles.css'
import { bootCanvasRenderer } from '../shared/boot-canvas-renderer'
import { RendererErrorBoundary } from '../shared/RendererErrorBoundary'

async function bootstrap() {
  const { initialData } = await bootCanvasRenderer({ errorReporterLabel: 'canvas-bg' })

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
