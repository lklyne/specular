---
description: Mine this session's transcript for papercuts and append them to PAPERCUTS.md
allowed-tools: Bash(pnpm papercut:review*), Read
---

Run `pnpm papercut:review` and report what it appended to `PAPERCUTS.md`.

The script picks the most recent transcript for this repo, sends it to Gemini
Flash (needs `GOOGLE_API_KEY` in `.env`), and appends the frictions it finds.
If it reports no papercuts, say so — don't invent entries or add them by hand.

$ARGUMENTS is an optional transcript path to review instead of the latest one.
