#!/usr/bin/env python3
"""Turn the design journal into a deck for someone outside the build.

An advisor, a client or a teammate reads a deck, not a log. This writes, beside
the journal:

- deck.html: a title slide, one slide per figure in the journal (its entry's
  heading, the figure, its caption and the entry's opening sentence), oldest
  first, then a slide of the questions still open (from decisions.json and
  answers.json), each slide open to comments;
- deck.txt: the same slides as plain text, without figures, to paste into a
  document when the link can't be opened.

Both are built from the journal, so nothing in them is typed twice. The deck
records a hash of what it was built from, and --check fails when the journal
or its questions have changed since.

    python3 tools/journal/present.py            write deck.html and deck.txt
    python3 tools/journal/present.py --check    fail if the deck is out of date

Publish deck.html like the journal (stage 8, only with a grant), declaring the
comments capability as composer only, so each slide's Comment button opens the
viewer's comment box: capabilities {"comments": {"composer_only": true}}.
"""

import hashlib
import html
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from journal_check import setting  # noqa: E402

MARK = "<!-- built by tools/journal/present.py from "


def text(s):
    s = re.sub(r"<(script|style)\b.*?</\1>", " ", s, flags=re.S)
    return re.sub(r"\s+", " ", html.unescape(re.sub(r"<[^>]+>", " ", s))).strip()


def journal_dir():
    brief = (ROOT / "BRIEF.md").read_text() if (ROOT / "BRIEF.md").exists() else ""
    kind, where = setting(ROOT, brief)
    return None if kind == "none" else where


def source_hash(jd):
    h = hashlib.sha256()
    for name in ("index.html", "decisions.json", "answers.json"):
        p = jd / name
        h.update(p.read_bytes() if p.exists() else b"")
    return h.hexdigest()[:16]


def slides(page):
    """(entry id, heading, opening sentence, figure html, caption) per figure, oldest entry first in each tab."""
    out = []
    # a journal with tabs keeps each tab newest first: take the tabs in order, each oldest first
    pages = re.split(r"<section\b[^>]*\bdata-page\b", page)[1:] or [page]
    arts = []
    for part in pages:
        arts += reversed(list(re.finditer(r'<article\b[^>]*\bid="([^"]+)"[^>]*>(.*?)</article>', part, re.S)))
    for m in arts:
        body = m.group(2)
        h3 = re.search(r"<h3[^>]*>(.*?)</h3>", body, re.S)
        lede = re.search(r'<p class="lede"[^>]*>(.*?)</p>', body, re.S) or re.search(r"<p[^>]*>(.*?)</p>", body, re.S)
        first = re.split(r"(?<=[.!?])\s", text(lede.group(1)), maxsplit=1)[0] if lede else ""
        for f in re.finditer(r"<figure\b[^>]*>(.*?)</figure>", body, re.S):
            cap = re.search(r"<figcaption[^>]*>(.*?)</figcaption>", f.group(1), re.S)
            fig = re.sub(r"<figcaption[^>]*>.*?</figcaption>", "", f.group(1), flags=re.S)
            out.append((m.group(1), text(h3.group(1)) if h3 else m.group(1), first, fig, text(cap.group(1)) if cap else ""))
    return out


def open_questions(jd):
    p = jd / "decisions.json"
    data = json.loads(p.read_text()) if p.exists() else {}
    a = jd / "answers.json"
    answers = json.loads(a.read_text()) if a.exists() else {}
    return [q for q in data.get("questions", []) if not answers.get(q["id"], {}).get("choice")]


def styles_and_scripts(page):
    css = "\n".join(re.findall(r"<style[^>]*>(.*?)</style>", page, re.S))
    js = "\n".join(s for s in re.findall(r"<script[^>]*>(.*?)</script>", page, re.S) if "data-scene" in s or "SCENES" in s)
    return css, js


DECK_CSS = """
.deck { max-width: 1100px; margin: 0 auto; padding-block: 16px; padding-left: max(16px, env(safe-area-inset-left, 0px));
  padding-right: max(16px, env(safe-area-inset-right, 0px)); }
.slide { min-height: 80vh; display: grid; align-content: start; gap: 14px; padding-block: 24px; }
.slide[hidden] { display: none !important; }
.slide h2 { margin: 0; text-wrap: balance; }
.slide .lead { font-size: 1.15em; max-width: 70ch; margin: 0; }
.deckbar { position: sticky; top: env(safe-area-inset-top, 0px); display: flex; flex-wrap: wrap; gap: 8px; align-items: center;
  padding-block: 8px; background: inherit; z-index: 2; }
.deckbar .count { font-family: monospace; font-size: 13px; }
.q { border-left: 3px solid currentColor; padding-left: 12px; }
.q ol { margin: 6px 0 0; }
"""

