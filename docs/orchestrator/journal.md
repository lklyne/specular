# Orchestrator journal

Append-only log for the Specular workflow orchestrator (a scheduled Claude routine).
Each run reads the last ~5 entries before acting, then appends one entry at the bottom.

Write **insight**, not activity. "Surveyed 8 PRs, triaged 2" is noise. "PRs in
`canvas-bg/entity-renderers/` keep needing a follow-up for keyboard handling —
worth a deepening proposal" is signal.

Format per entry:

```
## YYYY-MM-DD
Surveyed: <one line — what you looked at and the dominant theme>
Acted:
- <#issue/PR or path> — <what + why>
Noticed (no action): <patterns worth watching>
Hint for next run: <one sentence for tomorrow-you>
```

---

## 2026-05-10 (seed)
Surveyed: nothing yet — first run, system being set up.
Notes: scope clarified — orchestrator watches the *automation system*, not the product. Output is proposals to improve the system (skills, prompts, labels, habits), not actions inside the system. Operating skills already in place: `/triage`, `/to-issues`, `/to-prd`, `/grill-with-docs`, `/improve-codebase-architecture`.
Hint for next run: on first real run, baseline the system. Count merged PRs in the last week, what fraction came via the AFK pipeline, current `needs-triage` and `ready-for-agent` queue depths, and when the skills directory was last touched. That snapshot is what future runs compare against to spot leaks.

---

## 2026-05-10 (first real run)
Surveyed: 30 merged PRs, Lyle's commit log, queue depths, CI state, skills history. Dominant theme: establishing baseline.
Acted:
- #61 — CI check for SKILL.md drift (three observed instances of manual sync: April repair commit, May-06 skills-only touch, May-08 rename sweep). Smallest fix: a diff gate in CI.
- #63 — CI workflow proposal (typecheck + test:unit + SKILL.md diff). Explicitly absorbs #61. Two instances of Lyle manually noting "typecheck + test:unit pass" in commit messages.
- #62, #64 — journal PRs filed (duplicate; the run fired twice, producing two PRs for the same date — a self-failure)
Noticed (no action): /simplify being invoked at the tail of most PRs (pattern in commit messages). AFK pipeline at 0/12 PRs — same as the seed hint predicted.
Hint for next run: check whether #61 was closed when #63 superseded it; check AFK fraction; watch for journal PR accumulation.

---

