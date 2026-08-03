import { useEffect, useState } from 'react'
import type { OnboardingProgressEvent, SettingsBootstrapData } from '../../shared/types'
import type { SettingsElectronAPI } from '../../shared/electron-api/settings'
import { Sidebar, type SettingsSection } from './Sidebar'
import { GeneralPane } from './GeneralPane'
import { SkillsPane } from './SkillsPane'
import { FixConfigPane } from './FixConfigPane'
import { ReposPane } from './ReposPane'

export default function App({ api }: { api: SettingsElectronAPI }) {
  const [section, setSection] = useState<SettingsSection>('general')
  const [data, setData] = useState<SettingsBootstrapData | null>(null)

  useEffect(() => {
    let cancelled = false
    api.getInitialData().then((initial) => {
      if (cancelled) return
      document.documentElement.classList.toggle('dark', initial.theme.isDark)
      setData(initial)
    })

    const patch = (next: Partial<SettingsBootstrapData>) =>
      setData((prev) => (prev ? { ...prev, ...next } : prev))
    const offTheme = api.onThemeChanged((theme) =>
      document.documentElement.classList.toggle('dark', theme.isDark),
    )
    const offFix = api.onFixConfigChanged((fixConfig) => patch({ fixConfig }))
    const offRepos = api.onConnectedReposChanged((connectedRepos) => patch({ connectedRepos }))
    const offSpace = api.onSpaceChanged((space) => patch({ space }))
    const offProgress = api.onSkillProgress((event: OnboardingProgressEvent) => {
      if ('kind' in event && event.kind === 'done') patch({ status: event.status })
    })
    return () => {
      cancelled = true
      offTheme()
      offFix()
      offRepos()
      offSpace()
      offProgress()
    }
  }, [api])

  return (
    <div className="flex h-full min-h-0">
      <Sidebar active={section} onChange={setSection} />
      <div className="flex flex-1 min-w-0 flex-col">
        <div className="titlebar-drag h-[34px] w-full shrink-0" />
        <main className="flex-1 min-w-0 overflow-y-auto px-7 pb-8 pt-2">
          {data === null ? null : section === 'general' ? (
            <GeneralPane
              api={api}
              version={data.version}
              space={data.space}
              onSpaceChange={(space) => setData({ ...data, space })}
            />
          ) : section === 'skills' ? (
            <SkillsPane
              api={api}
              status={data.status}
              onStatusChange={(status) => setData({ ...data, status })}
            />
          ) : section === 'models' ? (
            <FixConfigPane api={api} fixConfig={data.fixConfig} />
          ) : (
            <ReposPane api={api} connectedRepos={data.connectedRepos} />
          )}
        </main>
      </div>
    </div>
  )
}
