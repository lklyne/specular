import { Collapsible } from '@base-ui/react/collapsible'
import { Popover } from '@base-ui/react/popover'
import { ChevronDown, ChevronRight, FolderOpen, Loader2, Play, Settings, Zap } from 'lucide-react'
import { useMemo, useState } from 'react'
import { PRIMARY_BUTTON_CLASS } from '../../shared/primaryButton'
import type {
  Annotation,
  DevtoolsPanelPageSummary,
  FixConfig,
  FixModel,
  FixPermissions,
  FixProgressEntry,
  OriginBindings,
} from '../../../shared/types'
import { dividerClass, isUnresolved, mutedClass } from '../rightDetailsPanelHelpers'
import { usePaneTheme } from '../PaneContext'
import { groupAnnotationsByOrigin } from '../rightDetailsPanelSelectors'
import { rightDetailsPanelApi } from '../rightDetailsPanelApi'
import { useFocusedAnnotationScroll } from '../useFocusedAnnotationScroll'
import { CommentRow, CommentsPane } from './CommentsPane'
import { PaneHeader } from './PaneHeader'

export function DocumentPane({
  annotations,
  pages,
  focusedAnnotationId,
  annotateEnabled,
  annotateAvailable,
  originBindings,
  fixInProgress,
  fixProgress,
  fixConfig,
}: {
  annotations: Annotation[]
  pages: DevtoolsPanelPageSummary[]
  focusedAnnotationId?: string | null
  annotateEnabled: boolean
  annotateAvailable: boolean
  originBindings: OriginBindings
  fixInProgress: Record<string, number>
  fixProgress: Record<string, FixProgressEntry>
  fixConfig: FixConfig
}) {
  const isDark = usePaneTheme()
  const divider = dividerClass(isDark)
  const originGroups = useMemo(() => groupAnnotationsByOrigin(annotations), [annotations])

  return (
    <div className="flex h-full flex-col">
      <PaneHeader
        label="Document"
      />

      <div className="thin-scrollbar min-h-0 flex-1 overflow-auto">
      <Collapsible.Root defaultOpen>
        <div className="flex items-center">
          <Collapsible.Trigger
            className="group flex flex-1 items-center gap-1.5 px-3 py-2 text-[12px] font-medium"
          >
            Comments
            <ChevronRight size={10} className="transition-all ease-out group-data-[panel-open]:rotate-90" />
          </Collapsible.Trigger>
          {originGroups.length > 0 ? (
            <FixMenu
              isDark={isDark}
              originGroups={originGroups}
              originBindings={originBindings}
              fixInProgress={fixInProgress}
              fixConfig={fixConfig}
            />
          ) : null}
        </div>
        <Collapsible.Panel className="[&[hidden]:not([hidden='until-found'])]:hidden">
          <CommentsPane
            isDark={isDark}
            annotations={annotations}
            pages={pages}
            focusedAnnotationId={focusedAnnotationId}
            annotateEnabled={annotateEnabled}
            annotateAvailable={annotateAvailable}
            fixProgress={fixProgress}
          />
        </Collapsible.Panel>
      </Collapsible.Root>

      <ResolvedPane
        isDark={isDark}
        annotations={annotations}
        focusedAnnotationId={focusedAnnotationId}
        divider={divider}
        fixProgress={fixProgress}
      />
      </div>
    </div>
  )
}

