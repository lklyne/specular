import { useMemo, useState } from 'react'
import { Cloud, Link2, RotateCw, X } from 'lucide-react'
import type { ShareScope } from '../../shared/share'
import { PresetPopover, toolbarStripAnchor } from '../shared/PresetPopover'
import { useShareState } from './useShareState'

const SCOPES: readonly ShareScope[] = ['view', 'comment', 'edit']
const SCOPE_LABEL: Record<ShareScope, string> = {
  view: 'Can view',
  comment: 'Can comment',
  edit: 'Can edit',
}

function iconBtnClass(isDark: boolean): string {
  return isDark
    ? 'toolbar-squircle-btn rounded-[8px] border border-transparent bg-transparent p-1.5 text-zinc-300 hover:bg-[var(--surface-interactive-hover)] hover:text-zinc-100 active:bg-[var(--surface-interactive)]'
    : 'toolbar-squircle-btn rounded-[8px] border border-transparent bg-transparent p-1.5 text-zinc-600 hover:bg-[var(--surface-interactive-hover)] hover:text-zinc-900 active:bg-[var(--surface-interactive)]'
}

/**
 * Toolbar-anchored share surface (ADR 0018 §4b), dev-flagged. Renders nothing
 * unless the `cloudShare` flag is on. The button doubles as the shared-state
 * indicator (a filled cloud once the canvas is published and connected); the
 * popover holds the whole surface — scope dropdown, Copy link, and the active
 * links list with reset/revoke.
 */
export function ShareButton({ isDark }: { isDark: boolean }) {
  const { enabled, state, busy, refresh, copyLink, resetLink, revokeLink } = useShareState()
  const [open, setOpen] = useState(false)
  const [scope, setScope] = useState<ShareScope>('comment')
  const [copied, setCopied] = useState(false)
  const anchor = useMemo(() => toolbarStripAnchor('[data-share-anchor]'), [])

  if (!enabled) return null

  const shared = !!state?.binding
  const connected = state?.status === 'connected'
  const links = state?.links ?? []

  function handleOpenChange(next: boolean) {
    setOpen(next)
    setCopied(false)
    if (next) void refresh()
  }

  async function onCopy() {
    const url = await copyLink(scope)
    if (url) {
      setCopied(true)
      window.setTimeout(() => setCopied(false), 1500)
    }
  }

  const copyLabel = busy ? 'Syncing…' : copied ? 'Copied' : 'Copy link'

  const trigger = (
    <button
      type="button"
      data-share-anchor
      aria-label="Share"
      title="Share"
      className={iconBtnClass(isDark)}
    >
      <Cloud
        size={15}
        className={shared && connected ? '' : 'opacity-60'}
        fill={shared && connected ? 'currentColor' : 'none'}
      />
    </button>
  )

  return (
    <div className="[-webkit-app-region:no-drag]">
      <PresetPopover
        isDark={isDark}
        open={open}
        onOpenChange={handleOpenChange}
        anchor={anchor}
        align="end"
        sideOffset={8}
        trigger={trigger}
      >
        <div className="flex w-64 flex-col gap-2 p-1 text-xs">
          <div className="flex items-center gap-1.5">
            <select
              aria-label="Link access scope"
              value={scope}
              onChange={(e) => setScope(e.target.value as ShareScope)}
              className={`h-7 flex-1 rounded-[6px] border px-2 text-xs ${
                isDark
                  ? 'border-white/10 bg-transparent text-zinc-200'
                  : 'border-zinc-900/10 bg-transparent text-zinc-700'
              }`}
            >
              {SCOPES.map((s) => (
                <option key={s} value={s}>
                  {SCOPE_LABEL[s]}
                </option>
              ))}
            </select>
            <button
              type="button"
              onClick={onCopy}
              disabled={busy}
              className={`flex h-7 items-center gap-1.5 rounded-[6px] px-2.5 font-medium disabled:opacity-60 ${
                isDark
                  ? 'bg-zinc-100 text-zinc-900 hover:bg-white'
                  : 'bg-zinc-900 text-zinc-100 hover:bg-zinc-800'
              }`}
            >
              <Link2 size={13} />
              {copyLabel}
            </button>
          </div>

          {links.length > 0 ? (
            <div className="flex flex-col gap-0.5">
              {links.map((link) => (
                <div
                  key={link.grantId}
                  className="flex items-center justify-between gap-2 rounded-[6px] px-1.5 py-1"
                >
                  <span className="truncate">{SCOPE_LABEL[link.scope]}</span>
                  <div className="flex items-center gap-0.5">
                    <button
                      type="button"
                      aria-label={`Reset ${link.scope} link`}
                      title="Reset link"
                      onClick={() => void resetLink(link.grantId)}
                      className={iconBtnClass(isDark)}
                    >
                      <RotateCw size={13} />
                    </button>
                    <button
                      type="button"
                      aria-label={`Revoke ${link.scope} link`}
                      title="Revoke link"
                      onClick={() => void revokeLink(link.grantId)}
                      className={iconBtnClass(isDark)}
                    >
                      <X size={13} />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <p className={isDark ? 'px-1.5 text-zinc-400' : 'px-1.5 text-zinc-500'}>
              {shared ? 'No active links yet.' : 'Copy a link to start sharing this canvas.'}
            </p>
          )}
        </div>
      </PresetPopover>
    </div>
  )
}
