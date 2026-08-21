import { useState } from 'react'
import { Plus, Trash2, X, Zap } from 'lucide-react'
import type { ConnectedRepo, RepoStatus } from '../../shared/types'
import type { SettingsElectronAPI } from '../../shared/electron-api/settings'

const STATUS_DOT: Record<RepoStatus, string> = {
  stopped: 'bg-zinc-400',
  starting: 'bg-amber-400',
  running: 'bg-emerald-500',
  errored: 'bg-red-500',
}

const STATUS_COPY: Record<RepoStatus, string> = {
  stopped: 'idle',
  starting: 'starting',
  running: 'running',
  errored: 'error',
}

export function ReposPane({
  api,
  connectedRepos,
}: {
  api: SettingsElectronAPI
  connectedRepos: ConnectedRepo[]
}) {
  return (
    <section>
      <header className="mb-4 mt-2">
        <p className="mt-1 text-[12px] leading-snug text-[var(--surface-toolbar-foreground)] opacity-70">
          Connect a local folder so Specular can run its dev server and apply in-place fixes from comments. Bind sites to a repo from the details panel while viewing a page.
        </p>
      </header>

      <ul className="flex flex-col gap-2">
        <li>
          <button
            type="button"
            onClick={() => api.repoConnectViaPicker()}
            className="flex w-full items-center gap-2 rounded-[8px] border border-dashed border-[var(--surface-popover-border)] px-3 py-[10px] text-[13px] text-[var(--surface-toolbar-foreground)] opacity-70 hover:opacity-100"
          >
            <Plus size={14} />
            <span>connect repo…</span>
          </button>
        </li>
        {connectedRepos.map((repo) => (
          <li
            key={repo.id}
            className="rounded-[8px] border border-[var(--surface-popover-border)] bg-[var(--surface-popover-subtle)] px-3 py-[10px]"
          >
            <div className="flex items-start gap-3">
              <span
                className={`mt-[6px] inline-flex h-1.5 w-1.5 shrink-0 rounded-full ${STATUS_DOT[repo.status]}`}
                title={repo.lastError ?? STATUS_COPY[repo.status]}
              />
              <div className="flex-1 min-w-0">
                <div className="truncate text-[13px] font-medium">{repo.label}</div>
                <div className="mt-[2px] truncate font-mono text-[11px] text-[var(--surface-toolbar-foreground)] opacity-70">
                  {repo.baseUrl ?? repo.absolutePath}
                </div>
              </div>
              <button
                type="button"
                aria-label={`Disconnect ${repo.label}`}
                onClick={() => api.repoDisconnect(repo.id)}
                className="shrink-0 rounded-[4px] p-1 text-[var(--surface-toolbar-foreground)] opacity-60 hover:bg-[var(--surface-popover-border)] hover:opacity-100"
              >
                <Trash2 size={14} />
              </button>
            </div>

            <div className="mt-2 flex flex-col gap-1 border-t border-[var(--surface-popover-border)] pt-2">
              {repo.boundOrigins.map((b) => (
                <div
                  key={b.origin}
                  className="flex items-center gap-2 text-[11px]"
                >
                  <span className="truncate font-mono text-[var(--surface-toolbar-foreground)] opacity-80">
                    {b.origin}
                  </span>
                  {b.autoFix ? (
                    <span
                      className="inline-flex items-center gap-1 rounded-[4px] bg-emerald-500/15 px-1.5 py-[1px] text-[10px] uppercase tracking-wide text-emerald-600 dark:text-emerald-400"
                      title="Auto-fix on for this origin"
                    >
                      <Zap size={9} />
                      auto
                    </span>
                  ) : null}
                  <button
                    type="button"
                    aria-label={`Remove binding for ${b.origin}`}
                    onClick={() => api.removeOriginBinding(b.origin)}
                    className="ml-auto shrink-0 rounded-[4px] p-0.5 text-[var(--surface-toolbar-foreground)] opacity-50 hover:bg-[var(--surface-popover-border)] hover:opacity-100"
                  >
                    <X size={11} />
                  </button>
                </div>
              ))}
              <AddOriginRow
                onSubmit={(origin) => api.repoBindOrigin(repo.id, origin)}
              />
            </div>
          </li>
        ))}
      </ul>
    </section>
  )
}

function AddOriginRow({
  onSubmit,
}: {
  onSubmit: (origin: string) => void
}) {
  const [value, setValue] = useState('')
  const submit = () => {
    const trimmed = value.trim()
    if (!trimmed) return
    onSubmit(trimmed)
    setValue('')
  }
  return (
    <div className="flex items-center gap-1.5">
      <input
        type="text"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault()
            submit()
          }
        }}
        placeholder="add a url…"
        spellCheck={false}
        className="min-w-0 flex-1 rounded-[4px] border border-transparent bg-transparent px-1 py-[2px] font-mono text-[11px] text-[var(--surface-toolbar-foreground)] placeholder:text-[var(--surface-toolbar-foreground)] placeholder:opacity-40 focus:border-[var(--surface-popover-border)] focus:outline-none"
      />
      <button
        type="button"
        onClick={submit}
        disabled={value.trim().length === 0}
        aria-label="Bind url"
        className="shrink-0 rounded-[4px] p-0.5 text-[var(--surface-toolbar-foreground)] opacity-50 hover:bg-[var(--surface-popover-border)] hover:opacity-100 disabled:pointer-events-none disabled:opacity-20"
      >
        <Plus size={11} />
      </button>
    </div>
  )
}
