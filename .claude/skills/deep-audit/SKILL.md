---
name: deep-audit
description: Repeatable architectural audit of one codebase area — fan out gather-agents over a static-analysis ground truth, synthesize the one missing-abstraction diagnosis, and emit tracer-bullet cleanup issues. Use when the user wants to audit/improve an area for bloat, code reduction, deepening, or continued architectural improvement, or says "deep audit", "audit <area>", "find what to cut".
---

# Deep Audit

Turn one area of the codebase into a ranked, no-regression cleanup plan. Conduct, don't do-it-all: agents gather, you synthesize. Reductions only — every proposal deletes, reuses, or collapses.

Default scope is `$ARGUMENTS` (a directory or domain). If none given, ask which area; never audit "the whole app" — smaller scope, deeper insight.

## Loop

### 1. Ground truth first (optional sharpener, NOT a gate)
If the area is JS/TS, run the `fallow` skill and hand its output to the agents as **evidence to interpret** for the dead-code section. It does not scope the audit: agents read/grep the whole area in step 3 regardless, and Fallow misses plenty (logic bugs, missing abstractions, non-exported dead symbols — all of which agents must still find). Unflagged ≠ unevaluated. Skip this step entirely if Fallow doesn't apply or isn't available. Where it does flag, note which "dead" symbols are exported — may be test-only, verify before deleting.

### 2. Read the domain
Read `CONTEXT.md`, the relevant `docs/adr/*`, and the nearest `CLAUDE.md`. Agents must use real glossary terms and must know which invariants are load-bearing.

### 3. Fan out a FIXED set of gather-agents
One per sub-domain (aim 4–6, not "the app"). Two passes on the highest-value sub-domain, one elsewhere. Hard-cap it.

Each agent reads/greps its **whole** sub-domain (Fallow flags are leads, not the boundary). Each agent: **foreground** (not background), model **sonnet**, given the Fallow output (if any) + the domain docs + this contract:

> One turn. Do the full analysis with Grep/Read now; your FINAL message MUST be the report — no status updates, no "I'll continue". Grep to verify before asserting unused. Apply the ponytail lens (delete/reuse/collapse over add). Return EXACTLY:
> 1. MAP — key files, LOC, responsibilities (one paragraph)
> 2. BLOAT & DEAD CODE — items with file:line + confidence
> 3. CONSOLIDATE — patterns to combine + rough LOC saved
> 4. DEEPEN — where one stronger primitive replaces several shallow ones
> 5. CUT CANDIDATES — what a minimal version drops + what's lost
> 6. GREENFIELD NOTE — what a lean rebuild MUST keep
> Each item: effort (S/M/L) + risk (low/med/high).

If an agent returns a placeholder instead of a report, re-dispatch it once. Don't let the set re-spawn beyond the fixed plan.

### 4. Synthesize (you, the lead — the valuable part)
Dedupe across agents, rank by leverage, and force the two questions that produce real insight:
- **"What single primitive, if it existed, would dissolve the most duplication?"** — the one diagnosis the lists are symptoms of.
- **"What here is load-bearing and must NOT be touched?"** — from the GREENFIELD notes + ADRs.

Separate **Tier-0 cleanup** (dead code, safe dedup, any correctness bugs the audit surfaced) from **macro proposals** (the missing abstractions). Flag decision-forks as HITL.

### 5. Emit issues
Run the `to-issues` skill on the synthesis: tracer-bullet slices, cold-agent context, and a "tests green, no functionality loss" gate on every refactor slice. ADRs/decisions become HITL issues; cleanups are AFK.

## Guardrails
- Reductions only. If a proposal adds code, it must delete more than it adds.
- Never propose refactoring away something a GREENFIELD note or ADR flags as load-bearing.
- Cap coverage at ~2 passes/sub-domain — re-treads add nothing.
- Sonnet to gather, lead model to synthesize.
