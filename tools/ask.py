#!/usr/bin/env python3
"""Ask Builder a question and get the few cards that answer it, instead of reading its docs.

Builder's Markdown (the spec, docs/, prompts/, kit READMEs, CLAUDE.md, the
journal's ENTRIES.md, and any game digests) is cut into cards: one per
section, one per spec row (spec.D20), one per CLAUDE.md rule (claude.r14),
long sections split into parts. Its code is cut too: a card per Python or
JavaScript function and class, per Rust item and test (code.core.tests.boundary.
core_source_has_no_io_and_no_nondeterminism), and per script header, each
naming path:line so only those lines need opening. A card is cut from the files each time this
runs, so it is never stale. Its id is stable while the heading it comes from
is, and journal entries and STATE.md cite cards as card:<id>.

    python3 tools/ask.py "how do saves load an old version"    the top 3 cards
    python3 tools/ask.py -k 5 "..."                             the top 5
    python3 tools/ask.py --ids "..."                            ids and titles only
    python3 tools/ask.py --card spec.D20 claude.r14             those cards in full
    python3 tools/ask.py --code "..."  /  --docs "..."           only code cards / only document cards
    python3 tools/ask.py --start                                the start pack, for the opening entry
    python3 tools/ask.py --list                                 every card id and its size
    python3 tools/ask.py --check                                cards, start pack and spec rows in order
    python3 tools/ask.py --qa tests/builderqa.json              a question set: each answer in the top 5

Your own work (the work index):
    python3 tools/ask.py --mine "where does the deck get its counts"
        searches this project instead of Builder: journal entries, SECOND-ORDER.md
        and TRIAGE.md rows, STATE.md, PLAN.md, BRIEF.md and HANDOFF.md sections,
        commits, every function in the project's code, the summaries of long runs
        in notes/runs/, and the tool results this project's sessions have seen.
        Cards are cut fresh each run, as Builder's are. Cite them as card:mine.<id>.
    python3 tools/ask.py --target "how is a submission saved"
        searches the codebase named in notes/target (one line, a path) or
        $BUILDER_TARGET, for a project that explains an existing codebase.
    python3 tools/ask.py --all "..."                             Builder and your own work together
    python3 tools/ask.py --warm                                  embed any new cards now (a hook runs this)
BUILDER_MEMORY=off turns --mine, --target and embeddings off (a trial's control arm).

Where Builder is: $BUILDER_HOME, else the repository this file sits in
(template/ or examples/minimal/), else ../builder/ beside this project. Without
it, the cards come from this project's own copies (CLAUDE.md, BUILDER-SPEC.md,
ENTRIES.md, STATE.md). Game digests come from $BUILDER_REFS or ../refs/.

Ranking: keyword ranking (BM25), standard library only. When the optional local
embedding model is installed (./scripts/setup-embed.sh), cosine similarity over
card embeddings is added, and the two rankings are merged by reciprocal rank
fusion: each card scores 1/(60 + rank) in the keyword list and half that in the
cosine list (chosen on BuilderQA and a WorkQA set; see docs/context.md). Vectors are kept in
notes/index/ by a hash of each card's text, so a changed card is embedded again
and an unchanged one never is. --lexical uses BM25 alone.
"""

import hashlib
import json
import math
import os
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PROJECT = Path(os.environ.get("BUILDER_PROJECT") or ROOT).resolve()
EMBED_MODEL = os.environ.get("BUILDER_EMBED_MODEL", "fastembed:BAAI/bge-small-en-v1.5")
EMBED_VENV = Path.home() / ".cache" / "builder" / "embed-venv" / "bin" / "python"
RRF_K = int(os.environ.get("BUILDER_RRF_K", "60"))
SEM_WEIGHT = float(os.environ.get("BUILDER_SEM_WEIGHT", "0.5"))  # the cosine list counts this much against BM25's 1.0


def memory_on():
    return os.environ.get("BUILDER_MEMORY", "on").lower() != "off"
LIMIT = 1800          # characters per card part, about 450 tokens
START_BUDGET = 3500   # tokens the start pack may take
CITE = re.compile(r"card:([a-z0-9][a-z0-9._~-]*[a-z0-9_])")

# The start pack: what an opening entry needs that CLAUDE.md does not already say.
# The agent reads CLAUDE.md in full anyway, so its rules are not repeated here.
START = [
    "docs.context.how-to-read-builder",
    "docs.procedure", "docs.procedure.p2", "docs.procedure.p3",
    "docs.journal.the-opening-entry",
    "entries.kinds", "entries.kinds.p2", "entries.kinds.p3",
    "kits.readme.the-kits",
]

STOP = set("""a an the and or of to in on for with by is are be it this that as at from how what when where which
who why do does i you we my your our can should into not no its their there them then than so if else
about use using used get put make makes made one each all any exist exists""".split())


