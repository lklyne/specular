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

async function bootstrap() {
  const initialData = await api.getInitialData()
  document.documentElement.classList.toggle('dark', initialData.theme.isDark)

  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <App api={api} initialData={initialData} />
    </StrictMode>,
  )
}

bootstrap().catch((err) => {
  console.error('settings bootstrap failed', err)
  document.getElementById('root')!.textContent = `Settings failed to load: ${String(err)}`
})
