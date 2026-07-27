import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import './styles.css'
import { initRendererSentry } from '../shared/sentry-init'
import { installFocusModality } from '../shared/focusModality'
import { toolbarApi } from './toolbarApi'

initRendererSentry()
installFocusModality()

async function bootstrap() {
  const initialData = await toolbarApi.getInitialData()
  document.documentElement.classList.toggle('dark', initialData.theme.isDark)

  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <App initialTheme={initialData.theme} />
    </StrictMode>,
  )
}

void bootstrap()
