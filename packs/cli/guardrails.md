# CLI self-heal guardrails

Hard limits. These win over the charter and the backlog. A fire that cannot
proceed without breaking one of these should stop and leave a `REVIEW:` note.

## Branch & landing

- **Branch only, never main.** Commit to the checked-out self-heal branch. Never
  push to `main`/`master`. The loop refuses to run on those branches.
- **No auto-merge.** This branch is reviewed by a human before it reaches main.
  Your job ends at a green commit on the branch.
- **One unit of work per fire**, one commit (code + backlog.md together).

## Don't game the metric

- **Never weaken, skip, or delete a probe or test to go green.** If a probe is
  genuinely wrong, leave a `REVIEW: probe X looks wrong because …` note in
  backlog.md ## Todo and move on. Changing the bar is a human decision.
- **Never delete a feature** to "simplify." If removal looks right, write a
  `REVIEW: consider removing X because …` note instead of doing it.

## Verify, always

- `pnpm typecheck && pnpm test:unit` must pass before you commit.
- The probe command in charter.md must pass before you commit.
- Heal (red suite) preempts improve (new work). Never start a Todo while red.

## Skill edits

The CLI's agent-friendliness includes its skill docs. If you change CLI behavior
that the skill describes, update the skill — and per CLAUDE.md, update **both**
`resources/skills/specular/SKILL.md` and `.claude/skills/specular/SKILL.md` in the
same commit (the installed `~/.claude` copy is auto-overwritten on app launch).

A skill edit is only safe if the probes still pass using it — a degraded skill
poisons every future fire. If you can't verify that, leave it as a Todo.
