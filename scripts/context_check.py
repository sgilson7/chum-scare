#!/usr/bin/env python3
"""Check the files that let an agent carry less context (docs/context.md).

- the cards: every spec row and CLAUDE.md rule has one, none is oversized, and,
  where Builder is on disk, the start pack's cards exist within its budget
  (tools/ask.py --check);
- STATE.md: at most MAX characters (about two pages); once it names a commit,
  the commit exists and is at most BEHIND commits older than HEAD; every card
  it cites exists;
- STATE.md and HANDOFF.md kept current: neither was last rewritten before the
  newest commit that changed TRIAGE.md, since a person's request is new work;
  HANDOFF.md is judged once its 'Commit:' line names a commit;
- every digest BRIEF.md names under References (a path ending in .md) has the
  sections a digest needs, fits on a page, and has had its "What to copy"
  section filled in (tools/digest.py --check).

What a pass establishes: the files are in form. Whether STATE.md says the
right things is for the person and the next session to judge.

    python3 scripts/context_check.py
"""

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import ask  # noqa: E402
import digest  # noqa: E402

MAX = 7000
BEHIND = 40


def git(*args):
    return subprocess.run(["git", "-C", str(ROOT), *args], capture_output=True, text=True)


def state_problems(text):
    out = []
    if len(text) > MAX:
        out.append(f"STATE.md is {len(text)} characters; keep it within {MAX} (about two pages)")
    m = re.search(r"^Commit:\s*(\S+)", text, re.M)
    if not m:
        out.append("STATE.md has no 'Commit:' line naming the commit it describes")
    elif not m.group(1).startswith("("):
        sha = m.group(1).strip("`")
        if git("rev-parse", "--git-dir").returncode == 0:
            if git("cat-file", "-e", f"{sha}^{{commit}}").returncode != 0:
                out.append(f"STATE.md names commit {sha}, which this repository does not have")
            else:
                n = git("rev-list", "--count", f"{sha}..HEAD").stdout.strip()
                if n.isdigit() and int(n) > BEHIND:
                    out.append(f"STATE.md describes {sha}, {n} commits ago; rewrite it at each milestone (at most {BEHIND} behind)")
    for cid in ask.unknown_citations(text):
        out.append(f"STATE.md cites card:{cid}, which does not exist (python3 tools/ask.py --ids \"...\")")
    return out


def last_commit(name):
    return git("log", "-1", "--format=%h", "--", name).stdout.strip()


def freshness_problems():
    """A request recorded in TRIAGE.md after STATE.md (or HANDOFF.md) was last rewritten."""
    out = []
    if git("rev-parse", "--git-dir").returncode != 0:
        return out
    t = last_commit("TRIAGE.md")
    if not t:
        return out
    for name in ("STATE.md", "HANDOFF.md"):
        path = ROOT / name
        if not path.exists():
            continue
        if name == "HANDOFF.md":
            m = re.search(r"^Commit:\s*(\S+)", path.read_text(), re.M)
            if not m or m.group(1).startswith("("):
                continue
        s = last_commit(name)
        if s and git("merge-base", "--is-ancestor", t, s).returncode != 0:
            out.append(f"TRIAGE.md changed in {t}, after {name} was last rewritten ({s}); "
                       f"rewrite {name} for the new request (rule 21)")
    return out


def digest_paths(brief):
    out = []
    for line in brief.splitlines():
        for m in re.finditer(r"\(([^)\s]+\.md)\)", line):
            if line.lstrip().startswith("- **"):
                out.append(m.group(1))
    return out


def main():
    problems = []
    cards, _ = ask.load()
    found, size = ask.check(cards)
    problems += found
    state = ROOT / "STATE.md"
    if not state.exists():
        problems.append("STATE.md is missing; copy it from Builder's template/STATE.md")
    else:
        problems += state_problems(state.read_text())
        problems += freshness_problems()
    brief = (ROOT / "BRIEF.md").read_text() if (ROOT / "BRIEF.md").exists() else ""
    for rel in digest_paths(brief):
        path = (ROOT / rel).resolve()
        if not path.exists():
            problems.append(f"BRIEF.md names the digest {rel}, which is not on disk")
            continue
        problems += [f"{rel}: {p}" for p in digest.problems(path.read_text())]
    for p in problems:
        print(f"FAIL {p}")
    if problems:
        return 1
    where = "Builder on disk" if ask.builder_home() else "this project's own copies"
    print(f"context: {len(cards)} cards from {where}, STATE.md in form, digests in form")
    return 0


if __name__ == "__main__":
    sys.exit(main())
