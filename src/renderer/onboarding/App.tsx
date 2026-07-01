import { useCallback, useEffect, useMemo, useReducer, useState } from 'react'
import type {
  OnboardingBootstrapData,
  OnboardingComponentId,
  OnboardingComponentStatus,
  OnboardingElectronAPI,
  OnboardingProgressEvent,
  OnboardingStatusSnapshot,
} from '../../shared/types'
import { SkillInstaller, type InstallerRowSnapshot, type RowProgress } from '../shared/SkillInstaller'

type RowProgressMap = Record<OnboardingComponentId, { progress: RowProgress; detail?: string }>

type ProgressAction =
  | { kind: 'reset' }
  | { kind: 'start'; id: OnboardingComponentId }
  | { kind: 'success'; id: OnboardingComponentId; detail?: string }
  | { kind: 'error'; id: OnboardingComponentId; detail: string }

const INITIAL_PROGRESS: RowProgressMap = {
  cli: { progress: 'idle' },
  skill: { progress: 'idle' },
  agentBrowser: { progress: 'idle' },
}

function progressReducer(state: RowProgressMap, action: ProgressAction): RowProgressMap {
  switch (action.kind) {
    case 'reset':
      return INITIAL_PROGRESS
    case 'start':
      return { ...state, [action.id]: { progress: 'installing' } }
    case 'success':
      return { ...state, [action.id]: { progress: 'success', detail: action.detail } }
    case 'error':
      return { ...state, [action.id]: { progress: 'error', detail: action.detail } }
  }
}

function defaultSelected(status: OnboardingComponentStatus): boolean {
  return status.kind !== 'installed'
}

function anyInstallable(
  selections: Record<OnboardingComponentId, boolean>,
): boolean {
  return Object.values(selections).some(Boolean)
}

function allInstalledOrSkipped(
  status: OnboardingStatusSnapshot,
  progress: RowProgressMap,
): boolean {
  const ids: OnboardingComponentId[] = ['cli', 'skill', 'agentBrowser']
  return ids.every(
    (id) => status[id].kind === 'installed' || progress[id].progress === 'success',
  )
}

function WelcomeScreen({ onContinue, onSkip }: { onContinue: () => void; onSkip: () => void }) {
  return (
    <div className="flex h-full flex-col">
      <div className="titlebar-drag h-[34px] w-full shrink-0" />
      <div className="flex flex-1 flex-col items-center justify-center px-8 pb-8">
        <h1 className="text-[28px] font-bold tracking-tight">Welcome to Specular</h1>
        <p className="mt-4 max-w-[380px] text-center text-[15px] leading-relaxed text-[var(--surface-toolbar-foreground)] opacity-70">
          Part browser and part canvas, Specular is a visual way to build with agents.
        </p>
      </div>
      <footer className="flex shrink-0 items-center justify-between gap-3 border-t border-[var(--surface-popover-border)] bg-[var(--surface-panel)] px-8 py-4">
        <button
          type="button"
          className="text-[12px] text-[var(--surface-toolbar-foreground)] opacity-70 hover:opacity-100"
          onClick={onSkip}
        >
          Skip for now
        </button>
        <button
          type="button"
          className="rounded-[6px] bg-[var(--surface-toolbar-foreground)] px-4 py-[6px] text-[12px] font-medium text-[var(--surface-panel)] shadow-sm hover:opacity-90"
          onClick={onContinue}
        >
          Get started
        </button>
      </footer>
    </div>
  )
}

