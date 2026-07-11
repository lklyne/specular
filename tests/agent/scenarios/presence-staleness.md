---
name: Presence staleness (R1 measurement)
timeout: 120s
---

## Scenario

This is the R1 measurement scenario from issue #319 / ADR 0029: the pre-act
dwell widens the gap between agent-browser resolving an element's rect and
the click actually dispatching, so on a page that keeps moving, clicks can
land where the target *was*. This scenario measures the miss rate against
today's dwell settings — it does not change any constants itself.

1. Serve the fixture page locally, e.g.:
   `python3 -m http.server 8934 --directory tests/agent/fixtures &` (note
   the PID so you can kill it during cleanup).
2. Add it to the canvas: `specular add page http://127.0.0.1:8934/presence-staleness.html`.
3. The page has a button (`#target`) that re-layouts every 800ms (it swaps
   `left: 40px` / `left: 440px` on a `setInterval`) and a `Hits: N` counter
   that increments only on a real click event dispatched at the button.
4. Take a snapshot (`specular snapshot -i -f <pageId>`) and click the ref for
   `#target` (e.g. `specular click @eN -f <pageId>`). Repeat this **10
   times**: re-snapshot before each click (the ref itself doesn't go stale
   between rounds since the DOM structure is unchanged, but re-snapshotting
   between attempts keeps each click's resolve→dispatch gap comparable and
   matches how a real agent would drive this loop) and click the button's
   ref each time. Don't add artificial delay between snapshot and click —
   the point is to observe the dwell's natural timing against the page's own
   800ms cadence, not to dodge it.
5. After the 10 attempts, read `Hits: N` from the page (e.g.
   `specular eval "document.getElementById('hits').textContent" -f <pageId>`
   or a snapshot showing the text) and report `N / 10` as the observed
   landed-click rate.

## Expected outcomes

- 10 click attempts are issued against the moving `#target` button
- The final `Hits: N` count is read back and reported as `N / 10`
- Report notes whether any click's `specular click` call itself errored
  (e.g. stale ref) vs. silently landing on the wrong coordinate (a "miss"
  that reports success from the CLI's point of view but doesn't increment
  the counter) — these are different failure modes and both matter here

## Notes

- This is a **measurement**, not a pass/fail correctness check — the result
  file's outcome table should record the hit rate as data (e.g.
  `Landed: 8/10`), not force a PASS/FAIL verdict on a specific threshold.
  Report `PASS` if all 10 attempts were issued and a hit count was
  successfully read back; the hit rate itself is the payload this scenario
  exists to produce.
- This run measures today's dwell configuration only. Comparing dwell at
  0/120/300ms (the adaptive-dwell go/no-go data referenced in issue #319) is
  done by tuning `PRESENCE_STEP_DELAY_MS` / the adaptive-dwell constants in
  `src/shared/presence-timing.ts` and re-running this same scenario at each
  setting — not by varying anything inside a single run.
- The 800ms re-layout interval was chosen to sit comfortably above the
  ~300ms full dwell (`PRESENCE_STEP_DELAY_MS`) so a single run has a
  reasonable chance of observing both hits and misses; if every attempt
  hits or every attempt misses, note that in the report as a signal the
  interval may need retuning for future runs, rather than discarding the
  result.

## Cleanup

- Delete the page created for this test
- Kill the local `http.server` process started in step 1
