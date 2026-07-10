import { spawn } from 'child_process'
import { readFile, unlink } from 'fs/promises'
import { readFileSync } from 'fs'
import { join } from 'path'
import { callApp, sessionId, getClientName } from './app-client'

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

export const COMMAND_LABELS: Record<string, string> = {
  snapshot: 'inspect_page',
  click: 'click_target',
  fill: 'type_text',
  type: 'type_text',
  select: 'select_option',
  wait: 'wait_page',
  scroll: 'scroll_page',
  get: 'read_content',
  'query-elements': 'find_target',
  screenshot: 'take_screenshot',
  // Passthrough verbs (not specular-owned shortcuts) still drive presence so
  // the cursor doesn't go dark for the skill's documented passthrough
  // surface. There's no generic "interacting with page" key in the
  // PresenceLabelKey allowlist (src/shared/presence-label-keys.ts), so these
  // reuse the closest existing label rather than send a labelKey that
  // `coercePresenceLabelKey` would silently drop.
  eval: 'inspect_page',
  find: 'find_target',
  keyboard: 'type_text',
  focus: 'inspect_page',
  clipboard: 'read_content',
}

const VALUE_FLAGS = new Set([
  '--cdp', '--session', '--viewport', '--timeout', '--selector',
  '--format', '--depth', '--wait', '--attr',
  '--baseline', '--screenshot-format', '--screenshot-quality', '--screenshot-dir',
  '--max-output', '--download-path', '--executable-path', '--extension',
  '--headers', '--body', '--filter', '--profile', '--session-name',
  '--device', '--color-scheme', '--idle-timeout',
  '-s', '-d', '-p',
])

export const MUTATION_VERBS = new Set(['click', 'fill', 'type', 'select'])

export const GLOBAL_AB_FLAGS = ['--content-boundaries', '--max-output', '100000']

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

/**
 * Quote an argv token for re-joining into a shell-ish command string.
 *
 * `splitShellArgs` is the inverse: it strips quote chars while honoring them
 * as grouping delimiters. So to round-trip argv through a joined string
 * without losing whitespace/quote content (e.g. for `eval 'foo.bar("baz")'`),
 * every arg containing shell-significant chars must be re-quoted here first.
 */
export function shellQuote(arg: string): string {
  if (arg === '') return "''"
  if (/^[A-Za-z0-9_\-@.:/=+,]+$/.test(arg)) return arg
  return `'${arg.replace(/'/g, "'\\''")}'`
}

/**
 * Split a command string on unquoted `&&` into chained command segments.
 * Returns the original string as a single-element array when no unquoted
 * separator is present — e.g. `&&` inside an `eval 'a && b'` JS literal.
 */
export function splitChainedCommands(cmd: string): string[] {
  const out: string[] = []
  let current = ''
  let inDouble = false
  let inSingle = false
  let escaped = false
  for (let i = 0; i < cmd.length; i++) {
    const ch = cmd[i]
    if (escaped) { current += ch; escaped = false; continue }
    if (ch === '\\' && !inSingle) { current += ch; escaped = true; continue }
    if (ch === '"' && !inSingle) { current += ch; inDouble = !inDouble; continue }
    if (ch === "'" && !inDouble) { current += ch; inSingle = !inSingle; continue }
    if (!inDouble && !inSingle && ch === '&' && cmd[i + 1] === '&') {
      out.push(current.trim())
      current = ''
      i += 1
      continue
    }
    current += ch
  }
  const tail = current.trim()
  if (tail) out.push(tail)
  return out.length ? out : ['']
}

/** Split a command string into argv tokens, respecting quoted strings. */
export function splitShellArgs(cmd: string): string[] {
  const tokens: string[] = []
  let current = ''
  let inDouble = false
  let inSingle = false
  let escaped = false
  for (const ch of cmd.trim()) {
    if (escaped) { current += ch; escaped = false; continue }
    if (ch === '\\' && !inSingle) { escaped = true; continue }
    if (ch === '"' && !inSingle) { inDouble = !inDouble; continue }
    if (ch === "'" && !inDouble) { inSingle = !inSingle; continue }
    if (/\s/.test(ch) && !inDouble && !inSingle) {
      if (current) { tokens.push(current); current = '' }
      continue
    }
    current += ch
  }
  if (current) tokens.push(current)
  return tokens
}

