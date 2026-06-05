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
- **#152 stale warning**: Reaches 7-day threshold on May 26 (2 days). The problem it targets has not resolved — it has compounded. If it hits day 7 with no comment, the right call is to close-and-refile with the updated evidence (5 items, oldest at 17 days) rather than silently leaving a zombie open or closing without replacement.
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
- **7-day commit drought**: No commits to main since May 20. PR #166 (canvas-stack-order, 42 files, 17 commits, 6-item manual checklist all unchecked) remains the designed pause point. CI green. Expected.
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
Surveyed: PRs #159, #166, #92 (all merged today in a review burst), PR #136 status, proposal #168 (day 3), needs-triage queue. Dominant theme: designed pause broke; review burst cleared the backlog in a single session.
Acted:
- Nothing to close or file. #168 at day 3 (stale threshold: June 2). No second instances crossed.
Noticed (no action):
- **Review burst cleared the backlog**: PRs #166 (canvas-stack-order, 9-day pause), #92 (accessory mode smoke, 16 days), #159 (scale drawing strokes) all merged today. The integration PR designed pause resolved exactly as modeled — Lyle reviewed when ready, queue flushed. Also merged: prior journal PR #83 (consolidated May 13–28 entries).
- **PR #92 landing validates the pain-trigger model**: 16 days open, no comments, then merged in the same burst as #166. The accessory mode change was needed to support smoke tests running without stealing focus — #166's smoke test suite motivated the merge. Pain triggered, not calendar.
- **claude-review CI disable landed with #166**: From today, non-AFK PRs no longer get automatic claude-review passes. PR #136 (grid inspect) was opened before the disable; it merged today — but #136's CI was run *before* the disable landed, so it had an auto-pass. The canary is the next non-AFK PR opened *after* today.
- **needs-triage queue flushed partially**: #122, #146, #167 moved to ready-for-agent (Lyle routed them). Residue: #53 (20 days, markdown undo — architectural decision) and #124 (13 days, pages select-first — ADR revision required). These are not triage failures; they are human architectural decisions parked for bandwidth. Proposal #168's target has partially self-corrected.
- **#168 (triage drain) at day 3, 0 comments**: Stale threshold June 2. The queue it targets now has 2 items (down from 5). If #168 reaches day 7 with no comment, the correct closure note is "partially self-corrected; residue is human-decision items, not triage failures."
Hint for next run: watch for the first non-AFK PR opened post-disable to check whether claude-review fires; check #168 stale status (June 2 deadline).

---

## 2026-05-30
Surveyed: Non-AFK review canary watch (first PR opened after claude-review disable landed May 29), proposal #168 staleness (day 4), open PR aging. Dominant theme: canary fired, gap confirmed on first post-disable PR.
Acted:
- Nothing to close or file. #168 at day 4 (stale threshold: June 2). No second instances yet for the non-AFK review gap — canary fired once; one more needed.
Noticed (no action):
- **Canary PR #174 opened and merged today**: Branch `fix/canvas-bg-bg-colour`, Lyle's manual branch, open ~2 hours before merge. CI shows only `fallow=success` and `check=success` — no `claude-review` job ran. This is the first non-AFK PR opened after the May 29 disable. One instance confirmed; need a second to cross the proposal threshold.
- **PR #136 (grid inspect) clarification**: Merged on May 29 as part of the review burst. Its `claude-review` CI run completed on May 20 (before the disable). Not a post-disable instance.
- **needs-triage queue stable at 2 items**: #53 (21 days) and #124 (14 days). Both are architectural decisions; not automation targets. #168's residue target shrank to these two — both require human bandwidth, not a scheduler. The proposal's window is narrowing.
- **#168 stale deadline: June 2 (2 days)**: At current trajectory (0 comments, problem partially self-corrected), the closure note is already written. No reason to refile unless the queue rebounds to 3+ new items before then.
- **ready-for-agent refilling**: #122, #146, #167 all moved from needs-triage to ready-for-agent on May 29. AFK pipeline should pick these up next session.
Hint for next run: watch for a second non-AFK PR without claude-review; check #168 status on June 2 (stale deadline).

---

