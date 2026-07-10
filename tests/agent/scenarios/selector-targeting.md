---
name: Selector-based targeting
timeout: 60s
---

## Scenario

Add a page pointing at any working URL that has at least one clickable
link or button, using `specular add page <url>`. Without taking a snapshot
first, drive an interaction by a re-resolving target instead of an `@eN`
ref: try a CSS selector (`specular click "a" -f <pageId>`), a text selector
(`specular click "text=..." -f <pageId>`), or a semantic locator
(`specular find role link click --name "..." -f <pageId>`). Confirm the
interaction lands (e.g. navigation occurs, or `specular get url -f <pageId>`
shows the expected result). Then repeat the same selector-based command a
second time after the page has changed — it should re-resolve and succeed
again without needing a snapshot in between.

## Expected outcomes
- A selector- or role-based `specular` command succeeds without any prior `specular snapshot` call
- The interaction visibly changes the page (navigation, or a `get url` change)
- The same selector-based command re-resolves and succeeds again after the DOM has changed, with no snapshot re-run in between
- No `@eN` ref is used anywhere in the interaction

## Notes
- This exercises the `specular` CLI directly — the quoted-target fix
  (`shellQuote` in `src/main/cli-commands.ts`) is what makes a
  multi-word target like `"text=Sign in"` survive the round-trip through
  `splitShellArgs` in `browse-handler.ts` instead of being re-split into
  two tokens.

## Cleanup
- Delete the page created for this test
