#!/usr/bin/env python3
"""Write a one-page digest of an earlier project, for later projects to read instead of its transcripts.

A digest says what the project is, what worked, what to copy, and where those
things are (files, journal entries, cards). This script fills in what it can
read from the project's folder and journal; an agent then writes "What to
copy" once, from the journal and the code, and removes the TO FILL line.
tools/ask.py searches digests in $BUILDER_REFS or ../refs/ as ref.<name> cards.

    python3 tools/digest.py ../ape-race --out ../refs/ape-race.md [--journal ../ape-race-journal]
    python3 tools/digest.py --check ../refs/ape-race.md
"""

import html
import re
import sys
from pathlib import Path

SECTIONS = ["What it is", "What worked", "What to copy", "Where things are"]
MAX = 6000
TODO = "TO FILL"


def problems(text):
    out = [f'no "## {s}" section' for s in SECTIONS if not re.search(rf"^## {re.escape(s)}\s*$", text, re.M)]
    if len(text) > MAX:
        out.append(f"{len(text)} characters; a digest fits on a page ({MAX})")
    if TODO in text:
        out.append(f'still says {TODO}: write "What to copy" from the journal and the code')
    return out


def first(text, pattern, default="", flags=re.S | re.M):
    m = re.search(pattern, text, flags)
    return m.group(1).strip() if m else default


def strip_tags(s):
    return html.unescape(re.sub(r"<[^>]+>", "", s)).strip()


def journal_page(project, journal):
    """The journal's page: the named or sibling journal folder, else a journal kept
    inside the project (journal/*.html, *journal*.html), else the project itself
    when it is a journal with no brief (a maintenance journal like ToyTalk's)."""
    if journal and (journal / "index.html").exists():
        return journal / "index.html"
    inside = sorted(project.glob("journal/*.html")) + sorted(project.glob("*journal*.html")) + sorted(project.glob("*/*journal*.html"))
    if inside:
        return max(inside, key=lambda p: p.stat().st_size)
    if (project / "index.html").exists() and not (project / "BRIEF.md").exists():
        return project / "index.html"
    return None


def build(project, journal):
    name = project.name
    read = lambda p: (project / p).read_text(errors="replace") if (project / p).exists() else ""
    readme, brief, handoff, claude = read("README.md"), read("BRIEF.md"), read("HANDOFF.md"), read("CLAUDE.md")
    title = first(readme, r"^#\s+([^\n]+)$", name, re.M)
    url = first(readme + handoff, r"(https://[\w.-]+\.github\.io/[\w./-]*)")
    words = first(brief, r"## The description, word for word\s*\n(.*?)\n## ")
    words = " ".join(l.lstrip("> ").strip() for l in words.splitlines() if l.startswith(">"))[:500]
    tool = first(brief, r"## The tool\s*\n(.*?)\n## ")[:400]
    if tool.startswith("What it does, who uses it"):   # the template's placeholder, never filled in
        tool = ""
    agreed = first(claude, r"## Rules agreed for this project\s*\n(.*?)\n## ")[:900]
    kits = sorted({m for m in re.findall(r"kits/([a-z-]+)", read("SECOND-ORDER.md") + readme + brief + handoff)})
    core = sorted((project / "crates" / "core" / "src").glob("*.rs")) if (project / "crates").is_dir() else []
    biggest = sorted(core, key=lambda p: -p.stat().st_size)[:6]
    titles = []
    page_file = journal_page(project, journal)
    if page_file:
        page = page_file.read_text(errors="replace")
        # each entry's heading with the first sentence after it, so the draft says what the entry found
        for m in re.finditer(r"<h3[^>]*>(.*?)</h3>(.*?)(?=<h3|\Z)", page, re.S):
            lede = first(m.group(2), r"<p[^>]*>(.*?)</p>")
            lede = re.split(r"(?<=[.!?])\s", strip_tags(lede), maxsplit=1)[0][:160] if lede else ""
            titles.append(strip_tags(m.group(1)) + (f": {lede}" if lede else ""))
            if len(titles) >= 12:
                break
    lines = [f"# {title}", "",
             f"Digest of `{name}` for later Builder projects. Read this instead of its transcripts; open a file only when this page names it.", "",
             "## What it is", "",
             (f"Live: {url}" if url else "Live: not deployed, or no address in README.md or HANDOFF.md"), ""]
    if words:
        lines += [f"> {words}", ""]
    if tool:
        lines += [tool, ""]
    lines += ["## What worked", ""]
    lines += ([f"Rules agreed with the person:\n\n{agreed}", ""] if agreed and "None yet" not in agreed else [])
    lines += (["Journal entries, newest first:", ""] + [f"- {t}" for t in titles] + [""] if titles else [])
    lines += ["## What to copy", "", f"{TO_FILL_LINE}", "",
              "## Where things are", ""]
    lines += [f"- Kits: {', '.join(kits) if kits else 'none named'}"]
    lines += [f"- Core modules: " + (", ".join(f"`crates/core/src/{p.name}`" for p in biggest) if biggest else "none")]
    if journal:
        lines += [f"- Journal: `{journal}`"]
    lines += [f"- Folder: `{project}`", ""]
    return "\n".join(lines)


TO_FILL_LINE = f"{TODO}: what a later project should take from this one, each with the file or journal entry that holds it, and card ids where Builder already covers it."


def main(argv):
    if "--check" in argv:
        bad = 0
        for f in [a for a in argv if not a.startswith("--")]:
            found = problems(Path(f).read_text())
            for p in found:
                print(f"FAIL {f}: {p}")
            bad |= bool(found)
            if not found:
                print(f"digest: {f} in form")
        return bad
    args = [a for a in argv if not a.startswith("--")]
    if not args or "--out" not in argv:
        print(__doc__)
        return 2
    project = Path(args[0]).resolve()
    out = Path(argv[argv.index("--out") + 1])
    journal = Path(argv[argv.index("--journal") + 1]).resolve() if "--journal" in argv else None
    if journal is None:
        guess = project.parent / f"{project.name}-journal"
        journal = guess if guess.is_dir() else None
    if out.exists():
        print(f"{out} exists; edit it instead")
        return 1
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(build(project, journal))
    print(f"wrote {out}; now write its What to copy section and remove the {TODO} line")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
