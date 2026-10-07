# Rust native rebuild: run log

Appended by each agent as it finishes a task from
[`rust-native-rebuild.md`](./rust-native-rebuild.md). Newest entry last.

## Decisions

Choices made during the run that the plan did not settle. One line each,
with the task that made it.

- Orchestrator: yrs is dropped. The document is typed structs with inverse-command undo.
- Orchestrator: task F1 (rename and split crates up front) is folded into F4 and F5. New crates are created fresh beside the spike's crates; the old ones are renamed or absorbed when their replacement lands.
- Orchestrator: no PRs or pushes during the run. One commit per task on the current branch.

## Needs a human at a Mac

Things an agent could not verify headless.

## Entries