def stem(w):
    for suf, rep in (("ies", "y"), ("ing", ""), ("ed", ""), ("es", ""), ("s", "")):
        if len(w) > 4 and w.endswith(suf):
            return w[: -len(suf)] + rep
    return w


# Builder's own vocabulary: a question in one word finds a card written in the other.
SAME = {"crate": "dependency", "dependencie": "dependency", "depend": "dependency", "verifi": "check", "verify": "check",
        "test": "check", "multiplayer": "online", "netcode": "online", "network": "online", "save": "saves",
        "persist": "saves", "deploy": "publish", "subagent": "agent", "parallel": "agent", "rule": "rule",
        "picture": "art", "image": "art", "phone": "mobile", "touch": "mobile"}


def words(text):
    out = []
    for w in re.findall(r"[a-z0-9]+", text.lower()):
        if w in STOP or len(w) < 2:
            continue
        w = stem(w)
        out.append(w)
        if w in SAME and SAME[w] != w:
            out.append(SAME[w])
    return out


CODE_WORDS = {"test", "function", "fn", "script", "code", "py", "rs", "js", "struct", "const", "scanner", "def", "implement"}


def slug(text):
    s = re.sub(r"`([^`]*)`", r"\1", text.lower())
    s = re.sub(r"[^a-z0-9]+", "-", s).strip("-")
    return s[:48].strip("-") or "section"


def builder_home():
    env = os.environ.get("BUILDER_HOME")
    if env and (Path(env) / "BUILDER-SPEC.md").exists():
        return Path(env).resolve()
    # R1.12: this project keeps Builder inside it, as builder/
    for cand in (ROOT.parent, ROOT.parent.parent, ROOT.parent / "builder", ROOT / "builder"):
        if (cand / "BUILDER-SPEC.md").exists() and (cand / "template").is_dir() and (cand / "docs").is_dir():
            return cand.resolve()
    return None


def refs_home():
    env = os.environ.get("BUILDER_REFS")
    if env and Path(env).is_dir():
        return Path(env).resolve()
    cand = ROOT.parent / "refs"
    return cand.resolve() if cand.is_dir() else None


def sources():
    """(key, path) for every file the cards come from."""
    out = []
    home = builder_home()
    if home:
        out += [("readme", home / "README.md"), ("spec", home / "BUILDER-SPEC.md")]
        out += [(f"docs.{p.stem}", p) for p in sorted((home / "docs").glob("*.md"))]
        out += [(f"prompts.{p.stem}", p) for p in sorted((home / "prompts").glob("*.md"))]
        out += [("kits.readme", home / "kits" / "README.md")]
        out += [(f"kits.{p.parent.name}", p) for p in sorted((home / "kits").glob("*/README.md"))]
        t = home / "template"
    else:
        t = ROOT
        out += [("spec", t / "BUILDER-SPEC.md")]
    out += [("claude", t / "CLAUDE.md"), ("brief", t / "BRIEF.md"), ("entries", t / "tools" / "journal" / "ENTRIES.md"),
            ("state", t / "STATE.md"), ("glossary", t / "GLOSSARY.md"), ("tone", t / "TONE.md")]
    out += [(f"skill.{p.parent.name}", p) for p in sorted((t / ".claude" / "skills").glob("*/SKILL.md"))]
    refs = refs_home()
    if refs:
        out += [(f"ref.{slug(p.stem)}", p) for p in sorted(refs.glob("*.md"))]
    return [(k, p) for k, p in out if p.exists()]


def split_long(text):
    """Paragraph groups of at most LIMIT characters; a longer paragraph is cut by lines."""
    parts, cur = [], ""
    for para in re.split(r"\n\s*\n", text):
        pieces = [para] if len(para) <= LIMIT else _lines(para)
        for piece in pieces:
            if cur and len(cur) + len(piece) + 2 > LIMIT:
                parts.append(cur)
                cur = piece
            else:
                cur = f"{cur}\n\n{piece}" if cur else piece
    if cur.strip():
        parts.append(cur)
    return parts or [""]


def _lines(para):
    out, cur = [], ""
    for line in para.splitlines():
        if cur and len(cur) + len(line) + 1 > LIMIT:
            out.append(cur)
            cur = line
        else:
            cur = f"{cur}\n{line}" if cur else line
    return out + ([cur] if cur else [])


ROW = re.compile(r"^\|\s*([RD][1-9]\d*)\s*\|")
RULE = re.compile(r"^([1-9]\d*)\.\s+")


