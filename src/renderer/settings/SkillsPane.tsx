import { useCallback, useState } from 'react'
import { Switch } from '@base-ui/react/switch'
import { Loader2 } from 'lucide-react'
import type { OnboardingComponentId, OnboardingComponentStatus, OnboardingStatusSnapshot } from '../../shared/types'
import type { SettingsElectronAPI } from '../../shared/electron-api/settings'

type RowConfig = {
  id: OnboardingComponentId
  title: string
  description: string
}

const ROWS: RowConfig[] = [
  {
    id: 'cli',
    title: 'Specular CLI',
    description: 'Adds the specular command so agents can interact with the app.',
  },
  {
    id: 'skill',
    title: 'Specular Skill',
    description: 'Teaches agents how to use the Specular CLI.',
  },
]

function statusDetail(status: OnboardingComponentStatus): string | undefined {
  if (status.kind === 'installed') return status.detail
  if (status.kind === 'outdated') return status.detail ?? 'update available'
  if (status.kind === 'blocked') return status.detail
  return undefined
}

export function SkillsPane({
  api,
  status,
  onStatusChange,
}: {
  api: SettingsElectronAPI
  status: OnboardingStatusSnapshot
  onStatusChange: (next: OnboardingStatusSnapshot) => void
}) {
  const [pending, setPending] = useState<Record<OnboardingComponentId, boolean>>({
    cli: false,
    skill: false,
  })
  const [errors, setErrors] = useState<Partial<Record<OnboardingComponentId, string>>>({})

  const handleToggle = useCallback(
    async (id: OnboardingComponentId, next: boolean) => {
      setPending((prev) => ({ ...prev, [id]: true }))
      setErrors((prev) => ({ ...prev, [id]: undefined }))
      try {
        const snapshot = await api.setComponentInstalled(id, next)
        onStatusChange(snapshot)
        const after = snapshot[id]
        const wantInstalled = next
        const isInstalled = after.kind === 'installed'
        if (wantInstalled !== isInstalled) {
          const message =
            after.kind === 'blocked'
              ? after.detail
              : next
                ? 'Install failed.'
                : 'Uninstall failed.'
          setErrors((prev) => ({ ...prev, [id]: message }))
        }
      } finally {
        setPending((prev) => ({ ...prev, [id]: false }))
      }
    },
    [api, onStatusChange],
  )

  return (
    <section>
      <header className="mb-4 mt-2">
        <h2 className="text-[15px] font-semibold">Skills</h2>
        <p className="mt-1 text-[12px] leading-snug text-[var(--surface-toolbar-foreground)] opacity-70">
          Toggle the integrations that let Claude Code drive Specular.
        </p>
      </header>

      {!status.claudeDirExists ? (
        <div className="mb-4 rounded-[8px] border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-[12px] text-amber-700 dark:text-amber-300">
          {`Claude Code doesn't seem to be installed (~/.claude not found). You can still install the skills; they'll activate once Claude Code is set up.`}
        </div>
      ) : null}

      <div className="flex flex-col gap-2">
        {ROWS.map((row) => {
          const componentStatus = status[row.id]
          const installed = componentStatus.kind === 'installed'
          const isPending = pending[row.id]
          const disabled = isPending
          const error = errors[row.id]
          const detail = error ?? statusDetail(componentStatus)
          const detailIsError = !!error || componentStatus.kind === 'blocked'

          return (
            <label
              key={row.id}
              className={`flex items-start gap-3 rounded-[8px] border border-[var(--surface-card-border)] bg-[var(--surface-card)] px-4 py-3 select-none ${
                disabled ? 'cursor-not-allowed' : 'cursor-pointer'
              }`}
            >
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-[13px] font-medium">{row.title}</span>
                  {isPending ? (
                    <Loader2
                      size={12}
                      className="animate-spin text-[var(--surface-toolbar-foreground)] opacity-70"
                    />
                  ) : null}
                </div>
                <p className="mt-1 text-[12px] leading-snug text-[var(--surface-toolbar-foreground)] opacity-70">
                  {row.description}
                </p>
                {detail ? (
                  <p
                    className={
                      detailIsError
                        ? 'mt-1 text-[11px] text-red-600 dark:text-red-400'
                        : 'mt-1 text-[11px] text-[var(--surface-toolbar-foreground)] opacity-60'
                    }
                  >
                    {detail}
                  </p>
                ) : null}
              </div>
              <div className="pt-[2px]">
                <Switch.Root
                  disabled={disabled}
                  checked={installed}
                  onCheckedChange={(checked) => handleToggle(row.id, checked)}
                  className="relative inline-flex h-[18px] w-[32px] shrink-0 cursor-pointer items-center rounded-full border border-[var(--surface-switch-track-border)] bg-[var(--surface-switch-track)] transition-colors data-[checked]:border-[var(--surface-switch-track-checked-border)] data-[checked]:bg-[var(--surface-switch-track-checked)] data-[disabled]:cursor-not-allowed data-[disabled]:opacity-50"
                >
                  <Switch.Thumb className="block h-[14px] w-[14px] translate-x-[1px] rounded-full bg-white shadow-sm transition-transform data-[checked]:translate-x-[15px]" />
                </Switch.Root>
              </div>
            </label>
          )
        })}

        <AgentBrowserStatusRow status={status.agentBrowser} />
      </div>
    </section>
  )
}

/**
 * agent-browser is bundled and auto-configured on launch — nothing here to
 * toggle, just readout of the bundled driver and any user-owned binary also
 * on PATH (D3, issue #318).
 */
function AgentBrowserStatusRow({ status }: { status: OnboardingComponentStatus }) {
  const detail = statusDetail(status)
  const detailIsError = status.kind === 'blocked'

  return (
    <div className="flex items-start gap-3 rounded-[8px] border border-[var(--surface-card-border)] bg-[var(--surface-card)] px-4 py-3 select-none">
      <div className="flex-1 min-w-0">
        <span className="text-[13px] font-medium">agent-browser</span>
        <p className="mt-1 text-[12px] leading-snug text-[var(--surface-toolbar-foreground)] opacity-70">
          Specular bundles Vercel's agent-browser to capture and interact with live webpages — no setup needed.
        </p>
        {detail ? (
          <p
            className={
              detailIsError
                ? 'mt-1 text-[11px] text-red-600 dark:text-red-400'
                : 'mt-1 text-[11px] text-[var(--surface-toolbar-foreground)] opacity-60'
            }
          >
            {detail}
          </p>
        ) : null}
      </div>
    </div>
  )
}
