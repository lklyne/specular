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
      pointing at full-URL usage, then make it pass.

- [ ] **`snapshot` output ergonomics.** Confirm whether `snapshot` needs a
      `--format` flag to produce parseable output; if so, make agent-friendly JSON
      the default (or document the flag in the skill). Write a probe.

- [ ] **Audit call counts for the prescribed workflows** in charter.md (annotate→
      resolve, group→auto-layout→focus). Where one intent takes 3+ calls, file a
      simplification item.

## Done

<!-- entries land here as: - [x] YYYY-MM-DD <what> (<short-sha>) -->
