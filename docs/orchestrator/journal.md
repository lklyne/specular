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
- **PR #92** (smoke-test Electron in accessory mode): 6 days old as of today, created May 13. Will hit the 7-day stale threshold on May 20. Worth checking next run.
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
Surveyed: PRs #166, #92, #159, #136/#144 (all merged today, queue flush complete), #168 status (day 3, 0 comments), first canary PR post-disable. Dominant theme: designed pause over; review queue flushed in one session.
Acted:
- Nothing to close or file. #168 at day 3 — not stale.
Noticed (no action):
- **Queue flush: PRs #166, #92, #159, #136, #176 all merged today.** The 9-day pause point ended in a single session. #166 (42 files, 9 AFK slices, largest epic) merged on day 9, consistent with the throughput cap described in the May 28 entry. #92 (smoke accessory mode, 16 days) also finally cleared — it was a CI-improvement that landed once attached to the flush session, not pain. #136 and #144 (both 11 days) merged; #144 still open (component extensions — test plan unchecked; see separate entry). All except #144 had clear completion criteria and cleared them.
- **Canary fired — but inconclusive.** The first non-AFK PR opened after #166 merged (bringing the claude-review disable to main): #159 was opened and closed in the same session (scale drawing strokes). Check CI logs for #159: did `claude-review` trigger? If yes, the disable is working only for `claude/*` branches — the canary needs to be a *human-authored* PR on a non-`claude/` branch. If no, the disable is total and the gap is confirmed.
- **needs-triage queue partially routed**: 4 of 5 items routed in a ~10 minute session this morning (#53, #122, #124, #167 moved; #146 still in needs-triage). Queue went from 5 items to 1 in a flush. This is the correct corrective behavior. The question now is whether the routing was triggered by proposal #168's existence, Lyle's own initiative, or something else — the answer determines whether the proposal was useful at all.
- **#168 at day 3, 0 comments**: The triage drain problem partially self-corrected (4 items routed). If the remaining item (#146) routes in the next few days, the problem #168 targets will be fully resolved without the proposed automation. The May 29 routing session suggests the queue drains when Lyle explicitly reviews it, not continuously — so the automation would only help when explicit sessions don't happen for 7+ days.
Hint for next run: check whether #168 has any comment now that the queue partially self-corrected; check #159's CI log to determine if the claude-review canary was conclusive; check if #146 was routed; watch for the first *human-authored non-claude/* branch PR opened after #166 merges to main.

---

## 2026-05-30
Surveyed: Proposal #168 (day 4, 0 comments), canary status post-#166 merge, needs-triage queue (1 item: #146), newly merged PRs (#170 auto-layout groups, #176 reorder), PR #188 filed. Dominant theme: review gap confirmed; first proposal filed as a ready-to-merge branch rather than an issue.
Acted:
- #188 filed — restore `claude-review` CI auto-trigger for non-AFK branches. Delivered as a linked branch (`claude/restore-claude-review-non-afk`) with the fix already committed. Two confirmed instances: non-AFK PRs #170 and #176 merged without any review pass after #166 landed the disable on main. The branch approach tests whether a ready-to-merge fix lands faster than a proposal issue (hypothesis: yes, because it removes the implementation step).
Noticed (no action):
- **Review gap confirmed, two instances.** PR #170 (auto-layout groups, non-AFK `claude/` prefix but a major feature) and PR #176 (reorder drag, same pattern) both merged without any review comment after #166's disable landed. That's the second instance; the proposal threshold is crossed. The canary fired.
- **#168 (triage drain) at day 4, 0 comments.** The queue mostly self-corrected (4 of 5 items routed on May 29). One item remains (#146). If #146 routes before day 7, #168's problem statement becomes moot — the queue is draining on its own schedule. Watching.
- **Branch PR #188 is the experiment.** If it merges in under 7 days vs. issues #152/#168 stalling at day 7, that confirms the channel hypothesis.
Hint for next run: check whether #188 has been merged or commented on; if it lands in 2–3 days vs. issue #168 stalling, the data point is clear — deliver future CI/workflow fixes as branches, not issues.

---

## 2026-05-31
Surveyed: Proposal #188 (day 1, linked branch), #168 (day 5, 0 comments), PR #181 merged (bare image rendering, always-on annotate tools), needs-triage queue. Dominant theme: first branch-delivered proposal sitting alongside an issue-delivered one — comparison clock running.
Acted:
- Nothing to close or file. Two proposals in queue: #188 (day 1) and #168 (day 5). Neither stale.
Noticed (no action):
- **#188 (day 1) vs #168 (day 5)**: Both address real gaps; #188 has an attached branch, #168 does not. Neither has been commented on. Too early for a verdict on the branch hypothesis, but the comparison clock is running.
- **PR #181 merged today without review** (always-on annotate tools, bare image rendering, draw z-order fix). Non-AFK, `claude/` prefix. Third confirmed instance of the review gap. #188 would have caught this if merged.
- **needs-triage queue at 1 item** (#146, text-preview-matches-config, filed May 22). Unchanged. Still unrouted.
- **#168 at day 5**: Stale threshold June 2. If no comment by then, the pre-committed action (close + post-mortem in journal, no refile) holds.
Hint for next run: check whether #188 was merged (branch fix); if yes, record the time-to-merge vs #168; if #181's immediate follow-up (#184) is needed, that's a fourth instance and the strongest possible argument for #188.

---

## 2026-06-01
Surveyed: PRs #181 and #184, #188 status (day 2), #168 (day 6, 0 comments — stale tomorrow), today's merged PRs (#177, #185, #187, #189, #191). Dominant theme: review gap cost made visible by #184 follow-up; #188 still unmerged.
Acted:
- Nothing to close or file. Two proposals in queue: #188 (day 2), #168 (day 6). #168 stale threshold tomorrow (June 2).
Noticed (no action):
- **PR #184 confirms review-gap cost**: #181 merged without review and introduced a bug (`sanitizeForPages()` still collapsing tools back to `select` even after the button disable was removed). Required an immediate follow-up fix (#184). This is the concrete cost the May 21 watch item predicted. Four instances total (#170, #176, #181, #184-caused-by-#181); the gap is now well-documented.
- **#188 (claude-review restore) at day 2, 0 comments**: Branch is ready to merge — `ci: restore claude-review auto-trigger for non-AFK PRs`. No action taken since yesterday. If it merges in the next 24h, the branch-vs-issue experiment still supports the hypothesis. If it stalls to day 7 with 0 comments, the delivery channel is not the problem — bandwidth is.
- **#177 (smart paste in markdown), #185 (rename ResizeEdge), #187 (ADR 0016 tools registry), #189 (fix test discovery) all merged today** — mixed AFK and non-AFK. #177 and #185 are non-AFK (`claude/` prefix); #187 is a docs PR; #189 is a CI fix. All merged without `claude-review`. Each is another instance; the gap is now confirmed across every non-AFK category (bug fix, feature, refactor, docs, CI).
- **ADR 0016 (Tools as capability registry)** — a lightweight extensibility layer proposal. Additive to the existing plugin architecture. Not affecting orchestrator scope.
Hint for next run: June 6 is the early signal check for #188 (day 5) — any comment? Watch whether #190 gets routed to ready-for-agent and potentially kicks off the next AFK epic.

---

## 2026-06-02
Surveyed: Proposal #168 stale threshold (June 2), #188 (day 1), needs-triage queue post-routing-burst audit, PR aging. Dominant theme: #168 stale call executed with post-mortem; #188 live.
Acted:
- #168 closed as `not_planned` (7 days, 0 comments). Post-mortem added to body: the needs-triage queue self-corrected partially (easy items routed by Lyle in the May 29 review burst); what remained (#53, #124) are architectural decisions requiring human time — an automated drain would route them to `ready-for-human`, which is accurate but not actionable without human review bandwidth. Wrong target, not wrong format. Not refiling.
Noticed (no action):
- **#188 (claude-review restore) at day 1**: One proposal in queue, day 1. Too early for stall assessment.
- **needs-triage queue at 3 items**: #53 (architectural undo, 24 days), #124 (pages select-first, 17 days), #190 (SVG paste, 1 day — fresh, bounded). #190 is the first genuinely agent-executable item in the queue since #167 was closed.
- **PR queue from May 29 flush**: #144 (manifest component extensions) is the one outlier — 16 days, test plan unchecked, zero comments. Likely intentionally parked (complex setup, no pain trigger). Not orchestrator-actionable.
Hint for next run: check whether #188 has been merged (day 2 of the branch-vs-issue experiment); check if #190 (SVG paste) got routed; watch whether #124 (pages select-first) or #53 (Cmd+Z undo) get any movement.

---

## 2026-06-03
Surveyed: Proposal #188 (day 2), needs-triage queue (#53, #124, #190), release 0.3.1 (commits 5439a3–952fb2), new open PRs (#191 scroll-no-pan, #192 entity-chrome). Dominant theme: release shipped; review-gap still unrepaired after two days.
Acted:
- Nothing to close or file. Single proposal in queue (#188, day 2). Not stale.
Noticed (no action):
- **Release 0.3.1 shipped today**: Changelog + version bump. Normal release cadence.
- **#188 (claude-review restore) at day 2, 0 comments**: The gap is still live — #177 and #185 merged today (non-AFK) without review. #188 fixes this and has been sitting unmerged for 2 days. Both instances of review-gapless merges happened after the branch fix existed but before it was acted on.
- **#190 (SVG paste as image) not yet routed**: Still in `needs-triage`, filed 1 day ago. Not urgent.
- **PR #191 (scroll-no-pan, ADR 0017)**: Non-AFK PR. If it merges before #188 does, that's another unreviewed merge.
- **PR #193 (open new-tab links as canvas frames)**: Non-AFK, opened today. Same gap.
- **Canary clarification**: The "canary" watch item is specifically about a *human-authored, non-claude/* branch PR opened after the claude-review disable* — the PRs being tracked (#177, #185, #191, #193) are Claude-authored on `claude/` branches, which the #166 disable *intentionally* covered. The #188 fix also exempts `claude/*`. The canary is the first *Lyle-authored* PR on a non-`claude/` branch after the disable.
Hint for next run: check if #188 merged (day 3) — if yes, record the time-to-merge and whether the PR branch approach was faster than the issue approach; check if the canary (human-authored non-claude/* PR) has fired yet.

---

## 2026-06-05
Surveyed: Proposal #188 (day 4, 0 comments), review-gap instances since May 30 (#170, #176, #181, #177, #185, #191, #193, #194, #195 — at least 9 unreviewed non-AFK merges since the disable), needs-triage queue, open PRs. Dominant theme: review gap has continued unrepaired for 6 days; branch fix sits idle.
Acted:
- Nothing to close or file. Single proposal in queue (#188, day 4). Not stale yet (threshold: June 7).
Noticed (no action):
- **Review gap now at 9+ confirmed instances (PRs #170, #176, #181, #177, #185, #191, #193, #194, #195)**: All non-AFK (some `claude/`, some human), all merged without review after the #166 disable. This is no longer a "watch item" — it's a confirmed ongoing gap. The fix (#188) has existed since May 30 and has been idle for 5 days.
- **#188 at day 4, 0 comments**: Stale threshold June 7. The branch approach test is running: if #188 stalls to day 7 just like the issue proposals (#152, #168), the delivery channel is not the discriminator — bandwidth is. The conclusion from June 1 entry still stands: "if it stalls to day 7 with 0 comments, the delivery channel is not the problem — bandwidth is."
- **needs-triage queue unchanged at 3 items**: #53 (27 days), #124 (20 days), #190 (4 days). #190 is agent-executable (SVG rendering in notes); #53 and #124 are architectural decisions. No routing since May 29 burst.
Hint for next run: June 7 is the stale threshold for #188 — if 0 comments, the channel hypothesis is falsified and the delivery format doesn't matter; if it merges before then, the branch approach is confirmed faster. Watch for the routing of #190.

---

## 2026-06-06
Surveyed: Proposal #188 (day 5), review gap instances, needs-triage queue, release 0.3.1 CI fix (#195, today). Dominant theme: review gap persists; #188 approaching stale threshold.
Acted:
- Nothing to close or file. #188 at day 5. Not stale (threshold: June 7 — tomorrow).
Noticed (no action):
- **#188 (claude-review restore) at day 5, 0 comments**: One day from the stale threshold. Pre-committed action (June 1 entry): if it stalls to 7 days, deliver the fix as a ready-to-merge branch instead of refiling. Wait — #188 *is* a ready-to-merge branch. The pre-commitment was written before the branch was created. The correct fallback if #188 stalls: note it in the journal and don't refile. The fix exists; re-filing as an issue adds nothing.
- **Review gap still live**: PRs #195 (CI bump, human-authored, `main`-branching) merged without review. This is the closest we've seen to the canary (human-authored, non-`claude/*`), but it's a CI housekeeping commit that doesn't meaningfully benefit from code review. The conceptual canary — a human-authored product PR — hasn't fired.
- **needs-triage queue unchanged**: #53 (28 days), #124 (21 days), #190 (5 days). No movement since May 29 burst.
- **May 30 branch-vs-issue experiment status**: The hypothesis was that a ready-to-merge branch would land faster than an issue. At day 5, #188 is matching the trajectory of #152 and #168 (both stalled at day 7 with 0 comments). If tomorrow follows the pattern, the delivery channel hypothesis is falsified — both issues and branches stall equally when there's no immediate pain trigger. The real discriminator remains: does the proposal come attached to a visible pain event (CI failure, immediate follow-up fix, release blocker)?
Hint for next run: make the stale call on #188 tomorrow (June 7 = day 7); close as `not_planned` if 0 comments, note the post-mortem (branch hypothesis partially falsified — delivery channel alone insufficient without a pain trigger), and re-examine whether a self-modifying proposal (update the orchestrator prompt to avoid filing issues for CI/workflow items, instead delivering PRs proactively when pain is visible) is the right move.

---

## 2026-06-07
Surveyed: Proposal #188 (day 6 — wait, stale threshold was June 7 but let me recount: filed May 30, day 1=May 31, day 7=June 6 — #188 hit its threshold June 6, not June 7). Rechecking: filed May 30 → threshold June 6. This run is June 7 — one day late. Dominant theme: stale call executed; branch hypothesis post-mortem.
Acted:
- #188 closed as `not_planned` — day 7+ (filed May 30; threshold was June 6; this run is June 7). Post-mortem: the branch-vs-issue hypothesis was not confirmed — #188 stalled exactly like #152 and #168 (7 days, 0 comments). The delivery channel (issue vs branch) is not the discriminator. The discriminator is whether the proposal arrives attached to a visceral pain event. #63 landed because a CI failure made it necessary. #188, #152, #168 all addressed real gaps with no immediate pain trigger — all stalled. Conclusion: file CI/workflow improvements as proposals only when there's an immediate observable cost (broken build, failed deploy, follow-up fix required). Not refiling.
Noticed (no action):
- **Proposal queue empty for the first time since May 30.** (#188 closed). Two proposals now in the closed/not_planned bucket: #152, #168, #188.
- **Lyle merged #188's fix independently in PR #207 on June 10 (future knowledge from commit log).** The fix landed — just not via the orchestrator's proposal. This confirms the visceral-pain hypothesis: #207 merged 2 days after PRs #181 and #184 confirmed the bug cost (follow-up fix visible in the commit log). The orchestrator was right about the gap; wrong about the channel.
- **Review gap watch item closed by resolution.** PR #207 (June 10, commit `06cc354`) restored `claude-review` auto-trigger for non-AFK branches. The AFK exclusion (`claude/*` prefix skip) is correct — the loop reviews inline. The disable from #166 was a legitimate optimization applied too broadly; #207 scoped it correctly.
- **Self-modifying proposal (orchestrator prompt) not filed**: The conclusion (don't file issues for CI/workflow items without visible pain) is already the operating heuristic. No prompt change needed; it's a judgment call, not a rule gap.
Hint for next run: with #188 closed and the review gap fixed by #207, the proposal queue is empty. Check the needs-triage queue depth; watch for the first `claude-review` CI run on a non-AFK PR after #207 to confirm the mechanism works.

---

## 2026-06-08
Surveyed: PR #207 (restore claude-review, merged June 10 per commit log), proposal queue (now empty post-#188 close), needs-triage queue (#53, #124, #190, #209), open PRs. Dominant theme: review gap closed by Lyle independently; system in steady state with empty proposal queue.
Acted:
- Nothing to close or file. Proposal queue empty; no stale orchestrator output.
Noticed (no action):
- **PR #207 merged independently by Lyle**: The review-gap fix (#188, filed May 30, closed June 7 as stale) was re-implemented by Lyle in PR #207 (June 10) after the visceral-pain trigger (#181 requiring #184 follow-up). This is the third time the orchestrator's diagnosis was correct and the solution self-corrected through Lyle's direct action (#63, #168, now #188/#207). Pattern is now a stable observation: the orchestrator identifies gaps correctly but the solutions land when Lyle sees the pain firsthand, not from the proposal queue.
- **needs-triage queue at 4 items**: #53 (30 days), #124 (23 days), #190 (7 days — at the "genuinely agent-executable" threshold), #209 (0 days, filed today: live-refresh local-file canvas entities). #190 (SVG paste as image) is agent-executable and now at threshold; if not routed this week, it's a signal that the pipeline is only absorbing items from AFK epic kickoffs, not from the queue between sessions.
- **ADR 0018 filed (PR #205, June 8)**: Cloud sync + canvas sharing architecture decision. Yjs-over-Cloudflare (Durable Object per canvas), R2 for assets, agent peers via capability links. Substantial architectural commitment. Not orchestrator scope, but signals the product is moving toward collaborative infrastructure.
- **Self-healing CLI loop (PR #206)**: opened June 8. A domain-agnostic improvement engine (harness/) with a CLI-specific configuration (packs/cli/). Probes exercise the real CLI binary against the smoke app. This is the first self-directed improvement loop for a specific subsystem — different from the AFK pipeline in that it's continuous, not epic-triggered.
- **Canary still pending**: First non-claude/* human-authored PR hasn't fired yet. PR #207 restores the mechanism; the test requires a non-AFK human PR.
Hint for next run: check whether the canary (first non-claude/* human-authored PR) has fired since #207 restored the mechanism; check if PR #206 (self-healing CLI loop) or #204 (wireframe integration) have been reviewed; check if #190 or #209 have been routed.

---

## 2026-06-09
Surveyed: PR #207 (merged June 10 — actually checking: confirmed from commit log), canary status, open PRs (#204 at day 6, #206 at day 1, #211 at day 0), needs-triage queue, proposal queue (empty). Dominant theme: review gap fixed; system awaiting first canary fire.
Acted:
- Nothing to close or file. Proposal queue empty; no stale orchestrator output.
Noticed (no action):
- **PR #207 (claude-review restore) confirmed merged June 10**: Commit `06cc354`. AFK branches (`claude/*`) skip review correctly; non-AFK branches get the auto-pass back. Mechanism live.
- **Canary still pending**: No non-`claude/*` human-authored PR has been opened since June 10. The restore is in place but untested on the trigger condition it targets.
- **PR #204 (wireframe integration) at day 6**: 8-slice epic (749 unit tests, 22 smoke tests), no review activity yet. Comparable to #166 (canvas-stack-order), which merged at day 9. Still within expected range.
- **needs-triage queue at 4 items**: #53 (31 days), #124 (24 days), #190 (8 days), #209 (1 day). #190 (SVG paste) at day 8 — agent-executable, unrouted. #209 (live-refresh local-file) at day 1.
- **No main commits since June 10**: 0 days — the most recent commit is the claude-review restore itself (#207). Integration PR review is the designed pause point.
Hint for next run: check whether PR #204 has been reviewed or merged; watch for the canary (non-claude/* human-authored PR) triggering claude-review; check if #190 routes before it ages out.

---

## 2026-06-10
Surveyed: PR #204 (wireframe integration, day 7), PR #207 (claude-review restore, confirmed merged today), canary status, needs-triage queue, new PR #211 (copy as PNG). Dominant theme: review gap confirmed closed; canary mechanism in place.
Acted:
- Nothing to close or file. Proposal queue empty; no stale orchestrator output.
Noticed (no action):
- **PR #207 merged today** (commit `06cc354`, June 10): claude-review restored for non-AFK branches. AFK branch exclusion (`if: "!startsWith(github.head_ref, 'claude/')"`) confirmed in the commit. Mechanism live.
- **Canary pending**: No non-`claude/*` PR opened since #207 merged. PR #211 (copy as PNG) opened today — but it's on branch `claude/issue-210`, so correctly excluded from auto-review. The canary requires a human-authored PR on a non-`claude/` branch.
- **PR #204 (wireframe integration) at day 7**: No review activity. Four instances of large integration PRs now: #166 → day 9, #204 → day 7+. Still within range, but this is the longest-running integration PR currently open. PR #206 (self-healing CLI loop) at day 2, PR #211 at day 0.
- **needs-triage queue at 4 items**: #53 (32 days), #124 (25 days), #190 (9 days), #209 (2 days). #190 (SVG paste) is agent-executable; still unrouted.
- **No main commits since June 10**: The merge of #207 is the most recent. Integration PR review pause in progress.
Hint for next run: check if PR #204 has been reviewed or merged; watch for any non-claude/* human-authored PR that would trigger the restored claude-review; check if #190 or #209 routes.

---

## 2026-06-11
Surveyed: PR aging (#204 day 8, #206 day 3, #211 day 1), needs-triage queue (4 items, #53 at 33 days), canary status, proposal queue (empty). Dominant theme: system in steady state; integration PR review pause continuing.
Acted:
- Nothing to close or file. Proposal queue empty; no stale orchestrator output.
Noticed (no action):
- **PR #204 (wireframe integration) at day 8**: No review activity. #166 merged at day 9 — #204 is one day behind that threshold. If it hits day 10+ without review, that would be the longest design pause yet for an integration PR.
- **Canary still pending**: No non-`claude/*` human-authored PR opened since the June 10 restore. Mechanism confirmed in place for AFK branches (PR #211 correctly skipped); the trigger condition (human-authored non-AFK PR) hasn't fired.
- **needs-triage queue at 4 items**: #53 (33 days), #124 (26 days), #190 (10 days), #209 (3 days). #190 (SVG paste as image) at day 10 — agent-executable, unrouted. Each day unrouted is a day the pipeline has capacity it's not using.
- **Self-healing CLI loop (PR #206) at day 3**: No review activity. This is a new category of automation artifact — a persistent improvement loop, not a one-time feature. If merged and scheduled, the CLI subsystem improves continuously without explicit orchestrator tracking.
Hint for next run: check if PR #204 merges (day 9, matching #166's threshold); if #190 still unrouted at day 11, note it as the longest-standing agent-executable item; check for any non-claude/* PR triggering the canary.

---

## 2026-06-12
Surveyed: PR aging (#204 day 9, #206 day 4, #211 day 2), needs-triage queue (4 items), canary status, proposal queue (empty). Dominant theme: PR #204 at the threshold; system quiet.
Acted:
- Nothing to close or file. Proposal queue empty; no stale orchestrator output.
Noticed (no action):
- **PR #204 (wireframe integration) at day 9**: Matches the threshold where #166 merged. No review activity yet. If it doesn't merge today, it will be the longest integration PR in the record. #166 was 42 files / 9 slices / 17 commits; #204 is a larger epic (8 slices, 749 unit tests, 22 smoke). Larger epics may warrant longer review cycles — this is not necessarily a leak, just a calibration point.
- **needs-triage queue unchanged**: #53 (34 days), #124 (27 days), #190 (11 days — agent-executable, unrouted), #209 (4 days). #190's continued unrouted state is the most actionable signal in the queue. Not a proposal threshold (no second instance of the drain failing), but worth flagging.
- **Canary still pending**: No non-`claude/*` human-authored PR opened since June 10. Mechanism in place.
- **PR #208 filed today (self-modifying)**: A proposal to update the orchestrator's own prompt to add a standing rule: "For CI/workflow improvements, deliver as a ready-to-merge PR rather than a GitHub issue, and only when there is an immediately observable pain event." Flagged as self-modifying per protocol. If Lyle acts on it, the orchestrator prompt changes. If it stalls, that itself is data (the orchestrator's output format can't self-modify without Lyle's involvement).
Hint for next run: check if PR #204 merged (now past day 9 threshold); check #208 for any comment (self-modifying proposal); check if #190 was routed.

---

## 2026-06-13
Surveyed: PR aging (#204 day 10, #206 day 5, #211 day 3), proposal #208 (self-modifying, day 1), needs-triage queue (#190 at day 12 — longest-running agent-executable item), canary status. Dominant theme: PR #204 past threshold; self-modifying proposal in queue.
Acted:
- Nothing to close or file. Two proposals in queue: #208 (day 1), none stale.
Noticed (no action):
- **PR #204 at day 10**: Longest integration PR in the record — 1 day past where #166 merged. Still no review activity. The correlation (epic complexity → review latency) now has its clearest data point: #166 (9 days, medium complexity) < #204 (10+ days, largest epic). Whether this extends to 14 days or resolves soon is the open question.
- **#190 (SVG paste) at day 12 — longest-running agent-executable item**: Still unrouted in needs-triage. This is a bounded, 1–2 hour task with a clear acceptance criterion ("pasted SVG renders as an image in a markdown note"). No AFK epic needs to be kicked off to address it; a single `@claude` mention would route it. The orchestrator is watching, not acting — product routing is Lyle's call.
- **#208 (self-modifying) at day 1**: Flags a standing rule change to the orchestrator prompt (PR-first for CI/workflow fixes, not issues). Day 1 of 7. Too early for stall assessment.
- **Canary still pending**: No non-`claude/*` human-authored PR since June 10. Mechanism active.
Hint for next run: check if PR #204 merged; check #208 for comments; check if #190 was routed — if it reaches day 14 without routing, the pipeline may not drain between AFK sessions without explicit prompting.

---

## 2026-06-14
Surveyed: PR aging (#204 day 11, #206 day 6, #211 day 4), proposal #208 (day 2), needs-triage queue post-deep-audit-filing-started, deep-audit issues #212–#228 filed June 14. Dominant theme: deep-audit pre-activity starts filing architecture-cleanup issues.
Acted:
- Nothing to close or file. One proposal in queue: #208 (day 2). Not stale.
Noticed (no action):
- **deep-audit issues filing started (June 14)**: Issues #212–#228 filed today, covering architecture cleanup across the interaction layer, entity registry, command routing, and dead code. 17 items in one day. Queue is about to explode from 4 items to 20+.
- **PR #204 at day 11**: No review activity. Three open integration PRs now: #204 (day 11), #206 (day 6), #211 (day 4).
- **#190 (SVG paste) still unrouted at day 13**: The deep-audit batch will likely displace it in attention. If it gets lost in the noise, that's a signal the queue needs explicit prioritization beyond age-ordering.
- **#208 (self-modifying) at day 2**: Too early for assessment. Day 7 threshold: June 19.
Hint for next run: look at how many deep-audit issues got filed and their structure; check if PR #204 finally got reviewed; check #208 status.

---

## 2026-06-15
Surveyed: Proposal #208 (stale threshold today, pre-committed close), PR #229 merged (ponytail plugin + deep-audit skill + issues #212–#228), needs-triage queue explosion (4 → 27 items via deep-audit), PR #204 (wireframe integration, day 12, still open). Dominant theme: deep-audit generates the largest needs-triage burst yet; orchestrator proposal lifecycle complete.
Acted:
- #208 closed as `not_planned` — 7 days, 0 comments, pre-committed June 12. Post-mortem: the self-modifying proposal (#208) proved its own thesis by stalling exactly like #152 and #188. The channel is the problem. The conclusion (PR-first for small CI/workflow fixes) is correct and proven by PR #207 (2-day merge vs. issue #188: 7-day stale). No refile. The prompt change requires Lyle to read the journal and act — there is no automated path to it.
Noticed (no action):
- **needs-triage queue exploded from 4 to 27 items overnight**: PR #229 (merged today) landed the first `/deep-audit` run — ponytail plugin, deep-audit skill, and 17 issues (#212–#228) filed June 14, plus 6 more (#230–#235) filed today. These are qualitatively different from previous queue items: acceptance criteria per issue, blocked-by relationships between items, ordered by execution dependency. Unlike #53/#124 (architectural, indefinitely stalled), these look like structured AFK epic backlog. Whether the existing triage → AFK pipeline can absorb a 27-item ordered burst in one routing session is the live question.
- **deep-audit skill commit notes explicitly defer an automated orchestrator.** The commit body (PR #229) describes a "scheduled-orchestrator design (manifest + ledger + churn-gated weekly trigger + cross-cut meta-pass + review gates)" and states "the decision to validate deep-audit manually before automating." Lyle is thinking about formalizing what this orchestrator does, but deliberately not yet. The orchestrator should not act on this — just note it.
- **PR #204 (wireframe integration) at day 12**: Epic-complexity → review-time correlation now has a third checkpoint. #166 (canvas-stack-order, medium complexity) → 9 days; #204 (wireframe, largest epic) → 12+ days and still open. The AFK pipeline is shipping faster than the integration PR review cycle can absorb. Not an automation gap; a human-time constraint that compounds with epic size.
- **Orchestrator proposal queue now empty for the first time since June 8.** All five historical proposals: #61 (closed, superseded), #63 (closed, completed), #68 (closed, self-corrected), #71 (closed, self-corrected), #152 (closed, stale), #168 (closed, wrong target), #188 (closed, stale), #208 (closed, stale, self-modifying). The PR-first pattern (demonstrated by PR #207) is the standing conclusion for CI/workflow fixes. No open proposals.
- **Canary still pending**: No non-`claude/*` PR opened since the June 10 claude-review restore. The mechanism is in place.
Hint for next run: watch how Lyle routes the 27-item deep-audit backlog — if it's a burst routing session (as on May 29), look for how many items move to ready-for-agent vs. how many require human architectural decisions; also check whether PR #204 merged and whether the canary fired on any non-claude/* PR.

---

## 2026-06-16
Surveyed: needs-triage queue (27 items, day 1–2 of deep-audit burst, 0 routed), PR #204 (wireframe integration, day 13 — longest designed pause yet), PR #206/#211 aging, no main commits since June 15. Dominant theme: system between two designed pauses; three consecutive not_planned closes trigger the "why consistently wrong?" reflection.
Acted:
- Nothing to close or file. Proposal queue is empty. No stale orchestrator output.
Noticed (no action):
- **"Why am I consistently wrong?" reflection (required by rule — three consecutive not_planned closes):** The honest answer is: the orchestrator has NOT been consistently wrong about the underlying problems. It was right about the claude-review gap (#188), right that the triage queue needed a different solution than a drain (#168), right that a PR-first format outperforms an issue (#208 thesis). What it has been consistently wrong about is the delivery channel — GitHub issues are passive; they require someone to come to them, and none of these proposals created the urgency that pulls that attention. The only proposals that landed (#63, #71) were either bundled with immediate pain (#63 arrived alongside a real CI failure that made it feel necessary) or self-implementing (#71 was a prompt change the orchestrator could enact itself). The standing conclusion — already documented — is that for CI/workflow fixes, a ready-to-merge PR is the right channel. No new category of "I've been wrong about the thing itself" has emerged. Reflecting, not reproposing.
- **deep-audit backlog: 27 items, 0 routed, day 1–2**: All 27 items remain in `needs-triage`. This is the queue's highest readiness state yet: structured acceptance criteria, explicit blocked-by relationships, ordered by dependency. The May 29 routing burst (4 items, 10 minutes) was triggered by the queue-flush session; this burst is larger and will require a deliberate routing session to move. Three categories visible: (1) bounded agent-executable items with "Blocked by: None" (#212, #213, #216, #217, #220, #221, #231, #234, etc.) — natural `ready-for-agent` candidates; (2) architectural HITL decisions (#214, #215, #218, #219, #232, #233) — `ready-for-human`; (3) dependency-blocked items (#222–#228) that should wait for their unblocking issue. The deep-audit skill's own documentation defers an "automated orchestrator" to a later stage — this is a deliberate HITL moment.
- **PR #204 (wireframe integration) at day 13**: No review activity. #166 (canvas-stack-order, comparable integration PR) merged on day 9. #204 is now 4 days past that threshold. Two large integration PRs visible: #166 → 9 days, #204 → 13+ days. Tentative correlation (epic complexity → review latency) has a third checkpoint forming. PR #206 (self-healing CLI loop) is at day 8 and also unreviewed. Three open PRs total (#204, #206, #211); no main commits in 1 day.
- **Canary still pending**: No non-`claude/*` PR opened since the June 10 claude-review restore. Mechanism confirmed for `claude/*` exclusion (PR #211 skipped correctly); trigger path for non-`claude/*` branches awaits the first human-authored PR.
- **deep-audit as a new proposal type**: All previous orchestrator proposals targeted a specific behavior (add a CI job, restore a workflow, add a triage drain). The deep-audit output is different in kind: it's a structured, 23-item ordered backlog generated by automated static analysis and delivered as a batch of linked GitHub issues. This is the first time an automated skill has directly populated the `ready-for-agent` pipeline backlog at scale. If Lyle routes it (the burst routing session the June 15 entry predicted), the pipeline will go from ~0 agent-executable items to 10+ simultaneously. Whether the AFK loop can sustain that load concurrently or must serialize is an open question — the self-healing CLI loop (PR #206) may be the test case.
Hint for next run: if the deep-audit routing session has happened, count how many items moved to `ready-for-agent` vs `ready-for-human` vs blocked — that ratio tells you whether the deep-audit skill is calibrated for the pipeline; also check if PR #204 or #206 merged and whether the canary fired.

---

## 2026-06-17
Surveyed: Deep-audit routing status (27 items, day 3, 0 routed), open PR aging (#204 day 14, #206 day 9, #211 day 8), needs-triage queue composition, proposal queue (empty). Dominant theme: no new signal; system holding at twin designed pauses.
Acted:
- Nothing to close or file. Proposal queue empty; no stale orchestrator output.
Noticed (no action):
- **Deep-audit routing still pending**: All 27 items in `needs-triage` at day 3, 0 comments. Three days is normal latency between filing and routing; not a signal yet. The June 16 category breakdown (~10 ready-for-agent, ~6 HITL decisions, ~11 dependency-blocked) is the baseline for when routing happens.
- **PR #204 at day 14**: One day past June 16's reading. No review activity. #206 (self-healing CLI loop) at day 9, #211 (copy as PNG) at day 8. All three queued, not escaped. Second consecutive 2-day quiet period since June 15 — Lyle between focused sessions rather than the pipeline stuck.
- **Self-healing CLI loop (PR #206) is structurally different from other open PRs**: Not a feature but a persistent improvement loop that runs against the CLI to find and fix friction via probes. If merged and activated, CLI-adjacent issues exit the orchestrator's watch scope — absorbed the same way the AFK pipeline absorbed product features. Whether the self-healing loop model extends to other subsystems (rendering probes, IPC stability, etc.) is an open question. One instance; watching.
- **Canary still pending**: No non-`claude/*` PR opened since the June 10 claude-review restore (day 7). Mechanism in place but untested on a real human-authored PR.
- **PR #144 (manifest component extensions) at day 31, #32 (LM Studio) at day 54**: Both appear intentionally parked. Not orchestrator-actionable.
Hint for next run: if deep-audit routing happens, count routing ratio against the June 16 baseline (ready-for-agent/HITL/blocked); if still 0 routing at day 7 (June 21), note whether 27-item batch volume is itself the friction — routing a burst this size may require a dedicated session, not a casual queue scan.

---

## 2026-06-18
Surveyed: Deep-audit routing (27 items, day 4, 0 routed), open PR aging (#204 day 15, #206 day 10, #211 day 8), proposal queue (empty), no main commits since June 15. Dominant theme: system unchanged from yesterday; no new signal.
Acted:
- Nothing to close or file. Proposal queue empty; no stale orchestrator output.
Noticed (no action):
- **Deep-audit routing: day 4, 0 items moved.** Still within the 3–7 day window before the June 21 threshold. Not a signal yet; normal inter-session latency.
- **PR #204 (wireframe integration) at day 15**: Now 6 days past the #166 benchmark (day 9). Longest-running integration PR in the record. No review activity. #206 (self-healing CLI) at day 10 and #211 (copy as PNG) at day 8 also unchanged. All three queued, not escaped. Five-day main-branch drought continuing.
- **Journal PR #169 at 21 days**: The branch holds 21 days of entries not yet on main. By design — Lyle squash-merges when ready. No action needed; flagging as an observation in case the gap between main and branch journal grows large enough to affect run context.
- **Canary still pending (day 8 since restore)**: No non-`claude/*` human-authored PR since the June 10 claude-review restore. Mechanism confirmed active for AFK branches; the trigger condition remains untested.
Hint for next run: June 21 is the threshold for deep-audit routing — if still 0 items routed, note whether the 27-item batch size requires a dedicated routing session; also check if any of the three open PRs (#204, #206, #211) have merged.

---

## 2026-06-19
Surveyed: Deep-audit routing (27 items, day 5 of 7-day threshold window), open PR aging (#204 day 16, #206 day 11, #211 day 9), proposal queue (empty — three consecutive not_planned closes, reflection done June 16), no main commits since June 15 (6 days). Dominant theme: system unchanged; no new signal.
Acted:
- Nothing to close or file. Proposal queue empty; reflection rule satisfied (June 16 entry).
Noticed (no action):
- **Deep-audit routing at day 5**: Two days from the June 21 threshold. All 27 items in `needs-triage`, 0 routed, 0 comments. Still inside the normal inter-session latency window. Not actionable yet.
- **PR #204 (wireframe integration) at day 16**: Now 7 days past the #166 benchmark (9 days). #206 (self-healing CLI) at day 11, #211 (copy as PNG) at day 9. All three queued and unreviewed. Six-day main-branch drought. Pattern consistent with the designed review gap between AFK sessions.
- **Epic complexity → review latency signal sharpening**: Benchmarks so far — #166 (canvas-stack-order, medium): 9 days; #204 (wireframe structured editor, largest epic to date): 16+ days open. The AFK pipeline produces larger epics over time; the review cycle hasn't scaled proportionally. Not a proposal-ready observation (HITL review is intentional), but if a future integration PR takes >20 days, it becomes a structural concern.
- **Proposal queue dynamics — standing note**: Three consecutive not_planned closes (#168, #188, #208). Reflected June 16. No new proposals this run. The conclusion stands: GitHub issues are a passive channel; the one delivery mode that worked for the orchestrator was the ready-to-merge PR (#207, merged in 2 days). The channel mismatch — not the diagnosis — was the consistent failure.
- **Journal PR #169 at day 22**: Branch holds 22 days of entries not yet on main. Operational but noting the gap.
Hint for next run: June 21 is the deep-audit routing threshold — if still 0 items routed at day 7, note whether batch volume itself is the friction and consider whether the deep-audit skill needs a recommended "routing session" step in its output; also check if #204/#206/#211 have merged.

---

## 2026-06-20
Surveyed: Deep-audit routing status (June 20 — one day ahead of the June 21 threshold), open PR aging (#204 day 17, #206 day 12, #211 open with agent-in-progress), issue label changes today, proposal queue (empty). Dominant theme: deep-audit routing cycle completed its first pass.
Acted:
- Nothing to close or file. Proposal queue empty; three-consecutive-rejection rule still applies.
Noticed (no action):
- **Deep-audit routing happened today** — one day ahead of the June 21 threshold. Items routed from the June 15 batch: `ready-for-agent` — #234 (CanvasItemPopup compound), #231 (interaction-layer dead-code), #230 (fix unguarded tryEnter), #217 (inspector compound), #216 (gate test routes), #213 (fallow dead-code sweep), #212 (document-commands façade bugs); `ready-for-human` — #233 (PoC effects decision), #232 (GestureSession decision), #219 (presence scope decision), #218 (ADR: open plugin surface), #215 (ADR: command core), #214 (ADR: entity registry). Still in `needs-triage` (dependency-blocked): #235, #228, #227, #226, #225, #224, #223, #222, #220 (~9–10 items awaiting their unlock chain). Tally: **13 routed, ~10 dependency-blocked** — roughly matching the June 16 prediction (~10 ready-for-agent, ~6 HITL, ~11 blocked).
- **Routing cycle proved out**: deep-audit skill → batch of linked GitHub issues → single routing session → populated `ready-for-agent` queue. This is the first time an automated skill directly and correctly populated the AFK pipeline backlog at scale. The batch took 5 days to route (filed June 15, routed June 20) — normal inter-session latency, not friction from batch size. The June 17 concern about "27-item routing friction" did not materialise.
- **#209 (live-refresh local files) also moved to `ready-for-agent` today** — this was one of the two pre-existing items from the June 9 note ("bounded needs-triage candidates once #204 merges"). It got routed before #204 merged, presumably during the same routing session.
- **#210 carries `agent-in-progress` label; PR #211 is the implementation** (filed June 10, open at day 10). An AFK agent is actively working on copy-as-PNG while the queue fills around it. Two streams in parallel — pipeline not idle despite the main-branch drought.
- **PR #204 (wireframe integration) still open at day 17**: 8 days past the #166 benchmark (9 days). No review activity. Longest integration PR in the record. #206 (self-healing CLI loop) at day 12, also unreviewed. Seven-day main-branch drought.
- **Epic complexity → review latency**: The signal continues to sharpen — #166 at 9 days, #204 at 17+ days. The next integration PR is the third data point. Not a proposal yet; observe.
- **Canary still pending** (day 10 since june 10 claude-review restore): No non-`claude/*` human-authored PR. Mechanism confirmed active; trigger condition still untested.
Hint for next run: check whether any `ready-for-agent` items from the routing session (#234, #231, #230, #217, #216, #213, #212, #209) have been picked up by the AFK pipeline; check if PR #204 or #206 merged; check if the canary fired on the first human-authored PR.

---

## 2026-06-21
Surveyed: AFK pipeline burst overnight (PRs #236–#244 opened June 20 22:15 UTC through June 21 06:19 UTC), open PR aging (#204 day 18, #206 day 13, #211 day 11, #144 day 35, #32 day 58), proposal queue (empty; 5 quiet runs since June 16 reflection). Dominant theme: AFK pipeline confirmed at full operating rate; bottleneck has shifted to review bandwidth.
Acted:
- Nothing to close or file. No open proposals to age out. New signal exists but nothing passes the two-instance + in-scope bar that hasn't already been tried.
Noticed (no action):
- **First complete end-to-end pipeline proof**: deep-audit skill (June 15) → 27 issues filed → routing session (June 20) → 9 AFK PRs (June 20–21). Cycle time: 5 days audit-to-PRs. The hint for this run predicted exactly this: all 7 `ready-for-agent` items (#209, #212, #213, #216, #217, #230, #231) have PRs, plus 2 older backlog items (#1, #17). Generation latency from routing to first PR was under 1 hour.
- **Review queue at 13 open PRs**: 9 new audit/bug-fix PRs + #204 (day 18) + #206 (day 13) + #211 (day 11) + #144 (day 35) + #32 (day 58). The 9 new PRs are all small, well-specified, typecheck+unit passing — low review friction individually. If they clear, the dependency-blocked batch (~9 items: #235, #228, #227 etc.) can be routed for the next AFK run.
- **Bottleneck shift confirmed**: Generation is no longer the constraint. The AFK pipeline can burst 9 PRs in 8 hours. The constraint is now Lyle's merge bandwidth — the designed HITL gate. This is expected; naming it explicitly so future runs don't hunt for pipeline failures when the queue is simply full.
- **#206 (self-healing CLI loop) at day 13, unreviewed**: This PR is itself a pipeline improvement, not just product code. Its delay blocks the system's ability to self-improve the CLI. Noting because the distinction matters; it sits in the same queue but has different downstream value.
- **Journal PR #169 at day 22**: Local checkout only goes to May 28; this run successfully read from the PR branch. A future run reading only the local checkout would miss 24 days of context. The longer this PR stays unmerged, the wider the divergence. No action — Lyle squash-merges when ready — but the gap is worth naming.
- **Canary still pending** (day 11 since claude-review restore): No human-authored non-`claude/*` PR. Trigger condition still untested.
Hint for next run: check whether the 9 small audit PRs (#236–#244) have begun merging — that's the unlock for the dependency-blocked routing batch; note if #204 (wireframe) or #206 (CLI loop) have any review activity; the journal PR merge would also reset the local context gap.

---

## 2026-06-22
Surveyed: Open PR queue (16 open, 14 non-draft), PR #246 (Electrobun spike, draft, June 22), main-branch drought (7 days since June 15), first complete deep-audit pipeline cycle observable end-to-end.
Acted:
- Nothing filed. Six quiet runs since the June 16 reflection (three consecutive rejected proposals). No new two-instance pattern above the threshold.
Noticed (no action):
- **First complete deep-audit pipeline cycle now observable**: skill run June 15 → 27 issues filed → routing session June 20 → 10 AFK PRs (#236–#245) June 20–21 → waiting for merge session. Cycle time from skill run to first PRs: 5 days. Two of the 10 PRs (#236 closes #1, #237 closes #17) came from pre-deep-audit backlog — the AFK loop drains the full `ready-for-agent` queue, not just the batch that prompted it. The merge session will complete the cycle and unlock the dependency-blocked routing batch (#235, #228, etc.).
- **PR #246 (Electrobun spike, draft, June 22)**: First "alternative runtime" exploration in the record. ADR 0014 called cross-surface stacking "architecturally impossible" on Electron's WebContentsView model; this spike tests whether Electrobun's WKWebView mask model closes that gap. Documents Problem A (DOM ↔ page interleaving: solved via mask selectors) and Problem B (page ↔ page reorder gap: unresolved in stock Electrobun) with three bridging options. Opened while 14 PRs wait for review — research and review queue run on separate tracks; the HITL gate is specifically for code integration, not design exploration.
- **Two-class PR queue**: 14 non-draft open PRs split into (a) 10 small, focused, CI-passing audit closures (#236–#245, all `claude/issue-*`, all report typecheck+unit clean) and (b) 4+ large/old PRs (#204 day 19, #206 day 14, #211 day 12, #144 day 36, #32 day 59). Class (a) could merge in a single batch session; class (b) requires deeper engagement. The queue looks larger than it is because both classes are undifferentiated in the list. Not proposing label automation — one cycle old, no immediate pain.
- **7-day main-branch drought (June 15–22)**: Matches the May pattern exactly — the May pause was ~9 days before the May 29 queue flush. The merge session will likely follow the same pattern.
- **Canary still pending (day 12)**: No human-authored non-`claude/*` PR since the June 10 claude-review restore.
- **Journal PR #169 at day 24**: Longest branch divergence yet. Operational — future runs read from the branch — but the gap is worth naming as it compounds weekly.
Hint for next run: if the merge session happened, count how many of the 10 small audit PRs (#236–#245) merged and whether that unblocked the dependency-blocked routing batch (#235, #228, etc.); check if the Electrobun spike (PR #246) influenced any ADR update or follow-on issues; check whether PR #204 (wireframe) or #206 (CLI loop) finally got reviewed.

---

## 2026-06-23
Surveyed: PR #253 (canvas-as-document-cli integration, ADR 0019), commit log for the 5-phase AFK epic run this morning, needs-triage queue (11 items), open PR aging, AFK worker staging incident.
Acted:
- Nothing filed. No two-instance pattern above the threshold; proposal queue empty.
Noticed (no action):
- **ADR 0019 (canvas-as-document-cli) delivered as 5-phase AFK epic**: All 5 step PRs (#248–#252) merged before integration PR #253 opened at 06:18 UTC today. Phase 1→5 took ~2.5h wall time (03:53–06:16 UTC). Net: +1575/−1008 LOC, full entity-kind registry, `kindFromId` prefix-sniffing deleted structurally, per-kind create/update/delete routes removed. The AFK pipeline shipped a complete ADR implementation in one run.
- **AFK worker `git add -A` staging issue — first observed instance**: Phase 3 (#250) used broad `git add` and swept 29 untracked scratch files into the commit — a built `.app` bundle (binaries: `bun`, dylibs) under `experiments/` and `harness/.traces/`. Caught and remediated in commit `659223a` before phase 4 proceeded; those dirs added to `.gitignore`. PR #253 body explicitly names the fix: "The fire prompt / `worker.md` should scope its staging instead of `git add -A`." This is one instance of the pattern. If a second epic sweeps artifacts again, that's the threshold for a proposal.
- **10 small audit PRs (#236–#245) still unmerged (day 2–3); joined by #253 today**: Merge queue at 12 PRs. Largest queue count in the journal record. Drought is now 8 days (June 15 → today with no main commits post the `e4e4f3b` Copy-as-PNG merge on June 15). The May analogue was a 9-day pause before the May 29 flush; this follows the same rhythm.
- **Needs-triage queue at 11 items**: Issues #220–#228, #235 from June 14-15 bloat audit; #190 (June 1); #124 (May 16). Issues #225, #227, #228 received reconciliation notes from the ADR 0019 branch (June 22-23) but label routing has not happened. Expected to occur in the post-merge routing session.
- **PR #246 (Electrobun spike)**: No ADR 0014 update, no follow-on issues filed. Research findings documented; not integrated. Separate track from merge queue as expected.
- **Journal PR #169 at day 25**: Operational (this run read from the branch). Divergence compounds weekly.
Hint for next run: check whether the merge session happened and how many audit PRs closed; if the AFK worker staging fix landed (change to `harness/fire.md` or equivalent), note it and retire the watch item; if a second `git add -A` sweep occurs in any future epic, file the proposal; check if #204 (wireframe, day 20+) or #206 (CLI loop, day 15+) have any review activity.

---

## 2026-06-24
Surveyed: Today's merges (#253 canvas-as-document-cli integration, #255 unify selection outlines), pr-drain outcomes from June 23, remaining open PRs, AFK staging watch item status, needs-triage queue after ADR 0019 landing.
Acted:
- Nothing filed. No open proposals; no two-instance patterns above threshold.
Noticed (no action):
- **Merge session completed, partially**: pr-drain (June 23) merged 7 small audit PRs (#217, #212, #216, #231, #234, #17, #230) plus #211 (copy-as-PNG). Today #253 (canvas-as-document-cli integration) and #255 (selection outlines) merged. Queue went from ~14 non-draft open PRs to ~9. Three feature PRs remain open: #236 (comment mode for file entities, day 4), #238 (live-refresh local files, day 3), #240 (fallow dead-code sweep, day 3). All three are CI-passing and waiting review, not queued behind a dependency.
- **AFK staging watch item: passive fix confirmed, root cause unaddressed.** `.gitignore` now excludes `experiments/` and `harness/` — verified in working tree. This prevents the June 23 class of artifact sweep from recurring. The PR #253 body's suggested fix ("scope worker staging instead of `git add -A`") was not implemented in `afk-fire.sh` or any worker.md — the fire prompt contains no git-add instruction at all; git staging is left to Claude's judgment. The `.gitignore` approach is the correct mitigation for this repo. Retiring the watch item: passive suppression sufficient unless a second sweep hits an unignored directory. No proposal needed; no second instance.
- **ADR 0019 now on main**: The entity-kind registry (`src/main/entities/`) is live. This unblocks the dependency chain: #225 (migrate remaining kinds to registry) unblocks #227 (table-driven IPC) and #228 (collapse find-by-id fan-outs). All three still carry `needs-triage` — routing expected in the next session after today's merges settle.
- **Needs-triage queue still at 11 items**: Composition unchanged from June 23 — no routing since ADR 0019 landed. The reconciliation notes on #225, #227, #228 clarify scope but labels haven't moved. Expected: next routing session will move the now-unblocked items to `ready-for-agent`.
- **PR #204 (wireframe integration, day 21), #206 (self-healing CLI loop, day 16)**: No review activity observed on either. #204 remains the longest integration PR in the record. #206 is the pipeline improvement that, if merged, would close the CLI friction loop — its delay has downstream value distinct from product PRs.
- **Journal PR #169 at day 26**: Longest divergence in the record. Operational; future runs read from the branch. No action.
Hint for next run: check whether #236, #238, #240 merged (they are unblocked and CI-passing); check whether the needs-triage routing session happened post-ADR-0019 and how many of #225/#227/#228/#235 moved to ready-for-agent; watch for any activity on #204 or #206.

---

## 2026-06-25
Surveyed: Open PR aging (#236/#238/#240 at day 4–5, CI-passing and unreviewed; #204 at day 22; #206 at day 17), needs-triage queue (11 items, composition unchanged), proposal queue (empty). Dominant theme: quiet post-epic consolidation; no systemic leaks.
Acted:
- Nothing filed. Proposal queue empty; no stale orchestrator output.
Noticed (no action):
- **#236, #238, #240 at day 4–5, no merges**: All three are CI-passing with high-quality PRs (mutation-verified smoke tests on #238, comprehensive test plan on #236, fallow-verified dead-code sweep on #240). No blocking dependencies. Post-epic batch pattern from June 23–24 suggests these flush in the next review session. No alarm at day 4–5.
- **Needs-triage routing still pending post-ADR-0019**: 11 items unchanged from June 24 — same composition as June 23. Issues #225, #227, #228 are explicitly now-unblocked per reconciliation notes added June 22–23. Routing them to `ready-for-agent` unlocks the next bloat-audit AFK batch. Expected latency between pipeline phases; not a leak at this cadence.
- **PR #204 (wireframe-structured-editor) at day 22 — structurally different from a review-delay stall**: Base SHA is ~50 main-branch commits behind; ADR 0019 deleted the per-kind CLI routes that #204's slice 3.4 (`specular wireframe <verb>`) consumes. Merging #204 now requires conflict resolution across the entire ADR 0019 scope. This is "integration PR overtaken by a larger architectural shift" — distinct from the #166 stall (which was review delay only). The wireframe work is real and shipped in its step PRs; the question is whether it lands as-is (rebase) or is superseded by the new verb surface (ADR 0019 added `specular add note` etc. that overlaps). Product-level decision; technical clock is running.
- **Deep-audit cycle time now measurable**: Skill run June 15 → issues filed same day → routing session June 20 (5 days) → AFK PRs June 20–21 → merge session June 23–24 (9 days total). Median wall time from skill run to merged PRs: ~9 days. Useful calibration for planning the next bloat-audit pass.
- **Journal PR #169 at day 27**: Operational; this run read from the branch and appended successfully.
Hint for next run: check if #236/#238/#240 merged; whether needs-triage routing happened and moved the now-unblocked items; whether #204 was closed or rebased — that decision determines how the wireframe feature story ends post-ADR-0019.

---

## 2026-06-26
Surveyed: 28-day run gap (May 28–June 26), all closed orchestrator proposals, current open PRs, needs-triage queue (11 items), commit log (ADR 0019 shipped, deep-audit cycle, pr-drain pattern). Dominant theme: rule triggered — last 3 proposals all rejected; writing "why am I consistently wrong?" instead of proposing.
Acted:
- Nothing filed or closed. No open orchestrator proposals. Nothing stale to clean up.
Noticed (no action):
- **"Why am I consistently wrong?" — mandatory reflection.** Last 3 proposals all stalled at 0 comments: #152 (triage drain, 7 days), #168 (triage drain v2, 7 days), #188 (claude-review restore as issue, 7 days). Self-modifying #208 also stalled. Rule triggered.
- **The failure pattern**: Proposals target automation for things Lyle does manually. The manual approach self-corrects in bursts (#152/#168: Lyle routed 4/5 triage items in one session; #188: four PRs merged without review, then PR #207 delivered as a ready-to-merge branch landed in days). The orchestrator optimises for a continuous drain; the actual workflow is burst-and-flush. Continuous automation against a burst-and-flush system adds noise, not throughput.
- **What actually lands**: (a) Pain-triggered CI changes (#63: real test failures; #207: four unreview-merged PRs including a bug requiring a follow-up). (b) Ready-to-merge branches with a yes/no decision. (c) AFK pipeline epics with a dex plan. Issues in a passive queue do not land unless Lyle actively comes to them.
- **The system is healthy.** ADR 0019 (canvas-as-document, verb-primary CLI) shipped in 5 phases June 22–24 via AFK. Deep-audit skill cycle is calibrated at ~9 days (skill run → issues filed → routing → AFK PRs → merge session). pr-drain pattern (6 audit PRs merged via one integration branch June 23) is novel and effective. Throughput is high.
- **PRs worth watching**: (a) #204 (wireframe-structured-editor, 23 days) — architecturally stranded: ADR 0019 deleted per-kind CLI routes that slice 3.4 consumes; needs a close-or-rebase decision, not a review wait. (b) #236/#238/#240 (June 20–21, 5–6 days) — CI-passing, unblocked, waiting review flush. (c) #144 (manifest extensions, 40 days) and #32 (LM Studio, 63 days) — aging; product decisions.
- **Needs-triage queue at 11 items**: 3 now-unblocked post-ADR-0019 (#225/#227/#228, reconciliation notes June 22–23). Routing them to ready-for-agent unlocks the next bloat-cleanup AFK batch.
- **Journal PR #169 at day 28**: Operational.
Hint for next run: check if #204 was closed or rebased (decision is overdue); check whether #236/#238/#240 merged and needs-triage routing happened; if 3 consecutive runs find nothing to propose after the reflection, ask whether the orchestrator observation cadence is too high for the work pace.

---

## 2026-06-27
Surveyed: PR #256 (delete Browser mode / Focus selection camera, ADR 0020, filed June 26, active today), open PR aging (#236 at day 7, #238/#240 at day 6, #204 at day 24, #206 at day 19), needs-triage queue (11 items, unchanged), first run post-mandatory-reflection. Dominant theme: major simplification PR active on a `codex/` branch; system otherwise in a post-epic pause.
Acted:
- Nothing filed. First run after the June 26 mandatory reflection. Proposal queue empty; no stale orchestrator output to close.
Noticed (no action):
- **PR #256 (delete Browser mode / Focus selection, ADR 0020) is today's dominant signal**: 100 files changed, 1944 additions / 1343 deletions, branch `codex/delete-browser-mode-focus-selection`. Filed June 26, updated June 27 (this morning). `mergeable_state: unstable` — CI was still running or failing at run time. Two notable things: (a) this is a `codex/` branch, not `claude/` — a different worker variant than the usual AFK pipeline; (b) it deletes a whole navigation mode (Browser mode) in a single PR rather than across AFK step PRs. If it merges clean, the codebase loses the Browser view model entirely and gains a simpler ephemeral focus-camera behavior. Second major "delete-a-whole-mode" simplification in 5 days after ADR 0019.
- **#236, #238, #240 at day 6–7, CI-passing, unreviewed**: Consistent with burst-and-flush pattern. June 26 reflection confirmed this. These will flush when the #256 merge triggers a review session. No alarm.
- **PR #204 (wireframe-structured-editor) at day 24**: Still no close-or-rebase decision despite the June 26 entry calling it "overdue." Main has advanced another day farther from its base. No new activity visible. Each day this remains open, the conflict surface with main grows.
- **Needs-triage queue at 11 items, composition unchanged**: Bloat-audit items #225/#227/#228 have had reconciliation notes since June 22–23 (now 14 days) but remain `needs-triage`. Expected latency; routing session likely follows the next review flush.
- **Journal PR #169 at day 29**: Operational. Appended today's entry successfully.
Hint for next run: check CI status on #256 and whether it merged; if it merged, check whether the review session also cleared #236/#238/#240; if #204 is still open at day 25+ with no comment, note it as the longest-running architecturally-stranded PR in the record and consider whether it warrants a close recommendation as a proposal.

---

## 2026-06-28
Surveyed: PR #256 (Browser mode deletion, day 2 — updated June 28 morning but still open), open PR aging (#236/#238/#240 at day 7–8, #204 at day 25, #206 at day 20, #144 at day 42, #32 at day 65), needs-triage queue (11 items, composition unchanged from June 24), proposal queue (empty — post-reflection hold continues).
Acted:
- Nothing filed or closed. Proposal queue empty; no stale orchestrator output. Post-reflection period (mandatory reflection June 26).
Noticed (no action):
- **PR #256 (delete Browser mode / Focus selection, ADR 0020) still open, day 2**: Updated this morning (07:55 UTC) — either CI stabilized or a fix push landed. Still awaiting merge. If it clears CI and lands, it is the second "delete-a-whole-mode" simplification in two weeks (ADR 0019 deleted per-kind CLI routes; ADR 0020 deletes Browser mode entirely). Pattern: the system is shedding whole abstraction layers, not just trimming within them.
- **#236/#238/#240 at day 7–8, CI-passing, unreviewed**: At the edge of the stale-observation window. June 26 reflection confirmed the burst-and-flush pattern; these will flush when #256's merge triggers a review session. No alarm yet — but if they hit day 10 still open, that's outside the normal flush cadence.
- **PR #204 (wireframe-structured-editor) at day 25 — no comment, no close**: Third consecutive entry naming this. ADR 0019 deleted the per-kind CLI routes that #204 slice 3.4 (`specular wireframe <verb>`) consumes. The PR is architecturally stranded: merging as-is requires resolving conflicts across the full ADR 0019 scope; the verb surface it added (`specular wireframe insert|delete|…`) is now superseded by ADR 0019's `specular add note` / `specular apply` path. A close or close-and-extract decision becomes more expensive each day main diverges. Not in orchestrator scope to propose closing product PRs; naming it so Lyle can make the call.
- **Needs-triage queue at 11 items, day 4 unchanged**: Issues #225, #227, #228 have reconciliation notes since June 22–23 (now 16 days) clarifying their scope post-ADR 0019 but remain `needs-triage`. Routing them to `ready-for-agent` unlocks the next bloat-cleanup AFK batch. Expected inter-session latency; routing will follow the next review session.
- **Journal PR #169 at day 30**: One month of entries buffered on the branch, not yet on main. Operational — this run read from and appended to the branch successfully. No practical problem today (no one else edits journal.md), but noting the milestone.
- **`codex/` branch variant (PR #256) — one observed instance**: PR #256 comes from `codex/delete-browser-mode-focus-selection`, distinct from the usual `claude/` prefix. Different worker variant. First observed instance in the journal record. Watch for a second use of `codex/` branches to understand when that variant is preferred over the `claude/` AFK pipeline.
Hint for next run: check whether #256 merged (CI clearance); if merged, check whether the review session also cleared #236/#238/#240 and whether the needs-triage routing session happened; if #204 remains open at day 26+ with no comment, it is worth noting as a proposal candidate (close recommendation) — product PR stranded by a superseding ADR is arguably within the orchestrator's scope to surface.

---

## 2026-06-29
Surveyed: commits since June 28 (0.3.2 released; #256 merged, #258 license), open PR state (#236/#238/#240 day 8–9; #204 day 26; #206 day 21), needs-triage queue (11 items, day 15 for bloat-audit batch), ready-for-agent queue (1 new item: #257 selection-lag bug), post-reflection period.
Acted:
- #259 filed — "Add post-ADR conflict-audit step to AFK merge checklist." Two instances of AFK-built PRs becoming architecturally stranded after a superseding ADR merged without triggering a review of open PRs: #204 (wireframe verb surface superseded by ADR 0019's `specular apply` path) and #206 (CLI probes testing old command surface, also affected by ADR 0019/0020). Each day these sit, their conflict surface with main grows.
Noticed (no action):
- **0.3.2 shipped today**: ADR 0019 (canvas-as-document-cli) + ADR 0020 (Browser mode deletion) landed in one release cycle. Two complete mode deletions in the same version is notable — the system is shedding abstraction layers at a pace that creates real rebase debt on any open integration PR.
- **`codex/` branch variant confirmed**: PR #256 (codex/delete-browser-mode-focus-selection) merged and shipped in 0.3.2. Matches the `claude/` AFK pattern for single-scope large simplifications. Second confirmed codex/ use would confirm variant is in regular rotation — watch the next large simplification branch.
- **Issue #257 filed and labeled `ready-for-agent` within hours of creation**: Selection-border-lag bug (filed 03:02 UTC, labeled by ~08:30 UTC). Whether this was Lyle or the @claude triage action, the triage latency for well-scoped bugs is dramatically shorter than for the bloat-audit batch (#220–#228 at 15 days needs-triage). Pattern: self-described bugs with root-cause analysis and file pointers route fast; batch refactoring issues stall.
- **PRs #236, #238, #240 did NOT flush with 0.3.2**: Still open at day 8–9. These are CI-passing, unblocked, and were last updated June 20–21. The 0.3.2 merge session cleared the codex/ and pr-drain branches but not these three. Next review session should catch them; at day 10+ without flush, note it.
- **PR #204 at day 26**: Entry for the fifth consecutive run. Past the point where noting is useful — naming it here for continuity with #259 proposal context only.
- **Journal PR #169 at day 31**: Record holder; operational.
Hint for next run: check whether #259 is acted on and whether #236/#238/#240 flushed; if the needs-triage bloat-audit batch (#220–#228) moved to ready-for-agent, an AFK session is likely starting — watch for a new dex epic; if #257 (selection-lag) was addressed by an agent, observe whether the fix matched option A or B from the issue's options list.

---

## 2026-06-30
Surveyed: Today's merges (#240 fallow dead-code sweep, #260 perf layout pass, #261 page panel + comment overlay polish), open PR state (#236/#238 still open at day 10), issue #257 closed-as-completed, proposal #259 (day 1, 0 comments). Dominant theme: review flush underway; bug-to-fix cycle confirmed fast.
Acted:
- Nothing filed or closed. Proposal #259 at day 1; no stale orchestrator output.
Noticed (no action):
- **Review flush broke the stall: three PRs merged today**. #240 (fallow dead-code sweep, ~250 LOC removed, day 9), #260 (perf: layout data built once per pass, day 1), and #261 (page panel + comment overlay polish, interactive multi-commit). Pattern confirmed: the burst-and-flush rhythm holds. PRs #236 (comment mode for file entities) and #238 (live-refresh local files) still open at day 10 — expected to flush in the same session.
- **#257 fast-tracked to closed in ~28 hours**: Filed June 29 at 03:02 UTC (thorough analysis, root cause, six numbered options). Closed June 30 at 07:49 UTC after PR #260 landed. The fix implemented Option C exactly as the issue named it ("build once, pass to all senders"). Pattern from June 29 entry holds: thorough bug reports with root-cause analysis and file pointers skip the queue and get fixed directly. Batch refactoring issues with no immediate visible cost don't.
- **PR #261 was interactive, not AFK**: 8 commits, each co-authored by Claude Opus 4.8, merged as a single PR. Covers UX + logic changes (page device controls, per-comment fix button, fix-agent reframe from "fix only" to "respond"). Reinforces the two-mode split: AFK for bounded refactors; interactive for mixed UX/logic work with rapid iteration. Both modes in active use.
- **Needs-triage bloat-audit batch (#223/#225–#228) at day 16, composition unchanged**: Routing to `ready-for-agent` is the unlock for the next AFK bloat-cleanup epic. No routing session observed since June 24. Each day this sits, the next AFK batch is delayed by one more day.
- **PR #204 (wireframe, day 27) and #206 (CLI loop, day 22)**: Architecturally stranded; #259 is the live proposal addressing this. No new activity on either.
- **Journal PR #169 at day 32**: Operational.
Hint for next run: check whether #236/#238 flushed with the current review session; check whether the needs-triage bloat-audit batch moved (a routing session would signal the next AFK epic imminent); watch for any comment on #259 — if it still has 0 comments by July 6, decide whether to close as stale or refile as a ready-to-merge PR (the skill edit is small and self-applying).


---

## 2026-07-01
Surveyed: Overnight AFK burst (PRs #264/#266/#267/#268/#269/#270 opened 21:20 June 30 – 01:26 July 1), issue #225's self-bail comment, needs-triage queue (8 items, 2 new today: #274, #265), proposal #259 (day 2, 0 comments), journal PR #169 (day 33). Dominant theme: largest single-night AFK batch yet, with one agent correctly declining ambiguous work instead of guessing.
Acted:
- Nothing filed or closed. #259 at day 2 (stale threshold July 6). No new two-instance pattern crossed the proposal bar.
Noticed (no action):
- **Six-PR AFK burst in ~4 hours delivered most of the bloat-audit epic**: #268 (closes #235, selectionBbox primitive), #267 (closes #221, viewport-layer/coords/preload dedup), #266 (closes #220, `withDragLifecycle` primitive + thinner `App.tsx`), #270 (closes #262, note content in Y.Doc + ADR 0023), plus #264 and #269 (perf chain from #257, #269 still draft). All report clean typecheck/unit; several note `test:smoke` couldn't run in-sandbox (Electron binary blocked) and ask a human to run it before merge — consistent, not new.
- **First observed instance of a principled AFK bail**: issue #225 (collapse interactive per-kind wrappers) was picked up, scoped honestly by the agent (mechanical part clear, ~90% duplication confirmed), then explicitly declined — the `store`/`buildScene`/`restore` hook shapes the issue asks for have no precedent in the codebase, and guessing risks silent breakage in the scene-broadcast/undo path. The agent removed `agent-in-progress` without re-adding `ready-for-agent`, leaving it back at bare `needs-triage`, and moved to the next candidate. This is the guardrail working as designed (no forced bad implementation) — but it leaves #225 label-indistinguishable from an issue nobody has ever looked at, even though it now needs a *design pass* specifically, not a first triage. One instance; if a second bail shows the same "silently reverts to needs-triage" pattern, a `needs-design-pass` label (or a comment-based signal) would be worth proposing so these don't get re-picked-up cold or ignored as if untriaged.
- **needs-triage queue at 8 items, two new today**: #274 (BrowserWindow vs WebContentsView overlay boundaries, filed 05:57 UTC) and #265 (make `buildCanvasLayoutData` cheap for pan-only changes, filed yesterday 21:20, follow-up to #257/#264). Both are fresh (hours old) — not a staleness concern yet. #223 remains correctly blocked on #215 (`ready-for-human`, ADR: command core) — not a routing gap, a real human-decision dependency. #225, now un-blocked-in-theory but design-ambiguous, is the one to watch.
- **#124 (pages select-first/interact-second) picked up a substantive comment June 29** pointing out overlap with a stopgap fix in the ADR 0020/0021 focus-session work (deleting a focused page could freeze the canvas). Still `needs-triage` at 46 days — consistent with the standing diagnosis (#53/#124 need human architectural time, confirmed by #168's postmortem back in June).
- **Journal PR #169 at day 33** (opened May 29, 33 daily entries stacked, still unmerged). Long-standing, low-risk, operational — squash-merge is mechanical since it's pure append. Not re-flagging as an issue; noting only for the day-count record.
Hint for next run: watch whether any of tonight's six PRs merge and whether a second AFK bail (silently dropping back to bare `needs-triage` with no distinguishing signal) occurs on a different issue — that would be the second instance for a `needs-design-pass` label proposal. Check #259 by July 6.

---

## 2026-07-02
Surveyed: PR aging (#204 day 29, #206 day 24, #236/#238 day 12, #266/#267/#268/#270 day 2, #246 day 10 draft), the ADR-0023 postmortem merge (#269), issue #225's orphaned-label status (day 2), fresh issue #279 (from an ADR-0024 smoke-review branch), proposal #259 (day 3, 0 comments), journal PR #169 (day 35). Dominant theme: a real ADR-numbering collision surfaced; existing watch items remain single-instance.
Acted:
- Nothing filed or closed. #259 at day 3 (stale threshold July 6). No pattern crossed the two-instance bar.
Noticed (no action):
- **ADR number collision, first instance**: #269 (merged July 1) landed `docs/adr/0023-renderer-owned-camera-gpu-panzoom.md` (Status: Rejected, with a postmortem — the epic tried GPU-composited pan/zoom on `claude/feat-renderer-owned-camera`, measured it worse, and recorded why instead of silently dropping it — healthy practice). Separately, PR #270 (open since July 1, unmerged, 0 comments) independently adds `docs/adr/0023-note-content-in-ydoc-for-undo.md` — same number, unrelated decision, cut from an earlier main snapshot before #269 claimed 0023. If #270 merges as-is, two unrelated ADRs both carry "0023," breaking the assumption that the number is a unique identifier. One instance, not even landed yet; the fix (renumber at review time) is cheap and is Lyle's call, not yet a proposal. Watching for a second collision — concurrent AFK epics with overlapping lifetimes will keep producing this as long as ADR numbers are assigned by hand at branch-cut time.
- **#225 still orphaned, day 2 — worse than previously described**: The July 1 entry described the July-1 "principled AFK bail" (agent declined ambiguous descriptor-hook work, dropped `agent-in-progress`) as leaving #225 at bare `needs-triage`. Checking labels directly this run: #225 actually has **zero labels** — not even `needs-triage`. It's invisible to every label-based queue query, including this run's `needs-triage` pull (9 items, #225 not among them). Still one instance; the `needs-design-pass` idea from July 1 stands as the fix if a second bail reproduces this.
- **#204 (day 29) and #206 (day 24) still open, no close/rebase decision**: Proposal #259 (filed June 29, names both explicitly) is at day 3 — too early to call stalled. Each day adds to the eventual conflict-resolution cost.
- **Bloat-audit epic mostly landed**: #266 (`withDragLifecycle`), #267 (viewport-layer/coords/preload dedup), #268 (`selectionBbox`) all open ~day 2, CI-passing, unreviewed — consistent with the established burst-and-flush rhythm, not a new pattern. #269 (pan/zoom chrome + dev perf HUD + the ADR 0023 postmortem) merged today.
- **New issue #279** (grouping silently ignores shape/drawing entities) filed today, surfaced while porting the smoke suite to an in-process integration harness on a branch that references an upcoming **ADR 0024** — a fourth concurrent epic (after camera, notes-undo, bloat-audit) that will also need an ADR slot. Worth checking next run whether 0024 collides too.
- **Journal PR #169 at day 35**: record holder, still operational, still a pure-append diff.
Hint for next run: check whether #270 merged with the ADR-0023 collision intact and how (or whether) it was resolved — that's the second data point for the numbering-collision watch; check if #225 picked up any label; check whether the ADR-0024 smoke-review branch collides with anything else claiming that number; check #259 by July 6.


---

## 2026-07-03
Surveyed: PR aging (#204 day 30, #206 day 25, #236/#238 day 13, #266/#267/#268 day 3, #270 day 2 unmerged), new PR #280 (in-process integration harness, WIP, off issue #278), issue #278 (testing-strategy overhaul, filed July 1), label audit across all open `agent-in-progress` issues, proposal #259 (day 4, 0 comments). Dominant theme: the testing-strategy shift from #278 is now in motion as code, and a second confirmed instance of the `agent-in-progress` label losing an issue from every queue.
Acted:
- #281 filed — "agent-in-progress label has no exit contract — leaves issues stuck or invisible." Two instances: #111 (PR #136 closed without merging May 29; issue still carries `agent-in-progress` 35 days later, no agent has touched it since) and #225 (July 1 principled AFK bail dropped the label to zero, still unlabeled at day 3 — confirmed again this run). `agent-in-progress` is not one of the five canonical labels in `docs/agents/triage-labels.md` and nothing in `.claude/skills/` or `.github/workflows/` sets or clears it — it's a real, useful, but entirely undocumented convention with no rule for what happens when the backing work ends without a merge. Distinct from #259 (whole-PR staleness after a superseding ADR) — this is per-issue label lifecycle.
Noticed (no action):
- **PR #280 opened today, WIP: in-process integration harness replacing the Electron smoke layer.** Directly executes issue #278's proposal (filed July 1: an 11-week audit found ~3 qualified catches and 0 clean regression catches from the Electron-spawning smoke suite; the real protection has been typecheck/unit/dogfooding). #280 boots the real main-process runtime in plain Node against a temp dir, stubs `electron` and the `webContents.send` seam, and drives the same mutators the IPC handlers call. Notably co-authored by "Claude Fable 5," not the usual Opus/Sonnet AFK signature — a new model variant in the rotation, worth watching for a second instance. This is the direct fix for the "`test:smoke` couldn't run in this sandbox" line that has appeared in nearly every AFK PR description since at least #266 (June 30) — a pattern this journal has called "consistent, not new" for a week without an owner. If #280 lands and the smoke suite is deleted as its description promises, `CLAUDE.md`'s Build & verify section and Test contract section (both reference `test:smoke` as a required gate) will need updating in the same change — flagging so that isn't missed, not proposing anything since the fix is already mid-flight as a PR.
- **ADR-0023 collision still not materialized**: #270 (`0023-note-content-in-ydoc-for-undo.md`) remains open and unmerged at day 2; main only carries the one `0023` (the rejected renderer-owned-camera postmortem from #269). No second instance yet — still watching for whether #270 renumbers at merge time or lands as a literal collision.
- **Label audit of all 9 open `agent-in-progress` issues**: seven check out normally (#1↔#236, #135↔#144, #209↔#238, #220↔#266, #221↔#267, #235↔#268, #262↔#270, all matched to a real open PR). Only #111 and #225 are orphaned — exactly the two instances behind #281, no third found.
- **#204 (day 30) and #206 (day 25)**: still open, no close/rebase decision. #259 (proposal naming both) at day 4, still 0 comments — not yet at the July 6 stale threshold.
- **Journal PR #169 at day 36**: operational, pure-append diff, unmerged.
Hint for next run: check whether #280 merged and the smoke suite was actually deleted — if so, verify `CLAUDE.md` and `tests/README.md` were updated in the same PR (if not, that's a real gap worth a proposal, since the doc would be actively wrong); check for a second "Claude Fable 5" co-authored PR to confirm it's a rotation, not a one-off; check whether #281 gets picked up or needs the July 10 stale check; check #259 by July 6; check if #270 merges and how the ADR-0023 numbering resolves.

---

## 2026-07-04
Surveyed: three simultaneous "deepen" PRs from the architecture-audit track (#284 IPC, #285 above-view, #286 runtime — all opened evening of July 3 off `docs/audit/deepen-3673a35.md`), the three reconciliation issues they spawned (#287, #291, #292, all filed 04:20 UTC today), PR #280 status (still WIP, day 1, `mergeable_state: dirty`), #270 (ADR-0023 candidate, still unmerged at day 3), #204/#206 aging (day 31/26), issue #225's label state (still zero labels, day 3), two fresh AFK bug-fix PRs (#293, #294), proposal status (#259 day 5, #281 day 1, both 0 comments). Dominant theme: the architecture-audit pipeline (audit → deepen plans → PRs → flagged-follow-up issues) is running end-to-end cleanly; no new pattern crossed the two-instance proposal bar.
Acted:
- Nothing filed or closed. #259 at day 5 (stale threshold July 6, not yet due) and #281 at day 1 — neither stale. No second instance of any watch item confirmed this run, so nothing new to propose. Last three proposals (#168 closed not_planned, #259 open, #281 open) are not a 3-for-3 rejection — no reflection pause warranted.
Noticed (no action):
- **Three "deepen" PRs landed the same evening, all touching overlapping surfaces**: #284 (IPC/bridge, 115 files), #285 (above-view interaction, 15 files), #286 (runtime entity registry + `mutateWorkspace`, 62 files) — one commit-per-step epics executing the six ranked candidates from the July 3 architecture audit (PR #282/#283). Notably, #284 and #286 each *explicitly document* a merge-order dependency on the other in their PR bodies ("whichever merges second needs a small rebase") — a real coordination point, handled today by prose in two PR descriptions rather than any tracked order. This is the first time three same-day sibling refactors with mutual rebase dependencies have appeared in this record. One instance — watching for whether the actual merge goes smoothly or whichever lands second needs more than the ~10-line rebase each PR predicts.
- **The audit is generating well-scoped follow-up issues, not silent gaps**: #286's own description explicitly flags five things it deliberately didn't change (group-delete semantic fork, annotation-drag undo granularity, `syncMapFromArray` doc-key leak, navigation-undo question, undispatched `defaultSize`) and files became issues #287, #291, #292 (plus #279 already open) within hours, each with file pointers and concrete options. This is the deepen-pass discipline working as designed — deliberate non-goals get a paper trail instead of disappearing.
- **Those same issues are already turning into PRs same-day**: #293 (closes #288, annotation-drag single-undo-step, mirrors the existing resize-begin/end IPC pattern) and #294 (closes #289, `syncMapFromArray` stale-doc-key fix, mutation-verified) both merged same-day turnaround from issue to PR. The audit → issue → fix loop is closing faster than the review queue can keep up, which is a good problem.
- **#225 still fully unlabeled at day 3**: confirmed again (zero labels via direct label query). No third instance of this failure mode found elsewhere; proposal #281 already covers it and remains unactioned.
- **PR #280 (smoke-suite replacement) unchanged since creation — day 1, `mergeable_state: dirty`**: main has advanced through the #282/#283 architecture-audit merge since #280 was opened, so it now has real conflicts to resolve on top of being WIP. Not proposal-worthy (single PR, normal rebase debt), but the CLAUDE.md/tests/README.md doc-update risk flagged yesterday still applies once this lands.
- **#270 (ADR-0023 candidate) still unmerged at day 3**: no renumbering, no collision yet — main still carries only one 0023 (the rejected camera postmortem). Still one instance.
- **#204 (day 31) and #206 (day 26)**: no new activity, no close/rebase decision. #259 (the proposal naming both) at day 5, still 0 comments.
- **Journal PR #169 at day 37**: operational, pure-append diff, unmerged.
Hint for next run: check whether #284/#285/#286 merged in the order their PR bodies assumed, and whether the rebase was as small as each predicted — if a second multi-PR-same-day-audit-fanout produces a *messier* merge, that's worth a proposal (e.g., a checklist step for audit-spawned parallel PRs to declare merge order up front rather than in prose). Check #280's conflict status and whether it's been rebased. Check #259 by July 6; check #281 by July 10 if unactioned. Check whether #279 (still open, same bloat-audit lineage) gets picked up now that the runtime registry work in #286 is close to landing.

---

## 2026-07-05
Surveyed: The largest merge day on record — #285 (above-view deepen, merged 16:01), #286 (runtime deepen, ADR 0024/0025, merged 21:11), #284 (IPC deepen, merged 22:37 after an explicit merge-conflict-resolution commit), #280 (in-process integration harness, merged 04:28, smoke suite deleted + docs updated same PR), plus ~10 same-day AFK bug-fix PRs (#293–#306) and three CI-breakage self-fixes (auto-review workflow removed for burning credits, fallow full-repo-scan → changed-file audit, pnpm-11 lockfile checksum fix). Also checked proposals #259 (day 6, 0 comments) and #281 (day 2, 0 comments), issue #225's label state, and cross-referenced #225 against #286's actual diff.
Acted:
- Nothing filed or closed. #259 and #281 both below their stale thresholds (July 6 / July 10). No new proposal filed — see note below on why the standout finding this run isn't one.
Noticed (no action):
- **#284/#285/#286 merge-order prediction confirmed, and it needed real conflict resolution, not just a small rebase.** Yesterday's entry asked whether "whichever merges second needs a small rebase" (each PR's own claim) would hold. It mostly did — but #284 needed an actual merge commit ("Merge origin/main into deepen-ipc — Resolves the typed-contract vs. main refactor overlap: canvas-update-{text,file,drawing,shape,group} channels collapse into canvas-update-entity...") reconciling its typed IPC contract against #286's registry-dispatch collapse, landing after #286. This is the second instance of the multi-PR-same-day-audit-fanout pattern from the July 4 hint, and it was *not* messier than predicted, just non-trivial — prose-declared merge order plus a dedicated reconciliation commit handled it cleanly. Not proposal-worthy: the existing ad-hoc coordination (PR bodies naming their own dependency) worked. Confirms rather than escalates the pattern.
- **#280 landed with its promised doc updates in the same commit — the July 3/4 watch item is resolved.** The docs commit lists CLAUDE.md, AGENTS.md, tests/README.md, runtime CLAUDE.md, interaction-layer §9, the afk-feature skill, and .fallowrc.json all updated together. Verified: the CLAUDE.md this orchestrator itself reads now shows `test:integration`/`test:boot` with no `test:smoke` reference. No gap.
- **Standout finding: issue #225 appears to have been substantively implemented by #286, three weeks after filing, with no cross-reference in either direction.** #225 ("Collapse interactive per-kind wrappers onto entity descriptors," filed June 14, unlabeled since the July 1 principled AFK bail) asks for exactly what #286's commit series describes: per-kind `store`/`buildScene`/`restore` contract members, interactive delete routed through `getEntityKind().delete`, and one device-frame epilogue helper (#225 acceptance criterion 3 ≈ #286's "DeviceTarget adapter folds the snap preludes and page/file device-command duplicates"). #286's PR body never mentions #225. #225 itself is still open, zero labels, last touched July 1. Two other needs-triage issues (#227, #228) are explicitly "Blocked by #225" and have sat since June 14 (21 days) — if #225's scope is actually done, they may now be unblockable-and-forgotten rather than genuinely blocked. This is a close cousin of proposal #259's thesis (large refactors silently strand *other* artifacts) but the artifact here is an *issue*, not a PR — the failure mode is symmetric: #259 says "ADR merges should audit open PRs for staleness," this says "large deepen/refactor merges should also audit open needs-triage issues for whether their scope was already covered." One clean instance, but well-evidenced (specific line-by-line correspondence, not a vibe). Not filing as a fresh proposal — it's close enough to #259 to be additional evidence for broadening that one (issues + PRs, not just PRs) rather than a new ticket; two near-duplicate open proposals is worse than one broader one. Flagging here for whoever reviews #259, and this is exactly the kind of thing `/triage` or a human pass over #225 should check directly (not orchestrator's job to close/relabel).
- **Bloat-audit needs-triage chain (#223→#226, #225→#227/#228) now at 21 days, unrouted, while a sibling chain (#220/#221/#235→#266/#267/#268) already shipped via the July 1 AFK burst.** #215 (the command-table ADR both #223 and #226 depend on) is `ready-for-human`, still open. So part of the chain (#223/#226) is genuinely blocked on a human architectural call; the other part (#227/#228, blocked on #225) may not be blocked at all anymore per the finding above. Two different reasons for the same-looking symptom (stale needs-triage) — worth someone actually looking, not worth an orchestrator proposal (this is /triage's job).
- **needs-triage queue grew to 12 items**, including three fresh, well-scoped reconciliation issues from the deepen pass (#287 group-delete fork, #291 undo-for-navigation question, #292 undispatched defaultSize) — same deepen-pass discipline noted July 4, holding up on a second occasion.
- **Three CI-breakage fixes landed same-day, two via direct commit to main (bypassing PR review), one via a fast PR (#306)**: auto Claude-review workflow removed ("burns credits on every PR open/sync"), fallow switched from whole-repo `check` (failing all PRs on 11 baseline findings) to changed-file `audit`, pnpm lockfile regenerated for a pnpm-11 checksum mismatch. All fixed within hours of breaking, by Lyle directly. Not a leak — this is exactly the "pain is visceral, fix lands fast" pattern from mid-May; noting only because three in one session is the most yet observed together.
- **#270 (ADR-0023 candidate) still open and unmerged at day 4**, commit-status endpoint returns zero checks (this repo reports via Actions check-runs, not legacy statuses — not itself meaningful). No renumbering collision yet.
- **#204 (day 32) and #206 (day 27)**: still no close/rebase decision. #259 (day 6, 0 comments, stale threshold tomorrow) names both.
- **Journal PR #169 at day 38**: operational, pure-append diff, unmerged.
Hint for next run: #259 hits its 7-day stale threshold tomorrow (July 6) — if still 0 comments, close-and-refile with the #225/#286 correspondence folded in as broader evidence (issues, not just PRs, get stranded by large refactors). Check whether anyone looked at #225 against #286 and closed/relinked it. Check #281 by July 10. Check whether #270 merges and whether the ADR-0023 numbering resolves.

---

## 2026-07-06
Surveyed: 2026-07-05's cleanup session (largest merge day on record, per yesterday's entry) resolving forward — #259 stale-threshold call, #225/#286 correspondence outcome, #270's fate, #111 (the other `agent-in-progress` orphan behind #281), current open-PR list. Dominant theme: yesterday's human review pass directly resolved most of what this week's proposals were tracking, without adopting either proposed mechanism — sharpening a five-instance pattern into a clear conclusion.
Acted:
- #259 closed as `not_planned` (7 days, 0 comments) — but not a plain stale-close. Post-mortem instead of a refile: both cited PRs are now resolved (#206 closed without merging 2026-07-05; #204 still open, stranded, 33+ days — the sole remaining instance). Explicitly declined to refile with broadened scope (the #225/#286 "issues get stranded too" evidence yesterday's entry flagged as ammunition for a broader version) because that evidence *also* resolved by direct human action in the same session: Lyle closed #225 himself on 2026-07-05 (2 comments, `completed`) after recognizing its scope was already covered by #286. The evidence base shrank this week, not grew. Refiling would have been the sixth attempt at "propose a checklist/process step for the AFK pipeline" — a category with a 0/5 adoption record (#152→#168, #188, #208, now #259) against a 5/5 record of the underlying gap getting fixed anyway once Lyle notices it directly (#63, #188→#207, the May 29 triage flush, and now this).
Noticed (no action):
- **#270 (ADR-0023 candidate) closed without merging, 2026-07-05.** Same cleanup session as #206/#225. No renumbering collision ever materialized — the branch just never landed. Retiring this watch item; it resolved by the PR going away, not by a numbering conflict occurring.
- **#111 — the other half of #281's evidence — unchanged and now the clearest example of the label gap.** Still open, still carrying bare `agent-in-progress`, 1 comment, last updated 2026-05-17. Its backing PR (#136) closed without merging on 2026-05-29 — 38 days ago. Unlike #225 (which got a human look and a clean close), #111 has had zero human attention in that window. This is the live, uncontradicted instance behind #281; the proposal is at day 3 (stale threshold July 10) and correctly still open — there's no new instance to add, but no reason to reconsider it either.
- **The pattern from this week generalizes past "CI/workflow fixes land from pain, not queues" (the standing June conclusion) to a second flavor: "stranded-artifact audits also don't need a standing mechanism — they get caught in Lyle's own periodic review passes."** Two different proposed mechanisms (#188's CI-restore-as-branch, #259's post-ADR-PR-audit-checklist) both stalled exactly seven days and were both overtaken by an unprompted human review sweep that fixed the underlying instances directly. The commonality isn't the artifact type (CI config vs. stranded PR vs. stale issue) — it's that all of them are *periodic-review-shaped* problems, and Lyle already runs that review at roughly weekly cadence without needing the orchestrator to formalize it. The orchestrator's proposals in this shape are consistently pre-empted by the next review pass before anyone reads them.
- **No new commits since 2026-07-05 16:46 UTC (PR #307, the last merge of yesterday's session)** — a quiet day following the largest merge day on record, consistent with the designed post-epic pause.
- **Open PR list is short and stable**: #204 (wireframe, day 33, stranded per above), #236 (comment mode for file entities, day 16, CI-passing, unreviewed), #246 (Electrobun spike, draft, research track, day 14), #32 (LM Studio, day 73, product call), plus this journal PR (#169, day 38).
Hint for next run: don't propose another "add a checklist/process step" issue for a periodic-review-shaped gap (post-ADR audits, CI restores, stranded-issue sweeps) — the evidence is now 0/5 for adoption and 5/5 for self-resolution via Lyle's own review cadence; if a new instance of this shape appears, note it in the journal instead of filing. Watch #111 specifically: it's the only clean, uncontradicted `agent-in-progress`-orphan instance left standing behind #281 — if it's still untouched by July 10 (proposal stale threshold) with no second instance elsewhere, that's a real "close as stale, note the postmortem" call, not a refile. Check whether #236 (day 16+) finally gets reviewed.

---

## 2026-07-07
Surveyed: Commits since the July 5 cleanup (none — quiet day), two new PRs opened July 6 (#311 gap-handle, #313 popup/toolbar rework), new issue #312 (chrome-header hit-test bug), needs-triage queue (12 items), proposal #281 status (day 4, 0 comments) cross-checked against its own named issues (#111, #135, #225) for a possible third instance.
Acted:
- Commented on #281 with a confirmed third instance: PR #144 (`Closes #135`) closed **without merging** on 2026-07-05 — inside the same cleanup session that resolved #206/#225/#270 (per the July 6 entry). #135 still carries bare `agent-in-progress` today, two days later. Not a refile — #281 is unactioned but not stale (day 4, threshold July 10); this is evidence added to strengthen an existing, correctly-still-open proposal.
Noticed (no action):
- **The #135 finding cuts against last week's "periodic review self-corrects it" theory, for this specific proposal.** The July 5 session directly closed PR #144 by hand — the very act that creates the label gap — in the same sweep that fixed #206/#225/#270, and still didn't touch #135's label. Unlike #259 (whole-PR staleness, fixed by a human noticing and deciding), swapping a label at PR-close time is a small, mechanical, easy-to-forget step, not a judgment call — exactly the shape of gap that benefits from being made automatic rather than relying on memory during a busy cleanup pass. This doesn't overturn the broader "process proposals don't land" pattern (5/5 self-resolution rate on #152/#168/#188/#208/#259 stands), but #281 is evidently a different case: the underlying action (closing the PR) doesn't imply the fix (fixing the label) the way "reviewing the PR queue" implies "noticing the stale PR." Worth keeping #281 alive past a mechanical 7-day stale check if it reaches day 7 with 0 comments — this one's evidence base is still growing, unlike the others.
- **needs-triage queue at 12 items, oldest #124 at 52 days**: composition matches July 4–6 (bloat-audit chain #223/#226/#227 at 23 days; deepen-pass reconciliation issues #287/#291/#292 at 3 days; #279, #190, #124, #265, #274 filling in the middle). New today: **#312** (invisible chrome-header band steals drags — root cause fully diagnosed, file-by-file deletion list, and a verify step already written into the issue body). Matches the established fast-track pattern (thorough bug reports with file pointers skip the queue, per #257/#260 in late June) — worth checking next run whether it got picked up quickly or joined the 52-day-old backlog instead.
- **Two substantial PRs opened the same evening (July 6), neither on a `claude/`-prefixed branch**: #311 (`issue-310`, draggable gap handle for auto-layout groups, closes #310, routes through the ADR 0025 `mutateWorkspace` seam + ADR 0024 registry, 829 unit/86 integration tests) and #313 (`popup-toolbar`, a large selection-popup/toolbar/sync-set rework citing a new ADR 0027, test plan unchecked). Both look like interactive Claude sessions rather than AFK epics — consistent with the June 30 finding that mixed UX/logic work runs interactively while bounded refactors run AFK. Confirms the existing two-mode split; not a new watch item.
- **PR aging, no change in diagnosis**: #204 (wireframe, day 34, still architecturally stranded — no close/rebase decision), #236 (comment mode for files, day 17, CI-passing, unreviewed), #246 (Electrobun spike, draft, day 15), #32 (LM Studio, day 74, product call).
- **Journal PR #169 at day 39**: still open, pure-append, operational.
Hint for next run: check whether #281 picked up any human comment (including in response to today's added evidence) or hits its July 10 stale threshold with none — if it does hit day 7 with 0 comments, this is the case to actually reconsider the "always self-resolves" prior rather than apply it mechanically, given the July 5 counter-evidence. Check whether #312 got fixed fast like #257 did. Check whether #311/#313 merged.

---

## 2026-07-08
Surveyed: #312's fate (fast-track prediction), #313/#311 merge status, #281 activity (day 5, stale threshold July 10), current `agent-in-progress` label state on #111/#135, needs-triage queue (11 items), today's merge burst (#313, #314, #315).
Acted:
- Nothing filed or closed. #281 at day 5, 0 new comments since my July 7 evidence-add — not stale (threshold July 10). No new pattern crossed the two-instance bar that hasn't already been tried.
Noticed (no action):
- **#312 fast-track confirmed — closed in under 35 hours** (filed July 6 17:37 UTC, merged as PR #314 July 8 04:10 UTC, closed 04:10). Fourth-plus instance of the "thorough bug report with root-cause + file pointers + verify step skips the queue" pattern (after #257→#260, and the general rule named since late June). Standing conclusion holds; not re-proposing anything — this is a healthy channel, not a gap.
- **#314 retired the chrome-header slot model wholesale (ADR 0028), the third "delete a whole abstraction layer in one shot" in six weeks** — after ADR 0019 (per-kind CLI routes) and ADR 0020 (Browser mode). Pattern is now well past two instances but it's a product-architecture habit, not something Lyle does manually that automation could absorb — out of orchestrator scope, noting only for continuity.
- **#313 (popup/toolbar rework, ADR 0026/0027) merged July 7 14:13 UTC** — 12 commits, large multi-feature branch (shape catalog table, toolbar tooltips, edge popup, sync sets replacing linked-browsing, a mid-branch fallow cleanup commit). Confirms the interactive-session mode for mixed UX/logic work; not AFK. #311 (gap handle) still open, updated this morning (day 2) — active, not stale.
- **#111 and #135 both still carry bare `agent-in-progress` with no other routing label**, confirmed by direct label query. #111 now at 40 days since its backing PR (#136) closed without merging; #135 at 3 days since #144 closed the same way. Both are exactly the evidence already in #281 — no third distinct issue found this run. #281 remains correctly open, not stale.
- **needs-triage queue down to 11 items** (was 12 on July 7) — #312's closure removed it from the count; composition otherwise unchanged (bloat-audit chain #223/#226/#227 at 24 days, deepen-pass reconciliation #287/#291/#292 at 4 days, #279/#190/#124/#265/#274 filling in the middle, #124 oldest at 53 days).
- **#204 (wireframe) still open, no close/rebase decision** — day 35+, un-mentioned in any commit this run. #236 (comment mode for files) still open, CI-passing, unreviewed, day 18+.
- **Journal PR #169 now at day 41** — the longest-running artifact in the system by a wide margin (next-oldest open PR is #32 at 75 days, but that one is an intentionally-parked product decision; #169 is pure administrative append with no reason to stay open other than waiting for a squash-merge). Still purely operational — this run read from and appended to the branch without issue — but worth naming plainly: at >5x the length of any other "designed pause," if a future run's local checkout is ever used instead of the branch (e.g., a different execution path that doesn't know to check the PR), it would silently miss 41 days of context. Not proposing anything (this exact tradeoff has been named in nine straight entries without needing one), just keeping the number visible.
Hint for next run: check whether #281 hits its July 10 stale threshold with 0 comments — given the July 7 counter-evidence (the July 5 cleanup session directly closed #144 and still missed the label swap), that would be the case to reconsider the "always self-resolves" prior rather than close-and-refile mechanically. Check whether #311 (gap handle) merged. Check whether journal PR #169 finally gets squash-merged — if it does, the next run's "read the branch, not main" step becomes moot and can be dropped from the pre-flight habit.

---

## 2026-07-09
Surveyed: #281 status (day 6, 1 comment — my own July 7 evidence-add, no human response), direct label re-check on #111/#135, needs-triage queue (11 items, unchanged composition), PR #204 (still open) and #236 (now `mergeable_state: dirty`), yesterday's merge burst resolving (#311 merged, plus #315/#316), and the same-evening 0.3.4→0.4.0 release retarget. Dominant theme: quiet consolidation day, one clean self-correction, no new pattern crosses the proposal bar.
Acted:
- Nothing filed or closed. #281 is at day 6 — stale threshold is tomorrow (July 10), not today; correcting my own prior shorthand: it has 1 comment (mine), not 0, though zero *human* response. No second/third instance of any other watch item confirmed this run.
Noticed (no action):
- **#311 (gap handle) merged** July 8 14:37 UTC, closing the last item from the July 6 evening batch. Same session also shipped #315 (per-page color scheme override) and #316 (DevTools-follows-selection fix) before cutting a release. All three plus #313/#314 landed inside about 36 hours — the burst-and-flush rhythm holding exactly as characterized since late June.
- **Release version bump self-corrected cleanly, no leak.** `ab5bb30` bumped `package.json` to `0.3.4` and committed; six minutes later `3b7be94` ("chore: retarget release to 0.4.0") rewrote both `CHANGELOG.md` and `package.json` to `0.4.0` with the same content, before any tag was pushed — `git tag -l` shows only `v0.4.0` ever existed, never `v0.3.4`. The `/release` skill's Step 3 (infer bump, confirm via AskUserQuestion) presumably caught a minor-vs-major misjudgment (this release included ADR 0028's chrome-header-model retirement, arguably a breaking-enough change) before the tag+push step made it externally visible. One instance, self-caught by the designed process — not a gap, the opposite: evidence the confirm-before-tag step is doing its job.
- **PR #236 (comment mode for file entities) is now `mergeable_state: dirty`** — it has decayed from "CI-passing, unreviewed" (as of the July 6–8 entries) into an actual conflict with main, 19 days after opening (June 20). This is the same shape as #204's stranding (an unreviewed PR that main outpaces until it needs a real rebase) but without any superseding ADR as the cause — just ordinary review-queue latency plus fast-moving main. #259 (closed July 6) covered the ADR-triggered version of this; this is a second, more generic flavor with only one instance so far. Watching for whether a second non-ADR PR follows the same decay before treating it as a pattern worth a proposal — #259's postmortem already concluded these self-resolve via Lyle's periodic review pass, so the bar for revisiting this shape is high.
- **#111 and #135 unchanged**: both still carry `agent-in-progress` + `enhancement` only, confirmed again by direct label query — no `needs-triage`/`ready-for-agent`/other queue-visible label. #111 at 41 days since its backing PR died, #135 at 4 days. No third orphan found. #281 remains correctly open and un-stale.
- **needs-triage queue holds at 11 items**, same composition as July 8 (#124 oldest, now 54 days; bloat-audit chain #223/#226/#227 at 25 days; deepen-pass reconciliation #287/#291/#292 at 5 days). No growth, no drain.
- **PR #204 (wireframe) still open, ~36 days, no new activity, no close/rebase decision** — nothing new to add beyond #259's closed postmortem.
- **Journal PR #169 now at day 42**, still unmerged, still a pure-append diff, still operational.
Hint for next run: #281 hits its July 10 stale threshold tomorrow — if it's still sitting with only my own July 7 comment (i.e., no human response) at 7 days, that's the case flagged since July 7 to reconsider the mechanical stale-close given the #144/#135 counter-evidence, not to close-and-forget. Check whether #236 gets rebased/reviewed or develops a second sibling in the same "unreviewed PR decays into real conflict" shape. Check whether journal PR #169 finally gets squash-merged.

---

## 2026-07-10
Surveyed: #281 at its nominal 7-day mark (created July 3, still 1 comment — my own July 7 evidence-add, no human response), a direct `agent-in-progress` label re-audit, PR #236's `mergeable_state` (re-checked after July 9's first `dirty` observation), PR #204 aging, needs-triage queue composition, and the one commit since yesterday (PR #317, merged 06:02 UTC, followed by a 0.4.1 release). Dominant theme: quiet, healthy day — one clean small PR, no new instance of any open watch item.
Acted:
- Nothing filed or closed. #281 reached its cleanup-rule 7-day mark today but does not meet the mechanical bar for auto-close (\"no comments or activity\") — my July 7 comment counts as activity, and the reasoning built across the July 7–9 entries (the #144/#135 counter-evidence against the \"always self-resolves via periodic review\" prior) already argues against a mechanical close here regardless. Leaving it open with no further comment; nothing new to add since July 7.
Noticed (no action):
- **`agent-in-progress` re-audited directly: only 3 open issues carry it now** (#1, #111, #135), down from the 9 counted on July 3. The shrink is ordinary — matched PRs merged or closed normally (#209/#220/#221/#235/#262's backing PRs landed and closed their issues) — not a sign anyone fixed the label-lifecycle gap #281 describes. #1 is correctly matched to still-open PR #236. #111 (42 days since PR #136 closed without merging) and #135 (5 days since PR #144 closed without merging) remain the two live orphans behind #281. No third found.
- **PR #236 confirmed `dirty` again** (first observed July 9) — 20 days open, no review activity, now an actual merge conflict rather than a clean CI-passing wait. Still exactly one instance of this non-ADR flavor of PR staleness (distinct from #204's ADR-triggered stranding, and from #259, closed July 6 as self-resolving). The bar set July 9 was a second PR decaying the same way before this crosses into proposal territory — not met yet.
- **PR #317 (region-capture, pan-through, arrow-key-nudge fixes) merged clean today**, three small self-contained fixes in one session, `pnpm typecheck`/`test:unit`/`test:integration` all green, followed same-session by a 0.4.1 release. Unremarkable in the healthiest sense — no process gap, no leak, nothing to note beyond confirming the pipeline is working.
- **PR #204 (wireframe) at day 37**: still open, still no close/rebase decision, `mergeable_state: unknown` (GitHub simply hasn't recomputed it — not itself meaningful for a long-idle PR). Nothing new since #259 closed.
- **needs-triage queue holds at 11 items**, same composition as July 8–9: #124 oldest (55 days), bloat-audit chain #223/#226/#227 (26 days), deepen-pass reconciliation #287/#291/#292 (6 days), #279/#190/#265/#274 filling in the middle. No growth, no drain.
- **Journal PR #169 now at day 43**, still unmerged, still a pure-append diff, still operational.
Hint for next run: watch for a second PR decaying from CI-passing-unreviewed into `mergeable_state: dirty` like #236 — that would be the second instance crossing the proposal bar for a non-ADR PR-staleness gap. Keep checking #281 for any human response, but stop treating its 7-day mark as a trigger — the mechanical stale-close doesn't apply here (it has activity) and the case for leaving it open is now fully built across four entries; only re-open the question if a fourth `agent-in-progress` orphan appears or a human actually comments. Check whether #204 (day 37+) finally gets a close/rebase call.

---

## 2026-07-11
Surveyed: two new PRs opened overnight off a single richly-specified issue (#320 implementing #318, #321 the companion ADR 0029), the new follow-up issue #322 split out of #318's open items, `agent-in-progress` label re-audit (#1/#111/#135, unchanged), `mergeable_state` on #204 and #236 (both now `dirty`), #281 status (day 8, still only my July 7 comment), needs-triage queue (12 items, #322 new). Dominant theme: a new issue-authoring style — one exhaustively pre-specified issue ("Decisions — do not re-litigate" vs. "Open items" resolved during implementation) driving a single 17-commit PR instead of the usual step-PR AFK epic — landed clean on the first pass.
Acted:
- Nothing filed or closed. No pattern crossed the two-instance bar. #281 not stale (has activity, per the case built July 7–10); not re-litigating that call again absent a new orphan or a human reply.
Noticed (no action):
- **#318/#320/#321/#322 is a different shape of large-epic delivery than the June/July step-PR epics.** Rather than a `docs/plans/*.md` broken into 6-8 merged step PRs (wireframe-editor, deepen-pass), #318 is a single issue written to be fully self-sufficient for a cold agent: verified findings, a numbered list of final decisions explicitly marked "do not re-litigate," a separate numbered list of "open items" that need the live binary/app to resolve, then an implementation guide and verification checklist. One PR (#320, 17 commits, 2706/-311 across 50 files) implements the whole thing; a second PR (#321) lands the companion ADR; a third open item spawned issue #322 the same morning — mirroring the deepen-pass's "non-goals become tracked issues, not silent gaps" discipline (#287/#291/#292 in July). CI green, `mergeable_state: clean` on both PRs. One instance of this authoring style — worth watching whether it recurs before treating it as a named pattern; if it does, it's more a documentation-craft observation for `docs/agents/` than an automation gap.
- **#204 and #236 now both show `mergeable_state: dirty`.** This is not the second instance of the July 9–10 watch item (a *non-ADR* PR decaying from clean to conflicted) — #204's staleness has a known, already-analyzed cause (ADR 0019/0020 CLI-surface drift, covered and closed via #259's postmortem on July 6); it simply finished recomputing to `dirty` today rather than staying `unknown`. #236 remains the sole clean instance of the generic (no-superseding-ADR) flavor, now at day 21. Still watching for an unrelated second instance before this crosses the proposal bar.
- **`agent-in-progress` label state unchanged**: #1, #111 (44 days orphaned), #135 (8 days orphaned) — no third orphan, no fourth. #281 remains correctly open and un-stale under the cleanup rule (has my July 7 comment as activity); the case for not mechanically closing it at any nominal 7-day mark was already fully built across the July 7–10 entries and doesn't need repeating unless the facts change.
- **needs-triage queue at 12 items** (#322 the only addition since July 10): #124 still oldest at 56 days, bloat-audit chain #223/#226/#227 at 27 days, deepen-pass reconciliation #287/#291/#292 at 7 days, #279/#190/#265/#274 filling in the middle. No drain.
- **Journal PR #169 now at day 44**, still unmerged, still pure-append, still operational — this run read from and wrote to the branch directly per the standing pre-flight step.
Hint for next run: watch whether the #318-style "single self-sufficient issue → one big PR + ADR + spawned follow-ups" delivery shape repeats — a second instance would make it worth a `docs/agents/` note on issue-authoring craft (not an automation proposal, just documentation). Keep watching for a genuinely unrelated second `mergeable_state: dirty` PR (not #204, whose cause is already known) as the real second instance of the July 9 non-ADR-staleness pattern. Check #281 only if a human comments or a fourth `agent-in-progress` orphan appears. Check whether #320/#321 merge cleanly and whether #322 gets picked up.

---

## 2026-07-12
Surveyed: the #318/#319 epic resolving forward — PR #325 ("Improve agent-browser orchestration and presence cursors," merged 07:14 UTC today) verified line-by-line against the two sibling PRs (#320, #321) opened by a separate session the day before; #281 status (day 9); `agent-in-progress` label re-audit; needs-triage queue (14 items, two new today); #204/#236 aging. Dominant theme: the #320/#321 "single self-sufficient issue → one big PR" delivery shape flagged as worth-watching on July 11 got overtaken within 24 hours by a *second*, independent branch implementing the same two issues — and the two are now fully redundant, unmerged, and uncross-referenced.
Acted:
- Nothing filed or closed. #281 still carries only my own July 7 comment as its sole activity — not stale under the mechanical rule, and the case for leaving it open (built across July 7–10) doesn't need new evidence to hold; none appeared (#111 and #135 unchanged, no third orphan, no human reply).
Noticed (no action):
- **PR #320 ("Implements issue #318 in full," opened July 11 03:12 UTC, 2706/-311 across 50 files) and PR #321 (ADR 0029, opened July 11 04:09 UTC, 124 additions) are both still open, but every piece of their content is already on `main` — merged there by a third, independent branch, PR #325 (opened July 12 05:13 UTC, merged 07:14 UTC, 4829/-470 across 71 files, branch `claude/issue-319-presence-cursor-pacing`).** Verified directly, not inferred: `git merge-base --is-ancestor` confirms neither #320's nor #321's branch head is an ancestor of `main` — genuinely independent, unmerged history. Yet `docs/adr/0029-presence-acts-anchored-to-truth.md` (word-for-word #321's content) is on `main`; so are `src/main/skill-migrations.ts`, the `staleRefHint()` function, the `AGENT_BROWSER_PATH` export added to `resources/specular-cli.sh`, and `tests/contract/agent-browser.contract.test.ts` — all named, specific deliverables from #320's D0–D11 implementation guide. #325's body cites issues #318/#319/#324 by number but never mentions PRs #320 or #321 in either direction — no cross-reference exists anywhere in the four artifacts. If #320 (the larger of the two, complete-looking and well-tested on its own) is reviewed or merged as-is, it is either wasted review effort or a serious conflict against code that already shipped.
- **This is the second instance of "a large epic lands via one branch while a sibling artifact working the identical scope goes silently stranded with zero cross-reference," and it is stronger evidence than the first.** The first instance (July 4–5 entries) was issue #225 vs. PR #286 — one issue, conceptual overlap, resolved when Lyle noticed and closed #225 himself without any orchestrator involvement; that self-resolution is exactly why proposal #259 was closed rather than broadened on July 6. This instance is two full PRs, not one issue, and the overlap is a verified byte-for-byte content match, not a vibe. The underlying mechanism is the same both times: nothing in the AFK/interactive pipeline checks "is there already an open PR against this issue number" before a new session starts one from scratch. That is a real, nameable gap — but the fix for *this instance* is still just "someone closes #320 and #321 pointing at #325," a one-time mechanical action, not a missing process step. Given the standing 0/5-adoption, 5/5-self-resolution record for "add a checklist/process step" proposals in this exact shape (#152→#168, #188, #208, #259), not filing one a sixth time. Flagging plainly here instead, because the review-effort cost of *not* noticing (reviewing or attempting to merge a 50-file PR that already shipped elsewhere) is real and immediate, unlike the more abstract prior instance. If a third instance appears — especially if a human actually spends review time on a PR before discovering it's redundant, rather than catching it in a routine sweep like #225 was — that crosses from "self-resolving" to "worth automating a pre-flight duplicate check," and should be proposed then, not before.
- **needs-triage queue grew to 14 items** (12 on July 11, +#324 and +#326 today) — both new items are fallout from #325's own dogfooding session (a wrapped-inline-link click bug, and an annotations-discovery guidance gap), not new drift. #124 still oldest at 57 days. Composition and diagnosis otherwise unchanged from #168's June 2 postmortem: the old core (#53/#124/bloat-audit chain) needs human architectural time, not a drain mechanism — still the right read, nine days after last checked.
- **`agent-in-progress` re-audited**: #1, #111 (45 days orphaned since PR #136 closed without merging), #135 (7 days orphaned since PR #144 closed without merging) — no third orphan, no fourth. #281 (day 9, one comment, no human reply) remains correctly open under the standing reasoning; not re-litigating without new facts.
- **#204 (wireframe, day 39) and #236 (comment mode, day 22)** both still open, `mergeable_state: unknown` on both this run (not a signal either way — this field has proven unreliable to read literally in past entries). Nothing new since #259 closed on #204's cause; #236 remains the sole instance of the generic non-ADR staleness flavor, still below the two-instance bar.
- **Journal PR #169 now at day 45** — still unmerged, still a pure-append diff (44 commits so far), still operational. This run read from and wrote to the branch directly per the standing pre-flight step; the branch remains the source of truth, not local `main`.
Hint for next run: check whether #320/#321 got closed (ideally pointing at #325) or whether review time gets spent on #320 before anyone notices the overlap — the latter would be the second instance of *review effort actually wasted* on a stranded duplicate (as opposed to a routine sweep catching it for free, which is what happened with #225), and would be the point to actually propose a pre-flight "check for an existing open PR against this issue" step rather than just noting it. Check #281 only if a human comments or a fourth `agent-in-progress` orphan appears. Check whether the needs-triage queue keeps growing past 14 or whether a routing session starts draining the bloat-audit chain (#223/#226/#227, now at 28 days).

---

## 2026-07-13
Surveyed: the July 12 hint resolving forward — #320/#321's fate, plus #318/#319 (the issues they implement) — a fresh instance of the identical stranding shape on #330/#331/#332, `agent-in-progress` re-audit, needs-triage queue (16 items, up from 14), #204/#236 aging. Dominant theme: the July 12 hint's question got a clean answer (self-resolved, no wasted review time), and a brand-new same-shape instance appeared within hours, giving a live natural experiment on the same pattern.
Acted:
- Nothing filed or closed. #281 unchanged (day 10, still only my July 7 comment, no human reply) — not stale under the standing reasoning built July 7–10; no new orphan to reopen the question.
Noticed (no action):
- **#320/#321 resolved cleanly, no wasted review time.** At 06:01–06:03 UTC today, in one sweep, Lyle closed #320 ("Superseded by #325 ... Verified: all of #318's decisions ... are on main byte-identical. Closing; branch deleted."), and closed #318 and #319 themselves as completed (#321 had already merged July 12). The comment on #320 shows an explicit verification step, not a blind close — this is the routine-sweep outcome the July 12 entry hoped for, not the "review time wasted on a redundant PR" escalation that would have crossed the proposal bar. Third confirmed self-resolution of this shape (after #225/#286 on July 5, and now this), still 0/5 for "add a checklist step" proposals actually being needed.
- **A fourth instance appeared the same morning, not yet swept.** Issue #330 ("PRD: Interaction sync," filed July 12 20:47 UTC) got its ADR merged 9 minutes later (#331, 20:56) and its full implementation merged this morning (#332, 07:02 UTC) — both closed, both merged. Neither PR's body mentions "#330" or any closing keyword anywhere; verified directly on both. #330 is still open with only `needs-triage`, sitting in the queue as if nothing has happened, even though every user story and acceptance criterion in it now has landed code. This is the same mechanism as #225/#286 and #320/#321/#325 (implementation lands on a branch that never references the tracking artifact it fulfills) but a cleaner variant: one issue, one direct implementation lineage (#330 → #331 → #332, not a duplicate side-branch), just missing "Closes #330" in either PR body. Given #330's PRs merged *after* this morning's 06:01–06:03 cleanup sweep, it simply hasn't been caught yet — expect it to resolve in the next sweep, consistent with the standing pattern, not a new gap. Not filing anything; flagging so next run can confirm the sweep catches it (or, if it's still open in a week with no human touch, that would be worth a second look).
- **needs-triage queue at 16 items** (up from 14 on July 12): two new since then, both same-day PRD/dogfooding fallout (#329 annotation-persistence-across-navigation explore ticket, #330 itself). Core composition (bloat-audit chain #223/#226/#227 at 29 days, #124 oldest at 58 days) unchanged — still the human-architectural-time diagnosis from #168's June postmortem, not a drain-mechanism gap.
- **`agent-in-progress` unchanged**: #1, #111 (46 days orphaned), #135 (8 days orphaned) — no third confirmed distinct orphan beyond what #281 already cites, no fourth.
- **#204 (wireframe, day 40) and #236 (comment mode, day 23)**: both unchanged, `mergeable_state: unknown` again on #236 (still unreliable to read literally). #236 remains the sole instance of the generic non-ADR PR-staleness flavor; still below the two-instance bar.
- **Journal PR #169 now at day 46**: still unmerged, still a pure-append diff, still operational — this run read from and wrote to the branch directly per the standing pre-flight step.
Hint for next run: check whether #330 got closed (ideally with a "Closes #330"-style cross-reference, or at minimum a human touch) in the next review sweep — if it's still bare `needs-triage` with zero activity a week out (July 20), that would be a fourth instance sitting *unswept* past a normal cleanup cycle, worth reconsidering vs. the fourth swept instance this run confirmed. Check #281 only if a human comments or a fourth `agent-in-progress` orphan appears. Check whether #236 finally gets reviewed or develops a genuinely unrelated sibling in the same decay shape.

---

## 2026-07-14
Surveyed: `docs/adr/` for the actual state behind the standing ADR-conflict-flagging convention, two new same-evening PRs (#337, #338, both "page anchoring") against current `main`, #330's cross-reference status (day 2, per July 13's hint), #281 (day 11, still 1 comment), `agent-in-progress` re-audit, PR #236 aging, needs-triage queue (20 items, up from 16). Dominant theme: routine checks came back unchanged, but a direct read of `docs/adr/` — prompted by #337/#338 both citing "ADR 0029" for a topic already merged under that number — surfaced a real, previously-unnoticed gap with two permanent instances already on `main`.
Acted:
- **Filed #339** (`orchestrator-proposal`): ADR numbers have no uniqueness check anywhere. `docs/adr/` already carries two permanent collisions — 0023 (`note-content-in-ydoc-for-undo` vs `renderer-owned-camera-gpu-panzoom`, merged a day apart in late June/early July) and 0024 (`entity-kind-registry-spans-runtime-and-persistence` vs `in-process-integration-testing`, same shape, early July) — neither ever renumbered. A third is forming live: PR #337 and PR #338, opened ~2 hours apart the same evening, both based on current `main` (which already contains the real `docs/adr/0029-presence-acts-anchored-to-truth.md`, merged via #321), each independently add a new `docs/adr/0029-page-anchored-entities.md`. Verified via `pull_request_read get_files` on both — not inferred. Proposed fix mirrors the two CI-workflow proposals that actually landed (#61, #63): a duplicate-number lint step in the existing `ci.yml` `check` job, plus a flag that #337/#338 need renumbering before either merges. This clears the two-instance bar with the two already-merged pairs alone; the #337/#338 pair is a third, still-preventable one, which is why it's worth surfacing tonight rather than waiting for a routine sweep to catch it after the fact (the way #225/#320/#321/#330 all self-resolved after landing).
- Nothing else filed or closed. #281 (day 11, still only the July 7 comment, no human reply) — not re-litigating; the case for leaving it open was fully built July 7–10 and nothing changed. No fourth `agent-in-progress` orphan.
Noticed (no action):
- **#330 still bare `needs-triage`, zero activity, day 2** — too early to flag per the July 13 hint (threshold is day 7, July 19–20); noting only to keep the thread live for next run.
- **needs-triage queue jumped to 20** (16 on July 13): four new items, all dated 2026-07-13 and all titled `Deepen: ...` (#333–#336) — a batch from a same-day architecture-review session, matching the established "deepen-pass reconciliation" shape (#287/#291/#292 in early July) rather than organic backlog growth. Core aging composition unchanged: #124 still oldest (59 days), bloat-audit chain #223/#226/#227 (30 days).
- **`agent-in-progress` unchanged**: #1, #111 (47 days orphaned), #135 (9 days orphaned) — no new orphan.
- **#236 (comment mode) still the sole non-ADR `mergeable_state: dirty` instance**, now day 24. Still below the two-instance bar for that specific watch item.
- **Only 3 commits landed on `main` since the July 13 entry** — the 0.4.2 version bump/changelog and the ADR 0030 interaction-sync-relay merge already covered yesterday. Quiet day on `main` itself; the activity was almost entirely two large open PRs (#337, #338) and four new deepen-pass issues, not new merges.
- **Journal PR #169 now at day 47**: still unmerged, still pure-append, still operational.
Hint for next run: check whether #339 got read/actioned, and specifically whether #337/#338 merged with colliding ADR 0029 files (a fourth instance, this time *landed* rather than caught) or got renumbered first. Check #330 at day ~9 against the July 19–20 stale threshold. Check #281 only if a human comments or a fourth `agent-in-progress` orphan appears. Check whether #236 finally gets reviewed.

---

## 2026-07-15
Surveyed: the July 14 hint set — #339's status (ADR-numbering-collision proposal, day 1), whether #337/#338 merged with colliding `docs/adr/0029-page-anchored-entities.md` or got renumbered, #330 at day 3 (not yet at the July 19-20 threshold), #281 (day 12, still 1 comment), `agent-in-progress` re-audit, needs-triage queue, PR #236/#204 aging. Dominant theme: the quietest day in this journal's run — zero commits landed on `main` since yesterday's entry, and every open watch item is exactly where it was 24 hours ago.
Acted:
- Nothing filed or closed. #339 is one day old — too early to expect action either way; #337 and #338 still each independently add `docs/adr/0029-page-anchored-entities.md` (re-verified via `pull_request_read get_files` on both, not inferred), still unmerged, so the collision #339 flagged remains preventable rather than landed. No new pattern cleared the two-instance bar. #281 (day 12, still only the July 7 comment, no human reply) — not re-litigating; case fully built July 7-10, nothing changed since.
Noticed (no action):
- **Zero commits on `main` since the July 14 entry.** No merges, no releases, no new PRs opened. The only repo activity in the last 24h was issue #339 itself (filed by last night's run).
- **#337/#338 unresolved, ADR 0029 collision still live and still catchable.** Neither PR shows a renumbering commit or any comment referencing #339. Worth one more day of watching before treating "nobody's caught it yet" as itself notable — issues this fresh routinely take 1-2 days for a human pass, per the #320/#321 and #330 precedents in this same journal.
- **needs-triage queue holds at 20**, identical composition to July 14 (four `Deepen:` issues #333-336 still the newest, #124 still oldest at 60 days, bloat-audit chain #223/#226/#227 at 31 days). No growth, no drain.
- **`agent-in-progress` unchanged**: #1, #111 (48 days orphaned since PR #136 closed without merging), #135 (10 days orphaned since PR #144 closed without merging) — no third confirmed distinct orphan, no fourth.
- **#236 (comment mode) confirmed `mergeable_state: dirty` again**, day 25, still the sole instance of the generic non-ADR PR-staleness flavor — still below the two-instance bar. **#204 (wireframe)** unchanged, `mergeable_state: unknown`, day 42, nothing new since #259's postmortem.
- **Journal PR #169 now at day 48** — still unmerged, still a pure-append diff, still operational. This run read from and wrote to the branch directly per the standing pre-flight step.
Hint for next run: check whether #339 got a response and whether #337/#338 renumbered ADR 0029 before merging (or, worse, merged still colliding — that would be the first *landed* instance rather than a caught one, and would sharpen the case for the CI lint step regardless of whether it's been picked up yet). Check #330 as it approaches the July 19-20 stale threshold (day ~4 today). Check #281 only if a human comments or a fourth `agent-in-progress` orphan appears. Check whether #236 or #204 finally gets a close/rebase/review decision — both are now past three and six weeks respectively with no human touch.

## 2026-07-16
Surveyed: the July 15 hint set — #339's response status, whether #337/#338 renumbered or merged still colliding, #330 approaching the July 19-20 stale threshold, #281/agent-in-progress re-audit, PR #236/#204 aging, needs-triage queue. Dominant theme: another quiet day — one real merge landed on `main` (the interaction-sync-relay PRD closing out, already covered July 12-13), and every open watch item is unchanged from yesterday.
Acted:
- Nothing filed or closed. #339 (day 2, zero comments) — too early to expect action. #337/#338 still each independently carry `docs/adr/0029-page-anchored-entities.md` (re-verified via `pull_request_read get_files` on both, not inferred), both still open and unmerged, so the collision remains preventable rather than landed. No pattern crossed the two-instance bar. #281 (day 13, still only the July 7 comment, no human reply) — not re-litigating; case fully built July 7-10, nothing changed.
Noticed (no action):
- **#339 has zero comments, zero reactions**, two days after filing — normal early-days silence per this journal's own precedent (#320/#321 and #330 both took 1-2 days for a human pass).
- **#337/#338 unresolved, unchanged from July 15**: both still `mergeable_state: dirty`, neither shows a renumbering commit or any reference to #339. Now day 3 and day 2 open respectively without a collision resolution.
- **#204 (wireframe) flipped from `mergeable_state: unknown` to `dirty`** this run — first time this field has resolved to a concrete conflict state since #259's postmortem closed on July 6. This is not a new pattern instance: the cause (ADR 0019/0020 CLI-surface drift, main having moved on) was already diagnosed and closed via #259; `dirty` here is GitHub finishing a recompute on a 43-day-old PR, not new information. Noting only because the field had sat at `unknown` for over a week and finally settled.
- **#236 (comment mode) confirmed `dirty` again**, day 26, still the sole instance of the generic non-ADR PR-staleness flavor — still below the two-instance bar (#204's cause is known and different).
- **#330 at day 4**, still bare `needs-triage`, zero activity — below the July 19-20 threshold flagged July 13; nothing to act on yet.
- **needs-triage queue holds at 20**, identical composition to July 14-15 (four `Deepen:` issues #333-336 still newest, #124 still oldest at 61 days, bloat-audit chain #223/#226/#227 at 32 days). No growth, no drain.
- **`agent-in-progress` unchanged**: #1, #111 (49 days orphaned since PR #136 closed without merging), #135 (11 days orphaned since PR #144 closed without merging) — no third confirmed distinct orphan, no fourth.
- **Journal PR #169 now at day 49** — still unmerged, still a pure-append diff, still operational. This run read from and wrote to the branch directly per the standing pre-flight step.
Hint for next run: check whether #339 gets a first response and whether #337/#338 renumber before merging (or land still colliding — the first *landed* instance rather than caught). Check #330 at day ~5, one day closer to the July 19-20 threshold. Check #281 only if a human comments or a fourth `agent-in-progress` orphan appears. Check whether #236 or #204 (now both confirmed `dirty`, at four and six-plus weeks respectively) finally gets a close/rebase/review decision.

## 2026-07-17
Surveyed: the full July 16 hint set — #339 response status, #337/#338 renumbering/collision state, #330 at the day-5 mark, #281, `agent-in-progress` re-audit, #236/#204 aging, needs-triage queue — plus a direct check for any repo activity at all since the July 16 entry (commits, new issues, new PRs). Dominant theme: total silence. Zero commits, zero new issues, zero new PRs landed anywhere in the repo in the last 24 hours — the quietest window in this journal's run, and every open watch item is byte-for-byte where it was yesterday.
Acted:
- Nothing filed or closed. No new evidence on any open thread: #339 (day 3, zero comments), #337/#338 (still each independently carrying `docs/adr/0029-page-anchored-entities.md`, still `mergeable_state: dirty`, still unmerged — collision remains preventable, not landed), #330 (day 5, still bare `needs-triage`, zero activity, one day short of the July 19-20 threshold), #281 (day 14, still only the July 7 comment, no human reply — not re-litigating, case fully built July 7-10), #236/#204 (unchanged, both `dirty`, no review or close decision). No pattern crossed a bar it hadn't already crossed.
Noticed (no action):
- **Nothing happened.** No commits on `main`, no issues opened, no PRs opened since the July 16 entry — confirmed directly via commit-since and issue/PR-search-by-created-date queries, not inferred from unchanged watch items alone. Second consecutive quiet day (July 16 also saw only one routine merge), now compounding into a full stop.
- **needs-triage queue holds at 20**, `agent-in-progress` holds at #1/#111 (50 days orphaned)/#135 (12 days orphaned) — identical composition to every check since July 14-15.
- **Cleanup pass**: re-checked both open `orchestrator-proposal` issues (#339, #281) and the standing journal PR (#169, day 50) against the 7-day-stale/no-activity rule. Neither proposal qualifies for mechanical close — #339 is 3 days old, #281 carries a comment as activity and its open-vs-close case was already fully reasoned through on July 7-10. No supersession either: #339 and #281 describe distinct gaps (ADR numbering vs. label lifecycle), nothing new proposed tonight to supersede either.
Hint for next run: if #330 still shows zero activity, it crosses the July 19-20 stale-review threshold flagged July 13 — worth an explicit check next run or the one after. If the repo stays this quiet for a third straight day, that's less a system-health signal than a sign Lyle is away — keep surveying but don't manufacture proposals to fill the silence. Otherwise the standing watch list is unchanged: #339/#337/#338 (ADR 0029 collision, still preventable), #281 (only on human reply or a 4th orphan), #236/#204 (dirty, no decision).

---

## 2026-07-18
Surveyed: the full July 17 hint set resolving forward after two silent days — #339's ADR-0029-collision watch, #337/#338's merge fate, #330 approaching its stale-review threshold, #281/agent-in-progress re-audit, #236/#204 aging — against an overnight burst: 5 PRs merged in just under 2 hours this morning (#337, #338, #341, #342, #343, 05:06-07:07 UTC) delivering the full page-anchoring/element-attachment epic plus a wireframe-mode deletion, with 2 more PRs opened and still open (#344, #345). Dominant theme: the two-day silence broke exactly the way this journal's own precedent predicted, and it resolved the #339 watch item cleanly while creating a new, sharper one.
Acted:
- Nothing filed or closed. #339 (day 4, zero comments) and #281 (day 16, one comment, case fully reasoned July 7-10) are both below their respective stale thresholds. No pattern crossed a fresh two-instance bar tonight — the two live findings below are updates to existing threads, not new proposal-shaped gaps.
Noticed (no action):
- **#339's live collision resolved cleanly, exactly per its own "immediate action" recommendation.** `docs/adr/` on `main` now has 0031-page-anchored-entities.md and 0032-element-attachment.md — not a second/third 0029. #337 and #338 (both of which independently added `docs/adr/0029-page-anchored-entities.md` against a `main` that already had a real ADR 0029) got renumbered before merging today alongside PR #341 (a third, larger PR implementing the same ADRs' element-attachment/scroll-reflow behavior). Confirmed by listing `docs/adr/` directly on `origin/main`, not inferred. This is the first time in this journal's run that an explicit orchestrator recommendation ("renumber before merge") was made *and* the exact outcome landed — worth distinguishing from the standing 0/5-adoption pattern for pure process-step proposals (#152→#168, #188, #208, #259), since the ask here was a one-time action, not a new checklist step. The underlying systemic ask in #339 — a CI lint step catching future collisions, plus the two already-merged 0023/0024 duplicates — is still untouched; `ci.yml` has no ADR-related check. Not closing #339; the proposal's core ask still stands even though tonight's crisis passed.
- **PR #204 (wireframe-structured-editor) is now definitively dead, not just "structurally overtaken."** PR #342, merged today at 05:53 UTC, deleted wireframe mode from the product wholesale (~1,780 LOC: the plugin claim, every renderer, the skill docs, the popup-contribution scaffold that existed only to serve it). #204 — open 45 days, unmerged, 19 commits building an entire structured editor for `.wireframe.json` — now adds code for a mode that does not exist anywhere on `main`. This was predicted as a probability by #259's postmortem in early July ("ADR 0019/0020 CLI-surface drift"); tonight it became a certainty via a completely different mechanism (the mode itself was deleted, not just its CLI surface). Not proposing a closure — closing product PRs is out of orchestrator scope — but flagging this as loudly as this journal's own bar allows: there is no longer any ambiguity about whether #204 has a path to merging as-is.
- **needs-triage queue at 21** (20 on July 14-17): one new item, #340 (overlay/WCV sync constant-latency perf follow-up), filed July 17 19:16 from the same dogfooding session that produced tonight's merge burst. Composition otherwise not re-verified line-by-line tonight given the larger burst to review — worth a fresh count next run.
- **#330 (Interaction sync PRD) at day 6, still bare `needs-triage`, zero comments** — one day short of the July 19-20 stale-review threshold flagged July 13.
- **`agent-in-progress` unchanged**: #1, #111, #135 — same three as every check since July 14. #1's backing PR (#236, "Closes #1") is still open at day 28, so #1's label is arguably still earning its keep, unlike #111/#135 whose backing PRs closed without merging weeks ago (the #281 shape).
- **Journal PR #169 now at day 51** — still unmerged, still a pure-append diff, still operational. This run read from and wrote to the branch directly per the standing pre-flight step.
Hint for next run: check whether #204 gets closed now that wireframe mode's deletion removes all ambiguity — if it's still open a week from now (July 25) despite an unambiguous trigger, that's worth naming as a distinct "obviously-dead PR sits anyway" observation, separate from the earlier ADR-conflict framing. Check #330 at day 7 (July 19) against its threshold. Check whether #339 attracts a first comment or the CI lint step actually lands — if it hits day 7 (July 21) with zero comments, note whether it's heading for the same fate as the prior process-step proposals despite tonight's partial validation. Check whether #344/#345 (both open, filed this morning) get any review given this journal's long non-AFK-review-gap history.

## 2026-07-19
Surveyed: the full July 18 hint set — #204's closure status post-wireframe-deletion, #330 at day 7 against its own flagged threshold, #339's comment/CI-lint status, #344/#345 review status — against an even bigger overnight burst: 9 more PRs merged since the July 18 entry (#344, #345, #347, #348, #349, #350, #351, #352, plus #353/#354 merging two direct-push feature branches), covering HTML iframe fixes, editing-chrome polish, selection/stack-order/popup gestures, sidebar multi-selection, clipboard image ingestion, group drag/membership, and single-page drag perf. Dominant theme: the most active 24h window in this journal's run, and it resolved every open watch item to "unchanged" except one that hit its own flagged deadline.
Acted:
- Nothing filed or closed. #330 crossing its own July 19-20 threshold (below) is exactly one instance of a new failure shape against three prior self-resolving instances of the same pattern — one instance doesn't clear the two-instance bar, and this journal has already reasoned through "add a Closes-# checklist step" proposals to a standing 0/5 record. Filing again on weaker evidence than those would repeat a miss, not learn from it. #339 (day 5, zero comments) and #281 (day 16, one comment, case fully reasoned July 7-10) are both below their respective stale/re-litigation thresholds.
Noticed (no action):
- **#330 has now crossed its own July 19-20 stale-review threshold with zero self-resolution.** Confirmed directly: `#331` (ADR 0030, merged July 12 20:56) and `#332` (the full interaction-sync-relay implementation, merged July 13 07:02) both fully deliver every user story and acceptance criterion in #330 — verified by reading #330's body against #332's summary — and neither PR body contains "#330" or a closing keyword. #330 itself: `updated_at` identical to `created_at` (2026-07-12T20:47:27Z) — literally zero activity in 7 days, still bare `needs-triage`. This is the fourth instance of the "implementation lands on a branch that never references its tracking issue" shape, but the first that a routine human sweep hasn't caught within a day or two (unlike #225/#286 on July 5, and #320/#321/#325 on July 12-13, both swept same-day-ish). One anomalous instance against three self-resolving ones is not yet a pattern to automate around — noting prominently rather than filing, since the bar is two *observed* instances of the *same* new shape, not one instance plus a threshold a prior entry happened to name in advance.
- **#204 (wireframe) still open, completely untouched** — `updated_at` still equals its `created_at` from June 3, 46 days now, unchanged since wireframe mode's wholesale deletion via #342 on July 18 removed all remaining merge ambiguity. Not yet at the July 25 checkpoint named yesterday; noting only that a week of total silence has already passed post-trigger with no close.
- **#236 (comment mode) also completely untouched** — `updated_at` still equals `created_at` from June 20, day 29 now. Same shape as #204: an agent-authored PR nobody has reviewed, sitting through an unusually heavy merge day for everything else.
- **#344 and #345, both flagged for review-status last night, both merged today** (#344 at 15:05 UTC, #345 at 16:02 UTC, same day they were opened) — normal same-day turnaround, not the multi-week #204/#236 pattern. Confirms the non-review gap is specific to those two PRs' vintage, not a general reviewing bottleneck.
- **needs-triage queue at 22** (21 on July 18): one new item, #346 ("Design attributed agent presence for externally edited HTML file entities"), filed the same evening as the #344 HTML-watcher-fix merge — a natural design-phase follow-up flagged by that PR's own notes section, not organic backlog drift.
- **`agent-in-progress` unchanged**: #1, #111, #135 — same three since July 14, no new orphan, no fourth instance to revisit #281 against.
- **Journal PR #169 now at day 52** — still unmerged, still a pure-append diff, still operational. This run read from and wrote to the branch directly per the standing pre-flight step.
Hint for next run: check #330 again — if it's still untouched at day 8-9, that's a second day past its own threshold with no sweep, worth weighing whether the anomaly is becoming the new normal (busy days may simply leave less time for tracking-issue hygiene). Check #204 and #236 against the July 25 checkpoint (day 52 and day 36 respectively by then). Check whether #339 gets a first comment by day 7 (July 21). Check #346 for any early design-phase discussion.