function SetupScreen({
  api,
  initialData,
}: {
  api: OnboardingElectronAPI
  initialData: OnboardingBootstrapData
}) {
  const [status, setStatus] = useState<OnboardingStatusSnapshot>(initialData.status)
  const [progress, dispatchProgress] = useReducer(progressReducer, INITIAL_PROGRESS)
  const [installing, setInstalling] = useState(false)
  const [selections, setSelections] = useState<Record<OnboardingComponentId, boolean>>({
    cli: defaultSelected(initialData.status.cli),
    skill: defaultSelected(initialData.status.skill),
    agentBrowser: defaultSelected(initialData.status.agentBrowser),
  })

  useEffect(() => {
    return api.onProgress((event: OnboardingProgressEvent) => {
      if ('kind' in event && event.kind === 'done') {
        setStatus(event.status)
        setInstalling(false)
        return
      }
      if ('component' in event) {
        if (event.state === 'installing') {
          dispatchProgress({ kind: 'start', id: event.component })
        } else if (event.state === 'success') {
          dispatchProgress({ kind: 'success', id: event.component, detail: event.detail })
        } else {
          dispatchProgress({ kind: 'error', id: event.component, detail: event.detail })
        }
      }
    })
  }, [api])

  const setSelected = useCallback(
    (id: OnboardingComponentId, selected: boolean) =>
      setSelections((prev) => ({ ...prev, [id]: selected })),
    [],
  )

  const rows = useMemo<Record<OnboardingComponentId, InstallerRowSnapshot>>(
    () => {
      const ids: OnboardingComponentId[] = ['cli', 'skill', 'agentBrowser']
      return Object.fromEntries(
        ids.map((id) => [id, {
          status: status[id],
          progress: progress[id].progress,
          progressDetail: progress[id].detail,
          selected: selections[id],
        }]),
      ) as Record<OnboardingComponentId, InstallerRowSnapshot>
    },
    [status, progress, selections],
  )

  const handleInstall = useCallback(async () => {
    if (installing || !anyInstallable(selections)) return
    setInstalling(true)
    dispatchProgress({ kind: 'reset' })
    try {
      const next = await api.install(selections)
      setStatus(next)
    } finally {
      setInstalling(false)
    }
  }, [api, installing, selections])

  const isSettingsMode = initialData.mode === 'settings'
  const canFinish = allInstalledOrSkipped(status, progress)
  const primaryLabel = canFinish
    ? isSettingsMode
      ? 'Done'
      : 'Get started'
    : installing
      ? 'Installing\u2026'
      : 'Install selected'

  const primaryAction = canFinish ? () => api.complete() : handleInstall

  return (
    <div className="flex h-full flex-col">
      <div className="titlebar-drag h-[34px] w-full shrink-0" />
      <div className="flex-1 min-h-0 overflow-y-auto px-8 pb-8">
        <header className="mb-6 mt-4">
          <h1 className="text-[18px] font-semibold">
            {isSettingsMode ? 'Specular Setup' : 'Setup'}
          </h1>
          <p className="mt-2 text-[13px] leading-snug text-[var(--surface-toolbar-foreground)] opacity-70">
            {isSettingsMode
              ? 'Install or re-check the integrations that let Claude Code drive Specular.'
              : "You'll need to set up a few tools so agents know how this thing works."}
          </p>
        </header>

        {!status.claudeDirExists ? (
          <div className="mb-4 rounded-[8px] border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-[12px] text-amber-700 dark:text-amber-300">
            {`Claude Code doesn't seem to be installed (~/.claude not found). You can still install the skills; they'll activate once Claude Code is set up.`}
          </div>
        ) : null}

        <SkillInstaller.Root rows={rows} setSelected={setSelected}>
          <SkillInstaller.Row
            id="cli"
            title="Specular CLI"
            description="Adds the specular command so agents can interact with the app."
          />
          <SkillInstaller.Row
            id="skill"
            title="Specular Skill"
            description="Teaches agents how to use the Specular CLI."
          />
          <SkillInstaller.Row
            id="agentBrowser"
            title="agent-browser"
            description="Specular uses Vercel's agent-browser to capture and interact with live webpages. You can install it here or at agent-browser.dev."
          />
        </SkillInstaller.Root>
      </div>

      <footer className="flex shrink-0 items-center justify-between gap-3 border-t border-[var(--surface-popover-border)] bg-[var(--surface-panel)] px-8 py-4">
        <button
          type="button"
          className="text-[12px] text-[var(--surface-toolbar-foreground)] opacity-70 hover:opacity-100 disabled:opacity-40"
          disabled={installing}
          onClick={() => api.dismiss()}
        >
          {isSettingsMode ? 'Close' : 'Skip for now'}
        </button>
        <button
          type="button"
          className="rounded-[6px] bg-[var(--surface-toolbar-foreground)] px-4 py-[6px] text-[12px] font-medium text-[var(--surface-panel)] shadow-sm enabled:hover:opacity-90 disabled:opacity-50"
          disabled={installing || (!canFinish && !anyInstallable(selections))}
          onClick={primaryAction}
        >
          {primaryLabel}
        </button>
      </footer>
    </div>
  )
}

export default function App({
  api,
  initialData,
}: {
  api: OnboardingElectronAPI
  initialData: OnboardingBootstrapData
}) {
  const [step, setStep] = useState<'welcome' | 'setup'>(
    initialData.mode === 'welcome' ? 'welcome' : 'setup',
  )

  useEffect(() => {
    return api.onThemeChanged((data) =>
      document.documentElement.classList.toggle('dark', data.isDark),
    )
  }, [api])

  if (step === 'welcome') {
    return (
      <WelcomeScreen
        onContinue={() => setStep('setup')}
        onSkip={() => api.dismiss()}
      />
    )
  }

  return <SetupScreen api={api} initialData={initialData} />
}
