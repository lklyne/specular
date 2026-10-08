# ADR 0042: One update, view, render loop, with effects as the only I/O

**Status:** Proposed. Built this way across `native/crates` during the Rust rebuild. The user has not reviewed this ADR.
**Date:** 2026-10-08
**Related:** [Rust native rebuild plan](../plans/rust-native-rebuild.md), "One loop, three pure functions". [ADR 0041](./0041-typed-document-inverse-command-undo.md). [ADR 0036](./0036-diffed-runtime-store.md) and [ADR 0025](./0025-single-workspace-mutation-seam.md), the Electron machinery this makes unnecessary.
**Code:** `specular-interact/src/{update,event,effect}.rs`, `specular-scene/src/view.rs`, `specular-compositor`, and the effect runners in `specular-app/src/app/`.

## Context

The Electron app spreads one feature over a main-process mutator, an IPC contract, a preload bridge, a broadcast and a renderer. Testing a gesture end to end needs a booted runtime. An agent changing a feature has to trace it through four files before touching it.

The rebuild runs in one process, so that split has no reason to exist. The open question was where I/O is allowed to happen.

## Decision

```
window, CEF, HTTP, files ──> Event
        update(&mut App, Event) -> Vec<Effect>     no I/O
        view(&App, viewport, &ViewCache) -> Scene  no GPU
        render(&Scene)                             wgpu
```

- `update` is the only thing that changes an `App`. It reads no file, socket, process, environment variable or clock.
- Anything that must happen outside comes back as an `Effect`: host a page, write a canvas, read the clipboard, run `claude`, show a folder dialog. The shell runs it. If there is an answer, it comes back as a later `Event`.
- Time is an input. The shell sends `Event::Tick { unix_ms }` once a loop turn, and debounces, blinks and quiet windows count against it. There is no timer effect.
- New ids come from a seeded sequence in the session, not from a random source.
- A question to a page is an effect answered by an event that repeats the question, so nothing waits inside the session.
- An agent's HTTP write is `Event::Api` and its answer is `Effect::ApiReply`. It goes through the same door as a key press and makes the same undo step.
- `view` builds a flat display list from the app. It names no renderer type. The compositor draws the list and knows nothing about entities or tools.
- Both shells share one `specular_app::Runtime`, which holds every effect runner. A new effect gets its runner once.

Today `Event` has 31 variants and `Effect` has 46.

## Alternatives

**Let `update` do small I/O directly**, such as reading the clipboard or checking which file names are taken. It would remove the two-step dances below. Turned down because one exception ends the guarantee that a test needs no window, and the headless runner depends on that guarantee.

**Async tasks or a command type that carries a future**, as Elm-style frameworks in Rust tend to do. Not tried. Plain effect values can be cloned, compared and asserted on in a test, and the shell's loop was already polling.

**Keep the Electron layering in one process**, with a store and subscriptions. This was the thing being left behind.

## What was measured

- The audit scanned `specular-doc`, `specular-interact`, `specular-scene` and `specular-api` for `std::fs`, `net`, `process`, `env`, `thread` and clock reads and found none. `update` itself is 109 lines.
- The suite is about 1,300 tests and the gate needs no window or CEF. GPU readback tests skip when there is no adapter.
- `specular-app --snapshot` and `--script` run whole sessions with no window. Eighteen scenario scripts under `native/fixtures/scenarios` replay sessions and check the canvases they save. A headless run's clock moves only on `wait`, so one script draws the same frames every time.
- Purity did not cost frame time. After the performance pass a pan frame costs 0.4 to 1.6 ms of CPU on the bench canvases, and an idle window draws 0 to 16 frames in 5 seconds where it drew 601. Those caches sit in the compositor and the shell. One later change moved the last cache out of `App` into a `ViewCache` that `view`'s caller owns.

## Consequences

- Operations that need a fact only the shell has take two steps. Creating a Document is `Effect::CreateNote`, then `Event::NoteCreated`, because only the shell knows which file names are taken. Choosing a space folder and picking a repo folder work the same way.
- `update` keeps nothing between calls, so it lays a Document out two or three times for each key. The memo that makes this cheap lives in the shell's text measure.
- State that looks like I/O state lives in the session as data: what each page last reported, image load state, the text of each Document.
- A shell is thin and replaceable. The GPUI Kit shell was added beside the winit one without touching `update` or `view`.
- A read made in the middle of one event can see a cached panel layout from before that event. It is rebuilt when the event ends. This is the one known place where "pure" has a visible seam.
- Reversing this is not one change. Every feature's tests assert on effects, and the headless runner, the scenario scripts and the API tests all assume `update` can run alone.
