import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import './styles.css'
import type { OsrLabElectronAPI } from '../../shared/electron-api/osr-lab'

const api = (window as unknown as { electronAPI: OsrLabElectronAPI }).electronAPI

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App api={api} />
  </StrictMode>,
)