def cut(key, path):
    """The cards of one file."""
    rel = path.as_posix()
    lines = path.read_text().splitlines()
    sections, trail, cur = [], [], {"level": 0, "title": "", "body": []}
    in_fence = False
    for line in lines:
        if line.lstrip().startswith("```"):
            in_fence = not in_fence
        m = None if in_fence else re.match(r"^(#{1,4})\s+(.*)", line)
        if m:
            sections.append(cur)
            level, title = len(m.group(1)), m.group(2).strip()
            trail = [t for t in trail if t[0] < level] + [(level, title)]
            cur = {"level": level, "title": title, "trail": [t for _, t in trail], "body": []}
        else:
            cur["body"].append(line)
    sections.append(cur)

    cards, used = [], set()

    def add(cid, title, text, where):
        base, n = cid, 2
        while cid in used:
            cid = f"{base}-{n}"
            n += 1
        used.add(cid)
        cards.append({"id": cid, "title": title, "text": text.strip(), "file": where})

    for sec in sections:
        body = sec["body"]
        if not sec["title"] and not "".join(body).strip():
            continue
        trail = sec.get("trail", [])
        title = " › ".join(trail) or path.name
        cid = key if sec["level"] <= 1 else f"{key}.{slug(sec['title'])}"
        if sec["level"] > 1 and f"{key}.{slug(sec['title'])}" in used and len(trail) > 1:
            cid = f"{key}.{slug(trail[-2])}-{slug(sec['title'])}"
        rest = []
        for line in body:
            row = ROW.match(line) if key == "spec" else None
            rule = RULE.match(line) if key == "claude" else None
            if row:
                cells = [c.strip() for c in line.strip().strip("|").split("|")]
                add(f"spec.{row.group(1)}", f"{row.group(1)}: {cells[1][:70]}", " | ".join(cells), rel)
            elif rule:
                add(f"claude.r{rule.group(1)}", f"Rule {rule.group(1)}: {line[rule.end():][:70]}", line, rel)
            else:
                rest.append(line)
        text = "\n".join(rest).strip()
        if not text and sec["level"] > 1:
            text = "(this section's rows and rules are cards of their own)"
        parts = split_long(text)
        for i, part in enumerate(parts):
            add(cid if i == 0 else f"{cid}.p{i + 1}", title + ("" if i == 0 else f" (part {i + 1})"), part, rel)
    return cards


CODE_SKIP = {"target", "dist", ".venv", "node_modules", "vendor", "__pycache__", "mutants.out", ".git", "notes"}
CODE_EXT = {".py", ".rs", ".sh", ".js"}
TARGET_EXT = CODE_EXT | {".ts", ".tsx", ".jsx", ".mjs"}


def walk_code(base, prefix, rel_to, exts=CODE_EXT, out=None, top="code."):
    """(key, path, shown path) for the code files under base."""
    out = [] if out is None else out
    if not base.is_dir():
        return out
    seen = {k for k, _, _ in out}
    for p in sorted(base.rglob("*"), key=lambda p: (str(p.parent), p.suffix != ".py", p.name)):
        if p.suffix not in exts or not p.is_file() or CODE_SKIP & set(p.relative_to(base).parts):
            continue
        rel = p.relative_to(base)
        parts = [x for x in rel.with_suffix("").parts if x not in ("crates", "src")]
        key = top + prefix + ".".join(slug(x).replace("-", "_") for x in parts)
        if key in seen:
            key += "_" + p.suffix.lstrip(".")
        seen.add(key)
        out.append((key, p, p.relative_to(rel_to).as_posix()))
    return out


def code_sources():
    """(key, path, shown path) for every Builder code file the cards come from."""
    home = builder_home()
    roots = [(home / "template", "", home)] if home else [(ROOT, "", ROOT)]
    if home:
        roots += [(home / "kits", "kits.", home), (home / "scripts", "repo.scripts.", home)]
    out = []
    for base, prefix, rel_to in roots:
        walk_code(base, prefix, rel_to, out=out)
    return out


ITEM = {
    ".py": re.compile(r"^(\s*)(?:async\s+)?(def|class)\s+([A-Za-z_]\w*)"),
    ".rs": re.compile(r"^(\s*)(?:pub(?:\([^)]*\))?\s+)?(fn|struct|enum|trait|const|static|mod|type)\s+([A-Za-z_]\w*)"),
    ".js": re.compile(r"^(\s*)(?:export\s+)?(?:async\s+)?(function|class)\s+([A-Za-z_$][\w$]*)|^(\s*)(?:export\s+)?const\s+([A-Za-z_$][\w$]*)\s*=\s*(?:async\s*)?\("),
    ".sh": re.compile(r"^(\s*)()([A-Za-z_][\w-]*)\s*\(\)\s*\{"),
}
COMMENT = {".py": "#", ".rs": "//", ".js": "//", ".sh": "#"}


