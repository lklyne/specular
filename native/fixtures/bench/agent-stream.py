#!/usr/bin/env python3
"""Prints a script for the scripted agent runner (`SPECULAR_AGENT_SCRIPT`):
a run that streams a few words every 50 ms for 40 seconds and reads a file
every two, so the transcript grows and the run bar's label changes for as
long as a UI bench profile is measured.

    python3 fixtures/bench/agent-stream.py > agent-stream.txt
"""

import json

WORDS = (
    "The canvas holds a row of pages and a field of notes. Each note is a "
    "short line of text on a soft fill. **Groups** gather the ones that belong "
    "together, and `edges` say which leads to which.\n\n- A list item now and then.\n"
).split(" ")


def line(value):
    print(json.dumps(value, separators=(",", ":")))


def main():
    line({"type": "system", "subtype": "init", "model": "scripted", "session_id": "ui-bench"})
    line({"type": "stream_event", "event": {"type": "message_start"}})
    at = 0
    for step in range(800):
        print("wait 50")
        if step % 40 == 39:
            read = {"type": "tool_use", "name": "Read", "input": {"file_path": f"src/part-{step // 40}.rs"}}
            line({"type": "assistant", "message": {"content": [read]}})
            continue
        text = " ".join(WORDS[(at + i) % len(WORDS)] for i in range(3)) + " "
        at += 3
        delta = {"type": "text_delta", "text": text}
        line({"type": "stream_event", "event": {"type": "content_block_delta", "delta": delta}})
    line({"type": "result", "subtype": "success", "result": "Done."})


main()
