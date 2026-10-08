# Scenario scripts

Whole sessions, scripted, that use the features together. Each `.txt` is a
`--script` file and reads as what a person would do. The step names are in
"Looking at what it draws" in [`native/CLAUDE.md`](../../CLAUDE.md).

```sh
fixtures/scenarios/run.sh                      # all of them
fixtures/scenarios/run.sh c-document           # one
QA_OUT=/tmp/qa fixtures/scenarios/run.sh       # somewhere else
```

`run.sh` builds `specular-app`, runs each script headless and leaves its
PNGs and saved `.canvas` files in `native/runs/qa/<scenario>/`, which is
not checked in. It prints any warning a run logged. `check.py` then
compares the saved canvases as JSON and exits non-zero if a session left
something it should not have. A PNG has no check. Open it with the Read
tool. A scene or compositor change is not done until the frames it touches
have been looked at.

| Script | The session |
|---|---|
| `a-first-session` | Empty canvas: two stickies, a pen and a highlight stroke, three labelled shapes, a move, a resize, undo to nothing, redo. |
| `b-kitchen-sink-selection` | A marquee over stickies, text, shapes and an edge; move, resize, duplicate, Option-drag, nudge, delete, each undone. |
| `c-document` | Add a Document, write markdown with the formatting shortcuts, end, reopen, scroll to the end and back, undo. |
| `d-text-edge-cases` | Emoji, CJK through the input method, a long word, empty lines, replace-all, a multi-line paste, a shape label, editing at zoom 0.25 and 3. |
| `e-clipboard` | Text between stickies, a cut, a copied entity, a pasted URL, pasted text with nothing selected. |
| `f1-reload-own-save` | The canvas `a-first-session` saved, reopened and saved untouched. Run `a-first-session` first. |
| `f2-electron-file-one-change` | The starter space's `Welcome.canvas`, which Electron wrote, with one shape nudged. |
| `f3-fixture-one-change` | The integration suite's `rich-workspace.canvas` with one shape nudged. |
| `g-zoom-and-pan` | Both zoom limits, Command-wheel zoom, and a pan or pinch in the middle of a move, a marquee, a resize and a stroke. |
| `h-tools-and-escape` | Tool keys in a row, Escape at each stage of each gesture, a tool change mid-drag and mid-edit. |

## What `check.py` holds

- In a, b, d, e and g, undoing every step gives back the canvas the session
  opened. In a, redoing gives back what it had.
- In h, an escaped gesture leaves nothing.
- In f1, a canvas saved, reopened and saved again is the same.
- In f2 and f3, one change to a canvas from the Electron app changes one
  field in the file. Every text keeps the size it was read with, though
  this renderer measures it differently.

The camera in `appState` is left out of every comparison.

## Writing one

- The first line names the canvas: `# canvas: ../kitchen-sink.canvas`,
  relative to this folder, or `runs:<scenario>/<file>` for one an earlier
  script saved.
- Start with a `camera x,y,zoom` step, so the positions that follow mean
  something. Positions are screen pixels in a 1600x1000 viewport.
- A press with no `move` or `click` before it lands on whatever is there.
  Snapshot and look.
- Cmd+D pans the camera when the copies land off screen. Put a `camera`
  step after it before going on by position.
- A headless run keeps the clipboard and new Documents in memory, so
  `key cmd+c`, `key cmd+v`, `clipboard some text` and `tool document` work
  and nothing is written beside the canvas.
- Number the snapshots and name them for what they should show. Where the
  outcome is a document, `save` it and add a line to `check.py` instead of
  a snapshot.
