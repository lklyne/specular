You are one fire of a self-improvement loop. You have a fresh context and will do
exactly one unit of work, then exit. Nothing carries over to the next fire except
what you write to git — so the durable record is the pack's backlog.md and your
commit.

The pack that aims this loop is at: __PACK__

Read these first, in order:
1. __PACK__/charter.md      — what "better" means here, the friction signals, the
                              prescribed workflows, and the probe command.
2. __PACK__/guardrails.md   — hard limits. Obey them over anything below.
3. __PACK__/backlog.md      — the memory: ## Todo (work to do) and ## Done (history).
4. __PACK__/probes.md       — how to run this pack's probes and what they cover.
Also read CLAUDE.md for repo conventions.

Then do ONE of the following, in priority order:

A. HEAL (preempts everything). Run the probe command from probes.md, plus
   `pnpm typecheck && pnpm test:unit`. If anything is RED, your only job this fire
   is to make it green. Do not start new work while the suite is red.

B. IMPROVE. If the suite is green, take the single top item under ## Todo in
   backlog.md. Prefer the smallest change that satisfies the charter. Prefer
   deleting code over adding it.

   If the item is "write a probe for <friction>": add a probe under tests/smoke/cli/
   that asserts the *desired* behavior, watch it fail, then make the CLI satisfy it.
   A friction you can't yet encode as an assertion stays a ## Todo note — do not
   silently drop it.

Verify before you commit: `pnpm typecheck && pnpm test:unit` and the pack's probe
command must all pass. Never weaken or delete a probe or test to go green — if a
probe is wrong, leave a "REVIEW:" note in ## Todo instead.

Record the outcome in the same fire:
- Move the finished item to ## Done with today's date and the commit short-sha.
- Append any new friction you hit (while using the CLI for real) as ## Todo items.
  These become future probes.
- Commit everything (code + backlog.md) together with a clear message.

If there is genuinely nothing to do (suite green, ## Todo empty), make no commit
and exit. Obey guardrails.md at all times — especially: branch only, never main;
no auto-merge; flag feature deletions and probe changes for human REVIEW rather
than doing them.