DECK_JS = """
(function () {
  const s = [...document.querySelectorAll('.slide')], count = document.querySelector('.count');
  let i = 0;
  function show(n) { i = Math.max(0, Math.min(s.length - 1, n)); s.forEach((x, j) => { x.hidden = j !== i; });
    count.textContent = (i + 1) + ' / ' + s.length; if (location.hash !== '#slide-' + (i + 1)) history.replaceState(null, '', '#slide-' + (i + 1)); }
  document.querySelector('[data-go=prev]').onclick = () => show(i - 1);
  document.querySelector('[data-go=next]').onclick = () => show(i + 1);
  document.addEventListener('keydown', e => { if (e.target.closest('input,textarea')) return;
    if (e.key === 'ArrowRight' || e.key === 'PageDown') show(i + 1); if (e.key === 'ArrowLeft' || e.key === 'PageUp') show(i - 1); });
  const m = location.hash.match(/^#slide-(\\d+)$/); show(m ? +m[1] - 1 : 0);
  const buttons = [...document.querySelectorAll('[data-comment]')];
  buttons.forEach(b => { b.hidden = true; });
  if (window.claude && window.claude.use) window.claude.use('comments').then(c => {
    if (!c) return;
    buttons.forEach(b => { b.hidden = false; b.onclick = () => c.openComposer({ element: b.closest('.slide') }).catch(() => {}); });
  }, () => {});
})();
"""


def build(jd):
    page = (jd / "index.html").read_text(errors="replace")
    title = text((re.search(r"<h1[^>]*>(.*?)</h1>", page, re.S) or re.search(r"<title>(.*?)</title>", page, re.S)).group(1)) \
        if re.search(r"<h1|<title>", page) else jd.name
    css, js = styles_and_scripts(page)
    defs = "\n".join(re.findall(r'<svg\b[^>]*\bwidth="0"[^>]*>.*?</svg>', page, re.S))   # shared arrowheads
    figs = slides(page)
    qs = open_questions(jd)
    e = html.escape
    comment = '<button type="button" class="act" data-comment hidden>Comment on this slide</button>'
    parts = [f'<section class="slide" id="s0" data-comment-target><h2>{e(title)}</h2>'
             f'<p class="lead">{len(figs)} figures from the design journal, oldest first, then {len(qs)} open questions. '
             f'Use the arrow keys or the buttons; comment on any slide.</p>{comment}</section>']
    txt = [title, "=" * len(title), "", f"{len(figs)} figures from the design journal, then {len(qs)} open questions.", ""]
    for n, (eid, head, first, fig, cap) in enumerate(figs, 1):
        parts.append(f'<section class="slide" id="s{n}" data-comment-target data-entry="{e(eid)}"><h2>{e(head)}</h2>'
                     + (f'<p class="lead">{e(first)}</p>' if first else "")
                     + f'<figure>{fig}<figcaption>{e(cap)}</figcaption></figure>{comment}</section>')
        txt += [f"{n}. {head}", ""] + ([first, ""] if first else []) + ([f"Figure: {cap}", ""] if cap else [])
    if qs:
        items = []
        txt += ["Questions still open", "--------------------", ""]
        for q in qs:
            opts = "".join(f"<li><b>{e(o['key'])}</b> {e(o['text'])}{' (default)' if o.get('default') else ''}</li>" for o in q["options"])
            items.append(f'<div class="q"><p><b>{e(q["id"])}</b> {e(q["question"])}</p><ol type="A">{opts}</ol></div>')
            txt += [f"{q['id']}. {q['question']}"] + [f"   {o['key']}. {o['text']}{' (default)' if o.get('default') else ''}"
                                                       for o in q["options"]] + [""]
        parts.append(f'<section class="slide" id="sq" data-comment-target><h2>Questions still open</h2>{"".join(items)}'
                     f'<p>Answer them on the journal\'s chips, or reply with a letter for each.</p>{comment}</section>')
    h = source_hash(jd)
    deck = (f"<title>{e(title)}: deck</title>\n{MARK}{h} -->\n<style>{css}\n{DECK_CSS}</style>\n"
            f'{defs}\n<div class="deck"><div class="deckbar"><button type="button" class="act" data-go="prev">Back</button>'
            f'<button type="button" class="act" data-go="next">Next</button><span class="count"></span></div>\n'
            + "\n".join(parts) + f"\n</div>\n<script>{js}</script>\n<script>{DECK_JS}</script>\n")
    return deck, "\n".join(txt) + "\n", len(figs), len(qs)


def main(argv):
    jd = Path(argv[argv.index("--journal") + 1]) if "--journal" in argv else journal_dir()
    if not jd or not (jd / "index.html").exists():
        print("no journal found (BRIEF.md's Journal section, or --journal DIR)")
        return 0 if "--check" in argv else 1
    if "--check" in argv:
        deck = jd / "deck.html"
        if not deck.exists():
            print("deck: none built (python3 tools/journal/present.py writes one)")
            return 0
        m = re.search(re.escape(MARK) + r"([0-9a-f]+)", deck.read_text())
        if not m or m.group(1) != source_hash(jd):
            print("FAIL the deck is out of date with the journal: run python3 tools/journal/present.py")
            return 1
        print("deck: up to date with the journal")
        return 0
    deck, txt, nf, nq = build(jd)
    (jd / "deck.html").write_text(deck)
    (jd / "deck.txt").write_text(txt)
    print(f"wrote {jd / 'deck.html'} ({nf} figure slides, {nq} open questions) and deck.txt")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