## 2026-05-11
Surveyed: AFK pipeline fraction (14 merged PRs week-to-date), needs-triage staleness (issue #53, 2 days old), orchestrator proposal status.
Acted:
- #68 — AFK habit gap proposal. Lyle's flow is idea → interactive Claude session → PR; zero issues routed through /triage → ready-for-agent. Two instances: 0/14 AFK fraction + empty ready-for-agent queue. Proposed a one-sentence routing heuristic added to /to-issues.
- #69 — journal PR filed.
Noticed (no action): #61 still open alongside #63 (not closed after being superseded). ADR cadence high: 5 ADRs (0003–0007) in 5 days. SKILL.md was touched in the May-08 and May-10 PRs — skills keeping pace with domain changes, not a leak.
Hint for next run: check whether any of the three proposals (#61, #63, #68) have been acted on; if all three are still open and unreviewed, the bottleneck is review bandwidth, not automation gaps — note it and don't add more proposals.

---

## 2026-05-12
Surveyed: Journal PR accumulation and proposal review status. Three journal PRs (#62, #64, #69) open unmerged; three proposals (#61, #63, #68) open unreviewed. needs-triage: 1 item (issue #53, now 4 days old). ready-for-agent: 0. AFK fraction: still 0/14+.
Acted:
- Filed self-modifying proposal (see issue below) — orchestrator creates one journal PR per run, causing PRs to pile up rather than stacking. Proposed: detect open journal PRs before filing a new one; add to the same branch instead of opening a second.
- Filed this consolidated journal entry covering 2026-05-10 (first real run), 2026-05-11, and 2026-05-12.
Noticed (no action): The system is producing output (proposals, journal PRs) faster than it is being reviewed. This is not an automation failure — the orchestrator is working. It is a human-bandwidth constraint. Adding more proposals into an unreviewed queue adds noise. Correct response: hold until existing proposals are actioned.
Hint for next run: if proposals #61, #63, #68 are still all open with no comments, write "why is nothing landing?" into the journal rather than proposing again. The answer is more useful than another proposal.

---

## 2026-05-13
Surveyed: Proposal review status, AFK pipeline fraction (PRs #74–#80 merged today), issue queue, housekeeping debt. Dominant theme: the AFK pipeline activated overnight; the concern in #68 self-corrected.
Acted:
- #61 closed — superseded by #63 since 2026-05-10 but never closed. Pending housekeeping debt settled.
- #71 closed — journal PR stacking fix is now implemented in the orchestrator's own prompt ("Journal PR mechanics" section). The pre-flight check this run found no open journal PR and correctly opened a single new one.
Noticed (no action):
- **AFK pipeline is live.** PR #76 (keyboard binding registry, dex epic `ypz5rro1`) merged today with 5 step PRs (#75, #77, #78, #79, #80) — all produced by the AFK agent in under 8 hours. AFK fraction flipped from 0/14 to a full epic overnight. The gap #68 described was real during the architectural sprint; it self-corrected once a bounded, well-specified plan (#74) existed.
- **Proposals #63 and #68 have zero comments and zero activity.** Asking "why?" yields a different answer than before: #68's gap has now closed on its own, so the proposal is likely moot. #63 (CI workflow) is the only live proposal. It requires adding one YAML file — that is agent-executable. It has never been labeled `ready-for-agent`, which means it has no path through the pipeline that just proved itself capable of delivering a 5-step refactor overnight.
- **Issue #81 filed today** — 5-phase test coverage plan. Each phase is bounded and has clear acceptance criteria. A natural candidate for the AFK pipeline once Phase 0 (decisions) is answered in comments.
- **Issue #53** (`needs-triage`) is now 4 days old and still unrouted. Pattern: bugs that require architectural discussion (it is) park here indefinitely.
Hint for next run: check if #63 has been labeled `ready-for-agent`; if not, that is the only systemic gap worth watching — a CI workflow addition is exactly the kind of bounded task the pipeline can absorb.

---

## 2026-05-14
Surveyed: PRs #84–#98 (two full AFK epics overnight + interactive fixes), afk-loop.sh mid-flight bug fixes visible in PR #90 and the #97 squash commit, open proposal status. Dominant theme: AFK pipeline at full velocity after activation yesterday.
Acted:
- #68 closed — definitively resolved. Two full AFK epics shipped in ~24h: test coverage overhaul (#84–#87, dex epic `nrsnaunt`) and canvas drag affordances (#93–#97, dex epic `awuhzpwz`). The pipeline concern #68 tracked is gone.
Noticed (no action):
- **AFK loop self-corrects mid-run, but each new worker variant reveals platform-specific bugs.** The canvas-drag epic encountered three infrastructure failures: codex's mcp_servers blocking startup, the sandbox blocking git writes, and dex completion not being committed to the feature branch (loop re-completed task 7kjdgp84 for ~16 fires before the fix). All three were absorbed into PR #90 and the #97 squash. This is healthy — the fixes compound into the infrastructure. But the pattern is consistent: first run of cloud→local→codex each time surfaces a new class of issue. Watch for the same on the next novel variant.
- **CLI smoke coverage still a floating deferred.** PR #86 explicitly deferred `tests/smoke/cli.test.ts` ("CLI subprocess testing needs a built CLI in the smoke setup pipeline") and documented it in the PR body — but did not file a tracking issue. `tests/README.md` (PR #85) lists intentionally-uncovered surfaces; if cli.test.ts is not there, the deferral is invisible to the pipeline. One instance; watch for a second.
- **#63 (CI workflow) noted for the third consecutive run with no review, no label, no comment.** Two runs ago the 2026-05-13 entry called it "agent-executable"; yesterday's entry noted it had no `ready-for-agent` label. Nothing has moved. This is now a pattern in the orchestrator output, not just the proposal queue.
Hint for next run: the stasis on #63 (noted 3 runs, 0 movement) is the signal — if it persists, the question is whether any proposal generated by this orchestrator has a path to action, and if not, the orchestrator's output format or routing is the problem.

---

## 2026-05-15
Surveyed: 0.3.0 release (commits d400bba–7f6881f), the post-release release.yml CI update, proposal #63 status (day 5, 0 comments), recent commit log.
Acted:
- No proposals filed. No stale orchestrator output to close (#63 is 5 days old; stale threshold is 7).
Noticed (no action):
- **#63 (CI workflow) — fourth run, zero movement.** Repeating the observation again is noise. By the orchestrator's own cleanup rule it ages out on 2026-05-17. Next run should close it as stale if still frozen; continuing to note it is itself the leak.
- **Release 0.3.0 shipped with a blank GitHub Releases page; Lyle patched release.yml immediately after (commit d400bba).** CI improvements that land arrive when the pain is visceral and immediate, not from a proposal queue. This may be the real diagnosis for why #63 hasn't moved — there's no failing PR to point at, only a hypothetical improvement. The trigger for a PR CI workflow is probably the first time a type error or failing test slips through a merged PR, not a calendar date.
- **AFK pipeline stable at full velocity.** `afk-local` skill added, codex worker option added, loop bugs fixed in #90. No watch items here.
- **Draft PRs aging** (#9 at 26 days, #29 at 22 days, #32 at 21 days). Lyle's product decisions; not orchestrator scope.
Hint for next run: close #63 if it hits 7 days (2026-05-17) with no comment; watch for a type error or test regression slipping through a PR as the natural trigger for the CI workflow — that's a better proposal moment than this one.

---

## 2026-05-16
Surveyed: Post-0.3.0 steady state, PRs #29 (fallow setup) and #121 (popup-menus-v2 integration) merging yesterday, #63 staleness status (day 6), proposal queue.
Acted:
- No stale items closed. #63 is 6 days old; the >7-day threshold triggers on 2026-05-17. One day short.
Noticed (no action):
- **#63 closes tomorrow.** Next run closes it as stale (0 comments, 0 activity in 7 days). After that, the orchestrator proposal queue is empty for the first time since the system started. That's a natural pause point.
- **Fallow is now live in the ecosystem** (PR #29, 22-day draft, merged 2026-05-15). It's a static analysis tool for dead code, circular deps, complexity. One instance of setup; watch whether it gets invoked regularly or was a one-time installation pass. Not actionable yet — need a second instance to know if it's a habit forming.
- **Three consecutive runs with no new proposals filed** (May 14–16). This is correct behaviour given the May 15 diagnosis: CI changes land from pain, not queues. Restraint here is the right call, not a failure to observe.
- **Two patterns from May 14 are still at one instance each:** (a) AFK loop first-run failures for novel worker variants; (b) CLI smoke coverage deferral with no tracking issue. Both need a second sighting before they're proposal-ready.
Hint for next run: close #63 as stale; then with a clear queue, do a fresh two-instance audit — have either of the May 14 watch items (AFK first-run failures, CLI smoke deferral) seen a second occurrence in the PRs since May 14?

---

## 2026-05-17
Surveyed: Proposal queue (#63 staleness), two-instance audit on May 14 watch items, PRs #125–#137, AFK loop restructure (#133). Dominant theme: first successful proposal landing + pipeline self-correcting.
Acted:
- No proposals filed. No stale orchestrator issues to close (#63 closed by Lyle as "completed" before this run).
Noticed (no action):
- **#63 LANDED** — Lyle closed it "completed" on 2026-05-16 after PR #125 shipped (CI: typecheck, lint, test:unit, fallow on every PR). First orchestrator proposal to reach "completed." The May 15 diagnosis held: CI improvements land from visceral pain, not queues — in this case, the pain was a real layout-pass refactor (PR #132) where test failures would have been invisible without CI.
- **AFK first-run failure watch item resolved (second instance confirmed).** Instance 1 (May 14): canvas-drag epic hit three infrastructure failures (mcp_servers, sandbox, dex completion). Instance 2 (May 16): layout-pass epic burned ~20 fires, ~50% wasted on CI polling. Lyle diagnosed and fixed proactively in PR #133: `afk-loop.sh` now uses `gh pr checks --watch` for CI waits instead of a polling fire; `afk-fire.sh` is stateless implement-only. Pattern is real, closed, self-corrected.
- **CLI smoke coverage deferral still at one instance.** PR #86 deferred `tests/smoke/cli.test.ts` with "CLI subprocess testing needs a built CLI in the smoke setup pipeline." Issue #81 (Phase 2) includes `cli.test.ts` on its checklist, but the underlying infrastructure gap (no built CLI in test harness) hasn't been addressed separately. Still watching.
- **Fallow in CI but soft-gated.** PR #125 added fallow to CI; PR #133 documents `AFK_SOFT_CHECKS` defaults to `fallow` so it never blocks a merge. Pre-existing issues (unlisted react/react-dom, circular deps) are the cause. Soft-gating is pragmatic, but if the gate stays soft indefinitely, fallow becomes decorative. One instance; watch whether fallow findings ever get acted on.
- **Proposal queue is empty; system is healthy.** Four proposals total since inception: #61 (closed, superseded by #63), #63 (closed, completed), #68 (closed, self-corrected), #71 (closed, self-corrected). Zero open proposals for the first time. AFK pipeline running at full velocity. CI live. This is the intended steady state.
Hint for next run: with an empty queue and healthy pipeline, shift focus to the two remaining watch items — (a) CLI smoke infrastructure gap (still one instance; look for cli.test.ts or a built-CLI step appearing in any PR since May 14) and (b) fallow findings ever causing a commit (not just running in CI). If both stay at one instance for another week, they're probably not leaks.

---

## 2026-05-18
Surveyed: PRs #138–#145, two watch items from May 17 (CLI smoke gap, fallow findings causing commits), open PR age distribution, issue queue. Dominant theme: watch items resolving; system at steady state.
Acted:
- Nothing to close, nothing to file. Proposal queue empty; no stale orchestrator output.
Noticed (no action):
- **Fallow watch item resolved (second instance confirmed).** PR #145 "Fix fallow check failures: dead code, circular deps, config gaps" just merged. Instance 1 (May 17): soft-gate added, pre-existing issues noted. Instance 2 (May 18): PR #145 cleared dead code and tuned `.fallowrc.json` (ignoreExports for ESLint rule files, ignoreDependencies for react/react-dom — both legitimate suppressions, not real issues being hidden). The lifecycle is complete: install → CI soft-gate → cleanup pass lands. Fallow is generating real signal. Watch item (b) closed.
- **CLI smoke infrastructure gap still at one instance.** No second PR deferral, no tracking issue, no built-CLI step in any recent PR. Issue #81 Phase 2 lists `cli.test.ts` on its checklist but the underlying "no built CLI in smoke setup" constraint remains unaddressed. Still watching; threshold for a proposal is a second instance.
- **PR backlog from May 17 session: four open, none merged yet** (#136 grid gaps, #137 hit-test fix, #143 pointer events migration, #144 manifest component extensions). Normal after a concentrated AFK session. Not a concern today; worth checking age next run — if any are still open at seven days, review bandwidth may be the constraint.
- **Pointer events invariant now hard-gated.** PR #143 upgraded the `no-mouse-events` ESLint rule from `warn` to `error`. Pattern: spec doc → prose rule → ESLint enforce → CI gate. This is the interaction-layer enforcement model working as intended. Each spec invariant that gets this treatment removes a whole class of silent regressions.
Hint for next run: check whether the May 17 PR batch (#136, #137, #143, #144) has been reviewed — if any are seven days old and unmerged, that's the first review-bandwidth signal worth noting. CLI smoke gap remains the only active watch item.

---

## 2026-05-19
Surveyed: May 17 PR batch aging (#136, #137, #143, #144), open `needs-triage` queue depth and age, PR #92 age, CLI smoke gap, @claude GitHub Actions integration (#138 merged May 17). Dominant theme: needs-triage queue accumulating without a drain — two confirmed instances crossed the proposal threshold.
Acted:
- #152 filed — automated drain for `needs-triage` issues with no comments after 3 days. Two instances: #53 (10 days, 0 comments, architectural undo bug) and #124 (3 days, "Blocked by: None — can start immediately", 0 comments). PR #138 (Claude Code GitHub Actions, merged May 17) makes the fix cheap: one scheduled workflow that @mentions Claude on stale issues. Proposal scoped to the mechanical gap; routing logic is unchanged.
Noticed (no action):
- **May 17 batch**: #143 (pointer events) merged today; #136, #137, #144 still open at 2 days — below 7-day threshold, normal.
- **PR #92** ("Run smoke-test Electron in accessory mode"): 6 days old as of today, created May 13. Will hit the 7-day stale threshold on May 20. Worth checking next run.
- **CLI smoke gap**: Still one instance. No `cli.test.ts` PR or built-CLI-in-smoke-harness step observed in commits since May 14.
- **@claude GitHub Actions live**: #138 merged May 17. Claude is now invocable via GitHub issue/PR comments. Too early to characterize usage. The triage drain proposal (#152) is the first concrete use case.
- **needs-triage queue depth**: 4 open — #146 (today, fresh), #124 (3 days), #122 (4 days, design discussion), #53 (10 days). The queue is not draining between AFK epic kick-offs.
Hint for next run: check PR #92 (7-day threshold hits May 20); check whether #136 or #144 have been reviewed; watch for any @claude activity on triage items if #152 is acted on.

---

## 2026-05-20
Surveyed: PR aging (#92 at day 7, #136 at day 3, #144 at day 3), needs-triage queue (4 items, 3 past 3-day threshold), proposal #152 status, today's merged PRs (#153–#157). Dominant theme: triage queue accumulating with no drain while the @claude action that would enable it sits idle.
Acted:
- Nothing to close or file. Single proposal in queue (#152, day 1, 0 comments) — not stale.
Noticed (no action):
- **PR #92 (smoke-test Electron accessory mode) hit 7 days with no comments or merge.** Created 2026-05-13, test plan unchecked. Pattern matches the #63 trajectory: an infra improvement with no immediate pain trigger. #63 landed once a CI failure made the improvement visceral. Watch whether #92 lands after a smoke test causes a real disruption, or whether it stalls indefinitely.
- **needs-triage queue unchanged at 4 items (#53 at 11 days, #122 at 5 days, #124 at 4 days, #146 at 1 day).** Three are past the 3-day threshold that proposal #152 targets. @claude has been live since May 17; no @claude invocations observed on any triage issue. This confirms the gap #152 identifies: the mechanism exists but has no scheduler.
- **PR #136 (grid inspect) at 3 days, no review. PR #144 (manifest extensions) updated today** — more active. Neither is concerning yet.
- **Five PRs merged today (#153–#157):** sticky dark mode, middle-mouse pan fix, multi-selection undo batching, Cmd+1 generalization, selection padding cleanup. All single-issue, short-cycle.
- **CLI smoke gap still one instance.** No built-CLI-in-smoke PR observed since May 14.
Hint for next run: watch whether #152 is acted on now that three needs-triage issues exceed its 3-day threshold; check #92 (now at 8 days — beyond orchestrator-stale window, though it's a product PR so closure is Lyle's call); check whether #136 gets reviewed.

---

## 2026-05-21
Surveyed: PR #166 (canvas-stack-order integration, ADR 0014 slices 5–9, claude-review CI change), needs-triage queue (4 items, #53 at 12 days), open PR aging, proposal #152 status. Dominant theme: AFK pipeline delivering largest epic to date; needs-triage drain still unacted.
Acted:
- Nothing to close (proposal #152 is 2 days old, below 7-day threshold). Nothing to file — no second instance of any watch item; one proposal already in queue.
Noticed (no action):
- **PR #166 disabled the `claude-review` CI auto-trigger**, switching it to `workflow_dispatch` ("redundant in AFK loop context"). Rationale is sound for AFK PRs — the loop reviews inline. But non-AFK PRs (#136 at 4 days, #144 at 4 days, #159 at 1 day) will no longer get automatic review passes. One instance; watch whether non-AFK PRs start merging without any review.
- **needs-triage queue unchanged**: 4 items (#53 at 12 days, #122 at 6 days, #124 at 5 days, #146 at 2 days). Proposal #152 is 2 days old with 0 comments. Three of four items already past the 3-day threshold the proposal targets. The queue confirms the need; the proposal awaits action.
- **Canvas-stack-order epic complete** (PR #166, ADR 0014, 9 slices total across two sessions). Sidebar drag-to-reorder, keyboard shortcuts (`Cmd+[]/]`), edges in stack order, migration, HTTP API — all covered. Largest single AFK epic delivered so far. System handling scale well.
- **CLI smoke gap still one instance** since May 14. No `cli.test.ts` or built-CLI-in-smoke step observed in any recent PR.
Hint for next run: if a non-AFK PR merges without any review comment after claude-review was disabled, that's the second instance — make it a proposal; also check whether #152 has been acted on or whether the needs-triage queue has grown further.

---

## 2026-05-22
Surveyed: needs-triage queue (now 5 items), open PR aging (#32 at 28 days, #92 at 9 days, #136/#144 at 5 days, #159 at 2 days), proposal #152 status (day 3, 0 comments), claude-review disable watch item, merged PRs since yesterday. Dominant theme: evidence accumulating for existing proposal; no new second instances crossed the threshold.
Acted:
- Nothing to close or file. Single proposal in queue (#152, day 3, 0 comments) not stale. No watch item crossed the second-instance threshold.
Noticed (no action):
- **needs-triage queue grew to 5 items**: #53 at 13 days, #122 at 7 days, #124 at 6 days, #146 at 3 days (at threshold), #167 at 1 day (auto-focus rename inputs, filed 2026-05-21). Four of five items exceed the 3-day threshold proposal #152 targets. @claude is live since May 17; zero invocations observed on any triage issue. The mechanism exists, the scheduler doesn't. Proposal is right; it's unacted.
- **PR #32 (LM Studio support) open at 28 days, 0 comments**: Created April 24, predates the Telescope→Specular rename. Body still references "Telescope." Unclear if intentionally parked or fell out of scope during the rename sprint. Product decision; not closing.
- **PR #92 (smoke accessory mode) at 9 days**: Two days past the orchestrator-stale window, test plan unchecked, 0 comments. Consistent with the CI-improvement-lands-on-pain pattern. Not actionable by the orchestrator — product PR, Lyle's call.
- **Non-AFK PR review gap watch**: #136 (grid inspect, 5 days) and #144 (manifest extensions, 5 days) open and unreviewed since before claude-review was disabled May 21. Neither has merged post-disable. No second instance confirmed yet.
- **AFK integration PR #166 open**: canvas-stack-order epic awaiting merge. Review checklist unchecked. Normal — integration PR sits for human review as designed.
Hint for next run: watch whether #136 or #144 merges without any review comment (human or @claude) — that's the second instance for the non-AFK review gap and crosses the proposal threshold. If #152 reaches day 7 with 0 comments, note it as a stalled proposal and consider whether it needs a more concrete implementation stub to attract action.

---

## 2026-05-23
Surveyed: Open PR aging (#136/#144 at day 6, #159 at day 3, #166 at day 2), check run history on #144, needs-triage queue (5 items unchanged), proposal #152 (day 4, 0 comments). Dominant theme: watch items near threshold but not crossed; one clarification on the claude-review disable timing.
Acted:
- Nothing to close or file. Single proposal in queue (#152, day 4) not stale. No second instances crossed.
Noticed (no action):
- **claude-review disable clarification**: Previous entries treated the disable as already landed (from PR #166's body). Checking #144's actual CI check runs: `claude-review` ran and completed successfully on 2026-05-20 — before PR #166 (which contains the disable) has merged to main. The disable is still in the PR branch; the auto-trigger remains active for currently open PRs. Watch item becomes concrete only after #166 merges: the NEXT non-AFK PR opened after that will have no auto-review trigger.
- **#136 and #144 at day 6, zero review comments, still open**: Both non-AFK PRs. #144 had `claude-review` CI run (no findings) and `check`+`fallow` both green. Neither has merged. The watch item condition is *merge without review*, not just aging — both could merge tomorrow into an unreviewed state. If either merges with no comment from human or @claude after #166 lands, that's the second instance.
- **needs-triage queue stable at 5 items**: #53 at 14 days, #122 at 8 days, #124 at 7 days, #146 at 4 days, #167 at 2 days. All five have 0 comments. No @claude invocations on any issue since the action was enabled May 17. Proposal #152 is sitting next to the exact problem it would fix.
- **#152 at day 4, 0 comments**: Not stale (threshold is 7 days). The proposal body has a concrete implementation — one workflow file. If it reaches day 7 with no comment, the bottleneck isn't clarity; it's prioritisation bandwidth.
- **Quiet period**: No PRs merged since May 21. Three open non-AFK PRs (#136, #144, #159) and one large integration PR (#166) awaiting review. Pattern suggests a brief pause between AFK sessions.
Hint for next run: once PR #166 merges (bringing the claude-review disable), the first new non-AFK PR opened after that is the canary — check whether it gets any review pass. Also check if #152 hits 7 days; if it does with no comment, file a note rather than re-proposing.

---

## 2026-05-24
Surveyed: Open PR aging (#136/#144 at day 7, #159 at day 4, #166 at day 3), needs-triage queue (5 items, all 0 comments, #53 at 15 days), proposal #152 (day 5, 0 comments), commit log (4-day drought since May 20). Dominant theme: integration PR review bottleneck; triage drain proposal approaching stale threshold while the problem it targets continues to compound.
Acted:
- Nothing to close or file. #152 at day 5 (stale threshold: May 26). No new second instances crossed.
Noticed (no action):
- **4-day product commit drought**: No commits to main since May 20. Four open product PRs (#166 day 3, #159 day 4, #144 day 7, #136 day 7). The brake is PR #166: 42 changed files, 17 commits, 6-item manual review checklist — all unchecked. Integration PRs with human verification requirements are the designed pause point in the AFK cycle. Expected, not a leak.
- **#136 and #144 both hit day 7 today, still open, 0 review comments**: Watch condition is *merge without review*, not just aging — neither has merged. They are waiting behind #166, not escaping review. Once #166 merges, the review queue should flush quickly.
- **needs-triage queue stable at 5 items, all 0 comments**: #53 at 15 days, #122 at 9 days, #124 at 8 days, #146 at 5 days, #167 at 3 days. @claude has been invocable since May 17 — one week, zero invocations on any triage issue. The queue has grown from 2 items (when #152 was filed) to 5 items; the evidence base for #152 is now stronger than when it was written.
- **#152 stale warning**: Reaches 7-day threshold on May 26 (2 days). The problem it targets has not resolved — it has compounded. If it hits day 7 with no comment, the bottleneck is prioritisation bandwidth, not proposal clarity. The right call is to close-and-refile with the updated evidence (5 items, oldest at 17 days) rather than silently leaving a zombie open or closing without replacement.
- **CLI smoke gap still one instance**. No built-CLI step or second deferral in any PR since May 14.
Hint for next run: make the stale call on #152 — if no comment by May 26, close and immediately refile with the current evidence (#53 at 17+ days, 5 items in queue). Check whether #166 has merged and whether the review queue flushed; if #166 merged and the first subsequent non-AFK PR opened without auto-review, that's the second instance for the review gap.

---

## 2026-05-25
Surveyed: Open PR aging (#166 day 4, #159 day 5, #144/#136 day 8, #92 day 12), 5-day commit drought, needs-triage queue (5 items, all 0 comments), proposal #152 at day 6 (stale tomorrow). Dominant theme: designed pause holding; all watch items unchanged.
Acted:
- Nothing to close or file. #152 at day 6 — stale threshold hits May 26 (tomorrow). No new second instances crossed.
Noticed (no action):
- **5-day commit drought (day 5)**: No commits to main since May 20. Unchanged from yesterday. PR #166 (42 files, 17 commits, 6-item manual checklist all unchecked) remains the designed pause point. Not a failure; expected.
- **#136 and #144 now at day 8, #92 at day 12**: All three stalled behind #166. Watch condition for review-gap second instance is *merge without review*, not aging — none have merged. All are waiting in queue, not escaping review.
- **needs-triage queue**: 5 items, all 0 comments — #53 (16 days), #122 (10 days), #124 (9 days), #146 (6 days), #167 (4 days). Four of five now exceed the 3-day threshold proposal #152 targets. @claude has been invocable for 8 days with zero invocations on any triage issue. The problem #152 describes is compounding, not resolving.
- **#152 stale tomorrow (May 26)**: Per May 24 decision: if no comment by May 26, close and immediately refile with current evidence (5 items, oldest at 17 days). The proposal text remains accurate; the numbers are worse.
- **CLI smoke gap still one instance**. No built-CLI step or second deferral observed in any PR since May 14.
Hint for next run: make the stale call on #152 — close and refile if 0 comments; check whether #166 merged and the queue flushed.

---

## 2026-05-26
Surveyed: Open PR aging (#166 day 5, #159 day 6, #144/#136 day 9, #92 day 13), proposal #152 at day 7 (stale threshold hit), needs-triage queue (5 items, oldest at 17 days), 6-day commit drought. Dominant theme: stale call executed on proposal; queue compounding.
Acted:
- #152 closed as `not_planned` (7 days, 0 comments). Refiled as #168 with updated evidence: 5 items in queue vs 2 when #152 was filed; oldest item (#53) now at 17 days; @claude invocable 9 days with zero invocations on any triage issue.
Noticed (no action):
- **PR #166 still open, day 5**: 42 files, 17 commits, 6-item manual review checklist all unchecked. CI (fallow + check) green as of May 22. This is the designed pause point; the AFK cycle explicitly requires human verification for integration PRs. 6-day main-branch drought is expected, not a leak.
- **#136 and #144 at day 9, #159 at day 6**: All stalled behind #166. Watch condition for review-gap second instance remains *merge without review* — none have merged yet. The disable of `claude-review` CI auto-trigger is still in the #166 branch, not on main. Once #166 merges, the first new non-AFK PR is the canary.
- **needs-triage queue still at 5 items, all 0 comments**: #53 (17 days), #122 (11 days), #124 (10 days), #146 (7 days), #167 (5 days). The queue has not grown but has not shrunk. Proposal #168 is now the live track for fixing this.
- **CLI smoke gap still one instance**: No `cli.test.ts` or built-CLI-in-smoke PR observed since May 14. 12 days at one instance; if no second instance appears within the next week, this is probably not a recurring pattern and can be dropped from the watch list.
Hint for next run: check if #166 merged and whether the review queue flushed; if a new non-AFK PR opened post-#166-merge without auto-review, that's the second instance for the review-gap proposal. Also check whether #168 attracted any comment — if it also stalls for 7 days, consider whether the proposal format is the problem.

---

## 2026-05-27
Surveyed: Open PR aging (#166 day 6, #159 day 7, #144/#136 day 10, #92 day 14, #32 day 33), needs-triage queue (5 items unchanged, #53 at 18 days), proposal #168 status (day 1, 0 comments), CLI smoke gap watch item (13 days at one instance). Dominant theme: designed pause holding; all watch items unchanged; CLI smoke watch item retired.
Acted:
- Nothing to close or file. #168 at day 1, well below 7-day threshold. No new second instances crossed.
Noticed (no action):
- **7-day commit drought**: No commits to main since May 20. PR #166 (canvas-stack-order integration, 42 files, 17 commits, 6-item manual checklist all unchecked) remains the designed pause point. CI green. Expected.
- **needs-triage queue unchanged**: 5 items, all 0 comments — #53 (18 days), #122 (12 days), #124 (11 days), #146 (8 days), #167 (6 days). @claude invocable 10 days with zero invocations on any triage issue. Proposal #168 is the live track; too early to assess (day 1).
- **Non-AFK review watch**: claude-review CI disable still in #166 branch, not on main. Canary (first new non-AFK PR after disable lands) not triggered. #136 and #144 now at day 10, #159 at day 7 — all waiting behind #166, not escaping review.
- **CLI smoke gap: retiring watch item**. 13 days since the May 14 observation, still one instance only. No second instance in any PR in that period. Pattern not confirmed — dropping from active watch.
- **#168 format question surfaced early**: The triage drain proposal has now been filed twice (#152 closed stale, #168 open). If #168 also stalls to day 7 with 0 comments, the issue is not proposal clarity (the implementation is 30 lines and unchanged); it is that issues are the wrong forcing function. A more direct channel — e.g. Lyle implementing it himself as a `ready-for-agent` task, or the orchestrator directly tagging the stale issues with `@claude` — may be needed.
Hint for next run: watch for PR #166 merge and queue flush; #136/#144/#159 should follow quickly. First new non-AFK PR opened after the merge is the canary for the review-gap second instance. Check if #168 has any comment — if still 0 by day 7, write a note on what the right channel actually is instead of refiling a third time.

---

## 2026-05-28
Surveyed: PR aging (#166 day 7, #159 day 8, #144/#136 day 11, #92 day 15), 8-day main-branch drought, needs-triage queue (5 items, #53 at 19 days), proposal #168 (day 2, 0 comments). Dominant theme: everything held steady; designed pause continues.
Acted:
- Nothing to close or file. #168 at day 2 (stale threshold: June 2). No second instances crossed.
Noticed (no action):
- **8-day drought; #166 at day 7**: PR #166 (canvas-stack-order, 42 files, 17 commits, 6-item manual review checklist all unchecked) is the designed pause point. CI green. The AFK pipeline is producing integration PRs that require human verification — at the current pace the effective throughput cap is roughly one major epic per 2 weeks. That's not a failure; it's the explicit design. Worth flagging if the integration PR review cycle grows further.
- **needs-triage queue unchanged**: 5 items, all 0 comments — #53 (19 days), #122 (13 days), #124 (12 days), #146 (9 days), #167 (7 days). @claude invocable 11 days with zero invocations on any triage issue. Proposal #168 is the live track; too early to draw conclusions.
- **#168 (triage drain) at day 2, 0 comments**: The real diagnostic window opens at day 5–7 when the #152 stall pattern would recur. No signal yet.
- **Non-AFK review watch still pending**: claude-review CI disable still in #166 branch (not on main). Canary (first new non-AFK PR after disable lands) has not triggered. #136, #144, #159 all still queued behind #166 — none escaping review.
Hint for next run: watch for #166 merge and queue flush; if it merges, the first new non-AFK PR is the canary. If #166 is still open at day 10 (2026-05-31), note the integration PR review cycle as a structural throughput cap. Check if #168 has attracted any comment by its day 7 deadline (June 2).

---

## 2026-05-29
Surveyed: End of 9-day drought (PRs #92, #159, #166 merged today), needs-triage queue shrink (5→2), open PR state, #168 status (day 3, 0 comments), claude-review CI disable landing. Dominant theme: review session burst cleared the queue across both PRs and issues simultaneously.
Acted:
- Nothing to close or file. #168 at day 3, below 7-day threshold. No new second instances crossed.
Noticed (no action):
- **Queue flush in one burst**: PRs #92 (day 15), #159 (day 8), and #166 (day 9) all merged today, plus journal PR #83. The 9-day main-branch drought ended in a single session. This confirms the pattern from May 15: CI/infra improvements land when pain is visceral, and review sessions happen in concentrated bursts not steady trickles. The pipeline is healthy; the cadence is bursty.
- **Triage queue self-corrected (partially) in the same session**: needs-triage shrank from 5 to 2 items. Issues #122, #146, #167 — all clearly-actionable "blocked by: none" items — moved to `ready-for-agent` without any automated intervention. The two remaining items (#53 at 20 days, #124 at 13 days) are the architectural decision items that `/triage` would route to `ready-for-human`. This is a meaningful data point for #168: the automated drain would add value for the easily-routable items, but the hard ones (#53, #124) have sat 13–20 days because they need human architectural decisions, not mechanical routing. @claude tagging them would likely produce a `ready-for-human` label — accurate, but not sufficient to unblock them.
- **claude-review CI disable now on main**: PR #166 carried the switch from `push`-triggered `claude-review` to `workflow_dispatch`. The canary watch is now active — the first new non-AFK PR opened after today has no automatic review trigger. PR #144 (manifest extensions, day 12) is still open and was created before the disable landed, so it retains auto-review on updates. Watch whether #144 gets reviewed and merged quickly now that #166 cleared, or whether it stalls further.
- **#168 (triage drain) at day 3, 0 comments**: The partial self-correction today weakens the urgency case — the queue shrank without automation. But the remaining 2 items (#53, #124) illustrate a real gap: they've been in `needs-triage` 13–20 days and need human decision signals, not just routing. The proposal still makes sense for the next wave of easy items; not closing.
- **ready-for-agent queue now has 3 fresh items** (#122, #146, #167) after being empty. This is the natural trigger for an AFK epic kick-off. Watch whether a new AFK session starts in the next 1–2 days.
Hint for next run: watch whether a new AFK session starts against #122/#146/#167 (fresh ready-for-agent queue); check if PR #144 finally merges (day 12+, now the queue is clear); check whether any new non-AFK PR is opened without auto-review (canary for the claude-review disable). Also check #53 and #124 — if they hit day 21+ with no human comment, they're the actual bottleneck, not the triage drain.
