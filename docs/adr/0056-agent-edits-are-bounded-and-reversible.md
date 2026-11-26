# ADR 0056: An agent's edits are reversible by the agent, land as one step per run, and run fenced in

**Status:** Proposed. Not built. Written from a read of Editroom (`robbietilton/Editroom` at `f6e947b`, 1.0.2), a Swift video editor whose agents edit the timeline through MCP. The user has not reviewed this ADR.
**Date:** 2026-11-26
**Related:** [ADR 0054](./0054-every-editor-is-an-actor.md), one undo stack per actor, which this builds on and whose open Cmd+Z question this answers. [ADR 0041](./0041-typed-document-inverse-command-undo.md), `Command` and `Document::apply`. [ADR 0046](./0046-agent-browser-announces-its-acts.md) and [ADR 0029](./0029-presence-acts-anchored-to-truth.md), agent presence. [ADR 0052](./0052-one-runner-one-state-document.md), the scenario runner that checks undo. [Big run decisions](../plans/rust-native-big-run.md) 1 and 10.
**Code today:** `native/crates/specular-api/src/act.rs` (`POST /history/undo`), `native/crates/specular-cli/src/main.rs` (`HELP`, `run`), `native/crates/specular-agent/src/cli.rs` (`claude_args`, `ALLOWED_TOOLS`), `native/crates/specular-shell/src/agent/{backend,claude}.rs`, `native/crates/specular-api/src/unported.rs` (the dropped `/mcp/session/*` routes).

## Context

Editroom shipped 1.0 in about ten days with agents as first-class editors. Most of what it does for agents Specular already has in some form: cursors per agent, one undo step per write, a skill. Six things it does are worth taking, and each one lands on a seam the Rust app already has.

Where Specular stands today:

