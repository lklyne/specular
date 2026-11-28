# 0061: Materials are system views under the window

**Status:** Accepted, built.

## Context

People asked for Liquid Glass and blur in the chrome, and for mods to have
it too. GPUI paints with Metal and has no blur or glass of its own; a
shader blur would not match the system's look, and macOS 26's glass
refracts and lights what is behind it in ways only AppKit draws.

GPUI 0.3.14 can composite an `NSView` beneath its content (`NativeView`),
and Specular's canvas is already a sibling view under GPUI's.

## Decision

A layout node's `"material": "glass" | "blur"` and the mods' `Material`
host component put a system view under the element: `NSGlassEffectView`
for glass on macOS 26, an `NSVisualEffectView` (sidebar material, blending
within the window, always active) for blur and for glass on an earlier
macOS. The view sits above the canvas and below GPUI, so the canvas shows
through it. The element keeps its view in its element state; a material no
longer drawn leaves the window with the state.

The material shows only where GPUI leaves the window clear. The app's own
panels leave their fill off while they are built on a material
(`panel_fill`); a node's translucent `background` tints it.

Native `NSMenu` context menus were weighed with this and not taken: a mod
can restyle the menu mod's every row, colour, font and corner, and an
`NSMenu` takes none of that.

## Consequences

- The look depends on the system: glass needs macOS 26, and headless runs
  and scenarios draw no material (a placeholder in mods).
- Mouse input over a bare stretch of material goes to the system view,
  which does nothing with it; the block occludes the canvas either way.
- An opaque paint over a material hides it. The layout root's `background`
  paints round the canvas under the blocks, so a layout leaves it off
  where a material should show the canvas.