def cut_code(key, path, shown):
    lines = path.read_text(errors="replace").splitlines()
    ext = {".ts": ".js", ".tsx": ".js", ".jsx": ".js", ".mjs": ".js"}.get(path.suffix, path.suffix)
    cards, used = [], set()

    def add(cid, title, text, line):
        base, n = cid, 2
        while cid in used:
            cid = f"{base}-{n}"
            n += 1
        used.add(cid)
        if len(text) > LIMIT:
            cut_at = text.rfind("\n", 0, LIMIT)
            more = text[cut_at:].count("\n")
            text = text[:cut_at] + f"\n… ({more} more lines: open {shown} from line {line})"
        cards.append({"id": cid, "title": title, "text": text.strip(), "file": f"{shown}:{line}"})

    # the file's own header: a docstring, //! or # comments, and a script's step names
    head = []
    if ext == ".py":
        m = re.match(r'(?:#![^\n]*\n)?\s*("""|\'\'\')(.*?)\1', "\n".join(lines), re.S)
        if m:
            head = m.group(2).strip().splitlines()
    else:
        mark = "//!" if ext == ".rs" else COMMENT[ext]
        for line in lines[1 if lines and lines[0].startswith("#!") else 0:]:
            if line.strip().startswith(mark) or (ext == ".rs" and line.strip().startswith("//")):
                head.append(line.strip().lstrip("/!#").strip())
            elif line.strip():
                break
    if ext == ".sh":
        head += [m.group(1) for m in (re.search(r'step\s+"([^"]+)"', l) for l in lines) if m]
    if head or ext in (".sh",):
        add(key, f"{shown} (the file)", "\n".join(head) or "(no header comment)", 1)

    starts = []
    rx = ITEM[ext]
    for i, line in enumerate(lines):
        m = rx.match(line)
        if not m:
            continue
        if ext == ".js" and m.group(5):
            indent, kind, name = m.group(4), "function", m.group(5)
        else:
            indent, kind, name = m.group(1), m.group(2) or "function", m.group(3)
        if ext == ".rs" and kind == "mod" and line.rstrip().endswith(";"):
            continue
        starts.append((i, len(indent or ""), kind, name))
    for n, (i, indent, kind, name) in enumerate(starts):
        end = len(lines)
        for j, ind, _, _ in starts[n + 1:]:
            if ind <= indent:
                end = j
                break
        # doc comments and attributes just above the item
        top = i
        while top > 0 and re.match(r"\s*(///|#\[|#\s|@)", lines[top - 1]) and not lines[top - 1].strip().startswith("#!"):
            top -= 1
        test = any("#[test]" in l for l in lines[top:i])
        body = "\n".join(lines[top:end]).rstrip()
        label = "test" if test else kind
        add(f"{key}.{name}", f"{shown}: {label} {name}", body, top + 1)
    return cards


def load():
    cards, keys = [], set()
    for key, path in sources():
        keys.add(key)
        for c in cut(key, path):
            c["path"] = path
            cards.append(c)
    for key, path, shown in code_sources():
        keys.add(key)
        for c in cut_code(key, path, shown):
            c["path"] = path
            cards.append(c)
    return cards, keys


# ---------------------------------------------------------------- the work index

def html_text(s):
    s = re.sub(r"<(script|style|svg)\b.*?</\1>", " ", s, flags=re.S | re.I)
    s = re.sub(r"<[^>]+>", " ", s)
    for a, b in (("&amp;", "&"), ("&lt;", "<"), ("&gt;", ">"), ("&quot;", '"'), ("&#39;", "'"), ("&nbsp;", " ")):
        s = s.replace(a, b)
    return re.sub(r"[ \t]*\n\s*", "\n", re.sub(r"[ \t]+", " ", s)).strip()


def journal_dir():
    env = os.environ.get("BUILDER_JOURNAL")
    if env and Path(env, "index.html").exists():
        return Path(env)
    cand = PROJECT.parent / f"{PROJECT.name}-journal"
    return cand if (cand / "index.html").exists() else None


def session_files():
    """Claude Code transcripts for this project and for the folder it sits in."""
    out = []
    for folder in (PROJECT, PROJECT.parent):
        d = Path.home() / ".claude" / "projects" / re.sub(r"[^A-Za-z0-9]", "-", str(folder))
        if d.is_dir():
            out += sorted(d.glob("*.jsonl"), key=lambda p: p.stat().st_mtime)
    return out


def seen_cards(limit_chars=300):
    """One card per tool result a session of this project has seen (EM-Q2 B)."""
    cards = []
    for f in session_files():
        sid = f.stem[:6]
        calls, n = {}, 0
        try:
            lines = f.read_text(errors="replace").splitlines()
        except OSError:
            continue
        for line in lines:
            try:
                r = json.loads(line)
            except ValueError:
                continue
            content = (r.get("message") or {}).get("content")
            if not isinstance(content, list):
                continue
            for b in content:
                if not isinstance(b, dict):
                    continue
                if b.get("type") == "tool_use":
                    i = b.get("input") or {}
                    what = i.get("command") or i.get("file_path") or i.get("pattern") or i.get("url") or json.dumps(i)[:120]
                    calls[b.get("id")] = (b.get("name", "tool"), str(what))
                elif b.get("type") == "tool_result":
                    name, what = calls.get(b.get("tool_use_id"), ("tool", ""))
                    c = b.get("content")
                    text = "".join(x.get("text", "") for x in c if isinstance(x, dict)) if isinstance(c, list) else str(c or "")
                    if len(text) < limit_chars or "ask.py" in what or name in ("Write", "Edit"):
                        continue
                    n += 1
                    more = len(text) - LIMIT
                    body = text[:LIMIT] + (f"\n… ({more} more characters; open the file or re-run the command for them)" if more > 0 else "")
                    first = what.strip().splitlines()[0][:90] if what.strip() else ""
                    cards.append({"id": f"mine.seen.{sid}.{n}", "title": f"seen: {name} {first}",
                                  "text": body, "file": f"session {f.stem[:8]}, tool result {n}", "path": f})
    return cards


