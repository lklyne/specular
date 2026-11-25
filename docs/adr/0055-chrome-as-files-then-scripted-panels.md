# ADR 0055: Chrome arrangement as files first, scripted panels over the models later

**Status:** Proposed, and deferred until the shell question ([ADR 0051](./0051-rune-shell-around-the-compositor.md)) is settled. Not built.
**Date:** 2026-11-25
**Related:** [ADR 0044](./0044-ui-as-pure-models-with-replaceable-renderers.md), the panel models. [ADR 0016](./0016-tools-as-capability-registry.md), tools as a registry. [ADR 0052](./0052-one-runner-one-state-document.md), the state document and action table this reuses.

## Context

Herdr's desktop app (`herdrdev/herdr-app`) lets an agent or a person rebuild its window. Read at its 2026-11-24 head:

- A **layout** is one JSON file: a tree of blocks, rows, columns and stacks that says which panel goes where.
- A **mod** is a folder with a JavaScript class whose `render()` returns a declarative element tree. Rust keeps the tree and replays it each frame; script runs on a state change or an event, never per frame.
- Mods read state through published JSON snapshots with read tracking, so only a mod that read a changed key re-renders. They change things only through one table of 78 named commands, the same one keys, menus and the CLI use.
- Nearly all of Herdr's own chrome is mods: about 9,100 lines of JavaScript on the API third parties get.
- The runtime is not Herdr's. It is `gpui-shell`, a crate of GPUI Kit, on QuickJS. Herdr carries 130 patch sites in it.
- Mods are trusted code with unrestricted exec and network. Herdr's docs say this is not a security boundary.
- Its changelog records what scripted UI cost: window freezes while mods ran, a VM memory leak that broke every mod, idle mods waking 20 times a second.

Specular already has the three parts that are hard to add late: one `Action` table every input goes through, panels as pure models of `&App`, and a renderer that is replaceable.

## Decision

Two steps, the second only after the first has been used.

**Step 0: arrangement, keys and theme as files. No scripting.**

- The chrome's arrangement is a JSON layout over the existing native panels: which panel sits in which region, sizes, what is hidden.
- Key bindings and the theme are files beside it.
- Shipped files are replaced on start. A person's copy is theirs and is never updated. A save reloads the window; a bad save keeps the last good version and writes the error beside the file.
- The `Action` table describes itself (names, arguments, help) over the API and CLI, and the state document of ADR 0052 is the read side.

**Step 1: scripted panels that return the semantic model.**

- A script is `render(snapshot) -> controls` plus handlers. It returns the controls the models already use (`Button`, `Toggle`, `Dropdown`, `Field` and the rest, each with an id and an `Action`), not layout primitives.
- A handler may call `run(action, args)` and set its panel's own state. Nothing else: no file, process or network access.
- Script runs on a snapshot change or an event. The camera is never a key a panel can read, so a pan or zoom cannot enter a script.
- A `.canvas` file or a page can never name or load a script. A layout can, so a shared layout is code and is treated as such.
- One real panel is rewritten as a script before any API is published.

Canvas item renderers, tools and gestures are not opened to scripts. For those the lanes are the native file-renderer registry and the fact that a page is already a sandboxed, scriptable canvas item.

## Why the semantic level

Herdr's mods emit layout primitives, which is what makes a full reskin possible. It also exposes the UI library's whole style and component surface as API, tied to that library. Returning our own controls keeps the API small enough to version, keeps it valid across a change of shell, and makes a panel checkable as data: script plus snapshot in, controls and actions out, no window. The cost is that a script cannot restyle the app.

## Alternatives

**Herdr's design on `gpui-shell`.** One dependency away while the shell is GPUI Kit. The crate calls itself a feasibility baseline, and it ends with a move off GPUI.

**Rune's Luau lane.** The equivalent if the shell moves. Either engine sits behind the same small host module, so the choice is reversible.

**Process plugins**, as Herdr's older terminal app had. Any language, no native UI.

## Consequences

- Step 0 gets "an agent can rearrange the app" with files and no new runtime.
- Step 1 adds a second language, a script watchdog, and an API that has to be kept stable.
- Script budgets must sit inside a frame. Herdr's are 50 ms a render and 500 ms a handler, which a 120 Hz canvas cannot share a thread with unless script is kept off the pan and zoom path entirely.
