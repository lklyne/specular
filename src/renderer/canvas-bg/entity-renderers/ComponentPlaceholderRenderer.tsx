import { useState } from 'react'
import type { CanvasSceneFileEntity } from '../../../shared/types'
import type { CanvasBgElectronAPI } from '../../../shared/electron-api/canvas-bg'

declare global {
  interface Window {
    electronAPI: CanvasBgElectronAPI
  }
}

export function ComponentPlaceholderRenderer({
  entity,
  isDark,
}: {
  entity: CanvasSceneFileEntity
  isDark: boolean
}) {
  // The host overlays a WebContentsView pointed at the dev-server URL on
  // top of this DOM. When a repo claims the file, the WCV will render
  // (or be in flight); painting the placeholder underneath bleeds through
  // any transparent regions of the React component. Suppress it then —
  // a brief flash of empty canvas while the WCV loads is preferable to
  // the "Connect a Vite repo" copy ghosting through afterwards.
  if (entity.componentHasRepo) return null

  const repoPath = entity.componentInferredRepoPath
  const homeMatch = entity.file.match(/^\/Users\/[^/]+/)
  const displayPath = homeMatch ? `~${entity.file.slice(homeMatch[0].length)}` : entity.file
  const [connecting, setConnecting] = useState(false)

  const handleConnect = async (event: React.MouseEvent) => {
    event.stopPropagation()
    if (!repoPath || connecting) return
    setConnecting(true)
    try {
      await window.electronAPI.repoConnect(repoPath)
    } finally {
      setConnecting(false)
    }
  }

  return (
    <div
      style={{
        width: '100%',
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 8,
        padding: 16,
        fontFamily: 'system-ui, sans-serif',
        color: isDark ? '#a8a29e' : '#78716c',
        fontSize: 11,
        textAlign: 'center',
      }}
    >
      <svg
        width="32"
        height="32"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <polyline points="16 18 22 12 16 6" />
        <polyline points="8 6 2 12 8 18" />
      </svg>
      <span style={{ wordBreak: 'break-all', maxWidth: '100%' }}>{displayPath}</span>
      {repoPath ? (
        <>
          <button
            type="button"
            onClick={handleConnect}
            disabled={connecting}
            className={`mt-1 inline-flex items-center rounded-md border px-2 py-1 text-[11px] font-medium transition-colors disabled:opacity-60 ${
              isDark
                ? 'border-zinc-600 bg-zinc-800 text-zinc-300 hover:bg-zinc-700'
                : 'border-zinc-300 bg-zinc-50 text-zinc-600 hover:bg-zinc-200'
            }`}
          >
            {connecting ? 'reconnecting…' : 'Reconnect'}
          </button>
        </>
      ) : (
        <span style={{ fontSize: 10, opacity: 0.7 }}>
          Connect a Vite repo to render this component
        </span>
      )}
    </div>
  )
}