def mine_cards():
    """The work index: this project's own records, code, runs and what its sessions have seen."""
    cards = []
    for name in ("STATE", "PLAN", "BRIEF", "HANDOFF", "DECISIONS", "MEASUREMENTS"):
        p = PROJECT / f"{name}.md"
        if p.exists():
            for c in cut(f"mine.{name.lower()}", p):
                c["path"] = p
                cards.append(c)
    for name, prefix in (("SECOND-ORDER.md", "notebook"), ("TRIAGE.md", "triage")):
        p = PROJECT / name
        if not p.exists():
            continue
        for i, line in enumerate(p.read_text().splitlines()):
            m = re.match(r"^\|\s*([RT]\d+(?:\.\d+)?)\s*\|(.*)", line)
            if m and m.group(2).replace("|", "").strip():
                rid = m.group(1)
                cells = [x.strip() for x in m.group(2).strip().strip("|").split("|")]
                cards.append({"id": f"mine.{prefix}.{rid.lower().replace('.', '-')}", "title": f"{name} {rid}: {cells[0][:60]}",
                              "text": f"{rid} | " + " | ".join(cells), "file": f"{name}:{i + 1}", "path": p})
    j = journal_dir()
    if j:
        html = (j / "index.html").read_text(errors="replace")
        for m in re.finditer(r'<article\b[^>]*\bid="([^"]+)"[^>]*>(.*?)</article>', html, re.S):
            title = re.search(r"<h3[^>]*>(.*?)</h3>", m.group(2), re.S)
            title = html_text(title.group(1)) if title else m.group(1)
            for i, part in enumerate(split_long(html_text(m.group(2)))):
                cid = f"mine.journal.{slug(m.group(1))}" + ("" if i == 0 else f".p{i + 1}")
                cards.append({"id": cid, "title": f"journal: {title}" + ("" if i == 0 else f" (part {i + 1})"),
                              "text": part, "file": f"{j.name}/index.html#{m.group(1)}", "path": j / "index.html"})
    if (PROJECT / ".git").exists():
        try:
            log = subprocess.run(["git", "-C", str(PROJECT), "log", "-n", "300", "--date=short",
                                  "--format=@@%h|%ad|%s", "--name-only"], capture_output=True, text=True, timeout=20).stdout
        except (OSError, subprocess.SubprocessError):
            log = ""
        for block in log.split("@@")[1:]:
            head, *files = [x for x in block.splitlines() if x.strip()]
            h, date, subject = (head.split("|", 2) + ["", ""])[:3]
            cards.append({"id": f"mine.commit.{h}", "title": f"commit {h} {date}: {subject[:70]}",
                          "text": f"{subject}\nfiles: " + ", ".join(files[:40]), "file": f"git show {h}", "path": PROJECT})
    home = builder_home()
    for key, path, shown in walk_code(PROJECT, "", PROJECT, top="mine.code."):
        if home:   # a file still as Builder's template has it is Builder's, not this project's work
            orig = home / "template" / shown
            try:
                if orig.is_file() and orig.read_bytes() == path.read_bytes():
                    continue
            except OSError:
                pass
        for c in cut_code(key, path, shown):
            c["path"] = path
            cards.append(c)
    runs = PROJECT / "notes" / "runs"
    for p in sorted(runs.glob("*.log"))[-60:] if runs.is_dir() else []:
        lines = re.sub(r"\x1b\[[0-9;]*m", "", p.read_text(errors="replace")).splitlines()
        errs = [l for l in lines if re.search(r"FAIL|error|panicked|failed|assert", l)][:12]
        steps = [l for l in lines if l.startswith("== ")][-3:]
        text = "\n".join(steps + errs + ["…"] + [l for l in lines if l.strip()][-8:])
        cards.append({"id": f"mine.run.{slug(p.stem)}", "title": f"run log {p.name}", "text": text[:LIMIT],
                      "file": f"notes/runs/{p.name}", "path": p})
    cards += seen_cards()
    # ids stay unique, as Builder's do
    used = Counter()
    for c in cards:
        used[c["id"]] += 1
        if used[c["id"]] > 1:
            c["id"] += f"-{used[c['id']]}"
    return cards


