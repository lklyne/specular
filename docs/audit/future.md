# Audit loop — future ideas (not built)

`deep-audit` runs on one area, on demand. These are notes for turning it into a
scheduled, looping system **later** — deliberately not built yet.

## Decision: validate manually first

Run `/deep-audit <area>` by hand on a few areas before automating anything.
Automate only the parts that prove tediously repetitive. Signals to watch
across manual runs (they decide whether/what to automate):

- Issue quality — are filed issues real and triageable, or noise?
- Dedup pain — does a second run re-propose existing issues?
- Triage throughput — can a human clear the `needs-triage` queue one area produces?
- Cross-area signal — does a recurring missing-primitive pattern emerge, or was it a one-off?

## The shape, if/when it's worth building

```
manifest (areas + last-audited SHA) ─pick stalest *changed* area─▶ /deep-audit <area>
       ▲                                                                 │
       │ update SHA + 1-line digest                       files issues (dedup) + appends synthesis
       │                                                                 ▼
  weekly trigger ◀──────────────────────────────────────────────  ledger (per-area diagnoses)
       │                                                                 │
       └── every N runs ─▶ cross-cut pass reads ledger ─▶ cross-area proposals as issues
```

- **Manifest** (`docs/audit/areas.md`) — logical groups seeded from `architecture.md`
  (runtime, canvas/interaction, preload/IPC, control-plane, panels, plugin), each with
  last-audited SHA + one-line digest. No auto-clustering; the seams are already known.
- **Pick = churn heuristic, not a planner** — area with the most `git diff` since its
  last-audited SHA. Quiet areas are skipped.
- **Ledger** (`docs/audit/ledger.md`) — append each area's synthesis (missing-abstraction
  diagnosis + GREENFIELD must-keeps). This is the substrate for cross-area insight.
- **Cross-cut pass** — every N runs, read the ledger; surface primitives that recur across
  3+ areas (e.g. this session's "entity kind is the wrong unit of code" spanned six layers).
- **Schedule** — weekly web trigger (or `/loop`), one area per fire. Weekly, not continuous:
  architecture moves slowly; re-auditing unchanged code is waste.

## Review gates (map to existing primitives)

- Loop files `needs-triage` issues → human triages (`ready-for-agent` / `ready-for-human` / `wontfix`).
- Macro proposals land as HITL ADR issues that block their implementation slices.
- `ready-for-agent` work is built by the `afk-feature` pipeline (one PR/step, integration PR to review).
- The loop **proposes only** — never edits code, never merges.

## Guardrails

- Dedup against open issues before filing.
- Backpressure: if the loop's own open `needs-triage` count exceeds a threshold, pause filing and just report.
- Churn-gate: no diff since last audit → skip.
- One area + at most one PR per fire. Bounded cost (~6 agents/run), bounded review.
