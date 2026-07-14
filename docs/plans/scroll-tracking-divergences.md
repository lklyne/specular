# Scroll tracking — divergences from the plan

Running log of where the implementation departs from `docs/plans/scroll-tracking.md`.
Each entry: what the plan said, what we did instead, and why.

## Status

- [ ] Phase 1 — Broadcast the page's absolute scroll offset
- [ ] Phase 2 — The transform learns about scroll
- [ ] Phase 3 — The document-anchored region variant
- [ ] Phase 4 — Clicking a comment scrolls its page to it
- [ ] Docs — ADR 0029 amend, CONTEXT.md, file-formats.md

## Environment / verification baseline (pre-change)

- `node_modules/.bin` is empty (hoisted-linker quirk) and `pnpm <script>`
  triggers a deps-status `pnpm install` that fails on an egress-blocked
  `codeload.github.com` 403. **Workaround:** invoke binaries via node directly
  — `node node_modules/typescript/bin/tsc --noEmit -p tsconfig.node.json`,
  `node node_modules/vitest/vitest.mjs run --config vitest.<x>.config.ts`.
- Electron's binary download is egress-blocked, so `node_modules/electron` has
  no `path.txt`/`dist`. **5 unit suites fail** for this reason alone
  (`binding-handlers-focus-restore`, `claude-spawner`, `doc-restore-roundtrip`,
  `layer-stack`, `page-bounds` — all import real electron). Unrelated to this
  feature; recorded so a later reviewer doesn't attribute them to scroll work.
- **Green baseline confirmed:** node + web typecheck clean; unit 819 tests pass
  (79/84 suites; 5 electron-blocked); integration 135 tests pass (23/23 suites).
  Integration uses `tests/integration/electron-stub.ts`, so it is unaffected.

## Divergences

_(none yet)_