- **Undo.** The API has `POST /history/undo` and `/redo`, but they walk the one shared history Cmd+Z walks (`act.rs:170`). The CLI has no `undo` verb. An agent that makes a mistake can only undo whatever came last, which may be the person's edit. ADR 0054 proposes a stack per actor and leaves open what Cmd+Z does to an agent's work, noting that its preferred answer needs a one-key way to revert a run before it ships.
- **Runs.** An in-app agent run issues many CLI calls. Each is one undo step, so a twenty-call run is twenty Cmd+Zs.
- **The `claude` fallback.** When `specular-helper` is missing, the app runs the user's `claude -p` (`backend.rs:98`). It passes a permission mode, an allowlist and a system prompt (`cli.rs:97`). It does not stop the user's own MCP servers, plugins or global settings from loading into the run, and `Permissions::Dangerously` maps to `bypassPermissions` with no allowlist.
- **The Rust CLI.** `--help` already answers without the app, because `run` prints `HELP` before it builds a `Client`. There is no `--version` and no machine-readable list of verbs. Every other path opens a session against the running app.
- **MCP.** The deleted Electron app had an MCP server (PR #430, open against `main`, brought it to parity with the CLI). The Rust app has none: `/mcp/session/open`, `ping` and `close` are answered as unported and dropped.
- **The repo.** `.gitignore` covers `.env` and `.mcp.json`. Nothing stops an agent from committing a key, a `settings.local.json` or a path under someone's home folder. More agents commit to this repo every week.

## Decision

### 1. An agent can undo its own steps

`specular undo` and `specular redo` are CLI verbs, and `undo` and `redo` are MCP tools. Under ADR 0054 they act on the calling session's stack, so an agent takes back its own mistake and never the person's edit. Until 0054 lands, they act on the shared history, the same as `POST /history/undo` does now, and the verb's help says so.

Each verb that writes stays exactly one step. That is already the rule (`patch.rs`, `tasks.rs`, `annotations.rs` each wrap their writes in one `Command::Batch`), and this makes it a rule agents can rely on: a verb an agent calls once, it can take back with one `undo`.

### 2. An in-app run is one step on the person's side

The person sees an in-app agent run as one edit. When the run ends, all of its steps collapse into one entry, so one undo reverts the run.

This answers ADR 0054's open question. Cmd+Z stays the person's own work. The chat panel gets "Undo this run", which reverts the run's collapsed step, and Cmd+Z in the chat panel while a finished run is selected does the same. That is the one-key way back 0054 says must exist before per-actor undo ships.

Editroom gets this by having the agent edit a hidden copy and replaying the result as one step. Specular does not hide the copy: the person watches the agent work (ADR 0046), and the cursor acting on the real canvas is the point. Instead the run's steps are grouped as they happen. The run's actor (its session id, which the shell already passes as `SPECULAR_SESSION_ID`) opens a group at run start and closes it at run end; under 0054 that is one undo manager whose captures merge for the run, and before 0054 it is a `History` group that `undo` pops whole. An agent's own `specular undo` inside a run still takes back a single step.

A hidden draft stays available for one case: a run started with no window to watch it, such as a scheduled or headless run. That run clones the `Document`, applies its commands to the clone, and lands them as one `Command::Batch` on the real canvas when it ends. Because every write goes through `Document::apply`, this is a clone plus a replay, and it rehearses the server-ordered command log the cloud plan needs.

### 3. The `claude` fallback runs fenced in

The fallback passes, in addition to what it passes now:

- `--strict-mcp-config` with an `--mcp-config` that names only Specular, so the user's other MCP servers do not load into a canvas run.
- `--setting-sources project` (or none, when the run's folder is the space folder), so the user's global hooks, plugins and permissions do not apply to a run the app started.
- `--disallowedTools` for the verbs a canvas run must not take on its own: `specular share`, `specular share join`, `specular tab delete`, and any verb that writes outside the space folder.

Editroom also runs its agent in an empty folder so no CLAUDE.md is read. Specular cannot, because an in-app run often edits the user's repo and needs it. The run's folder stays the request's `cwd` or the space folder, and the fence comes from the flags above.

`Permissions::Dangerously` keeps its meaning, but the MCP and settings fence still applies: bypassing prompts should not also mean loading every server the user has configured.

### 4. The CLI answers setup calls without the app

`specular --help`, `specular --version` and `specular schema` answer without opening a session. `schema` prints the verb table as JSON: each verb, its arguments and flags, and whether it writes. The same table builds `HELP`, so the two cannot drift. An agent setting up, or an MCP client listing tools, never launches the app or fails because it is closed.

### 5. An MCP server sits beside the CLI, from the same table

The Rust app gets an MCP server alongside the CLI. It is a thin layer: each tool is a CLI verb from the same table as `schema`, and each call is the same HTTP request the CLI makes. The skill text is the server's `instructions`, so an MCP client gets the guidance a CLI agent gets from the skill. A stdio entry point (`specular mcp`) relays to the running app and answers `initialize` and `tools/list` itself while the app is closed.

The `/mcp/session/*` routes stop being dropped: `open` names the actor ADR 0054 needs, for MCP and CLI alike.

One-click connect for Claude Desktop, Codex and Claude Code follows once the server exists. It reaches people who will never install a skill. The CLI stays the default, for the reasons PR #430 gives: the skill installs itself, and any agent with a shell can use it.

### 6. A pre-commit hook guards what agents commit

`.githooks/pre-commit`, enabled with `git config core.hooksPath .githooks` by a setup script, refuses a commit that adds:

- `.mcp.json`, `.codex/`, `settings.local.json`, any `.claude/*.lock`, and `.env*` other than `.env.example`
- anything that looks like an API key or token (`sk-ant-`, `sk-`, `ghp_`, `github_pat_`, `xox[bp]-`, `AKIA`, private key headers)
- a literal path under a home folder (`/Users/<name>/`, `/home/<name>/`)

It does not block `.claude/` as a whole, as Editroom's does: this repo commits `.claude/skills/` on purpose. A hook is opt-in per clone, so CI runs the same script over the PR's diff and fails on the same matches.

## Alternatives

**A hidden draft for every run, as Editroom does.** One clean step and no half-finished state on the canvas. It hides the agent's work while it happens, which is what presence exists to show, and a long run would look like nothing happening and then everything at once.

**Cmd+Z reaches across into agent steps** (ADR 0054's second option). Keeps today's behaviour. With runs collapsed to one step it is less painful than it sounds, but it breaks the multiplayer rule 0054 chooses and makes undo depend on who is local.

**An empty working folder for the fallback.** What Editroom does. It would stop most in-app runs from doing their job, which is often a change to the user's repo.

**MCP instead of the CLI.** Inline images and a clean session close are real advantages, but the CLI already works for every agent with a shell and installs with the app. Both, from one table, costs a thin layer.

**A secret scanner such as gitleaks in place of a hook.** Stronger patterns, but another tool to install. The hook plus the same script in CI covers what agents actually leak here, and a scanner can replace the pattern list later without changing where it runs.

## What was read, and what is not known

- Editroom at `f6e947b`: `MCPServer.swift`, `MCPBridge.swift`, `MCPTools.swift`, `AgentConnect.swift`, `MagicAgent.swift:483-520`, `MCPDraft.swift`, `MagicShow.swift`, `.githooks/pre-commit`. The notes are in the project's `research/editroom-lessons.md`.
- Specular `rust-native` at `d375c2e`, the files under "Code today", and ADR 0054 as it stands on PR #447.
- Whether yrs's `UndoManager` can merge a whole run into one capture on demand, rather than by its time window, is unchecked. If it cannot, the run's actor keeps its own list of the run's captures and undoes them together.
- Whether `--setting-sources` and `--strict-mcp-config` behave the same when the run resumes a session with `--resume` is unchecked.
- The key patterns will have false positives in test fixtures. The hook needs an allowlist file from the start.

## Consequences

- An agent can fix its own mistake without touching the person's work.
- A finished run is one undo for the person, and ADR 0054's open question is answered in favour of its preferred option.
- A canvas run started by the app no longer inherits whatever MCP servers and hooks the user has configured for their own Claude Code.
- One verb table drives `HELP`, `schema`, the MCP tool list and the skill's verb reference.
- `undo`, `redo`, `schema` and `mcp` are new verbs. The skill in `resources/skills/specular/` and `.claude/skills/specular/` gains `undo` and `redo` in the same change that adds them.
- Scenario tests gain one check, alongside `assert_undo_returns_to_start`: a scripted agent run is undone by one step.

## Not taken from Editroom, for now

Agent notes stored in the document (Editroom's `note_media`), headless self-check modes in the shipping binary, and design mocks kept in the repo are good ideas with a different home: the first is a document-format change, the second is ADR 0052's territory, and the third is a repo convention, not an architectural decision. Editroom's lack of tests and CI is not something to copy.
