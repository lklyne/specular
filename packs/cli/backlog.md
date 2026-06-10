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

- [ ] **W3: `focus` returns `{"focused": false}` with exit 0 — success is unverifiable.**
      Trace `trace-20260609-231915-r1` call 7: `["focus", "group_f42f9127-b045-4855-b009-4a15437a2baa"]`
      → exit 0, stdout `{"focused": false}`. Camera in the subsequent `workspace` readback (call 8)
      is identical to the pre-workflow state (call 1): `panX: 225.18, panY: 876.90, zoom: 0.23`
      unchanged. W3 acceptance requires "the camera/selection reflects the focus" — this criterion
      appears unmet. An agent receiving `{"focused": false}` after a `focus` call cannot distinguish
      success from a silent no-op or failure. Fix: `focus` should return `{"focused": true}` when
      the viewport was moved; if the target is already in view it should still return `{"focused": true}`
      to signal the postcondition is satisfied (not signal failure). Write a probe asserting
      `specular focus <existing-group-id>` exits 0 and returns `{"focused": true}`.
      REVIEW: if the camera intentionally doesn't change (target already in view), the response
      should still say `true` not `false`; a `false` with exit 0 is a misleading success.

- [ ] **W3: `create page` has no multi-URL form — 3 pages cost 3 calls (2 over ideal).**
      Trace `trace-20260609-231915-r1` calls 2–4: three separate `["create", "page", "http://localhost:4321"]`
      calls to create 3 pages. W3 ideal is ≤6 calls; actual was 8 (2 over). The per-page create
      is the gap. Check whether `create page` already accepts variadic URLs
      (e.g. `specular create page url1 url2 url3`) — if so, document it in the skill; if not,
      add that capability or document `upsert --json` as the batch-page path. Write a probe
      asserting `specular create page url1 url2 url3` (three URLs) exits 0 and returns all
      three ids in one response.

- [ ] **W1: `create page` has no `--preset` flag; `presetIndex` device mapping undocumented in skill.**
      Trace `trace-20260609-232718-r3` calls 2–3: goal was desktop + phone pages, but `create page
      http://localhost:4321` defaulted to `presetIndex 6` (laptop) and the `upsert --json` phone
      page also landed at `presetIndex 6` — the doer could not reach a phone preset. The skill does
      not list `presetIndex` values or their device-name mappings, and `create page` takes no
      breakpoint argument, so `upsert --json` is the only path — but only if the agent already knows
      the presetIndex→device mapping. Fix: document the full `presetIndex` → device name table in
      the skill under the `upsert` section; and/or add a `--preset <device>` flag to `create page`.
      Write a probe: assert that when the skill-documented presetIndex for "phone" is passed via
      `upsert --json`, `workspace` shows the resulting page with the correct `deviceId` (e.g.
      `"phone"` or equivalent).

- [ ] **W1: text entity's note content is stored in `preview` field — undocumented, non-obvious name.**
      Trace `trace-20260609-232718-r3` call 4 (workspace readback): `text_cffdfe28` shows
      `"preview": "W1 test note"`, not `text` or `content`. An agent creating a note from scratch
      would likely set a `text` or `content` key, find it absent in the workspace output, and have
      no signal that `preview` is the correct field. Fix: document the `preview` field name for
      text entities explicitly in the skill (under both `upsert` input schema and `workspace` output
      schema); consider renaming to `text` in the API if a breaking change is acceptable. Write a
      probe asserting that `upsert --json` with a text entity using the `preview` key round-trips
      through `workspace` under that exact field name.

## Done

<!-- entries land here as: - [x] YYYY-MM-DD <what> (<short-sha>) -->
- [x] 2026-06-09 HEAL: batch text/file entity create returned IDs before entities existed — race made grid-layout probes see 0 entities. Fixed by creating synchronously then responding; cursor animation moved to `animateCursorScan` (cosmetic only). (eb16a20)
- [x] 2026-06-09 DISCOVER W4 clean run (trace-20260609-232329-r2): 12-item 4×3 grid; all IDs round-tripped; no overlap; JSON output without --format flag; core workflow 2 calls (ideal 2).