function FixMenu({
  isDark,
  originGroups,
  originBindings,
  fixInProgress,
  fixConfig,
}: {
  isDark: boolean
  originGroups: { origin: string; unresolvedCount: number }[]
  originBindings: OriginBindings
  fixInProgress: Record<string, number>
  fixConfig: FixConfig
}) {
  const [showSettings, setShowSettings] = useState(!fixConfig.configured)
  const totalUnresolved = originGroups.reduce((s, g) => s + g.unresolvedCount, 0)
  const totalInFlight = Object.values(fixInProgress).reduce((s, n) => s + n, 0)
  const working = totalInFlight > 0
  const anyAutoFix = originGroups.some((g) => originBindings[g.origin]?.autoFix)
  const hasBinding = originGroups.some((g) => originBindings[g.origin])

  const faceLabel = anyAutoFix ? 'Auto Fix' : `Fix ${totalUnresolved}`
  const isPrimary = !anyAutoFix && totalUnresolved > 0

  const handleFixAll = () => {
    if (!hasBinding || totalUnresolved === 0 || working) return
    for (const group of originGroups) {
      if (originBindings[group.origin] && group.unresolvedCount > 0) {
        rightDetailsPanelApi.triggerFixComments(group.origin)
      }
    }
  }

  const borderClass = isPrimary
    ? 'border-transparent'
    : isDark ? 'border-zinc-600' : 'border-zinc-300'
  const primaryBtnClass = PRIMARY_BUTTON_CLASS
  const secondaryBtnClass = isDark
    ? 'bg-zinc-800 text-zinc-300 hover:bg-zinc-700'
    : 'bg-zinc-50 text-zinc-600 hover:bg-zinc-200'
  const btnClass = isPrimary ? primaryBtnClass : secondaryBtnClass

  const popupClass = `z-50 w-[256px] rounded-[10px] border shadow-xl outline-none ${
    isDark
      ? 'border-zinc-700 bg-zinc-900 text-zinc-100'
      : 'border-zinc-200 bg-white text-zinc-900'
  }`
  const dividerClass = isDark ? 'border-zinc-700' : 'border-zinc-200'
  const iconBtnClass = `rounded p-1 transition-colors ${
    isDark ? 'hover:bg-zinc-700' : 'hover:bg-zinc-100'
  }`
  const muted = isDark ? 'text-zinc-500' : 'text-zinc-400'

  return (
    <Popover.Root>
      <div className="mr-2 flex items-center">
        {/* Action face */}
        <button
          type="button"
          onClick={anyAutoFix ? undefined : handleFixAll}
          className={`inline-flex items-center gap-1.5 rounded-l-md border px-2 py-1 text-[11px] font-medium ${borderClass} ${btnClass}`}
        >
          {working ? (
            <Loader2 size={11} className="animate-spin shrink-0" />
          ) : anyAutoFix ? (
            <Zap size={11} className="shrink-0" />
          ) : (
            <Play size={11} className="shrink-0" />
          )}
          <span>{faceLabel}</span>
        </button>

        {/* Dropdown caret */}
        <Popover.Trigger
          render={
            <button
              type="button"
              className={`inline-flex items-center self-stretch rounded-r-md border border-l-0 px-1.5 ${borderClass} ${btnClass}`}
            />
          }
        >
          <ChevronDown size={10} />
        </Popover.Trigger>
      </div>

      <Popover.Portal>
        <Popover.Positioner sideOffset={4} align="end" collisionPadding={8}>
          <Popover.Popup className={popupClass}>
            {showSettings ? (
              <FixSettingsView
                isDark={isDark}
                fixConfig={fixConfig}
                dividerClass={dividerClass}
                muted={muted}
                onDone={() => setShowSettings(false)}
              />
            ) : (
              <FixOperationsView
                isDark={isDark}
                originGroups={originGroups}
                originBindings={originBindings}
                fixInProgress={fixInProgress}
                anyAutoFix={anyAutoFix}
                hasBinding={hasBinding}
                isPrimary={isPrimary}
                working={working}
                totalUnresolved={totalUnresolved}
                dividerClass={dividerClass}
                iconBtnClass={iconBtnClass}
                muted={muted}
                handleFixAll={handleFixAll}
                onOpenSettings={() => setShowSettings(true)}
              />
            )}
          </Popover.Popup>
        </Popover.Positioner>
      </Popover.Portal>
    </Popover.Root>
  )
}