export function parseCommandArgs(cmd: string): { argv: string[]; verb: string | null; ref: string | null } {
  const argv = splitShellArgs(cmd)
  let verb: string | null = null
  let ref: string | null = null
  let i = 0
  while (i < argv.length) {
    const arg = argv[i]
    if (VALUE_FLAGS.has(arg)) { i += 2; continue }
    if (arg.startsWith('-')) { i++; continue }
    if (!verb) { verb = arg; i++; continue }
    if (!ref && /^@e\d+$/.test(arg)) ref = arg
    i++
  }
  return { argv, verb, ref }
}

// ---------------------------------------------------------------------------
// CDP cache
// ---------------------------------------------------------------------------

const cdpUrlCache = new Map<string, { wsUrl: string; pageUrl: string; generation: number; expires: number }>()
const CDP_CACHE_TTL_MS = 60_000

interface CdpResolution {
  wsUrl: string
  /** The URL the page is expected to be showing. */
  pageUrl: string
  /** The page's navigation generation as of this resolution (see D8 below). */
  generation: number
}

async function resolveCdpUrl(pageId: string): Promise<CdpResolution> {
  const cached = cdpUrlCache.get(pageId)
  if (cached && cached.expires > Date.now()) {
    return { wsUrl: cached.wsUrl, pageUrl: cached.pageUrl, generation: cached.generation }
  }
  const result = await callApp<{ webSocketDebuggerUrl: string; url?: string; generation?: number }>(
    `/pages/${pageId}/cdp-target`,
  )
  const pageUrl = result.url ?? ''
  const generation = typeof result.generation === 'number' ? result.generation : 0
  cdpUrlCache.set(pageId, {
    wsUrl: result.webSocketDebuggerUrl,
    pageUrl,
    generation,
    expires: Date.now() + CDP_CACHE_TTL_MS,
  })
  return { wsUrl: result.webSocketDebuggerUrl, pageUrl, generation }
}

export function invalidateCdpCache(pageIds: string[]): void {
  for (const id of pageIds) cdpUrlCache.delete(id)
}

// ---------------------------------------------------------------------------
// D8 (issue #318): generation-based staleness detection — warn-only.
//
// Main bumps a per-page navigation generation on did-navigate/dom-ready.
// Snapshots record the generation they saw; a ref-based mutation issued
// against a page whose generation has since moved on gets a prepended
// warning. HMR partial updates never fire did-navigate, so the counter
// can't prove the DOM changed underneath a ref — it can only warn, never
// block.
//
// The snapshot-seen baseline lives on the main-process Page object
// (POST /pages/:id/snapshot-seen), not in module state here: every
// `specular` CLI invocation is a fresh short-lived process, so only the app
// outlives the snapshot→mutate loop the comparison spans. The baseline is
// per-page, not per-client — two agents driving the same page share it;
// acceptable for a warn-only heuristic.
// ---------------------------------------------------------------------------

/**
 * Record the generation an agent snapshot saw. Best-effort — a failed write
 * only means a later mutation can't warn; it never fails the snapshot.
 */
async function recordSnapshotGeneration(pageId: string, generation: number): Promise<void> {
  await callApp(`/pages/${pageId}/snapshot-seen`, {
    method: 'POST',
    body: JSON.stringify({ generation }),
  }).catch(() => {})
}

/**
 * Fetch the page's CURRENT navigation generation plus the snapshot-seen
 * baseline, bypassing `cdpUrlCache`. The cache has a 60s TTL, so a cached
 * read at mutation time would hide any navigation that happened within that
 * window — the fresh read is what makes the staleness check meaningful.
 * Failure-tolerant: a broken fetch just means the warning gets skipped,
 * never that the mutation fails.
 */
async function fetchFreshGeneration(
  pageId: string,
): Promise<{ generation: number; lastSnapshotGeneration: number | null } | null> {
  try {
    const result = await callApp<{ generation?: number; lastSnapshotGeneration?: number | null }>(
      `/pages/${pageId}/cdp-target`,
    )
    if (typeof result.generation !== 'number') return null
    return {
      generation: result.generation,
      lastSnapshotGeneration:
        typeof result.lastSnapshotGeneration === 'number' ? result.lastSnapshotGeneration : null,
    }
  } catch {
    return null
  }
}

