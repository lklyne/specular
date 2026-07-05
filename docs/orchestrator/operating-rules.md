# Orchestrator operating rules

Versioned rules the nightly orchestrator follows. The cron prompt is unversioned;
this file is not. Change behavior by editing this doc in a PR, not by editing the
prompt. The prompt's only job is to point here and run.

See `journal.md` for the run log these rules were distilled from.

## 1. Deliver fixes as PRs, not proposal issues

Default: a bounded, agent-executable fix (CI / workflow / config / docs,
roughly ≤30 lines, mechanical, no product-direction judgment) ships as a
**ready-to-merge PR** on a `claude/orchestrator-*` branch — already committed,
CI green, body explaining the two instances that justify it.

Reserve `orchestrator-proposal` **issues** only for changes that need a human
decision *before* code exists: design questions, product-direction calls,
anything where writing the code first would presume the answer.

Why: issues #152, #168, #188 each stalled at day 7 / 0 comments and were closed
stale. The identical fix re-delivered as PR #207 merged in 2 days. The delivery
channel was the bottleneck, not proposal clarity. An unreviewed issue queue is
where proposals go to age out; a PR is a decision Lyle can make in one click.

## 2. One line per unchanged watch item — never re-log state

A watch item unchanged from the previous entry gets at most one pointer line
(`still watching X — unchanged`). Do not restate its full numbers. If it is
unchanged for 3+ consecutive runs, drop it from the journal entirely until it
moves; the previous entry already holds the state.

Why: "Write insight, not activity." The needs-triage queue was re-logged
near-verbatim for ~15 consecutive runs ("5 items, all 0 comments, @claude
invocable N days") — pure noise the journal's own header forbids.

## 3. CI/infra improvements land on pain, not on a calendar

A hypothetical infra improvement with no failing PR to point at will stall (see
#63: filed, sat 7 days, landed the moment a real refactor made the gap visible).
Before filing/PRing an infra fix, name the concrete failure it would have caught.
No live failure → note it as a watch item and wait for the second instance,
don't open anything yet.

## 4. Two instances before automating; respect the verdict on closed proposals

A pattern needs two real, cited instances before it becomes a PR or proposal.
Do not re-file what was closed: the needs-triage drain (#152 → #168) was closed
twice because the residue is human-bandwidth-bound architectural decisions, not
an automation gap. That verdict stands — do not propose a v3.
