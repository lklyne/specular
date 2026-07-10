---
name: Stale-ref recovery
timeout: 90s
---

## Scenario

Add a page pointing at any working URL using `specular add page <url>`. Run
`specular snapshot -i -f <pageId>` and note one of the returned `@eN` refs.
Then invalidate that ref by mutating the DOM out from under it — run
`specular eval "document.body.innerHTML = '<button id=after>after</button>'" -f <pageId>`.
Without re-snapshotting, attempt to act on the noted ref, e.g.
`specular click @eN -f <pageId>`. The command should fail with an error that
points at recovery (re-snapshotting or targeting by selector), not a bare
CLI error. Recover using the hint: either re-run
`specular snapshot -i -f <pageId>` and act on a fresh ref, or switch to a
selector such as `specular click "#after" -f <pageId>`. Confirm the recovery
attempt succeeds.

## Expected outcomes
- `specular snapshot -i -f <pageId>` returns an accessibility snapshot with `@eN` refs
- `specular eval` mutating the DOM invalidates the previously captured ref
- Acting on the stale ref fails with an error mentioning re-snapshotting or targeting by text=/CSS selector
- Recovering via a fresh snapshot or a selector-based command succeeds against the post-mutation DOM

## Notes
- This exercises the `specular` CLI directly (not the `browse` MCP tool) —
  the stale-ref hint text lives in `handleBrowse`, which both surfaces sit
  on top of, but the CLI is the primary agent-facing surface for this
  targeting workflow (see `.claude/skills/specular/SKILL.md`, "Targeting &
  stale refs").

## Cleanup
- Delete the page created for this test
