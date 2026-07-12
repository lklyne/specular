---
name: Hot-reload iteration loop
timeout: 120s
---

## Scenario

Add a page pointing at any working URL using `specular add page <url>`. Run
the following round three times: snapshot with `specular snapshot -i -f <pageId>`,
act on one of the refs it returns (e.g. `specular click @eN -f <pageId>`),
then simulate a hot-reload-style update with
`specular eval "document.body.innerHTML += '<p>round N</p>'" -f <pageId>`.
This sandbox has no live dev server wired into the harness, so a source
edit is approximated by the `eval` mutation — the effect on refs (the whole
tree invalidates) is the same one a real HMR update produces. Each round
must take a fresh snapshot before acting — never reuse a ref returned by an
earlier round's snapshot. Confirm all three rounds complete without any
ref-based command failing on a stale ref and without retrying the same
command more than once per round.

## Expected outcomes
- Each round's `specular snapshot -i -f <pageId>` returns a fresh set of `@eN` refs
- Each round's `specular click @eN -f <pageId>` uses only a ref from that round's own snapshot
- No interaction fails with a stale-ref error across all three rounds
- No command needs more than one retry in any round (no thrashing on a dead ref)

## Notes
- This exercises the `specular` CLI directly, matching the iteration loop
  described in `.claude/skills/specular/SKILL.md`'s "Targeting & stale
  refs" section: re-snapshot after any source edit rather than reusing
  refs across it.

## Cleanup
- Delete the page created for this test
