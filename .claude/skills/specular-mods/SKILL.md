---
name: specular-mods
description: "Use Specular mods whenever you want to change the Specular app itself: redraw a panel (toolbar, tab row, sidebar, dock, palette, chat, menu), show agent chat threads in blocks, side-by-side panes or canvas items, make or switch a layout, theme, colours or fonts, rebind keys, open a dialog or popup, add a shape, a kind of canvas item, a tool or brush, or a command in the palette, keys or dock. To put things on a canvas, arrange them or drive pages, use the specular skill instead."
allowed-tools: Bash(specular:*)
---

This skill is for building Specular mods with the `specular` CLI. It is not
for driving the canvas: that is the specular skill.

If `specular state theme` answers, the app is running: build the mod there.
If not, say the Specular app isn't running; `specular mod check` still works.

Before writing a mod, run `specular --skill` and follow it.
It prints the manual for this version of the app.
