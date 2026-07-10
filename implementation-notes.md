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

(Filled in as phases complete.)

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
