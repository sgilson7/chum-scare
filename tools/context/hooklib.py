"""Shared by the Claude Code hooks in this folder: find the project, read the
hook's input, log to a trial harness, and answer the agent.

A hook runs from the folder the session started in. That is the project
itself, or (in a trial, or the folder flow) the folder above it, where the
project is the newest folder holding a STATE.md.
"""

import json
import os
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parents[2]


def project():
    env = os.environ.get("BUILDER_PROJECT")
    if env and Path(env, "STATE.md").exists():
        return Path(env).resolve()
    start = Path(os.environ.get("CLAUDE_PROJECT_DIR") or Path.cwd())
    if (start / "STATE.md").exists() and (start / "tools" / "ask.py").exists():
        return start.resolve()
    found = [p.parent for p in start.glob("*/STATE.md")
             if (p.parent / "tools" / "ask.py").exists() and p.parent.name not in ("builder", "template")]
    if found:
        return max(found, key=lambda p: (p / "STATE.md").stat().st_mtime).resolve()
    return HERE if (HERE / "STATE.md").exists() else None


def hook_input():
    try:
        return json.load(sys.stdin)
    except ValueError:
        return {}


def memory_on():
    return os.environ.get("BUILDER_MEMORY", "on").lower() != "off"


def log(event, **fields):
    """One line in the trial harness's log, when a trial set BUILDER_HARNESS_LOG."""
    path = os.environ.get("BUILDER_HARNESS_LOG")
    if not path:
        return
    try:
        with open(path, "a") as f:
            f.write(json.dumps({"event": event, "at": time.time(), **fields}) + "\n")
    except OSError:
        pass


def tell(event_name, text):
    """Put text in front of the agent (Claude Code's additionalContext)."""
    print(json.dumps({"hookSpecificOutput": {"hookEventName": event_name, "additionalContext": text}}))


def last_context(transcript):
    """The context size of the newest model call in a transcript, from its tail."""
    try:
        with open(transcript, "rb") as f:
            f.seek(0, 2)
            size = f.tell()
            f.seek(max(0, size - 400_000))
            tail = f.read().decode(errors="replace").splitlines()
    except (OSError, TypeError):
        return None
    for line in reversed(tail):
        if '"usage"' not in line:
            continue
        try:
            u = json.loads(line)["message"]["usage"]
        except (ValueError, KeyError, TypeError):
            continue
        return (u.get("input_tokens") or 0) + (u.get("cache_read_input_tokens") or 0) + (u.get("cache_creation_input_tokens") or 0)
    return None
