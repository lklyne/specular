# Scenario scripts

Whole sessions, scripted, that use the features together and check what
they leave. Each `.txt` here is one script in the one format that both
runners read (`specular_app::script`): the headless runner
(`specular-app --script`) and the Kit window (`SPECULAR_SHELL_SCRIPT_FILE`).
Scripts are grouped by user flow, one flow a file.

```sh
fixtures/scenarios/run.sh                    # every scenario, headless
fixtures/scenarios/run.sh first-session      # one
fixtures/scenarios/run.sh --kit              # in the Kit window (needs the display)
fixtures/scenarios/run.sh --cef              # cef/: real pages, needs a CEF bundle
fixtures/scenarios/run.sh --cli              # cli/: the CLI end to end
SCENARIO=first cargo test -p specular-app --test scenarios   # headless, from cargo
```

`run.sh` prints one `pass` or `FAIL` line a scenario and exits non-zero
when any fails. What a run saves and draws goes to
`native/runs/scenarios/<mode>/<scenario>/` with its `stderr.log`.
`cargo test --workspace` runs every headless scenario too
(`crates/specular-app/tests/scenarios.rs`), so CI gets them with the
tests.

Headless runs use the synthetic pages and no built-in panels: a
`control NAME` step presses the control from the models. Steps only a
window can do (`resize`, `keycode`, `swipe`, `menu-choose` ...) are skipped
headless and listed when the run ends. In the Kit window a `control` step
clicks the control where it is drawn; one that is not drawn (a sidebar row
out of view, a choice in a closed list, a settings switch) is pressed from
the models, and the run says so.

## A script

```text
# canvas: ../kitchen-sink.canvas
# What the session is, in a sentence or two.
camera 32,-368,0.8
tool sticky
click 200 160
type Ship the beta
key escape
expect count sticky 21
expect entity @top text Ship the beta
undo-to-start
```

- The first line names the canvas, relative to this folder. A script with
  none starts from `empty.canvas`, or from its `given` lines, which come
  before the first step and describe the document inline:
  `given sticky s1 100 100 200 200 Hello`, `given page p1 ...`,
  `given inside g1 shape r1 ...`, `given edge e1 s1 r1`
  (see `crates/specular-app/src/script/given.rs`).
- Start with a `camera x,y,zoom` step, so the positions that follow mean
  something. Positions are screen pixels in a 1600x1000 viewport.
- Name controls with `control text.color`, never by where they are drawn.
  Sidebar canvas rows are named by place: `sidebar.canvas.1`.
- Check state with `expect` lines, not pictures. `state out.txt` writes
  every fact a run can check, each line an `expect` without the word, so
  the way to write one is to run a `state`, read it and paste.
