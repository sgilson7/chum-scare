#!/usr/bin/env python3
"""Measure how many tokens this project's agent sessions re-read, and log it.

Every model call re-reads the whole context, so what a session costs is the
context size times the number of calls. This reads a Claude Code session
transcript (one JSON object per line) and reports, per session:
the context at the first plan or journal write, the peak and the mean context,
the number of compactions, the total re-read from cache, and the share of it
that was context above 200k tokens. It adds one row to notes/tokens.md.

    python3 tools/tokens.py                 the newest session for this folder or the one above it
    python3 tools/tokens.py SESSION.jsonl   that session
    python3 tools/tokens.py --print ...     print the row; do not log it
    python3 tools/tokens.py --json ...      print every number as JSON

Other agents keep other transcripts; for them, log the numbers by hand in the
same table. The harness in the reference repository (harness/) uses this file.
"""

import json
import re
import sys
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CAP = 200_000
PLAN = re.compile(r"(-journal/|/PLAN\.md$|/BRIEF\.md$)")


def project_dir(path):
    return Path.home() / ".claude" / "projects" / re.sub(r"[^A-Za-z0-9]", "-", str(Path(path).resolve()))


def newest_session(*folders):
    found = []
    for f in folders:
        d = project_dir(f)
        if d.is_dir():
            found += [p for p in d.glob("*.jsonl")]
    return max(found, key=lambda p: p.stat().st_mtime) if found else None


def calls(path):
    """[(usage, tool_uses)] per model call, in order; a call split over several lines is merged."""
    out, index = [], {}
    for line in open(path, errors="replace"):
        try:
            d = json.loads(line)
        except ValueError:
            continue
        if d.get("type") != "assistant":
            continue
        m = d.get("message") or {}
        mid = m.get("id") or id(m)
        uses = [c for c in (m.get("content") or []) if isinstance(c, dict) and c.get("type") == "tool_use"]
        if mid in index:
            out[index[mid]][1].extend(uses)
        else:
            index[mid] = len(out)
            out.append((m.get("usage") or {}, uses))
    return out


def context(u):
    return (u.get("input_tokens") or 0) + (u.get("cache_read_input_tokens") or 0) + (u.get("cache_creation_input_tokens") or 0)


def measure(path):
    cs = calls(path)
    if not cs:
        return None
    ctx = [context(u) for u, _ in cs]
    reread = [u.get("cache_read_input_tokens") or 0 for u, _ in cs]
    plan_at = None
    for i, (_, uses) in enumerate(cs):
        for t in uses:
            fp = str((t.get("input") or {}).get("file_path", ""))
            if t.get("name") in ("Write", "Edit") and PLAN.search(fp) and not fp.endswith("BRIEF.md"):
                plan_at = i + 1
                break
        if plan_at:
            break
    drops = [i for i in range(1, len(ctx)) if ctx[i] < 0.5 * ctx[i - 1] and ctx[i - 1] > 60_000]
    first_drop = drops[0] if drops else len(ctx)
    total_ctx = sum(ctx) or 1
    climb = (ctx[plan_at - 1] - ctx[0]) if plan_at else None
    startup = (sum(reread[:plan_at]) + climb * max(0, first_drop - plan_at)) if plan_at else None
    return {
        "session": Path(path).stem,
        "calls": len(ctx),
        "base": ctx[0],
        "plan_at": plan_at,
        "at_plan": ctx[plan_at - 1] if plan_at else None,
        "climb": climb,
        "peak": max(ctx),
        "mean": sum(ctx) // len(ctx),
        "compactions": len(drops),
        "compaction_calls": drops,
        "reread": sum(reread),
        "startup_reread": startup,
        "startup_share": (startup / sum(reread)) if startup and sum(reread) else None,
        "above_cap_share": sum(max(0, c - CAP) for c in ctx) / total_ctx,
        "ctx": ctx,
    }


def k(n):
    return "-" if n is None else (f"{n / 1e6:.1f}M" if n >= 1e6 else f"{round(n / 1000)}k")


HEAD = ("# Token use\n\nOne row per measured session, newest last (`python3 tools/tokens.py`). "
        "Each call re-reads the whole context, so re-read ≈ mean context × calls.\n\n"
        "| When | Session | Calls | Context at first plan | Peak | Mean | Compactions | Re-read | Above 200k |\n"
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- |\n")


def row(m):
    at = "" if m["plan_at"] is None else " (call %d)" % m["plan_at"]
    return (f"| {datetime.now():%Y-%m-%d %H:%M} | {m['session'][:8]} | {m['calls']} | "
            f"{k(m['at_plan'])}{at} | {k(m['peak'])} | {k(m['mean'])} | "
            f"{m['compactions']} | {k(m['reread'])} | {m['above_cap_share']:.0%} |")


def main(argv):
    args = [a for a in argv if not a.startswith("--")]
    path = Path(args[0]) if args else newest_session(ROOT, ROOT.parent)
    if not path or not path.exists():
        print("no session transcript found for this folder; pass one: python3 tools/tokens.py SESSION.jsonl")
        return 1
    m = measure(path)
    if not m:
        print(f"{path} holds no model calls")
        return 1
    if "--json" in argv:
        print(json.dumps({k_: v for k_, v in m.items() if k_ != "ctx"}, indent=2))
        return 0
    line = row(m)
    print(HEAD.splitlines()[-2])
    print(line)
    if "--print" not in argv:
        log = ROOT / "notes" / "tokens.md"
        log.parent.mkdir(parents=True, exist_ok=True)
        if not log.exists():
            log.write_text(HEAD)
        with log.open("a") as f:
            f.write(line + "\n")
        print(f"logged in {log.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
