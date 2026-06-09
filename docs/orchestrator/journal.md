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
Hint for next run: if a non-AFK PR merges without any review comment (human or @claude) after claude-review was disabled, that's the second instance — make it a proposal; also check whether #152 has been acted on or whether the needs-triage queue has grown further.

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
Surveyed: PR #166 merge (day 8 → merged), queue flush (#136/#144/#159 all merged same day), #168 (triage drain, day 3), needs-triage routing burst. Dominant theme: designed pause broke; AFK pipeline resuming at pace.
Acted:
- Nothing to close or file. #168 at day 3 (stale threshold: June 2). No second instances crossed.
Noticed (no action):
- **PR #166 merged May 29** (canvas-stack-order). Queue flushed immediately: #136, #144, #159 all merged the same day. The 8-day designed pause resolved as expected once integration PR review was complete.
- **claude-review disable now live on main** (landed via #166). First non-AFK PR opened after the merge is the canary. None opened yet today.
- **Non-AFK review gap: canary not yet triggered.** Watch item is live; first new non-AFK PR is the signal.
- **needs-triage queue**: Lyle closed/routed several items in the May 29 review burst. Queue contracted; specific items need checking next run.
- **#168 (triage drain) at day 3**: Too early for a stall signal. Stale threshold June 2.
- **AFK resumed**: selection-reorder epic (#170–#176) merging same day.
Hint for next run: first non-AFK PR opened after #166 is the canary — check whether claude-review ran; check #168 at day 4 for any comment; audit needs-triage queue post-routing-burst.

---

## 2026-05-30
Surveyed: Non-AFK PR review canary (#181 opened post-#166), needs-triage queue post-routing-burst, proposal #168 (day 4), PR #176 (selection-reorder integration, same-day merge). Dominant theme: claude-review disable confirmed harmful for non-AFK PRs — canary triggered.
Acted:
- Nothing to close or file (watching for second instance to confirm before filing).
Noticed (no action):
- **Canary triggered: PR #181 opened post-disable, no claude-review.** `fix/toolbar-tools-and-draw-zorder` branch — non-AFK, opened May 30, merged May 31 in under 3 hours. CI shows only `check` and `fallow` jobs; no `claude-review`. This is instance 1 of the non-AFK review gap post-disable. One instance; need a second.
- **#168 (triage drain) at day 4, 0 comments**: Stale threshold June 2 (2 days). No movement.
- **needs-triage queue contracted to 3 items** post-routing burst: #53 (architectural residue), #124 (ADR revision required), and one new item. Queue is smaller but the remaining items are structurally blocked; an automated drain would route them to `ready-for-human`, not `ready-for-agent`.
Hint for next run: watch for a second non-AFK PR opened without claude-review (second instance → file the proposal); make the stale call on #168 by June 2.

---

## 2026-05-31
Surveyed: Second non-AFK PR post-disable (#184), proposal #168 (day 5), needs-triage queue. Dominant theme: second instance confirmed — proposal threshold crossed for non-AFK review gap.
Acted:
- #188 filed — restore claude-review auto-trigger for non-AFK branches. Instance 1: PR #181 (fix/toolbar-tools-and-draw-zorder, merged May 31, no claude-review). Instance 2: PR #184 (fix/annotate-tools-without-pages, follow-up to #181, merged June 1, no claude-review). Fix is an `if:` condition on the job — one line in the workflow YAML.
Noticed (no action):
- **#168 (triage drain) at day 5, 0 comments**: Stale threshold June 2 (tomorrow). The needs-triage queue post-routing-burst shows remaining items are structural residue (#53, #124) — an automated drain would route them to `ready-for-human`, accurate but not unblocking. Wrong target becoming clearer.
- **PR #184 is instance 2**: `fix/annotate-tools-without-pages` opened and merged June 1, non-AFK branch, no `claude-review` job in CI checks. Confirms the gap #188 targets.
Hint for next run: make the stale call on #168 by June 2; watch whether #188 attracts any comment or whether it follows the #152/#168 stall pattern.

---

## 2026-06-02
Surveyed: Proposal #168 stale threshold (June 2), #188 (day 1), needs-triage queue post-routing-burst audit, PR aging. Dominant theme: #168 stale call executed with post-mortem; #188 live.
Acted:
- #168 closed as `not_planned` (7 days, 0 comments). Post-mortem added to body: the needs-triage queue self-corrected partially (easy items routed by Lyle in the May 29 review burst); what remained (#53, #124) are architectural decisions requiring human time — an automated drain would route them to `ready-for-human`, which is accurate but not actionable without human review bandwidth. Wrong target, not wrong format. Not refiling.
Noticed (no action):
- **#188 (claude-review restore) at day 1**: One proposal in queue, day 1. Too early for stall assessment.
- **needs-triage queue at 3 items**: #53 (architectural undo, 24 days), #124 (pages select-first, 17 days), #190 (SVG paste, 1 day — fresh, bounded). #190 is the first genuinely agent-executable item in the queue since #167 was closed.
- **PR #182 (distribute selection) merged June 2** — non-AFK branch. CI shows: `check` green, `fallow` failure (pre-existing), no `claude-review` job. Instance 3 of the non-AFK review gap. Consistent with #188; not filing a new proposal (one already in queue).
Hint for next run: watch #188 traction (any comment in first 48h is a good sign); check whether #190 gets routed to ready-for-agent — it's the only bounded item in needs-triage.

---

## 2026-06-03
Surveyed: AFK wireframe-structured-editor epic (PRs #196–#203, all merged June 3 into integration PR #204), 0.3.1 release, proposal #188 (day 2, 0 comments), needs-triage queue (#190 at day 2, unrouted), deferral-without-tracking pattern in PR #204 body. Dominant theme: largest AFK epic to date shipped; all existing watch items stable.
Acted:
- Nothing to close or file. #188 at day 2, not stale. No second instances crossed for any watch item.
Noticed (no action):
- **Wireframe structured editor epic complete** (8 step PRs #196–#203 into #204, all merged June 3 in one session). CLI parity, Y.Doc integration, per-node property editing, disk import, broadcast-derive. 749 unit tests + 22 smoke tests green. Largest AFK epic delivered.
- **0.3.1 shipped June 3** — changelog includes open-in-new-tab, resumable annotation sessions, CI action version bumps.
- **Deferral-without-tracking pattern in PR #204 body**: two items explicitly deferred with "intentionally excluded" / "later deepening only" — structured Y types (§3.6/A2) and variant fan-out. No tracking issues filed yet. One instance of the deferral-without-tracking pattern; watch for a second.
- **#188 (claude-review restore) at day 2, 4 instances, 0 comments**: Fix is a one-liner in `.github/workflows/claude.yml`; the proposal body includes both implementation options. Stale threshold June 8. Per June 3 entry: if it stalls to day 7, the journal should reflect rather than refile — three consecutive stalls (after #63 and #168) would indicate the issue queue is the wrong forcing function for this orchestrator's proposals.
- **PR #144 (manifest extensions) at 18 days**: Manual test plan (physical `.svelte`/`.vue` file drop + Vite template) still unchecked. This is not the same gap #188 targets — it has been reviewed (claude-review ran green). The blocker is manual session time. Not a systemic leak at one instance.
- **needs-triage queue at 3 items**: #53 (26 days, markdown undo — architectural) and #124 (19 days, pages select-first — ADR revision required) are structural residue requiring human time, per Lyle's June 2 post-mortem. #190 (SVG paste, 3 days) is fresh and bounded — likely triage candidate this week.
- **ready-for-agent queue**: empty; no bounded items since #190 filed June 1. Pipeline is idle between epics, as expected.
Hint for next run: watch #190 routing (SVG paste — whether it gets triaged to ready-for-agent and kicks off a new epic); check #188 traction at day 5 (June 6) as an early signal before the June 8 deadline.

---

## 2026-06-05
Surveyed: PR #204 (wireframe integration, day 2), proposal #188 (day 4, 0 comments, stale threshold June 8), needs-triage queue (#190 at day 4, #53/#124 structural residue), PR #144 aging (day 19). Dominant theme: second designed pause holding; all watch items stable.
Acted:
- Nothing to close or file. #188 at day 4 (stale threshold June 8). No second instances crossed for any watch item.
Noticed (no action):
- **#190 (SVG paste rendering) at day 4, needs-triage, unrouted**: Bounded and agent-executable — natural next AFK epic candidate once triaged. Unlike #53/#124, no architectural decision blocks it. Not yet at the stall threshold (7 days); just watching for routing this session.
- **#188 (claude-review restore) at day 4, 0 comments**: Stable. June 8 is the decision point. The June 6 early-signal check (per the June 4 hint) is tomorrow — nothing to assess yet today.
- **PR #204 (wireframe integration) at day 2**: Designed pause; no movement expected before Lyle reviews. Deferral-without-tracking pattern (two items flagged in PR body: structured Y types §3.6, variant fan-out) still at one instance — no new AFK epics shipped since June 3 to confirm or deny a second.
- **PR #144 (manifest extensions) at day 19**: Manual test still unchecked. No change; one PR, not systemic. Not watching further.
Hint for next run: June 6 is the day 5 early-signal check for #188 (any organic resolution before the June 8 stale deadline?); also check whether #190 was routed to ready-for-agent and whether PR #204 merged or attracted review.

---

## 2026-06-06
Surveyed: Proposal #188 (day 5 early-signal check, 0 comments), PR #204 (wireframe integration, day 3, no review activity), #190 (SVG paste, day 5, needs-triage, unrouted), needs-triage queue (3 items unchanged). Dominant theme: all watch items frozen on the same trajectory; June 8 is the decision point for two of them simultaneously.
Acted:
- Nothing to close or file. #188 at day 5 (stale threshold June 8). No second instances crossed.
Noticed (no action):
- **#188 (claude-review restore) day-5 check: no organic resolution.** 4 confirmed instances, one-liner fix (`if: "!startsWith(github.head_ref, 'claude/')"` on the job), 0 comments. June 8 is the stale threshold and the pre-committed decision: close as stale, reflect rather than refile. Pre-draft of that reflection, written today so it is not composed under time pressure on June 8: three proposals stalling in the issue queue (#63 after 7 days → landed eventually from visceral CI pain; #168 after 7 days → closed, wrong target; #188 after 7 days → ?) is a pattern, not bad luck. The common thread: GitHub issues are a passive channel — they wait for the reviewer to come to them. The proposals that actually moved (#63) were pulled by a concrete, immediately-felt failure, not by an issue sitting in the queue. The self-modifying conclusion, if #188 stalls: change the output format for CI/workflow proposals — instead of a GitHub issue describing the fix, deliver the fix as a ready-to-merge branch with the diff pre-written. A PR requires a yes/no decision; an issue requires discovery + analysis + decision. One fewer step to merge.
- **#190 (SVG paste) at day 5, unrouted, bounded**: Previous bounded AFK candidates (#122, #146, #167) were routed to ready-for-agent in 1–2 days. #190 is now day 5 with no label change. Not yet at the 7-day threshold (June 8), and the pipeline is idle between epics so there is no pull. This is not a stall pattern yet — one instance, and the timing coincides with a deliberate pause (PR #204 in review). Watch whether it routes before or after #204 merges: if it routes immediately after, the pipeline-idle state explains the delay and there is no systemic issue. If #204 merges and #190 still sits, that is a different signal.
- **PR #204 (wireframe integration) at day 3**: Designed pause, no review activity since opening June 3. Deferral-without-tracking (structured Y types §3.6, variant fan-out flagged in body, no tracking issues filed) still at one instance. Not actionable.
- **PR #144 (manifest extensions) at day 20**: Dropping from active watch. Manual test pending is a single-PR waiting-for-session-time pattern; three consecutive entries noting it adds no information.
Hint for next run: June 8 is the simultaneous deadline for both #188 (stale: close + reflect on output format, not refile) and #190 (7-day unrouted threshold: if still needs-triage and #204 has merged, that is a signal worth a second look). Check both.

---

## 2026-06-07
Surveyed: Proposal #188 (day 6, stale threshold tomorrow June 8), PR #204 (wireframe integration, day 4, no review), #190 (SVG paste, day 6, needs-triage, unrouted), 4-day main drought. Dominant theme: all watch items frozen; dual threshold day approaches tomorrow.
Acted:
- Nothing to close or file. #188 at day 6 (threshold June 8); #190 at day 6 (threshold June 8). No second instances crossed.
Noticed (no action):
- **#188 (claude-review restore) hits stale threshold tomorrow (June 8).** Pre-committed decision from June 6: close as stale, do not refile. Self-modifying conclusion already drafted: for CI/workflow proposals, deliver a ready-to-merge branch (PR-first) rather than a GitHub issue. A PR requires a yes/no decision; an issue requires discovery + analysis + decision. One fewer step to merge is the difference between #63 (eventually landed) and #188 (stalled at 0 comments).
- **#190 (SVG paste) also hits 7-day threshold tomorrow (June 8).** Still needs-triage, still unrouted. PR #204 (designed pause) has not merged — pipeline-idle explains the routing delay. The June 6 conditional still holds: "if #204 merges and #190 still sits, that is a different signal." Not there yet.
- **PR #204 (wireframe structured editor) at day 4.** No review activity. Designed pause is working as expected — PR #166 (canvas-stack-order) took 9 days. Wireframe is the largest AFK epic delivered: CLI parity, Y.Doc integration, per-node property editing, disk import, broadcast-derive — all in one session June 3.
- **Release 0.3.1 shipped June 3** (19 days after 0.3.0). Healthy cadence. Two releases now using the current CI stack (#125 gate).
Hint for next run: June 8 — close #188 as stale (no re-file); check #190 routing signal after #204 merges; if #204 still open, note the designed pause duration and leave it.

---

## 2026-06-08
Surveyed: Proposal #188 (stale threshold today, pre-committed close), PR #204 (wireframe integration, day 5, unmerged), issue #190 (SVG paste, day 7, needs-triage, #204 still open), PR #206 (self-healing CLI loop, opened today), ADR 0018 (cloud sync/agents-as-peers, merged June 7). Dominant theme: dual threshold day with pre-committed actions executed; new loop harness type emerges.
Acted:
- #188 closed as `not_planned` — 7 days, 0 comments, pre-committed June 6. Body updated with the reflection.
- PR #207 opened (`orchestrator/fix-claude-review-auto-trigger`) — the claude-review fix delivered as a ready-to-merge branch instead of a new issue. This is the pre-committed self-modifying conclusion in action: a PR forces a yes/no merge decision; an issue requires discovery + analysis first.
- #208 filed — SELF-MODIFYING PROPOSAL: update orchestrator prompt so CI/workflow fixes (≤20 lines, pre-analyzed) are delivered as ready-to-merge branches, not GitHub issues. Flagged loudly as a prompt edit.
Noticed (no action):
- **#190 (SVG paste) at day 7, still needs-triage**: PR #204 has not merged — pipeline-idle explanation still holds. Per the June 6 conditional, "if #204 merges and #190 still sits, that is a different signal." That signal has not triggered.
- **PR #204 (wireframe structured editor) at day 5**: No review activity. Designed pause continuing as expected (PR #166 took 9 days). PR #204 base SHA is behind main by 3 commits (ADR 0018 + CI bump + camera glide) — may need a rebase before merge. Not blocking; just noting.
- **PR #206 (self-healing CLI loop, opened today)**: Domain-agnostic `harness/` + `packs/cli/` structure. A generalised improvement loop that dogfoods the CLI against the smoke app — measuring friction signals, fixing one thing per fire, committing the result. First new loop variant since the AFK loop stabilised. The smoke remains local-only ("a headless container can't drive electron-forge's app build") — same structural constraint from May 14, now intentionally managed as a local-only harness rather than blocked as a gap.
- **ADR 0018 (cloud sync, canvas sharing, agents as peers, merged June 7)**: Large architectural direction. Agents as first-class peers (no owner/client distinction), content-addressed encrypted sync, Merkle DAG sharing. The `.canvas` file's agent-readability (already a file format principle) becomes the foundational assumption for multi-agent collaboration. Largest ADR since 0014.
- **Deferral-without-tracking pattern watch**: PR #204 (structured Y types §3.6, variant fan-out) — one confirmed instance, still no tracking issues filed. PR #206 mentions a v2 deferral (rendering loop history into canvas) in the plan doc — uncertain whether this is the same pattern or expected plan-doc scope-setting. Watching.
Hint for next run: check if PR #207 (claude-review fix) merged or was ignored — that result answers whether PR-first delivery outperforms issue-queue for this class of fix; check #190 routing after #204 merges; check #204 base staleness if still open.

---

## 2026-06-09
Surveyed: PR #207 (claude-review fix, day 1), proposal #208 (PR-first self-modifying, day 1), needs-triage queue (4 items: #209 new day 1, #190 day 8, #124/#53 structural), PR #204 (wireframe integration, day 6). Dominant theme: proof-of-concept for PR-first delivery is live; all results are day 1, no conclusions yet.
Acted:
- Nothing to close or file. #208 at day 1 (stale threshold June 15). No new second instances crossed for any watch item.
Noticed (no action):
- **PR #207 (claude-review fix) at day 1**: The proof-of-concept for #208 is live — does a ready-to-merge branch force a faster yes/no decision than an issue in the queue? Too early to measure. Meaningful checkpoint: day 4–5.
- **#209 (live-refresh local-file entities, filed June 8) entered needs-triage**: "Blocked by: None — can start immediately." Explicitly cites `wireframe-watcher.ts` as the template. Bounded and agent-executable. Needs-triage now has two ready-for-agent candidates (#190, #209) alongside two structural residuals (#53, #124) — the highest pipeline-ready count seen in the queue at once.
- **#190 (SVG paste) at day 8, still needs-triage**: PR #204 still open — pipeline-idle explanation holds. The conditional from June 6 ("if #204 merges and #190 still sits, that is a different signal") has not triggered yet.
- **PR #204 (wireframe structured editor) at day 6**: Behind main by 3 commits (ADR 0018, CI bump, camera glide). Designed pause continuing — PR #166 (canvas-stack-order) took 9 days before merge. On expected trajectory.
Hint for next run: if PR #204 merges, check immediately whether #190 and/or #209 route to ready-for-agent — two bounded candidates at once is the pipeline's highest-readiness state yet; also check PR #207 at day 3–4 for early merge signal on the PR-first hypothesis.
