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

## Done

<!-- entries land here as: - [x] YYYY-MM-DD <what> (<short-sha>) -->
- [x] 2026-06-09 HEAL: batch text/file entity create returned IDs before entities existed — race made grid-layout probes see 0 entities. Fixed by creating synchronously then responding; cursor animation moved to `animateCursorScan` (cosmetic only). (`<sha>`)
