import { useEffect, useState } from 'react'
import type { CursorTuningParams, DebugBootstrapData } from '../../shared/types'
import type { DebugElectronAPI } from '../../shared/electron-api/debug'
import { DEFAULT_CURSOR_TUNING } from '../../shared/cursor-tuning'
import { useTheme } from '../shared/hooks/useTheme'
import { PresenceSection } from './PresenceSection'
import { PerformanceSection } from './PerformanceSection'
import { ProcessesSection } from './ProcessesSection'

const SECTIONS = [
  { id: 'presence', label: 'Presence' },
  { id: 'performance', label: 'Performance' },
  { id: 'processes', label: 'Processes' },
] as const

type SectionId = (typeof SECTIONS)[number]['id']

export default function App({
  api,
  initialData,
}: {
  api: DebugElectronAPI
  initialData: DebugBootstrapData
}) {
  useTheme(initialData.theme, api.onThemeChanged)
  const [activeSection, setActiveSection] = useState<SectionId>('presence')
  const [splineViz, setSplineViz] = useState<boolean>(initialData.cursorSplineViz)
  const [cursorTuning, setCursorTuning] = useState<CursorTuningParams>(
    initialData.cursorTuning,
  )

  useEffect(() => api.onCursorSplineVizChanged(setSplineViz), [api])

  const commitSplineViz = (next: boolean) => {
    setSplineViz(next)
    api.updateCursorSplineViz(next)
  }

  const commitCursorTuning = (next: CursorTuningParams) => {
    setCursorTuning(next)
    api.updateCursorTuning(next)
  }

  const resetCursorTuning = () => {
    api.resetCursorTuning()
    setCursorTuning(DEFAULT_CURSOR_TUNING)
  }

  return (
    <div className="flex h-full flex-col">
      <div className="titlebar-drag h-[34px] w-full shrink-0" />
      <div className="flex min-h-0 flex-1">
        <nav className="flex w-44 shrink-0 flex-col border-r border-[var(--surface-popover-border)] px-2 py-3">
          <div className="mb-2 px-2 text-[10px] font-semibold uppercase tracking-wider opacity-50">
            Debug
          </div>
          {SECTIONS.map((section) => (
            <button
              key={section.id}
              type="button"
              onClick={() => setActiveSection(section.id)}
              className={
                activeSection === section.id
                  ? 'rounded bg-zinc-200 px-2 py-1 text-left text-[12px] text-zinc-900 dark:bg-zinc-800 dark:text-zinc-100'
                  : 'rounded px-2 py-1 text-left text-[12px] text-zinc-600 hover:bg-zinc-100 dark:text-zinc-400 dark:hover:bg-zinc-800/60'
              }
            >
              {section.label}
            </button>
          ))}
        </nav>
        <main className="flex min-h-0 min-w-0 flex-1">
          {activeSection === 'presence' ? (
            <PresenceSection
              splineViz={splineViz}
              onSplineVizChange={commitSplineViz}
              tuning={cursorTuning}
              onTuningChange={commitCursorTuning}
              onTuningReset={resetCursorTuning}
            />
          ) : activeSection === 'performance' ? (
            <PerformanceSection api={api} />
          ) : (
            <ProcessesSection api={api} />
          )}
        </main>
      </div>
    </div>
  )
}