function FixSettingsView({
  isDark,
  fixConfig,
  dividerClass,
  muted,
  onDone,
}: {
  isDark: boolean
  fixConfig: FixConfig
  dividerClass: string
  muted: string
  onDone: () => void
}) {
  const [model, setModel] = useState<FixModel>(fixConfig.model)
  const [permissions, setPermissions] = useState<FixPermissions>(fixConfig.permissions)

  const selectClass = `w-full rounded-md border px-2 py-1.5 text-[12px] ${
    isDark
      ? 'border-zinc-600 bg-zinc-800 text-zinc-100'
      : 'border-zinc-300 bg-white text-zinc-900'
  }`

  const handleConfirm = () => {
    rightDetailsPanelApi.setFixConfig({ model, permissions })
    onDone()
  }

  return (
    <div className="w-full">
      <div className={`px-3 py-2.5 text-[12px] leading-relaxed text-pretty ${isDark ? 'text-zinc-300' : 'text-zinc-600'}`}>
        Fix uses Claude Code to read your comments and make changes in linked repositories.
        Pick a model and permission level below.
      </div>

      <div className={`space-y-3 border-t px-3 py-3 ${dividerClass}`}>
        <div>
          <label className="mb-1 block text-[11px] font-medium">Model</label>
          <select
            value={model}
            onChange={(e) => setModel(e.target.value as FixModel)}
            className={selectClass}
          >
            <option value="opus">Opus</option>
            <option value="sonnet">Sonnet</option>
            <option value="haiku">Haiku</option>
          </select>
        </div>

        <div>
          <label className="mb-1 block text-[11px] font-medium">Permissions</label>
          <select
            value={permissions}
            onChange={(e) => setPermissions(e.target.value as FixPermissions)}
            className={selectClass}
          >
            <option value="dangerously">Bypass permissions</option>
            <option value="default">Default (approve each tool)</option>
          </select>
          {permissions === 'dangerously' ? (
            <p className={`mt-1 text-[10px] leading-snug ${muted}`}>
              Claude will read and write files without asking. Only use this on repos you trust.
            </p>
          ) : null}
        </div>
      </div>

      <div className={`border-t px-3 py-2.5 ${dividerClass}`}>
        <button
          type="button"
          onClick={handleConfirm}
          className={`w-full rounded-md border px-3 py-1.5 text-[12px] font-medium ${
            isDark
              ? 'border-zinc-600 bg-zinc-800 text-zinc-200 hover:bg-zinc-700'
              : 'border-zinc-300 bg-white text-zinc-700 hover:bg-zinc-100'
          }`}
        >
          {fixConfig.configured ? 'Save' : 'Get started'}
        </button>
      </div>
    </div>
  )
}

function FixOperationsView({
  isDark,
  originGroups,
  originBindings,
  fixInProgress,
  anyAutoFix,
  hasBinding,
  isPrimary,
  working,
  totalUnresolved,
  dividerClass,
  iconBtnClass,
  muted,
  handleFixAll,
  onOpenSettings,
}: {
  isDark: boolean
  originGroups: { origin: string; unresolvedCount: number }[]
  originBindings: OriginBindings
  fixInProgress: Record<string, number>
  anyAutoFix: boolean
  hasBinding: boolean
  isPrimary: boolean
  working: boolean
  totalUnresolved: number
  dividerClass: string
  iconBtnClass: string
  muted: string
  handleFixAll: () => void
  onOpenSettings: () => void
}) {
  return (
    <>
      {/* Global auto / fix now toggle */}
      <div className={`flex items-center gap-2 border-b px-3 py-2 ${dividerClass}`}>
        <button
          type="button"
          onClick={() => {
            for (const group of originGroups) {
              if (originBindings[group.origin]) {
                rightDetailsPanelApi.setAutoFix(group.origin, !anyAutoFix)
              }
            }
          }}
          className={`inline-flex items-center gap-1.5 rounded-md px-2 py-1 text-[11px] font-medium transition-colors ${
            anyAutoFix
              ? isDark ? 'bg-emerald-600/80 text-white' : 'bg-emerald-50 text-emerald-700'
              : isDark ? 'bg-zinc-800 text-zinc-300 hover:bg-zinc-700' : 'bg-zinc-100 text-zinc-600 hover:bg-zinc-200'
          }`}
        >
          <Zap size={11} className="shrink-0" />
          Auto
        </button>
        <button
          type="button"
          onClick={handleFixAll}
          disabled={!hasBinding || totalUnresolved === 0 || working}
          className={`inline-flex items-center gap-1.5 rounded-md px-2 py-1 text-[11px] font-medium transition-colors disabled:opacity-40 ${
            isPrimary
              ? PRIMARY_BUTTON_CLASS
              : isDark ? 'bg-zinc-800 text-zinc-300 hover:bg-zinc-700' : 'bg-zinc-100 text-zinc-600 hover:bg-zinc-200'
          }`}
        >
          {working ? <Loader2 size={11} className="animate-spin shrink-0" /> : <Play size={11} className="shrink-0" />}
          Fix all
        </button>
        <button
          type="button"
          onClick={onOpenSettings}
          className={`ml-auto ${iconBtnClass} ${muted}`}
          title="Fix settings"
        >
          <Settings size={13} />
        </button>
      </div>

      {/* Origin rows */}
      {originGroups.map((group, i) => {
        const binding = originBindings[group.origin]
        const port = `:${new URL(group.origin).port || '80'}`
        const repoLabel = binding
          ? binding.repoPath.replace(/^\/Users\/[^/]+/, '~')
          : 'Link repo…'
        const isAuto = binding?.autoFix ?? false
        const inFlight = (fixInProgress[group.origin] ?? 0) > 0

        return (
          <div key={group.origin}>
            {i > 0 ? <div className={`border-t ${dividerClass}`} /> : null}
            <div className="flex items-center gap-2 px-3 py-1.5">
              <span className={`shrink-0 text-[11px] font-mono ${muted}`}>{port}</span>
              <button
                type="button"
                onClick={() => rightDetailsPanelApi.pickRepoForOrigin(group.origin)}
                className={`min-w-0 flex-1 truncate text-left text-[11px] transition-colors ${
                  binding
                    ? isDark ? 'text-zinc-300 hover:text-white' : 'text-zinc-700 hover:text-zinc-900'
                    : isDark ? 'text-zinc-500 hover:text-zinc-300' : 'text-zinc-400 hover:text-zinc-600'
                }`}
                title={binding ? binding.repoPath : 'Choose a repo folder…'}
              >
                {binding ? repoLabel : (
                  <span className="inline-flex items-center gap-1">
                    <FolderOpen size={11} className="shrink-0" />
                    Link repo…
                  </span>
                )}
              </button>
              {binding ? (
                <>
                  <button
                    type="button"
                    onClick={() => rightDetailsPanelApi.setAutoFix(group.origin, !isAuto)}
                    className={`${iconBtnClass} ${isAuto ? (isDark ? 'text-emerald-400' : 'text-emerald-600') : muted}`}
                    title={isAuto ? 'Auto-fix on' : 'Auto-fix off'}
                  >
                    <Zap size={13} />
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      if (group.unresolvedCount > 0 && !inFlight) {
                        rightDetailsPanelApi.triggerFixComments(group.origin)
                      }
                    }}
                    disabled={group.unresolvedCount === 0 || inFlight}
                    className={`${iconBtnClass} disabled:opacity-30`}
                    title="Fix comments"
                  >
                    {inFlight ? <Loader2 size={13} className="animate-spin" /> : <Play size={13} />}
                  </button>
                </>
              ) : null}
              <span
                className={`ml-1 inline-flex min-w-[20px] shrink-0 items-center justify-center rounded px-1.5 py-0.5 text-[11px] tabular-nums ${
                  isDark ? 'bg-zinc-800 text-zinc-400' : 'bg-zinc-100 text-zinc-500'
                }`}
              >
                {group.unresolvedCount}
              </span>
            </div>
          </div>
        )
      })}
    </>
  )
}