def target_dir():
    env = os.environ.get("BUILDER_TARGET")
    if env and Path(env).is_dir():
        return Path(env).resolve()
    f = PROJECT / "notes" / "target"
    if f.exists():
        p = Path(f.read_text().strip()).expanduser()
        p = p if p.is_absolute() else (PROJECT / p)
        if p.is_dir():
            return p.resolve()
    return None


def target_cards():
    """Cards of the codebase this project explains, cut the way Builder's code is."""
    base = target_dir()
    if not base:
        return []
    cards = []
    for p in sorted(base.glob("*.md")) + sorted((base / "docs").glob("**/*.md")):
        for c in cut(f"target.doc.{slug(p.relative_to(base).with_suffix('').as_posix())}", p):
            c["path"] = p
            cards.append(c)
    for key, path, shown in walk_code(base, "", base, exts=TARGET_EXT, top="target."):
        try:
            for c in cut_code(key, path, shown):
                c["path"] = path
                cards.append(c)
        except (OSError, UnicodeError):
            continue
    return cards


# ---------------------------------------------------------------- embeddings

def embed_python():
    if os.environ.get("BUILDER_EMBED", "on").lower() == "off" or not memory_on():
        return None
    env = os.environ.get("BUILDER_EMBED_PYTHON")
    if env and Path(env).exists():
        return env
    return str(EMBED_VENV) if EMBED_VENV.exists() else None


def card_hash(c):
    return hashlib.sha1((EMBED_MODEL + "\n" + c["title"] + "\n" + c["text"]).encode()).hexdigest()[:20]


def embed_text(c):
    return (c["title"] + "\n" + c["text"])[:2000]


class Vectors:
    """Card vectors by hash, kept in notes/index/; only new text is embedded."""

    def __init__(self):
        self.py = embed_python()
        self.file = PROJECT / "notes" / "index" / f"vectors-{slug(EMBED_MODEL)}.json"
        self.cache = {}
        if self.py and self.file.exists():
            try:
                self.cache = json.loads(self.file.read_text())
            except ValueError:
                self.cache = {}
        self.dirty = False

    def ok(self):
        return bool(self.py)

    def _run(self, texts, mode="doc"):
        script = Path(__file__).resolve().parent / "context" / "embed.py"
        env = dict(os.environ, HF_HUB_OFFLINE=os.environ.get("HF_HUB_OFFLINE", "1"), TOKENIZERS_PARALLELISM="false")
        r = subprocess.run([self.py, str(script), EMBED_MODEL, mode], input=json.dumps(texts), capture_output=True,
                           text=True, env=env, timeout=600)
        if r.returncode != 0:
            raise RuntimeError(r.stderr.strip().splitlines()[-1] if r.stderr.strip() else "embedding failed")
        return json.loads(r.stdout)

    def for_cards(self, cards):
        todo = [c for c in cards if card_hash(c) not in self.cache]
        if todo:
            vecs = self._run([embed_text(c) for c in todo])
            for c, v in zip(todo, vecs):
                self.cache[card_hash(c)] = v
            self.dirty = True
        return [self.cache[card_hash(c)] for c in cards]

    def query(self, q):
        return self._run([q], "query")[0]

    def save(self, keep=None):
        if not self.dirty:
            return
        if keep is not None:
            self.cache = {h: v for h, v in self.cache.items() if h in keep}
        self.file.parent.mkdir(parents=True, exist_ok=True)
        self.file.write_text(json.dumps(self.cache, separators=(",", ":")))


class Index:
    def __init__(self, cards, vectors=None):
        self.cards = cards
        self.docs = []
        for c in cards:
            head = words(c["title"] + " " + c["id"].replace(".", " ").replace("-", " "))
            self.docs.append(Counter(head * 3 + words(c["text"])))
        self.avg = sum(sum(d.values()) for d in self.docs) / max(1, len(self.docs))
        df = Counter()
        for d in self.docs:
            df.update(d.keys())
        n = len(self.docs)
        self.idf = {w: math.log(1 + (n - f + 0.5) / (f + 0.5)) for w, f in df.items()}
        self.vectors = vectors if vectors and vectors.ok() else None
        self.vecs = None
        if self.vectors:
            try:
                self.vecs = self.vectors.for_cards(cards)
                self.vectors.save()
            except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as e:
                print(f"(embeddings unavailable, keyword ranking only: {e})", file=sys.stderr)
                self.vectors = None

    def lexical(self, query):
        q = words(query)
        code_q = bool(CODE_WORDS & set(re.findall(r"[a-z]+", query.lower())))
        scored = []
        for i, (c, d) in enumerate(zip(self.cards, self.docs)):
            size = sum(d.values())
            s = 0.0
            for w in q:
                f = d.get(w, 0)
                if f:
                    s += self.idf[w] * f * 2.2 / (f + 1.2 * (0.25 + 0.75 * size / self.avg))
            if s > 0:
                if ".code." in "." + c["id"] and not code_q:
                    s *= 0.8
                scored.append((s, i))
        scored.sort(key=lambda x: -x[0])
        return [i for _, i in scored]

    def semantic(self, query, n=100):
        qv = self.vectors.query(query)
        scored = sorted(((sum(a * b for a, b in zip(qv, v)), i) for i, v in enumerate(self.vecs)), reverse=True)
        return [i for _, i in scored[:n]]

    def search(self, query, k=3, lexical=False):
        lex = self.lexical(query)
        if lexical or not self.vecs:
            return [self.cards[i] for i in lex[:k]]
        try:
            sem = self.semantic(query)
        except (OSError, RuntimeError, ValueError, subprocess.SubprocessError):
            return [self.cards[i] for i in lex[:k]]
        score = Counter()
        for ranking, w in ((lex[:100], 1.0), (sem, SEM_WEIGHT)):
            for r, i in enumerate(ranking):
                score[i] += w / (RRF_K + r + 1)
        return [self.cards[i] for i, _ in score.most_common(k)]


