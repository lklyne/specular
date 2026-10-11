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

## What it writes

- `<name>.svg` for every `*/*/frame.g.dart`, where `<name>` is the folder
  name in lower case with dashes. The hand-written `generic/*/frame.dart`
  and `macos/macbook_pro/frame.dart` are not converted.
- `<name>-light.svg` for the devices in `LIGHT_DEVICES`: the same draws in a
  light colourway. See "The light colourway" below.
- `manifest.json`, one row per device, sorted by name. `lightFile` names the
  light drawing where there is one. `body` and
  `bodyCornerRadius` are the device's main outline without the buttons that
  stick out: the largest filled outline that does not cover the whole
  frame, cut to the view box. A device with no fill over four fifths of its
  frame gets neither field.
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

The package draws each device in one dark colourway. For the four devices in
`LIGHT_DEVICES` the script writes a second file with some fills changed and
nothing else: the geometry, the draw order and every other colour are the
dark file's. A fill is changed by what it is and its dark colour, from
`LIGHT_FILLS`:

| what the fill is | dark | light |
|---|---|---|
| body: the device's main outline, which shows as its edge | `#3A4245` | `#C9CACC` |
| body | `#454C60` | `#E3E4E5` |
| button: a fill that sticks out of the body | `#121515` | `#C9CACC` |
| button | `#454C60` | `#C9CACC` |
| face: the front, where it has a forehead and a chin | `#121515` | `#F4F4F5` |
| ring: a ring on the face, the home button's | `#262C2D` | `#C9CACC` |

What a fill is comes from its shape. A front that the screen covers at least
`GLASS_FRONT` (0.8) of is glass and stays dark, which is the edge-to-edge
phones and the iPad. Everything not in the table keeps its colour: glass,
the island, cameras, speaker and sensor dots, the hairline round the iPhone
SE's screen, antenna bands and shading. To tune a colour, change the table
and re-run.
