# Rust rebuild: handoff

**As of:** commit `07285583` on `claude/rust-chrome-spike`, 2026-10-08.
**For:** whoever owns `native/` next. Sources are in brackets: a name in capitals is an entry in the [run log](./rust-native-rebuild-log.md), and "code" means I read it in the tree at that commit.

The short version. In about a day, agents built a Rust app that opens Specular's real space folders, draws every kind of canvas item, hosts live pages through CEF, edits text, saves files Electron can read, answers the CLI, and has a chat panel, sync sets and an app bundle. It was checked by about 1,300 tests, 18 headless session scripts and scripted window captures. Nobody has sat down and used it. Expect the by-hand pass to find things.

## What works, and how we know

- **Files.** Every `.canvas` in the repo loads and saves to the same JSON, and three save byte for byte. One change to an Electron file changes one field. [F3, QA, scenarios `f1` to `f3`]
- **The canvas.** Select, marquee, move, resize, duplicate, delete, undo and redo, six kinds, edges, groups, auto-layout, guides, comments, text and markdown editing. Proven by tests on the document and by PNGs an agent read. [S1 to S9, K, T, C, ARRANGE entries]
- **Live pages.** Real pages load, take clicks, keys and a scripted composition, navigate, and keep their place in the stack order. Two pages were driven apart by the CLI's browse verbs, 37 checks in each shell. [CEF path, A3]
- **The API.** The Electron CLI runs against this app unchanged. `print-pdf`, `record`, `component-states` and `design-system` answer 501. [A1, A3, ARRANGE]
- **Speed.** A pan frame costs 0.4 to 1.6 ms of CPU on the bench canvases, and an idle winit window draws almost nothing. Settled idle memory for 20 static pages was 2,546 MB against Electron's recorded 3,623 MB. [Performance, part 2]
- **The GPUI Kit shell** held 120 fps with synthetic and CEF pages in debug builds, typed into a CEF field and a sticky by script. [GPUI-SHELL]
- **The bundle.** `Specular Native.app` builds, verifies under `codesign --verify --deep --strict`, asks for a space on first launch and reopens it on the next. [APP-BUNDLE]

## What no person has verified

- **Anything by hand.** Every window interaction was a scripted `NSEvent` or a headless script. Scroll and pinch were never sent to the GPUI window by script at all. [ADR 0040]
- **Real input methods, a real trackpad, a live window resize, a second display, VoiceOver, 40 CEF pages under the Kit.** [ADR 0040]
- **The settings dialog and the canvas after first run in the bundle.** The screen was locked for that whole run, so the Kit drew only its first frame. The checks on files and logs ran. The captures of those two did not. [APP-BUNDLE]
- **Finder opening a `.canvas`.** Not run. [APP-BUNDLE]
- **Spaces, the sidebar, page chrome and the context menu in a window.** "Nothing was run in a window" for each when it landed. [P5, P3/P4, Groups/edges]
- **Long frames.** The machine lost power and was reindexing during the reruns. Both old and new builds dropped frames that day. [Performance, part 2]
- **The performance numbers on the GPUI shell.** All were taken on the winit shell. [Performance, part 2]
- **A real `claude` run on a bound repo.** Two real runs were made in a scratch folder. The repo path was tested with the scripted runner only. [RIGHT-PANEL, INSPECT-LOOP]
- **Most tests under mutation.** 386 were checked and 32 of those had been passing for the wrong reason. The other 342 in `tests/it/` and all of the scene, app, bench and cef suites were not. [CLEANUP-A]

## The by-hand checklist

Merged from the log's "Needs a human at a Mac", ADR 0040's list and the per-entry notes, with repeats removed. In order. Each item is what to do, then what right looks like.

**Setup.** Quit the Electron app. Then:

```sh
cd native && export CEF_PATH="$HOME/.local/share/cef"
cargo build -p specular-shell --features cef
crates/specular-cef/scripts/bundle-macos.sh debug specular
APP="$PWD/target/debug/specular.app/Contents/MacOS/specular"
export SPECULAR_NATIVE_CONFIG_DIR=/tmp/specular-check     # keeps your own preferences out of it
cp -R fixtures /tmp/specular-fixtures                     # opening a fixture in place makes fixtures/.specular/
```

Items marked (winit) need `cargo run -p specular-app -- FILE`, because the GPUI shell does not have that feature yet.

**Window.** `"$APP" /tmp/specular-fixtures/kitchen-sink.canvas`