/** Pure comparison — exported for unit coverage. */
export function staleGenerationWarning(
  pageId: string,
  seenGeneration: number,
  currentGeneration: number,
): string | null {
  if (currentGeneration <= seenGeneration) return null
  return (
    `page changed since your last snapshot — refs likely stale; ` +
    `re-run specular snapshot -i -f ${pageId} or target by text=/CSS selector`
  )
}

/**
 * Check if a snapshot/screenshot output references a page URL that doesn't
 * match the page's expected URL.  Returns a warning string, or null.
 */
function checkOriginMismatch(output: string, expectedPageUrl: string): string | null {
  if (!expectedPageUrl) return null
  // agent-browser annotates output with `origin=<url>`
  const originMatch = output.match(/origin=(\S+)/)
  if (!originMatch) return null
  const actualOrigin = originMatch[1]
  try {
    const expected = new URL(expectedPageUrl).origin
    const actual = new URL(actualOrigin).origin
    if (expected !== actual) {
      return `⚠ CDP target mismatch: expected ${expected} but connected to ${actual}. ` +
        `The page may not have loaded yet, or the webview resolved to a different target. ` +
        `Try re-running the command or use \`specular annotation <id>\` for annotation-based inspection.`
    }
  } catch {
    // URL parsing failed — skip the check
  }
  return null
}

/**
 * `@eN` refs come from a prior snapshot's accessibility tree and go stale the
 * moment the DOM changes underneath them (re-render, route change, list
 * reorder). A failed ref-targeted mutation is the first signal of that —
 * point the caller at how to recover instead of leaving a bare CLI error.
 */
function staleRefHint(targetPageId: string): string {
  return `refs may be stale — re-run specular snapshot -i -f ${targetPageId}, or target by text=/CSS selector (re-resolves every call)`
}

// ---------------------------------------------------------------------------
// Page lock
// ---------------------------------------------------------------------------

const pageLocks = new Map<string, Promise<void>>()

function withPageLock<T>(pageId: string, fn: () => Promise<T>): Promise<T> {
  const prev = pageLocks.get(pageId) ?? Promise.resolve()
  const next = prev.then(fn, fn)
  // Store the void chain so the next caller waits for this one
  pageLocks.set(pageId, next.then(() => {}, () => {}))
  return next
}

// ---------------------------------------------------------------------------
// Agent-browser binary resolution
// ---------------------------------------------------------------------------

export function resolveAgentBrowserPath(): string {
  if (process.env.AGENT_BROWSER_PATH) return process.env.AGENT_BROWSER_PATH
  const pathDirs = (process.env.PATH ?? '').split(':')
  for (const dir of pathDirs) {
    try {
      const candidate = join(dir, 'agent-browser')
      readFileSync(candidate)
      return candidate
    } catch { continue }
  }
  return 'agent-browser'
}

// ---------------------------------------------------------------------------
// Process spawning
// ---------------------------------------------------------------------------

export function spawnAsync(
  cmd: string,
  args: string[],
  opts: { timeout: number; input?: string; maxBuffer?: number; cwd?: string },
): Promise<{ stdout: string; stderr: string }> {
  return new Promise((resolve, reject) => {
    const child = spawn(cmd, args, {
      stdio: ['pipe', 'pipe', 'pipe'],
      cwd: opts.cwd,
      // Auto-shutdown per-page daemons after 60s of inactivity. We spin one
      // daemon per page (via --session <pageId>) so without an idle timeout
      // they'd accumulate for the app's lifetime. User override still wins.
      env: {
        AGENT_BROWSER_IDLE_TIMEOUT_MS: '60000',
        ...process.env,
        NO_COLOR: '1',
      },
    })
    const maxBuf = opts.maxBuffer ?? 10 * 1024 * 1024
    const stdoutChunks: Buffer[] = []
    const stderrChunks: Buffer[] = []
    let stdoutLen = 0
    let stderrLen = 0

    child.stdout.on('data', (chunk: Buffer) => {
      stdoutLen += chunk.length
      if (stdoutLen <= maxBuf) stdoutChunks.push(chunk)
    })
    child.stderr.on('data', (chunk: Buffer) => {
      stderrLen += chunk.length
      if (stderrLen <= maxBuf) stderrChunks.push(chunk)
    })

    const timer = setTimeout(() => {
      child.kill('SIGKILL')
      reject(new Error(`Command timed out after ${opts.timeout}ms`))
    }, opts.timeout)

    child.on('close', (code) => {
      clearTimeout(timer)
      const stdout = Buffer.concat(stdoutChunks).toString('utf-8')
      const stderr = Buffer.concat(stderrChunks).toString('utf-8')
      if (code !== 0) {
        reject(new Error(stderr || stdout || `Process exited with code ${code}`))
      } else {
        resolve({ stdout, stderr })
      }
    })
    child.on('error', (err) => {
      clearTimeout(timer)
      reject(err)
    })

    if (opts.input != null) {
      child.stdin.write(opts.input)
      child.stdin.end()
    } else {
      child.stdin.end()
    }
  })
}

