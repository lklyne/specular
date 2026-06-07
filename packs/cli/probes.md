# CLI probes

Smoke-test-like tests that exercise the **real CLI binary** (not the HTTP client)
the way an agent would, then assert the charter's friction signals. A red probe is
a self-heal task (top priority). Writing a new probe that asserts a better
experience — and fixing the CLI until it's green — is how "looking for
improvements" happens concretely.

## Run

```
pnpm build:cli && pnpm test:smoke -- cli
```

## How they work

- They live in `tests/smoke/cli/*.probe.test.ts` and reuse the existing smoke
  harness: `tests/smoke/global-setup.ts` boots an isolated, throwaway Specular
  (temp user-data, private discovery file, random port) — never your real app.
- `cli-probe-utils.ts`'s `runCli()` points the built CLI at that instance via
  `SPECULAR_DISCOVERY_FILE` and returns `{ code, stdout, stderr, json }`.
- Assertions encode *agent-friendliness as fact*: parseable stdout, actionable
  stderr, non-zero exit on misuse, edits observable on read-back.

## Coverage today

- `canvas-workflow.probe.test.ts` — create page/note, read workspace back, a
  two-page + note workflow. Guards: parseable output, observable edits, clean exits.
- `error-ergonomics.probe.test.ts` — missing-arg usage messages, stderr/stdout
  separation. Guards: actionable, machine-legible failures.

## Gaps (candidate probes — convert friction here)

- `snapshot` / `screenshot` output shape and flags.
- `annotate` → `resolve` round-trip ergonomics.
- `group` → `auto-layout` → `focus` call count.
- `update` / `delete` with an unknown id: is the error actionable?
- `create page` with a bare path (`/garden`): should it be rejected with a message
  pointing at full-URL usage? (See backlog.md.)