## 2026-05-31
Surveyed: Open product PRs (#136 merged May 29, #144 at day 14, #32 at day 37), fallow CI status on open PRs, proposal #168 (day 5, 0 comments), needs-triage queue (#53/#124). Dominant theme: non-AFK review gap building evidence; fallow soft-check surfacing real issues in open PRs.
Acted:
- Nothing to close or file. #168 at day 5 (stale threshold: June 2, 2 days). One post-disable non-AFK PR confirmed; one more needed before proposing.
Noticed (no action):
- **Fallow failures on all 4 open product PRs** (#144, #32, and two others): fallow=FAILURE across the board. This is the soft-check doing its job — surfacing issues without blocking. Pattern: fallow finds real things (dead exports, unused deps), but soft-gate means they accumulate until a cleanup pass. Not a leak; the lifecycle (fail → accumulate → cleanup burst) is working. One cleanup burst already confirmed (PR #145, May 18).
- **Non-AFK review gap at one confirmed instance (PR #174)**: Need a second before filing. If a non-AFK PR merges in the next 48 hours without claude-review, that's the second instance.
- **#144 (manifest extensions) at day 14**: Manual test plan still unchecked. Oldest non-AFK non-draft open PR. Has `claude-review` run and pass (pre-disable); the missing piece is Lyle's physical test with a .svelte/.vue file. Not a review gap — it's a manual-verification gap. Different category.
- **ready-for-agent queue**: #122, #146, #167 all moved to ready-for-agent on May 29. No new AFK session started yet. Pipeline idle.
Hint for next run: check whether PR #181 (or any non-AFK PR opened since May 29) merged without claude-review — that's the second instance and crosses the proposal threshold.

---

## 2026-06-01
Surveyed: Non-AFK PRs #181 and #184 (both merged without claude-review, second instance confirmed), proposal #168 staleness (day 6, stale tomorrow), needs-triage queue (#53/#124 plus new #190). Dominant theme: non-AFK review gap crosses two-instance threshold; proposal filed.
Acted:
- #188 filed — restore claude-review auto-trigger for non-AFK PRs. Two instances: #181 (fix/toolbar-tools-and-draw-zorder, merged May 31, immediate follow-up #184 needed for a missed code gate) and #184 (fix/annotate-tools-without-pages, June 1, unreviewed). Fix is a one-liner: job-level `if: "!startsWith(github.head_ref, 'claude/')"` in `.github/workflows/claude.yml`.
Noticed (no action):
- **#181→#184 immediate-follow-up pattern**: PR #181 merged with a bug (sanitizeForPages guard collapsed after toolbar gate removal). #184 was immediate follow-up to fix it. This is exactly the kind of second-gate issue a review pass might have surfaced. Direct evidence, not hypothetical.
- **#168 stale tomorrow (June 2)**: 0 comments, problem partially self-corrected. Closure note: "partially self-corrected; #53 and #124 are human-decision architectural items that a drain would route to ready-for-human — accurate but not actionable without human bandwidth."
- **needs-triage queue**: #190 filed today (SVG paste rendering, follow-up to #177). Queue: #53 (23 days), #124 (16 days), #190 (fresh). #190 is bounded and agent-executable — natural next AFK epic candidate once triaged.
- **ready-for-agent**: #122, #146, #167 in queue. AFK session imminent.
Hint for next run: check whether #168 was closed (stale deadline June 2); check if a new AFK session started against ready-for-agent items; watch for additional non-AFK PRs merging without claude-review to add to #188's evidence base.

---

## 2026-06-02
Surveyed: Proposal #168 staleness (day 7 — stale threshold hit), AFK session PRs #177–#189 (concentrated review burst June 1), proposal #188 (day 1, 0 comments), needs-triage queue (#53/#124/#190). Dominant theme: AFK pipeline re-activated; #168 closed per stale policy.
Acted:
- #168 closed as `not_planned` (7 days, 0 comments). Post-mortem from Lyle's closure comment: "partially self-corrected; #53 and #124 are architectural decisions requiring human time — an automated drain would route them to `ready-for-human`, which is accurate but not actionable without human review bandwidth. Wrong target, not wrong format. Not refiling."
Noticed (no action):
- **Fallow watch item resolved**: All 4 PRs that showed fallow=FAILURE in the May 31 entry have now merged. PR #185 (ResizeEdge rename, filed June 1) was explicitly a fallow cleanup. The "install → CI fail → cleanup" lifecycle completed a second time — not decorative. Watch item closed.
- **#188 (claude-review restore) at day 1, 0 comments**: One-liner fix; too early to judge traction. This is the only live proposal. If it reaches day 7 (June 8) with 0 comments, the stall pattern from #63 and #168 has a third data point and the question becomes whether GitHub issues are the wrong forcing function for any proposal.
- **PR #144 (manifest extensions) at day 16**: Oldest open non-draft PR. Manual test plan (SVG/Vue files) still unchecked. Second-oldest non-draft open PR is #32 (LM Studio, day 39, predates the Specular rename). Neither is a systemic pattern; both are Lyle's product decisions.
- **June 1 burst merged 4 PRs** (#177, #185, #187, #189): the consistent pattern of concentrated review sessions between quiet stretches continues. The pipeline produces, Lyle reviews in burst, pipeline produces again. This rhythm is stable and by design.
Hint for next run: check if #190 (SVG paste) was triaged and whether the ready-for-agent queue refilled; check #188 traction (day 7 = June 8 stale threshold); watch whether #182/#179/#178 merged cleanly or if any AFK fix branch showed the review gap #188 targets.

---

## 2026-06-03
Surveyed: Check runs on non-AFK PRs #191 and #195 (both merged June 2–3), 0.3.1 release burst (#191–#195), needs-triage queue (#53/#124/#190), proposal #188 (day 2, 0 comments), ready-for-agent state (empty). Dominant theme: non-AFK review gap confirmed on two more PRs; pipeline re-idle after 0.3.1.
Acted:
- Nothing to close or file. #188 at day 2, below 7-day threshold. No stale orchestrator output.
Noticed (no action):
- **Non-AFK review gap: #191 and #195 confirm pattern (now 4 instances total).** PR #191 (`fix/note-wheel-scroll`, Lyle's branch, open 3 hours June 2) shows only `fallow=success` and `check=success` — no `claude-review`. PR #195 (`ci/node24-bump`, June 3) identical. Proposal #188 documented the first two instances (#181, #184); these add two more. No regression visible from either PR (both well-tested by Lyle), but the structural absence is confirmed. At 4 instances, the pattern is settled.
- **0.3.1 shipped in a concentrated burst**: PRs #191–#195 landed June 2–3 — "idle" to a tagged minor release in ~12 hours. AFK pipeline contributed #193 (new-tab links as frames) and #194 (resume Claude sessions across annotation replies). Lyle contributed #191 (scroll + browser-mode shell fix) and #195 (CI Node.js 24 bump). Burst–quiet–burst cadence continues; the system is healthy.
- **ready-for-agent empty; pipeline re-idle**: #190 (SVG paste rendering, needs-triage, now 2 days old) is the only fresh bounded item one triage step away from refilling the queue.
- **needs-triage structural residue unchanged**: #53 (25 days, markdown undo requires architectural choice) and #124 (18 days, pages select-first model, "HITL: must update ADR 0001"). Both require a human architectural decision. No automation unblocks them; not watching further unless they produce a downstream consequence.
- **#144 (manifest extensions) at day 17**: One manual test plan checkbox still unchecked. PRs requiring a physical test session (Vue/Svelte/SVG components) reliably stall until Lyle has time and a suitable project to test against. Not a systemic leak — one PR.
- **#188 stale threshold: June 8.** If it reaches day 7 with 0 comments, that is a third proposal stalling in the issue queue after #63 (landed from visceral pain) and #168 (closed, wrong target). At that point the question becomes whether GitHub issues are the wrong forcing function for any proposal from this orchestrator — and the journal entry on June 8 should reflect rather than refile.
Hint for next run: check if #190 (SVG paste) was triaged and the ready-for-agent queue refilled; check #188 traction at day 5 as an early signal before the June 8 deadline; if a fifth non-AFK PR merges without claude-review before that, note it but don't refile — the evidence is already sufficient.

---

## 2026-06-04
Surveyed: PR #204 (wireframe-structured-editor integration, day 1), PR aging (#144 at 18 days, #32 at 41 days), proposal #188 (day 3, 0 comments, 4 confirmed instances), needs-triage queue (3 items). Dominant theme: next designed pause point open; pipeline at rest; one live proposal.
Acted:
- Nothing to close or file. #188 at day 3 (stale threshold: June 8). No second instances crossed for any new pattern.
Noticed (no action):
- **PR #204 (wireframe-structured-editor) opens the next integration cycle**: 8 step PRs, 749 unit tests, 22 smoke tests — the most complete AFK epic to date. Agent CLI parity (`specular wireframe [op]`), Y.Doc-backed ops, external `.wireframe.json` watcher, broadcast-derive renderer. PR body flags two intentional deferrals (structured Y types §3.6, variant fan-out) — both documented in the PR body but no tracking issues filed yet. One instance of the deferral-without-tracking pattern; watch for a second.
- **#188 (claude-review restore) at day 3, 4 instances, 0 comments**: Fix is a one-liner in `.github/workflows/claude.yml`; the proposal body includes both implementation options. Stale threshold June 8. Per June 3 entry: if it stalls to day 7, the journal should reflect rather than refile — three consecutive stalls (after #63 and #168) would indicate the issue queue is the wrong forcing function for this orchestrator's proposals.
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
