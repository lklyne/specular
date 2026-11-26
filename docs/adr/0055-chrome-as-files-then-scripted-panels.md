# ADR 0055: The interface as files: scripted chrome, pages as canvas extensions

**Status:** On hold with [ADR 0051](./0051-rune-shell-around-the-compositor.md), deferred 2026-11-25. Not built. Depends on [ADR 0051](./0051-rune-shell-around-the-compositor.md): the scripting lane named here is Rune's. Step S7 of [the spike](../plans/rune-spike.md) tested it on Linux and it passed; the results and the three corrections they forced are below. Revised 2026-11-25, the day it was written: the first draft had scripts return the semantic controls only, which ruled out restyling.
**Date:** 2026-11-25
**Related:** [ADR 0044](./0044-ui-as-pure-models-with-replaceable-renderers.md), the panel models. [ADR 0016](./0016-tools-as-capability-registry.md), tools as a registry. [ADR 0052](./0052-one-runner-one-state-document.md), the state document and action table this reuses. [ADR 0054](./0054-every-editor-is-an-actor.md), actors.

## Context

Herdr's desktop app (`herdrdev/herdr-app`) is built so that a person, or their agent, can rebuild its window. Read at its 2026-11-24 head, what makes that work is less the script engine than the habits around it:

- **The interface is a folder of files.** Layouts, keys, themes and panels. The window redraws from them within a second of a save. The folder holds a guide the app rewrites on every start, with a test that it names every command.
- **Nothing shipped is special.** The sidebar, tab bar, menus and dialogs are mods on the API a user gets: 21 mods, 3,921 lines of JavaScript. A confirm dialog is 53 lines; the spaces sidebar, the largest, is 573.
- **Clone to own.** Shipped files are replaced on every start. A person's clone is theirs, is never updated, and records what it was cloned from.
- **It cannot be broken.** A bad save keeps the last good version and writes the error beside the file. One key combination nothing can rebind returns to the default layout.
- **A rule keeps the native core small.** Three questions are answered before anything is added in Rust, and a request a mod could meet is closed.
- **One way to read, one way to write.** Mods read published JSON snapshots, with read tracking so only a mod that read a changed key re-renders. They change things only through one table of 78 named commands, the one keys, menus and the CLI use.
- **Script never runs per frame.** `render()` returns a description that Rust keeps and replays.

Its headless core adds two things for extensions: an API that only grows, with a test that fails on a breaking change, and small shared stores (labels, and versioned JSON documents with compare-and-set).

What it cost them is in the changelog: window freezes while mods ran, a VM leak that broke every mod, idle mods waking 20 times a second. Mods are trusted code with unrestricted exec and network, which their docs say is not a security boundary. The runtime is `gpui-shell` on QuickJS, which calls itself a feasibility baseline and which Herdr carries 130 patch sites in.

Specular has the parts that are hard to add late: one `Action` table every input goes through, panels as pure functions of `&App`, and a replaceable renderer. Rune has the rest in `rune-luau` (read, not run): a sandboxed Luau VM whose `render` builds Rune elements, reused without entering the VM until the script's version moves; hot reload that carries state across; an error card in place of a crash; a 250 ms CPU watchdog and a 64 MB heap per script; host signals and async functions.

The Rune port rewrites about 5,700 lines of view adapters whatever is decided here. That is the cheapest moment to write them as scripts.

## Decision

Three lanes, and two rules.

**1. The chrome is scripts.**

- A panel is a Luau script: `render` over a read-only snapshot of the app, building Rune elements.
- A script changes the app only through `run(action, args)`, the `Action` table. It keeps its own panel state.
- The shipped toolbar, dock, sidebar, tab row, menus, palette, settings and chat panel are written this way, on the API a person's own panel gets.
- The models stay. `toolbar(&App)`, `dock(&App)` and the rest are what the snapshot is made of, so a script lays out and styles what the model says is there. Which controls exist for a selection, and what each does, stays in Rust and stays tested there.
- Script runs on a snapshot change or an event, never per frame. The camera is not in the snapshot a panel can read, so a pan or zoom cannot enter a script. Anything that shows a camera value, the zoom readout for one, is a node the host draws inside the panel.
- The script vocabulary is Rune's `@rune/ui` plus host nodes of ours for what it lacks. The first one is a control node that carries a checked state, a popup and the toolbar and menu roles.
- No file, process or network access to begin with. A capability is added when a shipped panel needs it, and granted per script.

**2. Canvas extensions are pages.**

