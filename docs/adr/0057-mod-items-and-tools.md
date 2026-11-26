# ADR 0057: Canvas items and tools as mods

**Status:** Proposed, split out of [ADR 0055](./0055-chrome-as-files-then-scripted-panels.md) on 2026-11-26. Not built, and not started before ADR 0055's phase 3 has shipped a mod.
**Date:** 2026-11-26
**Related:** [ADR 0055](./0055-chrome-as-files-then-scripted-panels.md), mods, the command registry and the runtime. [ADR 0042](./0042-update-view-render-loop-effects-only-io.md), the pure `update` loop. [ADR 0054](./0054-every-editor-is-an-actor.md), actors and sync. [ADR 0058](./0058-gpu-host-module-for-mods.md), a mod's own GPU shaders.

## Context

ADR 0055 makes the chrome mods. Its second version also let a mod add a new kind of thing to the canvas (an image feed, a recorder, a phone mirror) and a new tool (a brush). A review found four gaps in that part, which is why it is its own ADR:

- It listed four item surfaces. Two of them already exist in the code and the other two are the same thing as one of those.
- It did not say how a scripted tool fits a pure `update` that does no I/O.
- `Tool` is `Copy` and sits in const tables, and `ToolDefaults` has one typed field per tool. Neither takes a new tool at run time as written.
- The web viewer (`specular-web`) and sync peers have no script runtime, so they cannot draw what only a script knows.

Mods are trusted (ADR 0055, "Trust"), and a `.canvas` file never loads one.

## Decision

**A mod item is one new kind.**

- `Kind` gets one variant, `Kind::Mod(ModItem)`: the mod's name, its props as JSON, and the frame every item has. It moves, resizes, groups, syncs, undoes and saves like any other item, because the canvas does those on the frame.
- With one variant the no-wildcard rule holds, and the compiler lists every place that must handle it once.
- In the `.canvas` file it is a JSON Canvas node whose `specular` fields name the mod and hold its props. Its base `type` is a fallback other tools can show: a `link` or `file` node when the item wraps a URL or a file, otherwise a `text` node with the mod's name.
- A reader without the mod shows a placeholder and keeps the props untouched. That includes the web viewer and a peer, to begin with.

**An item shows one of two surfaces.** Both are composited by the canvas pass, so a pan or zoom never enters the script.

- **A display list.** A subset of `specular_scene::Draw` (rects, ellipses, paths, text, images) in item space, sent as JSON. The canvas keeps it and redraws from it until the script sends another. It is the vocabulary `view` already produces. An image viewer or a chart is this.
- **A texture from a source.** What `Draw::Page` is today: frames arriving as `FrameEvent`s into `Compositor::ingest_frame`, on the CPU or as a shared surface. `FrameEvent`'s key is widened from `PageId` to a source id, and that is the only change to the import path. Web content from the mod's folder, a camera, a screen, a video file or a helper process is each a source. `PageSource`, the browser contract, is left alone.

**A scripted tool runs outside `update`, through an effect and an event.**

- While a mod tool is armed, `update` turns each pointer event into `Effect::ModTool { tool, input }` and changes nothing else. The shell runs the mod's handler.
- The handler's preview comes back as `Event::ModPreview { tool, draws }`, a display list in canvas space that `view` adds to the scene. On release the mod commits through `run`, as one undo step, usually by placing one of its own items.
- Pointer moves are coalesced to the display rate before they reach script. The preview is one frame behind the pointer. Whether that is good enough for a brush is the first thing to measure.
- `Tool` gets one variant, `Tool::Mod(ModToolId)`. `ModToolId` is a small `Copy` index into the list of loaded mod tools, so `Tool` stays `Copy`. The const tables (`Tool::ALL`, the key bindings) keep the native tools, and the toolbar model appends the mod tools.
- A mod tool's defaults live in the mod's own storage. `ToolDefaults` does not change.

**An entered mod item gets input the same way.** Pointer, wheel and key events in item space reach its mod once it is entered, the way a page is entered. The canvas keeps select, move and resize when it is not.

**State.** Durable state is the item's props, changed through `run("item.set", {id, props})`, so it is one undo step on the actor that made it and syncs through ADR 0054's mirror. Transient state (a playing stream, a scroll offset) stays in the mod.

**One host module, `media`.** Decoding images and video, playing audio, and turning a stream into a texture source: a URL, a file, a camera, a microphone, a screen, or a local process that writes frames. Capture is part of it, not a module of its own. Network, files and processes are `gpui-shell`'s built-ins.

**The shipped kinds stay native.** Page, file, text, drawing and shape are not rewritten. The test of this ADR is that a mod can build a page-like item from the same two surfaces.

## Phases

1. **The image feed.** `Kind::Mod` with its `.canvas` round trip and undo round trip, the display list surface, and one mod: an image feed that fetches and shows pictures. Nothing else.
2. **A tool.** `Tool::Mod`, the effect and event, and the preview. The watercolor brush is the test: a tool whose items show web content from the mod's folder running [`webgpu-brush`](https://github.com/lklyne/webgpu-brush) in CEF, with the stroke's points as the item's props. This needs web content as a texture source and ADR 0055's page bridge.
3. **`media`.** A video recorder (a camera or screen source, saving to the space folder), then a phone mirror (a source from a helper process).

## What is not known

- Whether a preview one frame behind is acceptable for drawing, or the preview has to be kept host-side between script calls.
- Whether the web viewer and peers should draw a mod item's last display list. It would have to be saved in the item's fields, which makes the file larger and syncs every redraw. The placeholder comes first.
- What a canvas of many mod items costs: how many display lists the canvas pass keeps, and how many sources it imports at once.
- Whether web content in a mod item can share the page pipeline without a page's own state, or needs its own host.

## Consequences

- A new kind of canvas item or a new tool needs no Rust change unless it needs a native capability `media` does not have.
- Pointer events enter script while a scripted tool is armed or a mod item is entered. Pan, zoom and the built-in tools never do.
- `update` stays pure. The cost is a frame of latency on a scripted tool's preview.
- A `.canvas` file can hold items that need a mod the reader does not have. They show as placeholders and keep their data, and other JSON Canvas tools see the fallback node.
