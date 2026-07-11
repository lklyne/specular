import {
  createContext,
  useContext,
  type ReactNode,
} from 'react'
import { Switch } from '@base-ui/react/switch'
import { Check, CircleAlert, Loader2, Minus } from 'lucide-react'
import type {
  OnboardingComponentId,
  OnboardingComponentStatus,
} from '../../shared/types'

export type RowProgress = 'idle' | 'installing' | 'success' | 'error'

export type InstallerRowSnapshot = {
  status: OnboardingComponentStatus
  progress: RowProgress
  selected: boolean
  progressDetail?: string
}

type InstallerContextValue = {
  rows: Record<OnboardingComponentId, InstallerRowSnapshot>
  setSelected: (id: OnboardingComponentId, selected: boolean) => void
}

const InstallerContext = createContext<InstallerContextValue | null>(null)

function useInstaller(): InstallerContextValue {
  const ctx = useContext(InstallerContext)
  if (!ctx) throw new Error('Installer subcomponent used outside <Installer.Root>')
  return ctx
}

function Root({
  rows,
  setSelected,
  children,
}: {
  rows: Record<OnboardingComponentId, InstallerRowSnapshot>
  setSelected: (id: OnboardingComponentId, selected: boolean) => void
  children: ReactNode
}) {
  return (
    <InstallerContext.Provider value={{ rows, setSelected }}>
      <div className="flex flex-col gap-2">{children}</div>
    </InstallerContext.Provider>
  )
}

function rowBaseClass(progress: RowProgress): string {
  const base =
    'flex items-start gap-3 rounded-[8px] border border-[var(--surface-card-border)] bg-[var(--surface-card)] px-4 py-3 text-left cursor-pointer select-none'
  if (progress === 'installing') return `${base} opacity-90 cursor-not-allowed`
  if (progress === 'error') return `${base} border-red-500/50`
  return base
}

function Row({
  id,
  title,
  description,
}: {
  id: OnboardingComponentId
  title: string
  description: string
}) {
  const { rows, setSelected } = useInstaller()
  const snapshot = rows[id]

  return (
    <label className={rowBaseClass(snapshot.progress)}>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="text-[13px] font-medium">{title}</span>
          <Badge state={rowBadgeState(snapshot)} />
        </div>
        <p className="mt-1 text-[12px] leading-snug text-[var(--surface-toolbar-foreground)] opacity-70">
          {description}
        </p>
        <RowDetail snapshot={snapshot} />
      </div>
      <div className="pt-[2px]">
        <Switch.Root
          disabled={snapshot.progress === 'installing'}
          checked={snapshot.selected}
          onCheckedChange={(checked) => setSelected(id, checked)}
          className="relative inline-flex h-[18px] w-[32px] shrink-0 cursor-pointer items-center rounded-full border border-[var(--surface-switch-track-border)] bg-[var(--surface-switch-track)] transition-colors data-[checked]:border-[var(--surface-switch-track-checked-border)] data-[checked]:bg-[var(--surface-switch-track-checked)] data-[disabled]:cursor-not-allowed data-[disabled]:opacity-50"
        >
          <Switch.Thumb className="block h-[14px] w-[14px] translate-x-[1px] rounded-full bg-white shadow-sm transition-transform data-[checked]:translate-x-[15px]" />
        </Switch.Root>
      </div>
    </label>
  )
}

/**
 * A row with no install/uninstall action — status is reported, not toggled
 * (e.g. agent-browser, which is bundled and configured automatically; see
 * D3, issue #318). Shares the Row shell's visuals without the switch or the
 * click-to-select affordance.
 */
function StatusRow({
  title,
  description,
  status,
}: {
  title: string
  description: string
  status: OnboardingComponentStatus
}) {
  // No install/uninstall progress for a status-only row — synthesize the
  // 'idle' snapshot rowBadgeState needs so both rows share one badge mapper.
  const badgeState = rowBadgeState({ status, progress: 'idle', selected: false })
  return (
    <div className="flex items-start gap-3 rounded-[8px] border border-[var(--surface-card-border)] bg-[var(--surface-card)] px-4 py-3 text-left select-none">
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="text-[13px] font-medium">{title}</span>
          <Badge state={badgeState} />
        </div>
        <p className="mt-1 text-[12px] leading-snug text-[var(--surface-toolbar-foreground)] opacity-70">
          {description}
        </p>
        <StatusDetail status={status} />
      </div>
    </div>
  )
}

type BadgeState = 'installing' | 'installed' | 'blocked' | 'failed' | 'outdated' | 'not-installed'

function rowBadgeState(snapshot: InstallerRowSnapshot): BadgeState {
  if (snapshot.progress === 'installing') return 'installing'
  if (snapshot.progress === 'success' || snapshot.status.kind === 'installed') return 'installed'
  if (snapshot.progress === 'error') return 'failed'
  if (snapshot.status.kind === 'blocked') return 'blocked'
  if (snapshot.status.kind === 'outdated') return 'outdated'
  return 'not-installed'
}

function Badge({ state }: { state: BadgeState }) {
  if (state === 'installing') {
    return (
      <span className="flex shrink-0 items-center gap-1 text-[11px] font-medium text-[var(--surface-toolbar-foreground)] opacity-80">
        <Loader2 size={12} className="animate-spin" />
        Installing…
      </span>
    )
  }
  if (state === 'installed') {
    return (
      <span className="flex shrink-0 items-center gap-1 text-[11px] font-medium text-[var(--surface-toolbar-foreground)]">
        <Check size={12} strokeWidth={3} />
        Installed
      </span>
    )
  }
  if (state === 'failed' || state === 'blocked') {
    return (
      <span className="flex shrink-0 items-center gap-1 text-[11px] font-medium text-red-600 dark:text-red-400">
        <CircleAlert size={12} />
        {state === 'failed' ? 'Failed' : 'Blocked'}
      </span>
    )
  }
  if (state === 'outdated') {
    return (
      <span className="flex shrink-0 items-center gap-1 text-[11px] font-medium text-amber-600 dark:text-amber-400">
        Update available
      </span>
    )
  }
  return (
    <span className="flex shrink-0 items-center gap-1 text-[11px] text-[var(--surface-toolbar-foreground)] opacity-50">
      <Minus size={12} />
      Not installed
    </span>
  )
}

function RowDetail({ snapshot }: { snapshot: InstallerRowSnapshot }) {
  const text =
    snapshot.progress === 'error' || snapshot.progress === 'success'
      ? snapshot.progressDetail
      : snapshot.status.kind === 'installed' ||
          snapshot.status.kind === 'outdated' ||
          snapshot.status.kind === 'blocked'
        ? snapshot.status.detail
        : undefined
  if (!text) return null
  const cls =
    snapshot.progress === 'error' || snapshot.status.kind === 'blocked'
      ? 'mt-1 text-[11px] text-red-600 dark:text-red-400'
      : 'mt-1 text-[11px] text-[var(--surface-toolbar-foreground)] opacity-60'
  return <p className={cls}>{text}</p>
}

function StatusDetail({ status }: { status: OnboardingComponentStatus }) {
  const text =
    status.kind === 'installed' || status.kind === 'outdated' || status.kind === 'blocked'
      ? status.detail
      : undefined
  if (!text) return null
  const cls =
    status.kind === 'blocked'
      ? 'mt-1 text-[11px] text-red-600 dark:text-red-400'
      : 'mt-1 text-[11px] text-[var(--surface-toolbar-foreground)] opacity-60'
  return <p className={cls}>{text}</p>
}

export const SkillInstaller = { Root, Row, StatusRow }
