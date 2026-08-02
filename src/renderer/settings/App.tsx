import { useEffect, useState } from 'react'
import type { ConnectedRepo, FixConfig, OnboardingStatusSnapshot, SettingsBootstrapData } from '../../shared/types'
import type { SettingsElectronAPI } from '../../shared/electron-api/settings'
import { Sidebar, type SettingsSection } from './Sidebar'
import { GeneralPane } from './GeneralPane'
import { SkillsPane } from './SkillsPane'
import { FixConfigPane } from './FixConfigPane'
import { ReposPane } from './ReposPane'

export default function App({
  api,
  initialData,
}: {
  api: SettingsElectronAPI
  initialData: SettingsBootstrapData
}) {
  const [section, setSection] = useState<SettingsSection>('general')
  const [status, setStatus] = useState<OnboardingStatusSnapshot>(initialData.status)
  const [fixConfig, setFixConfig] = useState<FixConfig>(initialData.fixConfig)
  const [connectedRepos, setConnectedRepos] = useState<ConnectedRepo[]>(
    initialData.connectedRepos,
  )
  const [space, setSpace] = useState(initialData.space)

  useEffect(() => {
    const offTheme = api.onThemeChanged((data) =>
      document.documentElement.classList.toggle('dark', data.isDark),
    )
    const offFix = api.onFixConfigChanged((next) => setFixConfig(next))
    const offRepos = api.onConnectedReposChanged((next) => setConnectedRepos(next))
    const offSpace = api.onSpaceChanged((next) => setSpace(next))
    return () => {
      offTheme()
      offFix()
      offRepos()
      offSpace()
    }
  }, [api])

  return (
    <div className="flex h-full min-h-0">
      <Sidebar active={section} onChange={setSection} />
      <div className="flex flex-1 min-w-0 flex-col">
        <div className="titlebar-drag h-[34px] w-full shrink-0" />
        <main className="flex-1 min-w-0 overflow-y-auto px-7 pb-8 pt-2">
          {section === 'general' ? (
            <GeneralPane api={api} space={space} onSpaceChange={setSpace} />
          ) : section === 'skills' ? (
            <SkillsPane api={api} status={status} onStatusChange={setStatus} />
          ) : section === 'models' ? (
            <FixConfigPane api={api} fixConfig={fixConfig} />
          ) : (
            <ReposPane api={api} connectedRepos={connectedRepos} />
          )}
        </main>
      </div>
    </div>
  )
}
