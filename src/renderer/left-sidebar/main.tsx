import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import './styles.css'
import { installFocusModality } from '../shared/focusModality'
import type { LeftSidebarElectronAPI } from '../../shared/electron-api/left-sidebar'

installFocusModality()

const api = (window as unknown as { electronAPI: LeftSidebarElectronAPI }).electronAPI

async function bootstrap() {
  const initialData = await api.getInitialData()
  document.documentElement.classList.toggle('dark', initialData.theme.isDark)

  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <App
        initialSidebarData={initialData.sidebarData}
        initialTheme={initialData.theme}
      />
    </StrictMode>,
  )
}

void bootstrap()
