# CLI self-heal backlog

The loop's memory. A fresh fire reads this, does the top `## Todo` item (unless a
probe is red — then it heals that first), then moves it to `## Done` with a date +
commit sha and appends any new friction it found. Git history of this file is the
self-heal record.

Top of `## Todo` = next up. Keep items small and concrete.

## Todo

- [ ] **`specular dev restart` is unreliable.** A clean restart is hard: sticky
      single-instance lock, stale `~/.specular/specular-mcp.json` after SIGKILL,
      orphaned Vite children, port contention; `scripts/dev-restart.sh` is
      macOS-only. Add an idempotent, cross-platform restart path (kill process
      tree → wait for port free → remove stale discovery file → relaunch → poll
      `/health`). This is the highest-leverage agent-friendliness fix. Write a
      probe where feasible (may need to assert on the restart script's behavior
      rather than via the smoke app).

- [ ] **`create page` accepts bare paths silently.** CLAUDE.md and the skill say
      always pass full URLs (a bare `/garden` is ambiguous across origins), but
      `src/main/cli-commands.ts` passes the value straight through. Write a probe
      asserting `specular create page /garden` exits non-zero with a message
      pointing at full-URL usage, then make it pass. The same applies to
      `specular breakpoints <url>` (also takes a URL).

- [ ] **Skill drift: `breakpoints` argument.** `.claude/skills/specular/SKILL.md`
      documents `specular breakpoints <id>` / "cycle through device breakpoints,"
      but the CLI takes a URL and applies a breakpoint *map*
      (`cli-commands.ts` `breakpoints` handler: `usage: specular breakpoints <url>`).
      Fix the skill (both copies) to match; verify against the actual handler.

- [ ] **Audit the CLI docs against the command surface.** Diff CLAUDE.md
      `## Specular CLI` + `## Agent integration` and both `SKILL.md` copies against
      the `VERBS` map and per-verb usage strings in `cli-commands.ts`. File each
      drift (missing/renamed command, wrong flag or argument, stale guidance) as its
      own Todo here. Recurring: re-run after CLI changes land.

- [ ] **Write a doc-truth probe.** Add a probe under `tests/smoke/cli/` that asserts
      the documented command list and the CLI's real verbs agree — e.g. parse the
      command table in `.claude/skills/specular/SKILL.md`, run `specular --help`
      (and/or read the `VERBS` map), and fail if a documented verb doesn't exist or
      a real verb is undocumented. This is the mechanical net for *removed/renamed*
      commands; argument-level drift (like `breakpoints`) still needs the audit above.

- [ ] **`snapshot` output ergonomics.** Confirm whether `snapshot` needs a
      `--format` flag to produce parseable output; if so, make agent-friendly JSON
      the default (or document the flag in the skill). Write a probe.

- [ ] **Audit call counts for the prescribed workflows** in charter.md (annotate→
      resolve, group→auto-layout→focus). Where one intent takes 3+ calls, file a
      simplification item.

- [ ] **Batch page/mixed-entity create has the same async race.** `/pages/create`
      batch (pages.ts:90-98) and `/entities/create` mixed batch (entities.ts:313-338)
      respond before entities exist (same `staggerOperation` pattern). Not tested
      by current CLI probes but will bite agents doing multi-page batch creates.
      Fix: create synchronously, then `animateCursorScan` for cursor animation only.

- [ ] **W2: `annotate` → `resolve` round-trip needs an intermediate `annotations` list call.**
      Trace `trace-20260609-231057-r1`: after `annotate` returned a complete object including
      the id (`ann_315d6e2d-a9b5-494f-a5c7-d0c466f186c9`), the doer still ran `annotations`
      (call 4) to verify the id before passing it to `resolve` — adding 1 unnecessary call
      (6 actual vs 4 ideal). The friction is that the skill does not explicitly state the id
      returned by `annotate` is stable and can be fed directly to `resolve`/`reply`. Fix: add a
      note to the skill under `annotate` confirming id stability. Write a probe: assert that
      `annotate` → `resolve` succeeds without an intermediate list call, and the annotation
      reads `status: "resolved"` in `annotations --all`.

- [ ] **W2: `annotations` hides resolved items by default; `--all` required to confirm resolve.**
      Trace `trace-20260609-231057-r1`: after `resolve`, plain `annotations` returned 0 matching
      items — the resolved annotation was invisible. The doer needed `--all` to confirm the
      operation succeeded (call 6). An agent cannot verify its own `resolve` without a
      non-default flag, and the flag may not be discoverable from `--help` or the skill. Fix:
      check whether `--all` is documented in the skill; if not, add it. Write a probe asserting
      that after `specular resolve <id>`, `specular annotations --all` shows that id with
      `status: "resolved"`, and `specular annotations` (no flag) does NOT include it.

- [ ] **W2: `specular delete <ann-id>` silently exits 0 but does not delete the annotation.**
      Trace meta `trace-20260609-231057-r1`: "specular delete silently lies for annotation ids —
      known limitation." A no-op that exits 0 is worse than a clear error — it poisons any
      agent doing cleanup. Fix: `delete` on an annotation id should exit non-zero with an
      actionable message (e.g. "Annotations cannot be deleted; use `specular resolve` to close
      them"). Write a probe asserting this exit code and message.

## Done

<!-- entries land here as: - [x] YYYY-MM-DD <what> (<short-sha>) -->
- [x] 2026-06-09 HEAL: batch text/file entity create returned IDs before entities existed — race made grid-layout probes see 0 entities. Fixed by creating synchronously then responding; cursor animation moved to `animateCursorScan` (cosmetic only). (eb16a20)
