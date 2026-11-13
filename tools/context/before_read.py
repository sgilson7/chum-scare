#!/usr/bin/env python3
"""Run by Claude Code before each Read (.claude/settings.json, PreToolUse "Read").

Recall before read (rule 24), as advice: when the agent reads a long file
whole (over 300 lines, $BUILDER_READ_LINES, with no offset or limit), it names
the file's cards and their line numbers, so the next time the agent needs a
part of this file it asks for that part. It never blocks a read, and it says
nothing with BUILDER_MEMORY=off. Claude Code shows the advice beside the
read's result.
"""

import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import hooklib  # noqa: E402

LINES = int(os.environ.get("BUILDER_READ_LINES", "300"))


def main():
    data = hooklib.hook_input()
    if not hooklib.memory_on():
        return 0
    i = data.get("tool_input") or {}
    path = Path(str(i.get("file_path", "")))
    if i.get("offset") or i.get("limit") or not path.is_file():
        return 0
    try:
        raw = path.read_bytes()
    except OSError:
        return 0
    if b"\0" in raw[:8192]:          # an image or other binary: its "lines" mean nothing
        return 0
    text = raw.decode(errors="replace")
    n = text.count("\n") + 1
    if n <= LINES:
        return 0
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))   # the ask.py beside this hook
    try:
        import ask
        if path.suffix in ask.TARGET_EXT:
            cards = ask.cut_code("file", path, path.name)
        elif path.suffix == ".md":
            cards = ask.cut("file", path)
        else:
            cards = []
    except Exception:  # a hook must never stop a read
        cards = []
    hooklib.log("read_advice", file=str(path), lines=n, cards=len(cards))
    where = [c["file"].rsplit(":", 1)[-1] for c in cards]
    listing = "; ".join(f"{c['title'].split(': ', 1)[-1][:50]} (line {w})" for c, w in zip(cards[:12], where)
                        if w.isdigit())
    msg = (f"{path.name} has {n} lines (about {len(text) // 4000}k tokens), and all of it now rides along on every "
           "later call. Next time, take only what you need: ")
    msg += (f"its parts are {listing}; Read with offset and limit for one of them, or " if listing else "")
    msg += "ask python3 tools/ask.py --mine \"what you need from it\" (cards from your own work) first."
    hooklib.tell("PreToolUse", msg)
    return 0


if __name__ == "__main__":
    sys.exit(main())
