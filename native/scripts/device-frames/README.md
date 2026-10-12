# Device frame converter

`convert.py` turns the painters of the Flutter package
[`device_frame`](https://github.com/aloisdeniel/flutter_device_preview/tree/master/device_frame)
(MIT) into the SVGs in `crates/specular-app/assets/device-frames/`. The
package has no SVG files. Each device is a Dart `CustomPainter`, so the
script runs every `paint()` body through a small interpreter for the subset
of Dart those files use and writes the draw calls out as SVG.

Upstream commit used: `ee630a9de852f2520d395f61abb4b9d0a902d807`.

## Re-run

```sh
git clone https://github.com/aloisdeniel/flutter_device_preview /tmp/fdp
git -C /tmp/fdp checkout ee630a9de852f2520d395f61abb4b9d0a902d807
python3 native/scripts/device-frames/convert.py /tmp/fdp
```

Python 3 standard library only. The output is deterministic, so a re-run on
the same commit leaves no diff. `--out DIR` writes somewhere else, and
`--commit SHA` names the commit when the checkout is not a git repository.
A device whose Dart uses something the interpreter does not know is skipped
with the reason on stderr, and the exit code is 1.

## Whose drawings these are

Every drawing but two is upstream's, converted as it is, and upstream's MIT
licence (`LICENSE-device_frame`) covers them. `ipad-mini` and `iphone-air`
are ours: the package has neither device, so the script builds them from
upstream drawings. Their manifest rows carry `"source": "specular"`. They
reuse upstream's outlines, so the same licence notice stays with them. See
"Our own drawings" below.

## What it writes

- `<name>.svg` for every `*/*/frame.g.dart`, where `<name>` is the folder
  name in lower case with dashes. The hand-written `generic/*/frame.dart`
  are not converted.
- `macbook-pro.svg`, from the hand-written painter in `HAND_WRITTEN`. It
  has no `screen.g.dart`: upstream lays a wallpaper and a window over the
  panel and calls the window the screen. Here the screen is the whole
  panel, and the wallpaper and the window bar are left out. Its
  `logicalScreen` is the panel over the pixel ratio, not upstream's window.
- `ipad-mini.svg` and `iphone-air.svg`, from `OWN_DEVICES`.
- `<name>-light.svg` for the devices in `LIGHT_DEVICES`: the same draws in a
  light colourway. See "The light colourway" below.
- `manifest.json`, one row per device, sorted by name. `lightFile` names the
  light drawing where there is one. `body` and
  `bodyCornerRadius` are the device's main outline without the buttons that
  stick out: the largest filled outline that does not cover the whole
  frame, cut to the view box. A device with no fill over three quarters of
  its frame gets neither field. A laptop's body is its lid, and its base
  sticks out of it as a phone's buttons do.
- `LICENSE-device_frame`, the upstream licence with the repo and commit.

## How the SVG maps to the Dart

- The view box is the Dart `frameSize`, and `size` in `paint()` is that size.
- Draws keep their order. `drawPath`, `drawRect`, `drawRRect` and
  `drawCircle` become `<path>`, `<rect>` and `<circle>`.
- `arcToPoint` becomes an `A` segment. Flutter's `clockwise` defaults to
  true, which is sweep flag 1.
- `ui.Gradient.linear` becomes a `<linearGradient>` in user space.
- `MaskFilter.blur(BlurStyle.normal, sigma)` becomes `feGaussianBlur` with
  `stdDeviation` set to sigma. `BlurStyle.inner` adds an `feComposite in`
  against `SourceAlpha`, which keeps the blur inside the shape. Filter
  regions are explicit and in user space, because a stroked straight line
  has an empty bounding box and that would switch the filter off.
- Only SVG 1.1 elements are used, with no CSS. Checked with `resvg` 0.46.

## The screen is transparent

Flutter paints the frame and then lays the app over it, clipped to
`_screenPath`. Several painters fill the whole front of the device, screen
included. The script cuts the screen path out of everything: all draws sit
in one group whose clip path is the frame rectangle plus the screen path
under the even-odd rule. No draw is left out. Where the screen path has a
hole for a dynamic island or a punch-hole camera, the art in that hole is
kept. A notch is part of the screen outline, so it is kept too.

## Manifest fields

- `screen` is the exact bounding box of the largest part of `_screenPath`,
  in frame units.
- `screenCornerRadius` is an estimate: the radius of the circle that meets
  the screen outline where each corner's 45° diagonal crosses it, as the
  median of the four corners. Apple's corners are not circular arcs, so a
  rounded rectangle with this radius matches at the corner's apex and is off
  by a little on either side of it.
- `logicalScreen` and `pixelRatio` are upstream's values as written.
  Upstream's `iphone_12` carries an iPad's numbers (1024 x 1366 at 2), which
  is wrong; the row is passed through unchanged.

## The light colourway

The package draws each device in one dark colourway. For the devices in
`LIGHT_DEVICES`, which are the ones the size menu uses, the script writes a second file with some fills changed and
nothing else: the geometry, the draw order and every other colour are the
dark file's. A fill is changed by what it is and its dark colour, from
`LIGHT_FILLS`:

| what the fill is | dark | light |
|---|---|---|
| body: the device's main outline, shown as a wide band | `#272727` `#434343` `#454C60` `#4E4E4E` `#4F5359` | `#E3E4E5` |
| body, shown as a thin line | `#3A4245` `#3B393B` | `#C9CACC` |
| body, the iPhone 13's outer line | `#213744` | `#B4B6B9` |
| button: a fill that sticks out of the body | any of the body colours, `#121515`, `#1C3343` | `#C9CACC` |
| face: the front, where it has a forehead and a chin | `#121515` | `#F4F4F5` |
| ring: a fill with a hole, the home button's | `#262C2D` | `#C9CACC` |
| ring, the iPhone 13's highlight band | `#8EADC1` | `#DADBDD` |
| edge: a small fill on the body's edge, an antenna break | `#36454C` | `#A9ABAE` |
| the thumb scoop in the MacBook's base | `#4E4F53` | `#C4C5C8` |

The MacBook's base is drawn in gradients, and their stops are changed by
colour alone, from `LIGHT_STOPS`: space grey to silver.

What a fill is comes from its shape. A front that the screen covers at least
`GLASS_FRONT` (0.75) of is glass and stays dark, which is the edge-to-edge
phones and the iPads. Only the iPhone SE has a painted face. Everything not in the table keeps its colour: glass,
the island, cameras, speaker and sensor dots, the hairline round the iPhone
SE's screen, translucent antenna bands and the blurred strokes that shade an edge. To
tune a colour, change the table and re-run.

## Our own drawings

`OWN_DEVICES` lists drawings made from an upstream one. Each names the
drawing it is made from, its logical screen and both devices' logical points
to the inch, which puts the two at the same real scale. The base drawing's
bezel, corners, edge and buttons keep their real size, and the screen
between them is resized.

The resize cuts the frame in four at the middle of the screen. A part keeps
its shape and its distance from its own corner. A part that spans the cut,
such as the body or the glass, is stretched across it, which only lengthens
straight edges; a curve across the cut stops the run. A part in the middle,
such as the island and the lens inside it, stays in the middle.

- `ipad-mini` is `ipad-air-4` with a 744 x 1133 screen at 163 points to the
  inch against the Air's 132. The bezel is the Air's width, so it is a
  larger share of the smaller screen. The Air's drawing has no camera, so
  the lens is `ipad-pro-11-inches-m4`'s, scaled to the same real size and
  put in the middle of the top bezel, the short edge.
- `iphone-air` is `iphone-16-pro` with a 420 x 912 screen. Both are 460
  pixels to the inch at 3, so nothing but the screen changes size.

The frames are not whole numbers, because the screen keeps the logical
aspect exactly.