1. Look at it. One menu bar. Text is sharp on a 1x and a 2x display. Cards have a soft shadow. The traffic lights sit in the toolbar's left padding, the window drags by the toolbar, and a double click there zooms it.
2. Drag the window's edge. The canvas should not stretch or trail the frame. Then full screen, and a second display at 60 Hz or another scale.
3. Two-finger scroll, then pinch. Scroll pans with momentum. Pinch zooms about the cursor, glyphs do not shimmer or go blocky, and they sharpen a frame after you stop. Pan slowly over stickies: text should not shimmer against its note. Pause mid-gesture and resume: no stutter. Repeat with a Kit popover open.

**Gestures.**

4. Drag a sticky. It snaps to the grid and feels attached. Press Shift mid-drag: one axis. Option-drag: a faded copy of the item shows where it will land. Try every handle on each kind and watch the corner cursors. Drag a marquee.
5. Drag from where an edge crosses a sticky: the sticky moves. Click there: the edge is selected.
6. Drag an item past another's edge: a guide line shows and does not pull the item.
7. Drag an item over a group: a ring shows and the release puts it in. Drag it out: the group hugs what is left. Double-click a group to work inside, Escape to step out. Double-click its title and type, also zoomed out past half.
8. Hover an item and drag from its dot to another item's dot, then to another item's body. Grab an edge's end and drop it on nothing: the edge goes, and Cmd+Z brings it back. Double-click an edge and type a label.
9. Select a group, press Shift+Cmd+A. Drag a member's dot along the row: judge the half-slot swap. Find the 2 px gap bar at 50% zoom and drag it.
10. Cmd+] and Cmd+[ with and without Shift restack. Cmd+G on two items groups, Cmd+Shift+G ungroups.
11. Cmd+D, Cmd+Z, then an arrow key: the originals move. Cmd+Z, D, A, = and 1 each act once, not twice. Every menu item shows its shortcut and runs once.
12. (winit) Undo, Copy and Delete are grey with nothing to act on, and the active tool is checked in Tools. Right-click each kind and empty canvas: a menu opens at the pointer.

**Text.**

13. Double-click a sticky. The caret sits between glyphs at zoom 0.25, 1 and 3, blinks once a second, and typing does not lag in a few hundred words.
14. With a Japanese or Pinyin input method, type into a sticky, a page's text input, the chat field and a sidebar rename. Marked text is underlined, the candidate window is by the caret, letters arrive once. In a sticky, Escape cancels the composition before it ends the edit. In the chat field, Enter commits and does not send.
15. Tools > Document, click, type. `Untitled Note.md` appears and fills in a third of a second after you stop. Edit the file elsewhere while the edit is open: a conflict copy appears beside it. Quit mid-edit: the last keys are in the file. Try Cmd+Option+1, which macOS may take.

**Pages.** `"$APP" /tmp/specular-fixtures/input.canvas`, then `pages.canvas`

16. One click selects, a second enters, Escape leaves. A click in the page flips its background.
17. Type into the text input. Letters arrive once. Backspace removes one character. Cmd+A, C, V, Z and Tab act in the page, and no menu shortcut also fires. V, R and Backspace typed in the page do not switch tool or delete it.
18. Open the `<select>` at zoom 1, 0.5 and 2. The list draws over the page in the right place and a pick closes it.
19. Entered, scroll the tall page: it feels like a browser and does not pan the canvas. Not entered, the same gesture pans the canvas.
20. The title above a page reads `Title — address` and `Loading…` during a load. Click a link: the title follows and the saved file has the new URL. Entered, Cmd+[ goes back, Cmd+] forward, Cmd+R reloads, Cmd+. stops. Only selected, Cmd+[ restacks it.
21. Type a URL into the page popup's address field: the page goes there.
22. Press C and click the tall page's heading: the composer opens with the heading outlined. Drag a region round the button, then scroll the page: the region moves with the button, is cut at the page's edge and gone past it. A drag over blank page makes a region that stays put.
23. Drop a sticky on the tall page and scroll: it moves with the content and can be grabbed where it is drawn.
24. Start a second copy of the app on the same file: both stay up.

**Clipboard, drop and files.**

25. Copy two shapes and their edge, paste. A URL from a browser pastes as a page, a sentence as a sticky, a screenshot (Cmd+Ctrl+Shift+4) as a file in `assets/`.
26. Drop a png and a `.md` from Finder, from inside and outside the space folder. Note where they land. It may be wrong.
27. Make a change and look at the file: it is on disk a third of a second later with the camera in `appState`. Edit the file from outside: the window follows. Quit within that third of a second: saved. R, Shift+R, quit, start: the shape tool is still a diamond.

**Spaces and first run.** Use a copy of your space, or the scratch space.

28. `"$APP"` with no path on a fresh config folder: the first-run view. Click each button and use the real folder dialog. "Create a new space…" on an empty folder gives the Welcome canvas with its note's text showing.
29. Sidebar: add a canvas, rename it in place, duplicate it, delete it. After each, look at the folder in Finder: made, renamed, copied, in the Trash. Scroll the sidebar with the trackpad.
30. Switch canvases. The pages of the one you left stop and the other's load. Come back: camera and selection are where you left them, and Cmd+Z still undoes what you did there.
31. Edit a background canvas's file in a text editor, then switch to it: it shows the edit.
32. Quit, open the same folder in the Electron app: it lists the same canvases under the same names.
33. Settings > General > Change… with the dialog open. Then check "Show the sidebar at launch", which is a known bug below.

**Panels, chat, inspect, sync.**

34. Pick each tool from the toolbar. Recolour a sticky, change a shape's kind and border, restyle an edge, change a page's size. Check the glyphs beside the Electron toolbar. A popup follows its item through a pan and a zoom without lag, and nothing under a popup takes the click. Select text by dragging, then press a formatting button: the selection is kept.
35. Open the chat panel. Paste a real screenshot with Cmd+V. Scroll up during a run. Drag the panel's edge.
36. Inspect tool (`I`) over a real site: judge whether the outline keeps up. Bind a repo through the page popup's folder control and the real dialog. Run one real `claude` turn on that origin and check it edits the repo.
37. Select two pages, press the chain button. Hover a menu on one: the other shows `:hover`. Scroll one with the trackpad and judge whether the follower's jump needs easing.

**API.**

38. With the app open: `specular canvas`, `specular add note "hi"`, `specular add page https://example.com`, `specular focus <id>`. Each shows in the window and Cmd+Z takes it back. Watch a page while `specular click` and `specular fill` run on it. `specular tab new x`, `specular add note "hi" --tab x`, `specular tab switch x`: the note is there, and you were not moved before the switch.
39. `SECRET=$(jq -r .secret ~/.specular/specular-mcp.json)`, then `curl -X POST -H "x-specular-secret: $SECRET" localhost:29979/window/screenshot -d '{"path":"/tmp/shot.png"}'`: the PNG is what the window shows, right way up, right colours. The BGRA swap and the 2x size are unchecked.
40. Start the Electron app first and this one second: the log names the fallback port and file, and the CLI with that `SPECULAR_DISCOVERY_FILE` reaches this app.

**Bundle and speed.**

41. Build the bundle and run `fixtures/scenarios/app/first-run.sh` with the screen on. Read `2-space-created.png` and `3-settings.png`.
42. Open a `.canvas` from Finder with the app running and with it closed. Log in to a site, quit, reopen: still logged in.
43. On a quiet machine, rerun the bench loop in `native/README.md` and look at long frames. Open 40 pages with the Kit's panels open.
44. Open `native/bakeoff/golden/compare-5x-zoom-*.png` and confirm ADR 0039's sharpness call. Its two egui checks no longer apply.

## Known bugs

Each with the shortest way to see it. "Seen once" means one scripted run.

- **Backspace removes two characters in a page field.** GPUI shell, enter the input on `input.canvas`, type, press Backspace. Seen once, never checked in the winit shell. [GPUI-SHELL]
- **Cmd+Z in the chat field undoes the canvas.** Type in the chat field, press Cmd+Z. Seen once. Menu keys other than the eight held back still reach the canvas while typing: select two items, click the chat field, press Cmd+G. [RIGHT-PANEL]
- **GPUI menu items are never greyed.** Open Edit with nothing selected. A disabled item does nothing when chosen. [GPUI-SHELL]
- **The GPUI shell has no context menu on canvas items, and no sidebar toggle or folds.** Right-click a sticky. This is from the log. A parity branch was in progress when this was written. [GPUI-SHELL, P3/P4]
- **"Show the sidebar at launch" does nothing on screen.** Turn it off in Settings, relaunch. [APP-BUNDLE]
- **The GPUI shell never rests.** It calls `turn` and `draw` every frame, so an idle window still draws. Watch its CPU with a still canvas. [Performance, part 2; code: no `frame_wanted` call in `specular-shell`]
- **A click that dismisses a Kit dropdown may also start a gesture on the canvas.** Seen in the spike, not rechecked in the real shell. Open the zoom list, click the canvas. [ADR 0040]
- **Dropped files may land in the wrong place.** They go to the pointer's last position before the drag. [log, "Needs a human" 9]
- **An outside edit is overwritten when you have unsaved changes.** Change the canvas and, inside the 350 ms before the save, write the file from another tool. A warning is logged. [S9]
- **No maximum wait on autosave.** Change the document at least every 350 ms without pause: nothing is saved until you stop. [S9]
- **Deleting a page leaves its comments bound to it, and they stop drawing.** Comment on a page, delete the page. [C1 to C3]
- **An undone "add Document" leaves its `.md` on disk.** Tools > Document, click, Cmd+Z. [T4]
- **A thread closed here shows as open in Electron.** Close a thread, open the space in Electron. [RIGHT-PANEL]
- **Anchored text edited on a scrolled page has its caret off by the scroll.** Drop a text on the tall page, scroll, edit it. A resize there does not fold the shift either. [CEF path]
- **A slow page drops inspect answers.** Sweep the inspect tool fast over a heavy site. [INSPECT-LOOP]
- **The repo folder dialog has no parent window and is opened from inside a GPUI event.** Page popup, folder control, Choose. No failure was seen. [APP-BUNDLE, INSPECT-LOOP]
- **A `.canvas` added to the folder by another tool is not seen** until the space is reopened. [P5]

Known gaps against Electron, not bugs: hand and mono fonts fall back to system fonts, an edge label has the line running through it, a comment badge has no icon, the highlighter has no gradient or grain, there is no dark theme, and followers in a sync set jump instead of easing. [log, "Needs a human"; SYNC-SETS]

## Decisions waiting on the user

1. **ADR sign-offs.** Six are Proposed: [0039](../adr/0039-rust-canvas-render-stack.md) (render stack, whose egui half was never built), [0040](../adr/0040-gpui-kit-hybrid-shell.md) (GPUI Kit shell, built at your direction, text not signed off), and [0041](../adr/0041-typed-document-inverse-command-undo.md) to [0044](../adr/0044-ui-as-pure-models-with-replaceable-renderers.md), written after the fact for decisions the run made on its own.
2. **The four cleanup cuts** in the plan's "Cleanup tasks":
   - Task 1, retire the built-in panel renderer, about 7,000 lines. The Kit cannot run headless, so first choose: scripts name controls through the models with nothing drawn, or the built-in renderer stays as the headless one.
   - Task 2, retire the winit window, about 1,700 lines. `--bench` runs only there. Port it and compare one run across both shells first.
   - Task 3, one shell crate, after task 2.
   - Task 4, retire the Electron comparison half of `specular-bench`, about 1,200 lines. Keep it only if another comparison is planned.
3. **Distribution.** Nothing is started and each needs your credentials or a call: Developer ID signing with CEF's helper entitlements, notarization, a disk image, an update feed, a universal build, the sandbox (CEF runs with `no_sandbox`), and whether to bundle the CLI and the skill. [`docs/native-app-bundle.md`]
4. **Whether to go on.** The spike's performance case came out neutral to mildly good. The case for the rebuild is the architecture. That is a product call, and the by-hand pass is the evidence for it.

## Risks an owner should know

- **The GPUI pin.** `gpui-kit = "=0.7.1"` and `gpui-pre = "=0.3.8"` come from one maintainer's weekly snapshot of Zed's GPUI. The shell relies on three facts about GPUI's macOS backend that no API promises. `NativeCanvas::install` checks them at launch and a test fails if the lockfile moves, so a bump breaks loudly, on the first launch. If the snapshots stop, the fallback is to vendor the last one. [ADR 0040, GPUI-SHELL]
- **The mock keychain.** CEF runs with `use-mock-keychain`, so cookies in `cef-profile/` are encrypted with Chromium's fixed mock key and are not protected by the login keychain. My reading: a saved login is only as safe as the data folder's file permissions. [APP-BUNDLE; code: `specular-cef/src/config.rs`]
- **Captures taken with the screen locked.** The app-bundle run's captures of settings and the post-first-run canvas do not exist. More broadly, a covered window, a locked screen or a sleeping display makes a capture look like blank pages, so an agent's "PNG read" is only as good as the screen state it ran under. [APP-BUNDLE, GPUI-SHELL]
- **Your real space is opt-in, and autosave is why.** `specular-app` opens a scratch copy unless given `--space user` or a path. `specular` opens only a folder you chose in it, or the Electron space if you click "Use the space from Specular". Once open, it is autosaved into. Do not run both apps on one space: each rewrites `.specular/workspace-meta.json`. [Scratch space, APP-BUNDLE, P5]
- **The agent port.** Whichever app starts first takes 29979. If this one does, the Electron app starts with no API. [A1]
- **Agents graded their own work.** The log is candid about what was not run, and this page repeats it, but the tests and the PNG readings were made by the same kind of agent that wrote the code.
- **Nothing has been pushed or reviewed.** One commit a task on a local branch with no upstream and no PRs. [Orchestrator; `git` at this commit]
- **macOS on Apple Silicon only.** The GPUI shell and both menu bars are macOS code. [GPUI-SHELL, Shell batch]