- Every scenario that changes the document checks `undo-to-start`: undo
  until nothing is left, then compare with the canvas the run opened. What
  undo does not take back (a page's address after it navigates) is named:
  `undo-to-start except /nodes/p1/url`.
- A page answers in its own time. After a click or a scroll that asks a
  page something, `wait 100` before checking what came back.
- New ids differ from run to run in a window, so name what a script made
  with `@top` (the front-most entity), `@N` (the Nth from the back) or
  `@selected`.
- `snapshot NN-name.png` is kept for a few draw rules a state check cannot
  see. A person reads them; nothing compares them.

## The facts

```text
expect tool sticky                  expect count sticky 2
expect selection s1 s2 | none       expect count entities|edges|comments|canvases|selected N
expect focus canvas | page p1       expect entity s1 kind sticky
expect editing Some text\nmore      expect entity s1 rect 100 100 200 200
expect showing canvas | p1          expect entity s1 at 100 100 | size 200 200
expect canvas Notes                 expect entity s1 text Ship it
expect zoom 0.5                     expect entity d1 note # Plan
expect camera -400,-900,2           expect entity s1 parent g1 | none
expect sidebar on|off               expect entity s1 gone
expect menu open|closed             expect undo some|none, expect redo some|none
expect saved /nodes/s1/color = "4"
expect saved /nodes/s1/specular/pageAnchor absent
expect saved /annotations/-1 keys id anchor text ...
expect saved /annotations length 6
expect saved same-as start | mark NAME | ../kitchen-sink.canvas
expect saved changed-from start at /nodes/s1/x
```

The canvas facts read what the pointer and the selection show, and
what the last step asked the shell to do; the full list is at the top of
`crates/specular-app/src/script/facts_canvas.rs`:

```text
expect hit 400 185 anchor f1 top    expect guides horizontal top/b.top 100
expect cursor resize-nwse           expect gaps horizontal 100 | 200 300 150
expect handles entity s1            expect drop-target g | entered-group g
expect seen s1 200 160 100 100      expect page p1 100 100 400 300 viewport 400 300
expect order a b @top               expect edge @last @3 @4
expect sent viewport p 500 300      expect sent no save | sent nothing
```

With them come the steps `page-says p1 scrolled 0 40` (or `url`, `fit`,
`element`, `composition`), `appearance dark`, `theme-loaded light`,
`repeat down`, `menu Edit > Undo`, and `act show:p1`. A headless run
does `drop` too, a relative path read from the canvas's folder.

`saved` reads the canvas as a save would write it now; a path steps
through arrays by item id or index (`-1` is the last). Numbers compare by
value. `same-as` and `changed-from` leave out `appState` (the camera and
the open tabs). `mark NAME` remembers the canvas for a later `same-as`, and
`reopen` writes the canvas and reads it back as a relaunch would. Text
uses `\n`, `\t` and `\s` (a space at either end).

## The scenarios

| Script | The session |
|---|---|
| `first-session` | Empty canvas: two stickies, a pen and a highlight stroke, three labelled shapes, a move, a resize, undo to nothing, redo, and the save read back. |
| `selection-kitchen-sink` | A marquee over stickies, text, shapes and an edge; move, resize, duplicate, Option-drag, nudge, delete, each undone. |
| `document-editing` | A Document written with the formatting shortcuts, reopened, a long tail of tasks, scrolled, undone and redone; a second with a numbered list nested and brought back. |
| `text-edge-cases` | Emoji, CJK through the input method, a long word, empty lines, replace-all, a multi-line paste, a shape label, editing at zoom 0.25 and 3. |
| `clipboard` | Text between stickies, a cut, a copied entity, a pasted URL, pasted text with nothing selected. |
| `reload-electron-welcome` | The starter space's `Welcome.canvas`, which Electron wrote, saved as read, one shape nudged changes one field, read back. |
| `reload-rich-workspace` | The Electron suite's `rich-workspace.canvas`, the same way. |
| `zoom-and-pan` | Both zoom limits, Command-wheel zoom, and a pan or pinch in the middle of a move, a marquee, a resize and a stroke. |
| `tools-and-escape` | Tool keys in a row, Escape at each stage of each gesture, a tool change mid-drag and mid-edit. |
| `comments` | Comments on the kitchen sink in Electron's shape: points, a page element, two regions, an empty draft, a two-line draft, delete and undo, annotate a selection, resolve. |
| `toolbar-and-popups` | Every change made by naming a control: a sticky and shapes placed with their tool defaults, restyled, an edge dragged and restyled, undo and redo across a property change, the zoom list. |
| `page-chrome` | The page controls: the page tool's preset, an address entered (a local one over http) and one abandoned, a custom size, undone. |
| `sidebar` | Canvases added, renamed (Enter, Escape) and deleted, sections folded, rows that select and reveal, zoom to fit with and without the sidebar; the kitchen sink is unchanged. |
| `context-menu-and-arrange` | The right-click menu on a sticky, empty canvas and a page, a row and a column arranged and undone, bold put on mid-edit. |
| `chrome-session` | A page from its row, its address and size list, a duplicate from the menu, a second canvas, a page shown alone and a sticky hooked to it there. |
| `guides` | Alignment guides on a move, a copy and a resize; one snapshot of the lines. |
| `auto-layout` | Three shapes made a managed row, regapped, turned to a column and back, reordered by a dot; a loose pair reordered with no group. |
| `page-anchoring` | Items hooked to a scrolled page: a resize, an edit and a region attached to the cell under them, a duplicate that is free, the page deleted and brought back; one snapshot of the fade. |
| `new-tab-links` | Links a page opens in a new tab, foreground and background, and a `javascript:` link that opens nothing. |
| `select` | Pages picked then entered, Shift-click, edges, the marquee's modes, handles. |
| `pointing` | What a point hits: page pixels, handles, anchors, ink, group borders and titles, hover. |
| `groups-enter` | A group picked whole, stepped into and out of, renamed by its title, the innermost drop target. |
| `groups` | Group and ungroup, a group refitting round its members, dragging into and out of groups. |
| `resize` | Every resize rule: cursors, ratios by kind, ink, a group alone, pages laid out, a scaled selection, text. |
| `edges` | Edges drawn, snapped, re-routed, deleted with their ends, followed, and labelled. |
| `moves` | Moves on and off the grid, Shift, Option copies, delete, duplicate, nudge, selection across undo. |
| `tools` | Tool keys and variants, the armed tool's ghost, shapes, text, stickies, pages and strokes placed. |
| `guides-cases` | Guides with Shift off the grid, the half-unit tolerance, neighbours fixed at the press, groups. |
| `arrange` | The stack order verbs and keys, and arranging in a row, a column and a grid. |
| `layout-lines` | A managed row laid out again by each change, reorder dots and gap strips, loose lines. |
| `anchor-cases` | Items hooked to pages by where they land, carried items resized and copied, element attachment. |
| `entered-page` | An entered page's pointer, wheel, keys, input method and Edit menu; what asks for a save. |
| `clipboard-items` | Items copied, cut and pasted with their edges, a group pasted whole, an address and text. |
| `canvases` | Canvases made, switched, renamed, duplicated and deleted, each with its own history. |
| `theme` | The theme button, the system's appearance, and the scheme pages are told. |
| `safe-area` | Framed pages laid out inside their device's safe area until the document covers it. |
| `html-file` | An HTML file node hosted, entered, commented on and shown in a tab, still a file. |
| `text-fit` | Text measured on load with no step, and saved at its size on disk until changed. |
| `images` | The kitchen sink's images loaded, missing or never asked for. |
| `drop` | Files dropped from outside and inside the space, and several cascading. |
| `reduced-motion` | A page's tab opened with transitions on, then with Reduce motion on, and the settings' cursor switches. |

`cef/` holds the real-page suite (`run.sh --cef`): `pages.txt` (comments
on page elements, a scrolled page, a reload) and `sync.txt` (a sync set
followed through a click, a link and a scroll), then `cef/cli-pages.sh`,
the browse verbs on real pages. `{site}` in a script is where `run.sh`
serves `cef/`.

`cli/run.sh` (`run.sh --cli`) is the `specular` CLI end to end: it starts
the Kit app on a scratch space and runs every verb against it, asserting
on output, exit codes and the saved canvas (`cli/cases.py`).
`cli/run.sh tabs_are_made_shown_written_in_the_background_and_deleted`
runs one case.

`app/first-run.sh` and `resize/` drive the release bundle for the first
run and the live-resize report; they are not scenarios.
