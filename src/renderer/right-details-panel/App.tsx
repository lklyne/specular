import type { ThemeData } from '../../shared/types'
import { useReportTextEditing } from '../shared/hooks/useReportTextEditing'
import { useTheme } from '../shared/hooks/useTheme'
import { PaneProvider } from './PaneContext'
import { ChatPane } from './components/ChatPane'
import { PaneHeader } from './components/PaneHeader'
import { rightDetailsPanelApi } from './rightDetailsPanelApi'
import { useRightDetailsPanelData } from './useRightDetailsPanelData'

export default function App({ initialTheme }: { initialTheme: ThemeData }) {
  const panelData = useRightDetailsPanelData()
  const { isDark } = useTheme(initialTheme, rightDetailsPanelApi.onThemeChanged)

  useReportTextEditing(rightDetailsPanelApi.setTextEditing)

  const pageClass = 'h-screen w-screen overflow-hidden border-l border-[var(--surface-chrome-border)] bg-[var(--surface-panel)] text-[var(--surface-foreground)]'

  if (panelData.activeTab === 'browser-devtools') {
    return (
      <div className={pageClass}>
        <PaneHeader
          icon={
            <svg className="h-3.5 w-3.5 shrink-0 opacity-50" viewBox="0 0 16 16" fill="currentColor">
              <path d="M4.708 5.578L2.061 8.224l2.647 2.646-.708.708L.646 8.224 4 4.87l.708.708zm7.292 0l2.647 2.646-2.647 2.646.708.708L16.062 8.224 12.708 4.87l-.708.708zM6.754 12.5l1.429-9h1.063l-1.429 9H6.754z" />
            </svg>
          }
          label="Dev Tools"
          actions={
            <button
              type="button"
              className={`rounded p-0.5 ${
                isDark ? 'hover:bg-zinc-700' : 'hover:bg-zinc-200'
              }`}
              onClick={() => rightDetailsPanelApi.closeBrowserDevTools()}
            >
              <svg className="h-3.5 w-3.5 opacity-50" viewBox="0 0 16 16" fill="currentColor">
                <path d="M12.354 4.354a.5.5 0 0 0-.708-.708L8 7.293 4.354 3.646a.5.5 0 1 0-.708.708L7.293 8l-3.647 3.646a.5.5 0 0 0 .708.708L8 8.707l3.646 3.647a.5.5 0 0 0 .708-.708L8.707 8l3.647-3.646z" />
              </svg>
            </button>
          }
        />
      </div>
    )
  }

  return (
    <PaneProvider isDark={isDark}>
      <div className={pageClass}>
        <div className="flex h-full min-h-0 flex-col">
          <ChatPane data={panelData} />
        </div>
      </div>
    </PaneProvider>
  )
}
