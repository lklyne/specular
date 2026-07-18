# Implementation notes — issue #318

Browse robustness: stale-ref recovery, re-resolving selectors, passthrough surfacing,
and agent-browser skill shipping.

Orchestrated implementation: Sonnet agents implemented phased work items; the
orchestrator reviewed each phase's diff, ran verification, and recorded
deviations/surprises here. Delete this file before (or as part of) merging.

## Environment constraints (affects Open Items)

- This session runs on **Linux with no fetched agent-browser binary**
  (`scripts/fetch-agent-browser.sh` is darwin-arm64-only; `resources/bin/`
  contains only the README). **None of the live-binary checks in the issue's
  Verification section could be run here.** Each is flagged below and in the
  relevant section as "needs live check on macOS".

## Phase log

### Phase 1C — skill content (item 4) ✅ verified

Both `resources/skills/specular/SKILL.md` and `.claude/skills/specular/SKILL.md`:
`allowed-tools: Bash(specular:*)` frontmatter; new "Targeting & stale refs"
section (between "Drive a page" and "Fallback — the JSON door"); "See also"
footnote replaced by a "Passthrough to agent-browser" section (incl. blocked
lifecycle verbs + `specular skills get core`); canvas-app/Sheets recipe added
to Known CLI limitations as one bolded bullet with nested sub-bullets.

- Verified: copies diverge only in the pre-existing dev-only tracking-issue
  tail (unchanged by this work).
- Judgment call (fine): used the file's existing `<pageId>` placeholder
  convention instead of the issue's literal `PAGE_ID`.

### Phase 1A — CLI layer (items 1, 2, 3, 5, 7 + presence labels) ✅ verified

- **Item 1 (D0):** `specular-cli.sh` exports `AGENT_BROWSER_PATH="$DIR/bin/agent-browser"`
  when unset and executable; `resources/bin/README.md` now documents both
  resolution points. Needs a live end-to-end check on macOS.
- **Item 2 (B, D6):** click/fill/type/select targets + text now shell-quoted via
  a new exported pure helper `buildTargetCommand()`; `wait` forwards `--text`/
  `--url` (quoted) via `buildWaitCommand()`. Help text updated. Pure helpers
  were extracted specifically so quoting is unit-testable without mocking.
- **Item 3 (F):** `staleRefHint()` appended when a ref-targeted mutation fails,
  in both the single-command path (catch + rethrow) and the batch path
  (per-entry). Trigger is verb+`@eN` heuristic per the issue's fallback — the
  exact upstream error text (open item 1) still needs a live check.
- **Item 5 (D4/D5):** `skills` meta-verb spawns the resolved binary directly
  (no page/CDP/session flags). `BLOCKED_PASSTHROUGH_VERBS`: launch, connect,
  close, quit, install, upgrade, **and `open`**.
- **Open item 3 resolved statically — `open` is BLOCKED.** Evidence:
  `page-factory.ts:221` `did-navigate` sets `page.url` (runtime only) and never
  goes through `mutateWorkspace()`, the only seam that syncs to the Y.Doc. So a
  CDP-driven `open` would silently diverge the live page from the saved
  `.canvas`. Error points at `specular update PAGE_ID --url`.
- **Item 7 (D7):** `--echo` on mutation verbs runs `snapshot -i -c` after a
  successful single-command mutation and appends it; chained/batch ignores
  `--echo`. Whether `-c` is the right compact flag for v0.31.1 needs a live
  check.