function ResolvedPane({
  isDark,
  annotations,
  focusedAnnotationId,
  divider,
  fixProgress,
}: {
  isDark: boolean
  annotations: Annotation[]
  focusedAnnotationId?: string | null
  divider: string
  fixProgress: Record<string, FixProgressEntry>
}) {
  const resolved = annotations.filter((a) => !isUnresolved(a.status))
  if (!resolved.length) return null
  resolved.sort((a, b) => Date.parse(b.createdAt) - Date.parse(a.createdAt))

  const muted = mutedClass(isDark)
  const bubbleClass = isDark ? 'bg-zinc-800/45' : 'bg-zinc-100/85'
  const rowHoverClass = isDark ? 'hover:bg-zinc-700/55' : 'hover:border-zinc-300'
  const focusRowClass = isDark ? 'bg-blue-500/20' : 'bg-blue-500/10'
  const { registerAnnotationElement } = useFocusedAnnotationScroll(
    focusedAnnotationId,
    annotations,
  )

  return (
    <Collapsible.Root defaultOpen={false}>
      <Collapsible.Trigger
        className={`group flex w-full items-center gap-1.5 border-t px-3 py-2 text-[12px] font-medium ${divider}`}
      >
        Resolved ({resolved.length})
        <ChevronRight size={10} className="transition-all ease-out group-data-[panel-open]:rotate-90" />
      </Collapsible.Trigger>
      <Collapsible.Panel className="h-[var(--collapsible-panel-height)] overflow-hidden transition-all ease-out data-[ending-style]:h-0 data-[starting-style]:h-0 duration-150 [&[hidden]:not([hidden='until-found'])]:hidden">
        <section className="space-y-1 px-2 pt-1 pb-3">
          {resolved.map((annotation) => (
            <CommentRow
              key={annotation.id}
              annotation={annotation}
              isDark={isDark}
              mutedClass={muted}
              rowHoverClass={rowHoverClass}
              focusRowClass={focusRowClass}
              focusedAnnotationId={focusedAnnotationId}
              registerAnnotationElement={registerAnnotationElement}
              progress={fixProgress[annotation.id]}
            />
          ))}
        </section>
      </Collapsible.Panel>
    </Collapsible.Root>
  )
}
