import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import '../above-view/styles.css'
import { bootCanvasRenderer } from '../shared/boot-canvas-renderer'

async function bootstrap() {
  const { initialData } = await bootCanvasRenderer()

  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <App initialLayoutData={initialData.layoutData} />
    </StrictMode>,
  )
}

void bootstrap()