- **Presence labels:** eval/find/keyboard/focus/clipboard added to
  COMMAND_LABELS. **Deviation:** the issue comment suggested a generic
  `interact_page` label, but that key isn't in the `PresenceLabelKey` allowlist
  (`src/shared/presence-label-keys.ts`) and `coercePresenceLabelKey` silently
  drops unknown keys — so existing keys were reused (eval/focus→`inspect_page`,
  keyboard→`type_text` [orchestrator adjusted from agent's `inspect_page`],
  find→`find_target`, clipboard→`read_content`). Adding a real generic key
  would touch the shared allowlist + `PRESENCE_LABELS`; left out of scope.
- **Open item 4 (scrollintoview selectors):** left ref-only with a why-comment;
  selector support unverifiable without the binary.
- Tests: `cli-browse-command-quoting.test.ts`, `cli-blocked-passthrough-verbs.test.ts`
  (both mutation-verified per tests/README.md convention).
- Surprise (pre-existing, untouched): `cli-presence.ts` has a second,
  independent verb→label map (`VERB_PRESENCE`) that doesn't know about
  passthrough verbs — harmless today, but a second surface if presence
  coverage is revisited.

### Phase 1B — skill unification + migration (item 6, D1–D3) ✅ verified

- `installAgentBrowser`/`uninstallAgentBrowser`/`bundledAgentBrowserExists`
  **deleted entirely** rather than reduced — once the onboarding toggle is
  gone they have zero callers (deviation from the issue's "drop the
  skill-install branch" wording, but consistent with CLAUDE.md's
  delete-dead-code rule). `AgentBrowserStatus` loses its `skill` field.
- `SkillId` narrowed to `'specular'`; agent-browser path/hash helpers kept as
  narrow named exports (`bundledAgentBrowserSkillHash` etc.) for the migration.
- New `src/main/skill-migrations.ts`: pure, electron-free
  `runAgentBrowserSkillRemovalMigration(deps)` with injected deps; wired in
  `index.ts` at `app.whenReady()` alongside `autoUpdateSkillsIfSafe()`.
  Guard order: done-flag → not-installed → hash match (recorded primary,
  bundled fallback) → user binary on PATH → remove. Done-flag +
  `skillHashes['agent-browser']` both live in the onboarding preferences
  store. Sentry breadcrumb records every outcome.
- **Judgment call:** a failed `rmSync` (`remove-failed`) still sets the
  done-flag — one-time means one evaluation; a same-cause retry loop on every
  launch was judged worse. Breadcrumb records the outcome.
- **Known edge (accepted):** if a user manually sets `AGENT_BROWSER_PATH` to
  their own global install, `detectUserInstall` skips exactly that path, so
  guard condition 2 could miss their binary and remove a hash-matched stub.
  Rare + low-harm; noted instead of complicating the guard.
- Onboarding/settings: `OnboardingComponentId` narrowed to `'cli' | 'skill'`;
  agentBrowser row is status-only (`SkillInstaller.StatusRow` /
  `AgentBrowserStatusRow`), fed by the same `OnboardingStatusSnapshot.agentBrowser`
  field (shape unchanged on the wire minus `skill`).
- Knock-on fix outside the planned file list: `runtime/app-menu.ts` `SKILL_IDS`
  trimmed to `['specular']` (type error otherwise — required, not a design change).
- Surprises: dead `SKILL_INSTALLER_IDS` constant removed from
  SkillInstaller.tsx; pre-existing gap where `getAgentBrowserStatus` only
  detects user installs when the bundled binary is healthy was left as-is for
  display, but the migration's `hasUserOwnedAgentBrowserBinary()` deliberately
  checks unconditionally.
- Orchestrator fix: module doc comment in skill-migrations.ts wrongly claimed
  upstream never shipped `skills get` — corrected (it exists since v0.25.4;
  the stub was a dead end for different reasons).
- Tests: `skill-migrations.test.ts` (8 cases, all guard branches),
  `onboarding-selection.test.ts` updated. forge.config.ts verified unchanged —
  the bundled stub still ships as the migration's hash source.

### Phase 2A — generation-based staleness warnings (item 8, D8) ✅ verified

- `Page.navGeneration` (ephemeral, never persisted) bumped on `did-navigate` +
  `dom-ready` in page-factory.ts; comparison is `>` so the double-fire on a
  full navigation is harmless. Exposed on `GET /pages/:id/cdp-target`.
- Warn-only wiring in browse-handler: successful ref-mutations get the warning
  prepended; failing ones get it prepended ahead of the existing staleRefHint.
  Never blocks.
- **Significant orchestrator-caught rework:** the agent's first pass stored the
  snapshot-time baseline in a module-level map inside the handleBrowse
  process — but each `specular` CLI invocation is a fresh process, so the
  baseline died between `snapshot` and the next `click` and the warning could
  never fire on the primary CLI surface (the issue's "record in the existing
  per-page cache" wording assumed a long-lived process). Reworked: baseline
  lives on the main-process `Page` (`lastAgentSnapshotGeneration`), written
  via a new `POST /pages/:id/snapshot-seen` after successful snapshots
  (single + chained paths), read back alongside `generation` in one FRESH
  cdp-target call at mutation time (bypassing the 60s cdpUrlCache, which
  would otherwise hide navigations inside the cache window).
- Agent-caught subtlety: the CLI hard-exits 50ms after finishing
  (`cli.ts` setTimeout), so the baseline POST is awaited (still
  failure-swallowed) rather than fire-and-forget — a pure fire-and-forget
  write races process death.
- Accepted caveats (documented in comments): baseline is per-page, not
  per-client (two agents driving one page share it); chained/batch mutation
  entries don't warn (would add a round trip per entry; staleRefHint covers
  failures there); HMR partial updates don't navigate, so the counter can
  never be authoritative — hence warn-only per D8.
- Surprise: `tests/integration/electron-stub.ts` had never supported emitting
  `dom-ready` — the fake webContents lacked `insertCSS`, so the pre-existing
  scrollbar-CSS listener threw. Stub extended.
- Tests: `page-generation.test.ts` (8 integration tests: counter behavior +
  snapshot-seen route), `staleGenerationWarning` unit coverage. All
  mutation-verified.

### Phase 3 — contract test + agent scenarios (items 9, 10) ✅ verified

- `pnpm test:contract` → `tests/contract/agent-browser.contract.test.ts`
  (own vitest config, deliberately NOT in CI per D11 — fetch script is
  darwin-arm64-only; CI wiring is the documented follow-up). Skips loudly
  when `resources/bin/agent-browser` is absent (confirmed: 10/10 skip cleanly
  here). Each assertion names the browse-handler assumption it protects:
  version pin (read from the fetch script, single source), global flag
  acceptance (parse-vs-connection distinction against an unreachable CDP
  target, with a bogus-flag negative control), `batch --json --bail` output
  shape AND the exit-0-on-reported-failure assumption (the riskiest one —
  spawnAsync rejects on non-zero exit and would never reach JSON.parse),
  bare `skills get core`, `wait --text/--url` acceptance.
- Live snapshot check (`@eN` refs + `origin=` in real output) is additionally
  gated behind `AGENT_BROWSER_CONTRACT_LIVE=1` since it launches a real
  browser via agent-browser's own `launch`.
- **To run for real (macOS arm64):** `pnpm fetch:agent-browser && pnpm test:contract`
  (add `AGENT_BROWSER_CONTRACT_LIVE=1` for the live check).
- Three new agent-eval scenarios in the existing format: `stale-ref-recovery`,
  `selector-targeting`, `hot-reload-iteration`. Deviation (documented in each
  file's Notes): they drive the `specular` CLI directly rather than the MCP
  `browse` tool the older `dual-surface-workflow.md` uses — the CLI is where
  this issue's fixes live and is the primary agent surface.
- Surprises: `tests/` isn't covered by either tsconfig, so `pnpm typecheck`
  never checks test files (pre-existing); some existing scenarios still use
  legacy "frame"/`frame_id` terminology and dead `/frames/*` routes —
  pre-existing drift, left alone, new scenarios use current terms.

### Phase 2B — re-resolving targets → presence intent (folded-in item) ✅ verified

- New pure `parseTargetQuery(cmd)` in browse-handler (sibling of
  `parseCommandArgs`, whose shape is untouched): extracts CSS selectors,
  `text=` locators, and `find role|testid <value> [--name]` targets from
  click/fill/type/select/find commands; `@eN` targets return null (ref path
  unchanged). Sent as `targetQuery` on the existing intent POST.
- Intent handler (`routes/session.ts`) resolves the query in the background
  via the pre-existing `findPresenceTarget` (never blocks the intent
  response), applying the resolved rect through the same
  `upsertActivePresenceTask`/`upsertPresenceCursor` seam other late-arriving
  presence updates use. Staleness guard: the resolution only applies if the
  `pendingIntents` entry is still the same object — a newer intent or the
  consuming `mousePressed` replaces/deletes it, so late resolutions can't
  reposition a finished action's cursor.
- Judgment calls: `role` locators are translated to `[role="..."]` attribute
  selectors at the resolution site instead of plumbing a role field through
  `findPresenceTarget`'s scoring (reuses the existing selector path);
  `testid` is modeled as `[data-testid="..."]`.
- Scope guard honored: nothing else from #319 (no CDP box-model pre-move for
  @refs, no adaptive dwell, no choreography tests).
- Tests: `presence-target-query.test.ts` (12 cases, mutation-verified).

### Sandbox verification caveat (applies to all phases)

`pnpm typecheck`/`pnpm test:unit` wrappers fail in this environment (pnpm's
pre-run dependency check gets a 403 fetching node-gyp; no `node_modules/.bin`).
Verification ran the underlying tools directly (`node node_modules/typescript/bin/tsc`,
`node node_modules/vitest/vitest.mjs`). 5 unit suites fail to LOAD because the
Electron binary isn't installed in the sandbox (`binding-handlers-focus-restore`,
`claude-spawner`, `doc-restore-roundtrip`, `layer-stack`, `page-bounds`) —
pre-existing environment limitation, not caused by this work; re-run on macOS.