def tokens(text):
    return max(1, len(text) // 4)


def show(c):
    return f"── card:{c['id']}  {c['title']}  ({c['file']}, ~{tokens(c['text'])} tokens)\n{c['text']}\n"


def known_ids(with_mine=False):
    """(ids, keys) for checks that verify card citations; a citation whose key was not loaded is not judged."""
    cards, keys = load()
    ids = {c["id"] for c in cards}
    if with_mine:
        # a citation of your own work is judged too; what a session saw (mine.seen.*) is not kept
        ids |= {c["id"] for c in mine_cards() if not c["id"].startswith("mine.seen.")}
        keys |= {"mine"}
    return ids, keys


def unknown_citations(text):
    cited = sorted(set(CITE.findall(text)))
    ids, keys = known_ids(with_mine=any(c.startswith("mine.") and not c.startswith("mine.seen.") for c in cited))
    bad = []
    for cid in cited:
        key_known = any(cid == k or cid.startswith(k + ".") for k in keys)
        if key_known and cid not in ids:
            bad.append(cid)
    return bad


def check(cards):
    problems = []
    by = {c["id"]: c for c in cards}
    seen = set()
    for c in cards:
        if c["id"] in seen:
            problems.append(f"two cards share the id {c['id']}")
        seen.add(c["id"])
    for c in cards:
        if not c["id"].startswith("code.") and len(c["text"]) > LIMIT * 1.6:
            problems.append(f"card {c['id']} is {len(c['text'])} characters; the limit is {LIMIT}")
    missing = [i for i in START if i not in by]
    problems += [f"the start pack names card {i}, which does not exist" for i in missing]
    size = sum(tokens(by[i]["text"]) for i in START if i in by)
    if size > START_BUDGET:
        problems.append(f"the start pack is ~{size} tokens; the budget is {START_BUDGET}")
    spec = next((c["path"] for c in cards if c["id"].startswith("spec.")), None)
    if spec:
        rows = {m.group(1) for line in spec.read_text().splitlines() for m in [ROW.match(line)] if m}
        problems += [f"spec row {r} has no card" for r in sorted(rows) if f"spec.{r}" not in by]
    claude = next((c["path"] for c in cards if c["id"].startswith("claude.")), None)
    if claude:
        rules = {m.group(1) for line in claude.read_text().splitlines() for m in [RULE.match(line)] if m}
        problems += [f"CLAUDE.md rule {r} has no card" for r in sorted(rules, key=int) if f"claude.r{r}" not in by]
    return problems, size


def qa(index, path, k=5, lexical=False):
    data = json.loads(Path(path).read_text())
    items = data["questions"] if isinstance(data, dict) else data
    ids = {c["id"] for c in index.cards}
    misses, unknown, at = [], [], Counter()
    for it in items:
        want = set(it["cards"])
        unknown += [w for w in want if w not in ids]
        got = [c["id"] for c in index.search(it["q"], k, lexical=lexical)]
        hit = next((i for i, g in enumerate(got) if g in want), None)
        if hit is None:
            misses.append((it["q"], sorted(want), got[:3]))
        else:
            for n in (1, 3, 5):
                if hit < n:
                    at[n] += 1
    return items, misses, sorted(set(unknown)), at


OFF = ("The work index is off in this project (BUILDER_MEMORY=off). Ask Builder with "
       "python3 tools/ask.py \"question\", and read your own files as usual.")


def main(argv):
    k = 3
    if "-k" in argv:
        i = argv.index("-k")
        k = int(argv[i + 1])
        del argv[i:i + 2]
    lexical = "--lexical" in argv
    mine, target, both = "--mine" in argv, "--target" in argv, "--all" in argv
    if (mine or target or "--warm" in argv) and not memory_on():
        print(OFF)
        return 0
    if mine or target:
        cards = mine_cards() if mine else target_cards()
        if not cards:
            print("no cards: " + ("nothing of this project's own work was found" if mine else
                                   "name the codebase in notes/target (one line, a path) or $BUILDER_TARGET"))
            return 1
    else:
        cards, _ = load()
        if not cards:
            print("no Builder files found: set BUILDER_HOME to the Builder folder")
            return 1
        if both and memory_on():
            cards = cards + mine_cards()
    if "--warm" in argv:
        vec = Vectors()
        if not vec.ok():
            print("no embedding model installed (./scripts/setup-embed.sh); keyword ranking only")
            return 0
        allc = load()[0] + mine_cards() + target_cards()
        vec.for_cards(allc)
        vec.save(keep={card_hash(c) for c in allc})
        print(f"warm: {len(allc)} cards have vectors in {vec.file.relative_to(PROJECT)}")
        return 0
    wants_rank = not any(a in argv for a in ("--list", "--check", "--start", "--card"))
    index = Index(cards, Vectors() if wants_rank and not lexical else None)
    by = {c["id"]: c for c in cards}
    if "--list" in argv:
        for c in cards:
            print(f"{c['id']:<52} ~{tokens(c['text']):>4}  {c['title'][:70]}")
        print(f"{len(cards)} cards, ~{sum(tokens(c['text']) for c in cards)} tokens in all")
        return 0
    if "--check" in argv:
        problems, size = check(cards)
        for p in problems:
            print(f"FAIL {p}")
        if not problems:
            print(f"cards: {len(cards)} cards from {len({c['file'] for c in cards})} files, "
                  f"every spec row and CLAUDE.md rule on a card, start pack ~{size} tokens (budget {START_BUDGET})")
        return 1 if problems else 0
    if "--qa" in argv:
        qfile = argv[argv.index("--qa") + 1]
        name = Path(qfile).stem.replace("qa", "QA").replace("builder", "Builder").replace("work", "Work")
        data = json.loads(Path(qfile).read_text())
        floor = data.get("floor") if isinstance(data, dict) else None   # a set of hard questions records how many pass now
        items, misses, unknown, at = qa(index, qfile, lexical=True)
        for u in unknown:
            print(f"FAIL the question set names card {u}, which does not exist")
        n = len(items)
        word = "FAIL" if floor is None or n - len(misses) < floor else "miss"
        for q, want, got in misses:
            print(f"{word} {q!r}: none of {', '.join(want)} in the top 5 (got {', '.join(got)})")
        print(f"{name} (keyword): {n - len(misses)}/{n} answered in the top 5; "
              f"top 1 {at[1]}/{n}, top 3 {at[3]}/{n}" + (f" (floor {floor})" if floor is not None else ""))
        bad = bool(unknown) or (bool(misses) if floor is None else n - len(misses) < floor)
        if index.vecs:
            _, hmiss, _, hat = qa(index, qfile, lexical=False)
            print(f"{name} (hybrid):  {n - len(hmiss)}/{n} answered in the top 5; "
                  f"top 1 {hat[1]}/{n}, top 3 {hat[3]}/{n}")
            if len(hmiss) > len(misses):
                print("FAIL hybrid ranking answers fewer questions in the top 5 than keyword ranking alone")
                bad = True
        else:
            print(f"{name} (hybrid):  skipped, no embedding model installed")
        return 1 if bad else 0
    if "--start" in argv:
        print("Start pack: the Builder cards an opening entry needs. Cite them as card:<id>.\n"
              "CLAUDE.md's rules are not repeated here; read CLAUDE.md in full.\n"
              "Ask for anything else with: python3 tools/ask.py \"your question\"\n")
        for i in START:
            if i in by:
                print(show(by[i]))
        return 0
    if "--card" in argv:
        bad = 0
        for cid in argv[argv.index("--card") + 1:]:
            cid = cid.removeprefix("card:")
            if cid in by:
                print(show(by[cid]))
            else:
                print(f"no card {cid}; try: python3 tools/ask.py --ids \"{cid.replace('.', ' ')}\"")
                bad = 1
        return bad
    if "--code" in argv or "--docs" in argv:
        want_code = "--code" in argv
        index = Index([c for c in cards if (".code." in "." + c["id"]) == want_code], Vectors() if not lexical else None)
    ids_only = "--ids" in argv
    query = " ".join(a for a in argv if not a.startswith("--"))
    if not query.strip():
        print(__doc__)
        return 2
    found = index.search(query, k, lexical=lexical)
    if not found:
        print("no card matches; try other words, or --list")
        return 1
    for c in found:
        print(f"card:{c['id']:<48} ~{tokens(c['text'])} tokens  {c['title']}" if ids_only else show(c))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
