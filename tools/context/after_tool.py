#!/usr/bin/env python3
"""Run by Claude Code after each tool call (.claude/settings.json, PostToolUse).

Two jobs, both quiet unless needed:
- The context warning (rule 21). When the context passes 200k tokens
  ($BUILDER_CONTEXT_WARN), and again at each further 100k, it tells the agent
  once: rewrite STATE.md, then ask the person to type /compact. Under a
  compaction cap ($CLAUDE_CODE_AUTO_COMPACT_WINDOW) the mark is two thirds of
  the cap instead: Claude Code compacts at about five sixths of it, so a 200k
  mark under a 200k cap is never reached, and two thirds left 7 to 25 calls
  before each compaction in the platformer trials.
- The work index (rule 24). After a write, an edit or a commit it embeds the
  new cards in the background, so tools/ask.py --mine stays fast. Nothing is
  embedded without the optional model, and nothing at all with BUILDER_MEMORY=off.
"""

import json
import os
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import hooklib  # noqa: E402

def warn_at():
    mark = int(os.environ.get("BUILDER_CONTEXT_WARN", "200000"))
    try:
        cap = int(os.environ.get("CLAUDE_CODE_AUTO_COMPACT_WINDOW", "0"))
    except ValueError:
        cap = 0
    return min(mark, cap * 2 // 3) if cap > 0 else mark


WARN = warn_at()
STEP = 100_000


def warn(data, proj):
    ctx = hooklib.last_context(data.get("transcript_path"))
    if not ctx or not proj:
        return
    state = proj / "notes" / "index" / "context-warn.json"
    seen = {}
    try:
        seen = json.loads(state.read_text())
    except (OSError, ValueError):
        pass
    sid = data.get("session_id", "session")
    band = -1 if ctx < WARN else (ctx - WARN) // STEP
    if band <= seen.get(sid, -1):
        if band < seen.get(sid, -1):          # compacted: start again
            seen[sid] = band
            state.write_text(json.dumps(seen))
        return
    seen[sid] = band
    state.parent.mkdir(parents=True, exist_ok=True)
    state.write_text(json.dumps(seen))
    hooklib.log("context_warning", context=ctx, session=sid)
    hooklib.tell("PostToolUse",
                 f"The context is now about {ctx // 1000}k tokens, past the {WARN // 1000}k mark (rule 21). Every later "
                 "call re-reads all of it. At the next stopping point: rewrite STATE.md (where the build is, the latest "
                 "commit, decisions in force, what is next, and card ids for what you would otherwise re-read, "
                 "card:mine.<id>), commit, and ask the person to type /compact. Afterwards, get things back with "
                 "python3 tools/ask.py --mine \"question\" instead of re-reading files.")


def warm(data, proj):
    if not proj or not hooklib.memory_on():
        return
    tool = data.get("tool_name", "")
    cmd = str((data.get("tool_input") or {}).get("command", ""))
    if tool not in ("Write", "Edit", "MultiEdit", "NotebookEdit") and "git commit" not in cmd:
        return
    lock = proj / "notes" / "index" / "warm.lock"
    try:
        if lock.exists() and time.time() - lock.stat().st_mtime < 300:
            return
        lock.parent.mkdir(parents=True, exist_ok=True)
        lock.write_text(str(os.getpid()))
        subprocess.Popen(
            [sys.executable, "-c",
             "import subprocess,sys,os;"
             f"subprocess.run([sys.executable,{str(proj / 'tools' / 'ask.py')!r},'--warm'],"
             "stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL);"
             f"os.remove({str(lock)!r})"],
            env=dict(os.environ, BUILDER_PROJECT=str(proj)), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            start_new_session=True)
    except OSError:
        pass


def main():
    data = hooklib.hook_input()
    proj = hooklib.project()
    warn(data, proj)
    warm(data, proj)
    return 0


if __name__ == "__main__":
    sys.exit(main())