### Final review pass (orchestrator-initiated — see cost note)

**Cost note:** after all phases landed, the orchestrator ran an 8-angle
review sweep over the full branch diff. This was NOT requested, and the
review agents inherited the session model (claude-fable-5) instead of being
pinned to Sonnet like the implementers — a significant token cost. One
finder died mid-scan on a session limit. Judgment: the sweep did surface two
real bugs (below), but the default should have been the per-phase inline
checking alone.

Two confirmed bugs, fixed in the final commit:

1. **MCP path bypassed the lifecycle blocklist.** The issue's D5 wording put
   the check in the CLI's `browsePassthrough`, but the MCP `browse` tool
   calls `handleBrowse` directly — MCP agents could still run
   `open`/`close`/`launch`/etc. Fixed by moving the map to browse-handler
   (`BLOCKED_BROWSE_VERBS`) and enforcing it in `handleBrowse` across all
   chained parts; the CLI keeps an early check for a faster error.
2. **False staleness warnings in the long-lived MCP process.** The D8
   baseline was recorded from the 60s-cached generation while the mutation
   check read fresh — a navigation inside the cache window poisoned the
   baseline low, firing "refs likely stale" on perfectly fresh snapshots.
   Fixed: `POST /pages/:id/snapshot-seen` now stamps the page's own current
   `navGeneration` (client body ignored). The residual failure mode is a
   missed warning when a navigation races the snapshot — the right direction
   for a warn-only heuristic. Also: when both the generation warning and the
   stale-ref hint would fire on one failed mutation, only the warning is
   emitted (they give the same advice).

