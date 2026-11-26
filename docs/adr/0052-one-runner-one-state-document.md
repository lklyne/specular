# ADR 0052: One runner and one state document for driving and checking the app

**Status:** Proposed. Not built. The parts that name Rune depend on [ADR 0051](./0051-rune-shell-around-the-compositor.md), which is deferred, so they are on hold; the rest does not depend on it and is to be revised for the Kit shell before it is built.
**Date:** 2026-11-25
**Related:** [ADR 0044](./0044-ui-as-pure-models-with-replaceable-renderers.md), whose "the Kit cannot run headless" consequence this removes. [ADR 0042](./0042-update-view-render-loop-effects-only-io.md), why a run needs no window. [ADR 0051](./0051-rune-shell-around-the-compositor.md).

## Context

There are about a dozen ways to drive the app, and they overlap:

- `TestApp` in the testkit, for pure rules.
- Scenario scripts headless (`script/`, 4,210 lines, and `headless/`, 830).
- The same scripts in the Kit window, through a second interpreter of the same `Step` enum (`kit/debug_input`, 1,910 lines) and a hand-kept copy of the Kit's layout (`script/chrome.rs`, `Event::ChromeLayout`) so positions agree.
- The CLI end to end, which needs the window app because a headless run starts no server.
- In-process API tests that cover the same verbs again.
- Env-var hooks: an inline script, a scripted agent, folder-dialog answers.

Checks are a hand-written vocabulary. About 2,700 lines of `expect` facts name things one at a time; 25 fact kinds are used five times or fewer. The panel topics (`controls`, `tabs`, `sidebar`, `menubar`, `palette`, `settings`, `onboarding`, `chat`) are text formatters over the models, about 630 lines.

Two faults follow from the arrangement. `tests/scenarios.rs` returns early when there is no wgpu adapter, so in a container all 59 scenarios report as passing; the GPU is needed only because text measure lives in the compositor. And a control named in a script is pressed from the models, so a script cannot tell whether the control is drawn, reachable or covered.

## Decision

One script language, one runner, one state document, reachable the same way in every mode.

- **The step verbs stay.** The existing input steps are the language (about 50 verbs; 59 scenarios, 5,696 steps).
- **One runner.** The same interpreter runs in `cargo test` with no GPU, headless with a GPU for PNGs, and in a window behind a flag. `control NAME` is input to the element whose key is the model's `ControlId`. There is no press-from-models fallback and no layout emulation.
- **One state document.** `state` returns JSON with four parts:
  - `app`: the session and a summary of the document, by `serde`;
  - `saved`: the canvas as a save would write it;
  - `effects`: what the last step asked the shell to do;
  - `ui`: the UI framework's element and accessibility tree.
- **One check.** `expect <path> <op> <value>` over that document, with `undo-to-start`, `mark` and `same-as` kept. About 15 derived canvas facts that are not state (what is under a point, which guides show, a handle's rect) stay as named probes.
- **The same two operations over HTTP, in every mode.** `POST /debug/step` takes a script line and `GET /debug/state` returns the document; the CLI gains `specular step` and `specular state`. A headless run can serve the API (`--serve`), so `cases.py` runs on Linux and an agent drives a live window, a headless run and CI the same way.
- **Text measure works with no GPU.** It moves out from behind the compositor, or the testkit's fixed-advance measure is used where pixels are not asked for. A run that cannot do what a script asks fails; it does not skip.

Removed once this lands: `kit/debug_input`, `script/window.rs`, `script/chrome.rs`, `Event::ChromeLayout`, both `SPECULAR_SHELL_SCRIPT*` variables, the window-only step class, `headless/steps.rs` as a separate interpreter, the `facts_canvas` and `facts_panels` formatters, `SPECULAR_AGENT_SCRIPT` (the `agent-says` steps cover it), `tests/api/verbs.rs` (the CLI cases cover it), and `panel/named.rs` with `Event::Control` in `specular-interact`.

## Order

1. No dependency on the shell: `--serve` for headless runs, the JSON state document with `app`, `saved` and `effects`, the path matcher, text measure with no GPU, and the vacuous pass turned into a failure.
2. Migrate `expect` lines by dumping `state` at each one and rewriting the line as a path. Mechanical; 2,220 lines.
3. With the Rune shell: `ui` in the state document, `control` as element input, and the removals that depend on a renderer that runs headless.

## Alternatives

**Use Rune's `--script` directly.** Its scripts have about 35 input verbs and no assertions, waits or queries by role or text. Ours check state and the saved file and end with `undo-to-start`. Rune's runtime is the engine under our runner, not the language.

**Keep the fact vocabulary.** It reads well in a script. It costs a formatter for every new thing worth checking, and it cannot see what is drawn.

**Snapshot tests of the UI tree.** Useful for a new draw rule (the scene snapshots already do this). As the main check they fail on every cosmetic change.

## Consequences

- An agent has one loop: send a step, read the state, in any mode.
- A new model field is checkable the moment it is serialised, with no fact to write.
- `specular-interact`'s session and model types must derive `Serialize`. Two types do today.
- The state document's shape becomes something scripts depend on. It is internal, and a rename breaks scenarios; `state out.json` at a failing line shows the new path.
- The by-hand pass for real devices (trackpad momentum, an input method, VoiceOver) is still by hand.
