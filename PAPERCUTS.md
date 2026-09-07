# Papercuts

Small frictions hit while working in this repo — a tool call that missed, a
confusing setup step, a flaky command, a stale cache, a misleading error. None
of them are blocking on their own; logged together they show where the repo
needs sanding down.

Logged with `pnpm papercut -m <model> "message"`, or mined from a whole session
with `pnpm papercut:review`. Distinct from what an agent accomplished, and from
GitHub issues (real bugs / tracked work).

## 2026-08-05

- `claude-opus-5` — Ran a pnpm script in a fresh container → pnpm's verify-deps-before-run kicked off a full 'pnpm install' that died on a 403 fetching electron/node-gyp from codeload.github.com. Workaround: pnpm --config.verify-deps-before-run=false <script>.
