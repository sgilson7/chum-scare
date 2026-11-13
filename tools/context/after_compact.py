#!/usr/bin/env python3
"""Run by Claude Code after a compaction (.claude/settings.json, SessionStart "compact").

Whatever this prints is put back in front of the agent: STATE.md, and how to
get the rest without re-reading. It looks for STATE.md in this project, or,
when the session was started in the folder above it, in the newest project
folder there.

With the work index on (rule 24), it also asks the index for the step STATE.md
names under "What is next" and prints the top cards, so recall happens without
the agent having to remember to ask. In the platformer trials an agent with the
index asked it once in 208 calls and re-read 29 of its own files after
compactions instead. BUILDER_MEMORY=off leaves the cards out.
"""

import os
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))
import hooklib  # noqa: E402

PUSH_K = int(os.environ.get("BUILDER_PUSH_K", "5"))
PUSH_CHARS = 6000   # about 1.5k tokens: the cards must cost less than the re-reads they replace


def find_state(start):
    for cand in [start / "STATE.md"] + sorted(start.glob("*/STATE.md"), key=lambda p: -p.stat().st_mtime):
        if cand.exists():
            return cand
    return None


def next_step(state_text):
    """The text under STATE.md's "What is next" heading, or where the build is when there is none."""
    for head in (r"what is next|next", r"where the build is"):
        m = re.search(r"^##\s*(?:" + head + r")[^\n]*\n(.*?)(?=^##\s|\Z)", state_text, re.S | re.M | re.I)
        if m and m.group(1).strip():
            return " ".join(m.group(1).split())[:400]
    return ""


def push_cards(state):
    """The work index's top cards for the next step, as printed by tools/ask.py --mine."""
    if not hooklib.memory_on():
        return ""
    ask = state.parent / "tools" / "ask.py"
    query = next_step(state.read_text())
    if not ask.exists() or not query:
        return ""
    try:
        out = subprocess.run([sys.executable, str(ask), "--mine", query, "-k", str(PUSH_K + 2)], cwd=state.parent,
                             capture_output=True, text=True, timeout=60,
                             env=dict(os.environ, BUILDER_PROJECT=str(state.parent))).stdout
    except (OSError, subprocess.SubprocessError):
        return ""
    out = out.strip()
    if not out or "is off in this project" in out:
        return ""
    # STATE.md is printed above in full: drop its own cards and keep PUSH_K others
    blocks = re.split(r"(?m)^(?=── card:)", out)
    out = "".join([b for b in blocks if b.startswith("── card:") and not b.startswith("── card:mine.state")][:PUSH_K]).strip()
    if not out:
        return ""
    if len(out) > PUSH_CHARS:
        out = out[:PUSH_CHARS].rsplit("\n", 1)[0] + "\n…"
    hooklib.log("push_recall", query=query[:120], cards=out.count("── card:"), chars=len(out))
    return out


def main():
    state = find_state(Path.cwd()) or find_state(HERE)
    print("The context was just compacted. Builder's rules still hold (CLAUDE.md is loaded).")
    if state:
        print(f"Read this first; it is the project's long-term memory ({state}):\n")
        print(state.read_text())
        cards = push_cards(state)
        if cards:
            print("\nCards from this project's own work for the next step, asked for you (rule 24). "
                  "Use these before opening any file again; ask tools/ask.py --mine for more:\n")
            print(cards)
    print("Get anything else from cards, not by re-reading files: python3 tools/ask.py \"question\", "
          "or --card <id> for one. Open a file only where a card points, and only the lines it names.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