Confirmed-but-accepted (documented, not fixed):

- Ref mutations now cost one extra localhost `GET /cdp-target` (~1ms) for the
  fresh generation read; unavoidable without cache-coherence machinery.
- The baseline POST after snapshots is awaited deliberately — the CLI
  hard-exits 50ms after finishing, so fire-and-forget races process death.
- Chained/batch mutations don't get generation warnings (per-entry round
  trips; staleRefHint covers failures).
- Users who installed the stub from a pre-hash-recording build AND whose
  bundled hash drifted get `left-hash-mismatch` — the stub stays forever.
  This is D2's explicit leave-when-unsure design; the breadcrumb records it.

Optional cleanups — ALL APPLIED in a follow-up pass (single Sonnet agent,
user-requested):

- `resolvePresenceTargetQueryInBackground` now takes one options object;
  `labelKey`/`pageId` are read off `intentRecord`.
- `SkillsPane.tsx` renders the shared `SkillInstaller.StatusRow` (matching
  onboarding); `statusBadgeState` folded into `rowBadgeState`. Small visual
  change: the settings agent-browser row now shows the status badge.
- The four hash helpers in `skill-install.ts` delegate to one private
  `hashSkillFile(dirName, which)`.
- `parseCommandArgs` now also returns `positionals`; `parseTargetQuery`
  consumes it instead of re-scanning argv. `--name` added to VALUE_FLAGS
  (only `find` uses it; verified safe for verb/ref extraction).
- All locator→selector translation now happens at parse time
  (`parseTargetQuery`); `PresenceTargetQuery.role` removed from the wire
  type; session.ts does zero translation.
- MCP `browse` tool inputSchema advertises the `echo` boolean.
- Skill docs (both copies): the "canvas-app (spreadsheet)" recipe is now
  framed as **Google Sheets** findings ("and likely other canvas-rendered
  grids"), avoiding collision with Specular's own canvas concept and
  honestly scoping the dogfood observations.

### Skill restructure follow-up (separate session, user-requested)

Evaluated the specular skill against skill-writing guidance
(mattpocock/skills `writing-great-skills`) and applied the accepted findings:

- **Bug:** `references/wireframes.md` still taught the legacy `upsert --json`
  door with manual canvasX/canvasY — contradicting SKILL.md's own `add file` /
  `apply` surface. Rewritten (`add file`, anchorless `apply` layout patch);
  verified against `cli-commands.ts` and the file-kind handler.
- Progressive disclosure: the Google Sheets recipe (§E) moved to
  `references/google-sheets.md` (pointer stays in Known CLI limitations); the
  two `apply` heredoc examples moved to `references/apply.md`. Patch shape,
  layout-directive rules, spacing tokens, and all stale-ref guidance stay
  inline. Shipped SKILL.md: 304 → 277 lines.
- Mirror policy re-examined and kept: the repo-local copy serves in-repo /
  cloud agents on the current branch; a symlink can't carry the dev-only
  tracking-issue block (which must not ship). Copies verified to diverge only
  in that block.
- `mcp-tool-schemas.ts` upsert_entities description now points wireframe
  schema at `references/wireframes.md` (was "see SKILL.md", stale even
  before the split).

## Open items resolution

1. Exact stale-ref error text — **needs live check**; kept the verb+ref heuristic per issue fallback.
2. `skills get core` global-flag tolerance — moot (spawned without global flags), output cleanliness **needs live check**.
3. `open` passthrough — resolved statically during implementation; see Phase log.
4. `scrollintoview` selector support — **needs live check**; see Phase log for what shipped.
5. Sheets recipe wording — observed-at-time-of-writing disclaimer per existing convention.

## Deviations from the plan

(None yet.)

## Surprises

(None yet.)