- An HTML file in the space folder is already a live page on the canvas. A page from the space gets a bridge: the snapshot to read, and `run(action, args)` to write.
- A custom canvas widget is then one HTML file, which an agent can write, on an item that already moves, resizes, syncs and undoes like any other.
- The bridge is for pages loaded from the space folder. A page from the web never gets it.
- Tools, gestures and how the canvas draws its own kinds stay native. The file-renderer registry stays the lane for a new file kind.

**3. Arrangement, keys and theme are files.**

- A layout is JSON: which panel sits in which region, sizes, what is hidden. Keys and the theme are files beside it.
- Shipped files are replaced on start. A clone is the person's and is never updated.
- A save reloads the window. A bad save keeps the last good version and writes the error beside the file. For a script that holds for an error at compile or start; one that starts and then fails in `render` shows an error card in its panel until the next save. One fixed key returns to the shipped layout.
- The folder carries a generated guide for agents, checked against the action table by a test.

**Rule: nothing shipped is special.** A capability the shipped chrome uses is one a person's panel can use. If the chrome needs something scripts cannot do, the API grows; the chrome does not get a private door.

**Rule: the native core answers for itself.** Before a feature goes into Rust, say why a script or a page could not do it.

**Where extension state lives.** In the canvas's own `specular` fields. With ADR 0054 the mirror then syncs it and each actor can undo it, and it is in the `.canvas` for other tools to read.

## Trust

Specular loads live web pages and `.canvas` files from disk, so Herdr's "read the mod before you install it" is not enough.

- A `.canvas` file or a web page can never name, load or select a script.
- A layout names scripts, so a shared layout is code and is installed as code.
- Scripts start with no capabilities. The bridge in lane 2 is limited to the space folder's own files.

## Alternatives

**Scripts return the semantic controls only**, this ADR's first draft. A small API that survives a change of shell and is checkable as plain data. It cannot restyle anything, so the shipped chrome could not be written in it and nothing would prove the API by use.

**Keep the chrome in Rust; add scripting later.** No second language now. The adapters are then written twice, and the API is designed without a real client.

**`gpui-shell`**, Herdr's runtime. One dependency away on the GPUI Kit shell, and it ends with that shell.

**Process plugins**, as Herdr's older terminal app had. Any language, no native UI.

## What the spike showed

Step S7, run on Linux at Rune `a1fbe7ff`, debug build, no window. The toolbar was written as a Luau script (181 lines; the Rust one is 273) beside the Rust toolbar. [The scripted toolbar](./assets/0055/luau-toolbar-shape-armed.png).

- Its accessibility subtree is equal to the Rust toolbar's as JSON, and its pixels are identical at rest, with the zoom list open and after a pick.
- It reads `toolbar(&App)` as plain data sent on a host signal when the model changes, and writes through `app.run(id)`, which the host looks up in the current model and runs as the control's `Action`. The host, not the script, refuses a disabled control.
- A saved edit reloaded in about 20 ms with no rebuild. The app's tool and the script's open list survived it.
- The VM was not entered on 100 idle frames, 100 pan frames, or a selection change. A tool change cost one render, about 0.6 ms.
- The sandbox held: no `io`, `dofile` or `package`; a `require` outside the folder is refused; an endless loop is stopped at 250 ms and a memory hog at 64 MB, and the script answers its next call.
- `rune-luau` adds 15 packages and a 39 s C++ build.

Three things it corrected, now in the decision above:

- **Zoom entered the script.** The zoom readout is in the toolbar model, so 100 frames of pinch rendered the panel 68 times.
- **`@rune/ui` could not say the toolbar.** It has no `checked`, no popup flag and no toolbar or menu roles, so one host node was added. The API is Rune's vocabulary plus ours from the first panel.
- **Last-good covers compile and start errors only.**

Not tried: icons, tooltips, swatches, steppers, fields, dismissing a list on an outside press. Read, not run: a sidebar of hundreds of rows wants rows pulled by index with a version each (`ui.virtualList`), not one whole-model snapshot.

## What is not known

- `rune-luau` is as young as Rune and is not in Rune's default build; it needs a host C++ compiler. Only one small panel has been written on it.
- What a panel costs: a render of the sidebar on a canvas with hundreds of items, and how coarse the snapshot's keys must be to keep renders rare.
- How pages in lane 2 get the bridge: CEF's message router, or the devtools channel `dom_query` already uses.

## Consequences

- A person or an agent can rearrange, restyle and add to the window by editing files, and add to the canvas by writing a page.
- The script API is Rune's element vocabulary. It is tied to Rune and moves when Rune's does.
- A second language for the chrome, with its own tests: a script plus a snapshot gives a tree and the actions it ran, checked with no window.
- A watchdog, counters for script renders a second, and a rule that script stays off the pan and zoom path are needed from the first panel.
- The API has to be kept. Herdr's grows every release by request; the small-core rule is what pays for that.