// ---------------------------------------------------------------------------
// Browse tool handler
// ---------------------------------------------------------------------------

export async function handleBrowse(args: Record<string, unknown>): Promise<{
  content: Array<
    | { type: 'text'; text: string }
    | { type: 'image'; data: string; mimeType: string }
  >
}> {
  // Resolve page — default to selected page
  let pageId = args.page_id as string | undefined
  if (!pageId) {
    const sel = await callApp<{ selectedEntityId?: string; selectedEntityIds?: string[] }>('/selection')
    pageId = sel.selectedEntityId ?? sel.selectedEntityIds?.[0]
    if (!pageId) throw new Error('No page specified and nothing is selected.')
  }

  const rawCommand = (args.command as string).trim()
  const chainedParts = splitChainedCommands(rawCommand)
  const isChained = chainedParts.length > 1

  // Parse first command for presence animation
  const firstCmd = isChained ? chainedParts[0] : rawCommand
  const { verb, ref } = parseCommandArgs(firstCmd)
  const labelKey = verb ? COMMAND_LABELS[verb] ?? null : null

  const clientName = getClientName()

  return withPageLock(pageId, async () => {
    const { wsUrl: cdpUrl, pageUrl: expectedPageUrl, generation: resolvedGeneration } = await resolveCdpUrl(pageId)
    const abPath = resolveAgentBrowserPath()
    // One agent-browser daemon per page. Without --session, a single daemon
    // pins the first --cdp URL it saw and silently ignores subsequent --cdp
    // values — upstream bug in agent-browser (CLI skips `launch` when daemon
    // is already running; daemon's relaunch check doesn't compare cdp_url).
    // Keying by pageId sidesteps both gates.
    const sessionFlags = ['--session', pageId]

    // Fire presence intent (non-blocking). Include pageId so the cursor
    // follows the page we're actually driving — otherwise the server-side
    // fallback picks the first CDP proxy registration for this session and
    // the cursor sticks to whichever page was driven first.
    if (labelKey) {
      callApp('/session/presence/intent', {
        method: 'POST',
        body: JSON.stringify({
          sessionId,
          clientName,
          command: verb,
          labelKey,
          pageId,
          labelHint: verb === 'fill' || verb === 'type' ? 'editing control' : null,
          targetRef: ref,
          targetRefSource: ref ? 'agent-browser' : null,
        }),
      }).catch(() => {})
    }

    // Previously, each browse command sent eventType:'done' in a finally block,
    // which immediately killed the cursor after every CLI call. This made the
    // cursor flash briefly then disappear between calls while the LLM thinks.
    // Now we let the server-side 10s expiry handle cleanup instead.
    // If the cursor still feels too ephemeral, the next step is explicit
    // lifecycle commands (like the old POC's `presence start` / `presence done`)
    // that bracket a high-level task so the cursor persists with a 5-min TTL.

    try {

    if (isChained) {
      // ---- Chained commands: use batch --json --bail ----
      const parts = chainedParts
      // Auto-scroll refs into view before mutations. Ref-only: whether
      // agent-browser's scrollintoview accepts CSS/text selectors (not just
      // @eN refs) is unverified against the pinned binary, so selector
      // targets skip the pre-scroll rather than risk an unsupported call.
      const expanded: string[][] = []
      for (const p of parts) {
        const parsed = parseCommandArgs(p)
        if (parsed.verb && MUTATION_VERBS.has(parsed.verb) && parsed.ref) {
          expanded.push(['scrollintoview', parsed.ref])
        }
        expanded.push(splitShellArgs(p))
      }
      const batchInput = JSON.stringify(expanded)
      const hasWait = parts.some(p => parseCommandArgs(p).verb === 'wait')
      const timeoutMs = hasWait ? 60_000 : 30_000

      const { stdout } = await spawnAsync(
        abPath,
        [...GLOBAL_AB_FLAGS, ...sessionFlags, '--cdp', cdpUrl, 'batch', '--json', '--bail'],
        { timeout: timeoutMs, input: batchInput },
      )

      // Parse batch JSON results
      const results = JSON.parse(stdout) as Array<{
        command: string[]
        success: boolean
        error: string | null
        result: Record<string, unknown>
      }>

      const contentBlocks: Array<{ type: 'text'; text: string } | { type: 'image'; data: string; mimeType: string }> = []

      for (const entry of results) {
        if (!entry.success) {
          const entryVerb = entry.command[0]
          const hasRef = entry.command.some((tok) => /^@e\d+$/.test(tok))
          const hint = entryVerb && MUTATION_VERBS.has(entryVerb) && hasRef
            ? `\n${staleRefHint(pageId)}`
            : ''
          contentBlocks.push({ type: 'text', text: `> ${entry.command.join(' ')}\nError: ${entry.error}${hint}` })
          continue
        }

        const entryVerb = entry.command[0]

        // Screenshot: return as image content
        if (entryVerb === 'screenshot' && entry.result?.path) {
          try {
            const imgPath = entry.result.path as string
            const data = await readFile(imgPath)
            await unlink(imgPath).catch(() => {})
            const isJpeg = imgPath.endsWith('.jpg') || imgPath.endsWith('.jpeg')
            contentBlocks.push({
              type: 'image',
              data: data.toString('base64'),
              mimeType: isJpeg ? 'image/jpeg' : 'image/png',
            })
          } catch {
            contentBlocks.push({ type: 'text', text: `> ${entry.command.join(' ')}\n(screenshot file read failed)` })
          }
          continue
        }

        // Snapshot: use the snapshot text from structured result
        if (entryVerb === 'snapshot' && typeof entry.result?.snapshot === 'string') {
          const snapshotText = entry.result.snapshot as string
          const mismatch = checkOriginMismatch(snapshotText, expectedPageUrl)
          if (mismatch) contentBlocks.push({ type: 'text', text: mismatch })
          contentBlocks.push({ type: 'text', text: snapshotText })
          // D8: record the generation seen by this snapshot so a later
          // mutation against this page can compare against it. The chain
          // itself does not warn on its own ref-based mutation entries: a
          // fresh per-entry generation fetch would add a round trip to
          // every mutation inside an already-batched call, and a stale ref
          // inside one chain is already surfaced via staleRefHint on
          // failure.
          await recordSnapshotGeneration(pageId, resolvedGeneration)
          continue
        }

        // Other structured results: format key-value pairs
        const resultStr = Object.entries(entry.result ?? {})
          .map(([k, v]) => `${k}: ${typeof v === 'string' ? v : JSON.stringify(v)}`)
          .join('\n')
        if (resultStr) {
          contentBlocks.push({ type: 'text', text: resultStr })
        }
      }

      if (contentBlocks.length === 0) {
        contentBlocks.push({ type: 'text', text: '(no output)' })
      }
      return { content: contentBlocks }
    }

    // ---- Single command ----
    const { argv } = parseCommandArgs(rawCommand)
    const timeoutMs = verb === 'wait' ? 60_000 : 30_000

    // D8: for a mutation, check whether the page has navigated since the
    // last recorded agent snapshot. Fetched fresh (bypassing cdpUrlCache) so
    // a navigation inside the 60s cache window is still visible. No baseline
    // recorded yet (lastSnapshotGeneration null) means nothing to compare
    // against, so the warning is skipped rather than guessed at.
    let generationWarning: string | null = null
    if (verb && MUTATION_VERBS.has(verb)) {
      const fresh = await fetchFreshGeneration(pageId)
      if (fresh !== null && fresh.lastSnapshotGeneration !== null) {
        generationWarning = staleGenerationWarning(pageId, fresh.lastSnapshotGeneration, fresh.generation)
      }
    }

    // Auto-scroll ref into view before mutations (ref-only — see the chained
    // path above for why selector targets don't get this treatment).
    if (verb && MUTATION_VERBS.has(verb) && ref) {
      await spawnAsync(
        abPath,
        [...GLOBAL_AB_FLAGS, ...sessionFlags, '--cdp', cdpUrl, 'scrollintoview', ref],
        { timeout: 5_000 },
      ).catch(() => {}) // Best-effort — don't fail the click if scroll fails
    }

    // Use --json for screenshots to get structured path output
    const useJson = verb === 'screenshot'
    const extraFlags = useJson ? ['--json'] : []

    let stdout: string
    let stderr: string
    try {
      ;({ stdout, stderr } = await spawnAsync(
        abPath,
        [...GLOBAL_AB_FLAGS, ...sessionFlags, '--cdp', cdpUrl, ...extraFlags, ...argv],
        { timeout: timeoutMs },
      ))
    } catch (err) {
      // A ref-targeted mutation failure is most commonly a stale @eN ref —
      // surface the recovery path instead of a bare CLI error. D8's
      // generation warning is complementary: it fires on both success and
      // failure, since the mutation ran against a possibly-stale DOM either
      // way — never blocks the command, only adds context to the error.
      if (verb && MUTATION_VERBS.has(verb)) {
        const message = err instanceof Error ? err.message : String(err)
        const hint = ref ? `\n${staleRefHint(pageId)}` : ''
        const warningPrefix = generationWarning ? `${generationWarning}\n\n` : ''
        throw new Error(`${warningPrefix}${message}${hint}`)
      }
      throw err
    }

    // Screenshot: return image content
    if (verb === 'screenshot') {
      try {
        const parsed = JSON.parse(stdout)
        const imgPath = (parsed.data?.path ?? parsed.path) as string | undefined
        if (imgPath) {
          const data = await readFile(imgPath)
          await unlink(imgPath).catch(() => {})
          const isJpeg = imgPath.endsWith('.jpg') || imgPath.endsWith('.jpeg')
          return {
            content: [{ type: 'image' as const, data: data.toString('base64'), mimeType: isJpeg ? 'image/jpeg' : 'image/png' }],
          }
        }
      } catch {
        // Fall through to text output
      }
    }

    let output = (stdout + (stderr ? `\n${stderr}` : '')).trim()

    // Warn if the CDP target resolved to a different origin than expected
    if (verb === 'snapshot') {
      const mismatch = checkOriginMismatch(output, expectedPageUrl)
      if (mismatch) output = mismatch + '\n' + output
      // D8: record the generation this snapshot saw for later mutations
      // against this page to compare against.
      await recordSnapshotGeneration(pageId, resolvedGeneration)
    }

    // D8: prepend the staleness warning to a successful mutation's output too
    // — the mutation itself may have landed on the wrong element even though
    // the CLI call succeeded.
    if (generationWarning) output = `${generationWarning}\n\n${output}`

    // Auto-append URL after mutations
    if (verb && MUTATION_VERBS.has(verb)) {
      try {
        const { stdout: urlOut } = await spawnAsync(
          abPath,
          [...GLOBAL_AB_FLAGS, ...sessionFlags, '--cdp', cdpUrl, 'get', 'url'],
          { timeout: 5_000 },
        )
        output += `\nurl: ${urlOut.trim()}`
      } catch {
        // Best-effort — ignore failures
      }
    }

    const content: Array<{ type: 'text'; text: string } | { type: 'image'; data: string; mimeType: string }> =
      [{ type: 'text' as const, text: output || '(no output)' }]

    // --echo: re-snapshot after a successful mutation so the caller sees the
    // resulting DOM without a separate round trip. Only wired for the single
    // -command path — chained/batch calls ignore --echo.
    if (verb && MUTATION_VERBS.has(verb) && (args.echo as boolean | undefined)) {
      try {
        const { stdout: echoOut } = await spawnAsync(
          abPath,
          [...GLOBAL_AB_FLAGS, ...sessionFlags, '--cdp', cdpUrl, 'snapshot', '-i', '-c'],
          { timeout: 10_000 },
        )
        content.push({ type: 'text' as const, text: echoOut.trim() || '(no output)' })
      } catch {
        // Best-effort — the mutation already succeeded; don't fail the call for echo
      }
    }

    return { content }

    } finally {
      // no-op: let server-side expiry clean up the cursor
    }
  })
}
