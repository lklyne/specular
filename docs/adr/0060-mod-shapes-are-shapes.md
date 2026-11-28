# ADR 0060: Mod shapes are shapes

**Status:** Accepted, built on 2026-11-28.
**Date:** 2026-11-28
**Related:** [ADR 0057](./0057-mod-items-and-tools.md), mod items and tools. [ADR 0059](./0059-the-whole-window-from-mods.md), mod shape tools (`"group": "shape"`). The shape catalog in [CONTEXT.md](../CONTEXT.md).

## Context

API 1.2 let a mod sit among the shape tool's shapes with a tool of `"group": "shape"`, and the shipped `star` mod used it. The star was a mod item: a display list of a path with literal hex colours, drawn by the mod on every change. So it took none of what makes a shape a shape:

- The palette, fill and border styles, border width and label did nothing to it. The dock still showed the shape tool's colour dots while it was in hand, and they changed the shape tool's defaults, which the star never read.
- It did not follow the theme: its gold was the same in light and dark.
- A reader without the mod saw a snapshot, not a shape.

That was the API, not the mod: a display list was the only way a mod could draw a silhouette, and the guide said so ("a mod shape is a mod item, not a built-in shape").

## Decision

**A mod's shape is a `Shape` with an outline of its own.**

- `ShapeKind` gets one variant, `Custom`, and `Shape` gets `custom: Option<CustomShape>`: the shape's `<mod>/<shape>` name, the values (`params`) the mod made it from, and its outline as SVG path data in the same 0 to 100 box the app's own silhouettes are drawn in.
- Everything else about it is the app's: the shape tool draws it, selection, hit-testing and resizing treat it as a shape, the dock styles it, and `view` draws it with the shape's colours and the theme.
- In the `.canvas` file it is a `shape` node with `shapeKind: "custom"` and `specular.customShape: {shape, params, path}`. The outline is in the file, so the web viewer, a sync peer and a person without the mod draw it as it is, styled. An outline that does not read is dropped and the shape draws as a rectangle.
- The path takes `M`, `L`, `H`, `V`, `Q`, `C` and `Z`, absolute or relative. Arcs and the smooth forms are refused: a curve is written out with its control points. A path carries no style.

**A mod declares shapes and changes them; it does not draw them.**

- `mod.json` gets `shapes: [{name, title, icon, path, params}]` (API 1.4). A path that does not read refuses the manifest.
- The shape tool's list and a selected shape's shape list show every running mod's shapes after the app's. Picking one is a tool default (`ToolDefaultPatch::CustomShape`) or a property (`Property::CustomShape`), as a hexagon is.
- `shape.set {ids, params, path}` changes the values and outline of the named mod shapes, one undo step. `item.add {kind: "shape", shape: "<mod>/<shape>"}` places one by name. `canvas/items` gives each shape's `custom: {shape, params}`.
- A mod's dock commands show beside the style of its selected shapes, so "more points" sits next to the colours.

**Mod items and shape tools stay.** A mod item is for what is not a silhouette (a feed, a widget, a thread). A tool of `"group": "shape"` is for a tool that needs its own drag.

## Consequences

- The `star` mod is about thirty lines: an outline function and a dock command. It needs no tool, preview or item.
- A shape cannot have parts drawn apart from its silhouette (the cylinder's rim is the app's alone), inner lines or a style of its own. A mod that needs those is a mod item.
- The shape tool's list names a mod's shape by its mod's glyph, one of the app's icons.
- An older app reads a `custom` shape node as an item it cannot type and keeps it raw.
