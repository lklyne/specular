---
name: Multi-agent presence attribution
timeout: 150s
---

## Scenario

This is the regression scenario for issue #319's Phase 0/Phase 4 findings:
a module-singleton `thinkingTimer`, a global `activeScanId`, and
`clientName`-based cursor dedupe all break down with more than one
concurrent agent. It also covers Phase 0's verification that the
agent-browser daemon's `--session pageId` key causes two sessions driving
the *same* page to share one daemon (and therefore one cursor).

You are one Claude process, so "two CLI sessions" here means driving two
`specular` CLI invocations concurrently as separate OS processes with
distinct identity, per the session-identity fallback chain in
`src/main/shared/app-client.ts` (`resolveSessionId` / `SPECULAR_SESSION_ID`
override, `SPECULAR_CLIENT_NAME` override): set different
`SPECULAR_SESSION_ID` and `SPECULAR_CLIENT_NAME` env vars for each shelled-out
`specular` call and run them as background jobs so their CDP/HTTP traffic
overlaps in time. This does not require two separate agent-host processes —
the env var override is the documented, supported way to get two distinct
presence identities from one shell.

### Part A — different pages

1. Add two pages: `specular add page <urlA>` and `specular add page <urlB>`
   (any two working URLs; they must be genuinely different pages/frames).
2. Concurrently (background both, e.g. with `&` and `wait`), as session
   "agent-a" (`SPECULAR_SESSION_ID=presence-test-a SPECULAR_CLIENT_NAME=agent-a`)
   click something on page A, and as session "agent-b"
   (`SPECULAR_SESSION_ID=presence-test-b SPECULAR_CLIENT_NAME=agent-b`) click
   something on page B, at roughly the same time.
3. While both are in flight (or immediately after), poll
   `GET /session/presence` on the app control server and capture a response.

### Part B — same page

1. Add one page: `specular add page <urlC>`.
2. Concurrently, as session "agent-a" and session "agent-b" (same env-var
   pattern as Part A, same page id this time), each click a *different*
   element on the same page.
3. Poll `GET /session/presence` during/after and capture a response.

## Expected outcomes

- **Part A (different pages):** `GET /session/presence` returns two entries
  with distinct `sessionId`s, one attached to each page; each session's
  click only moves its own cursor (agent-a's cursor tracks page A's click
  target, not page B's, and vice versa); thinking→acting transitions for
  one session do not cancel or reset the other's cursor state.
- **Part B (same page):** same two distinct `sessionId`s in
  `GET /session/presence`; each session's click moves only its own cursor
  to its own click target — if instead one session's clicks visibly move
  the *other* session's cursor (or only one cursor ever appears), that
  reproduces the daemon-key misattribution named in issue #319's Phase 0
  ("the agent-browser daemon is keyed by `pageId` alone... two sessions
  driving the *same* page share one daemon pinned to the first session's
  CDP URL") — report it as a FAIL with the captured `/session/presence`
  payload and a description of which cursor moved.

## Notes

- If Phase 0/Phase 4 have not yet landed on this branch, Part B is expected
  to fail — that is the point of keeping it as a scenario: it stays red
  until the daemon-key fix ships, then becomes the regression test that
  keeps it fixed.
- Use the HTTP control server (`GET /session/presence`) for verification,
  not just visual screenshots — attribution is precisely what the visible
  cursor position can be ambiguous about with two cursors close together.
  Still take full-window screenshots at each step per the standard
  reporting format.
- "Correct thinking transitions" means: while one session is between CLI
  commands (simulate with a `sleep` before its next call), its cursor should
  enter a "thinking" presentation independent of what the other session is
  doing — the other session's activity must not reset or cancel the first
  session's `thinkingTimer` (the module-singleton bug named in issue #319).

## Cleanup

- Delete both/all pages created for this test
- No other cleanup needed — presence cursors expire server-side when their
  session goes idle
