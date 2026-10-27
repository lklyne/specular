# Rust native big run: by-hand checklist

**Status:** Walk it once, after `native/scripts/validate-all.sh` passes. It covers only what a script cannot judge. Each item says what to do, then what right looks like.

**Setup.** Quit any running Specular. Then, from `native/`:

```sh
export CEF_PATH="$HOME/.local/share/cef"
crates/specular-shell/scripts/bundle-app.sh --profile debug
APP="$PWD/target/debug/bundle/Specular.app/Contents/MacOS/Specular"
export SPECULAR_NATIVE_CONFIG_DIR=/tmp/specular-check      # keeps your preferences out of it
cp -R fixtures /tmp/specular-fixtures                      # opening a fixture in place writes into it
"$APP" /tmp/specular-fixtures/kitchen-sink.canvas
```

## Window and input

1. **Trackpad.** Two-finger scroll pans with momentum. Pinch zooms about the pointer. Text does not shimmer while it moves and is sharp a frame after you stop. Pause mid-gesture and resume: no stutter.
2. **Live resize.** Drag the window's edge, then go full screen and back. The canvas does not stretch or trail the frame, and the toolbar and sidebar stay put.
3. **Second display.** Move the window to a display of another scale or refresh rate. Text stays sharp and pans stay smooth.
4. **IME.** With a Japanese or Pinyin input method, type into a sticky, a page's text field, the chat field and a sidebar rename. Marked text is underlined, the candidate window sits by the caret, and letters arrive once. In a sticky, Escape cancels the composition before it ends the edit. In the chat field, Enter commits and does not send.
5. **VoiceOver.** Turn it on (Cmd+F5). The toolbar buttons, the sidebar rows and the settings switches are read by name. Note anything read as "group" or silent. Nothing is expected beyond what GPUI gives, so this is a survey, not a pass or fail.

## The context menu (redrawn in the Kit)

6. Right-click a sticky. A menu opens with its corner at the pointer, styled like the sidebar's canvas-row menu, in light and in dark. The sticky becomes the selection, lines split the sections, and Cut, Copy, Paste and Duplicate show ⌘X, ⌘C, ⌘V and ⌘D muted at the right.
7. Right-click a page: Back, Forward and Reload come first. Back is greyed on a page with no history and cannot be clicked.
8. Right-click empty canvas: Paste and Select all. Paste puts the item where the menu opened.
9. Right-click near the bottom-right corner: the menu stays inside the window.
10. Arrow keys move through the rows, Return picks and Escape closes. The arrows do not nudge the selection.
11. Click outside the menu, on the canvas, toolbar or sidebar: it closes and nothing else happens. No marquee starts and the selection stays.
12. Scroll or pinch with the menu open: the canvas does not move.
13. The menu shows the moment you right-click, with no flash of anything drawn in the canvas first.
14. Right-click a sidebar canvas row: its Rename and Delete menu works.

## Pages

Open `/tmp/specular-fixtures/pages.canvas`, or any canvas with a real site.

15. **New-tab links.** Click a `target=_blank` link, a link that calls `window.open`, then Cmd-click and middle-click ordinary links. Each opens a new page beside the source at the source's size. The first two select it and bring it into view. Cmd-click and middle-click leave the selection and the camera alone. A plain click still navigates in place. Cmd+Z removes the new page.
16. One click selects a page, a second enters it, Escape leaves. Typed letters arrive once and Backspace removes one character. V, R and Backspace typed in the page do not switch tools or delete the page.
17. Entered, a scroll scrolls the page. Not entered, the same gesture pans the canvas.

## Settings, motion and cursors

18. **Reduce motion.** With the app open, turn on System Settings > Accessibility > Display > Reduce motion. Within a second, opening a page tab jumps to Fill instead of springing, the thread avatars hold still, and an agent cursor lands without travelling. Turn it off: motion returns.
19. **Cursor switches.** Settings > Cursors. With an agent working on a page, turn off "Agent cursors": the cursor goes, and the agent still clicks and types. With two pages in a sync set, hover one: a blue arrow follows on the other. Turn off "Synced cursors": the arrow goes, and the other page still gets the click. Relaunch: both switches kept.
20. **Folder dialogs.** File > Open space, the first-run "Choose folder", and the page dock's repo folder control each open the system folder dialog over the window. Cancel leaves everything as it was.
21. **Sidebar.** Shift-click and Cmd-click rows in the Pages section: they add to the selection as in Finder.

## Agents and the CLI

22. **Setup dialog.** Specular > Set up Specular…. Install the command, then in a new terminal run `specular canvas`: it prints the open canvas. If an old `specular-cli.sh` link was on the PATH, the dialog offers to replace it.
23. Run `specular add note "hi"`, then `specular add page https://example.com`. Each shows in the window, and Cmd+Z takes it back.
24. **One cursor per session.** In a Claude Code session, run several `specular` browse verbs on a page in a row. One agent cursor moves between them rather than one per command.
25. **Fix comments.** On a page whose origin is bound to a repo, turn on the composer's `auto` chip, or leave two comments and call `/annotations/fix` with the origin. One agent run starts per thread, and the comments show as sent.

## Errors and release

26. **Error log.** With a page open, kill its renderer (`pkill -f "Specular Helper (Renderer)"`). `~/Library/Logs/Specular/errors.log` gets a line naming the page's URL and the reason. The app keeps running.
27. **Release rehearsal.** Run the release workflow by hand (Actions > release > Run workflow, rehearsal mode). It builds, signs and notarizes without the deleted root files.
28. **Branch protection.** Remove the `check` and `fallow` required checks, which no workflow reports any more, and require "Native" instead.
29. **Kit scenario failures.** Read the `kit` table in `runs/validate/kit/log.txt`. Phase 9 left one: `tabs`, where closing two tabs one after the other at the same spot loses the second close in the window (the close button gets the press and never the release). Close three page tabs by clicking the same spot quickly: each click closes one. If one is lost, that is the bug; if not, the script's posted events are to blame. Any other failure is listed with its reason in the big-run plan's "Validation".

## Fixed in the Kit window by phase 9

30. Select a sticky, open its colour list and pick a colour. The list stays open. Press Escape: the list closes and the sticky stays selected. Press Escape again: the selection goes.
31. With a dock list open, click a tool button or the canvas. The list closes and nothing else happens: the tool stays, no marquee starts.
32. Drag a sticky that sits on a page off the page, press Control before letting go, then let go. The drag ends there and the sticky keeps its page (it moves with the page). No context menu opens.
