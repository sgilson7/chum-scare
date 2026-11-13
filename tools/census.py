"""The scare census report (T5).

Reads the census the core wrote (`cargo run --release -p tool_core --example
census -- 200 > notes/census/runs.jsonl`) and writes:

    notes/census/summary.json   the figures quoted in the journal
    notes/census/page.html      the journal's "How scary" page: tiles, charts
                                (inline SVG in the journal's colours) and a
                                table under each chart

Every number here is a count or an average of what the core's runs recorded;
nothing here decides a rule.

    python3 tools/census.py [runs.jsonl]
"""

import html
import json
import pathlib
import statistics as st
import sys
from collections import Counter

ROOT = pathlib.Path(__file__).resolve().parents[1]
RUNS = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "notes/census/runs.jsonl"
OUT = ROOT / "notes/census"

# a 16:9 screen's edge is 51° off centre at the 70° vertical view
# (view::FOV); the bands a player feels
BANDS = [
    ("centre", "middle of the screen (under 30°)", 0, 30),
    ("edge", "edge of the screen (30° to 51°)", 30, 51),
    ("beside", "just off the screen (51° to 110°)", 51, 110),
    ("behind", "behind you (110° and more)", 110, 181),
]
# designed ceiling: world::per_minute
CAP = [1, 2, 3, 3, 4]
SOURCES = ["director", "spirit", "possession", "cult"]
NAMES = {
    "fall_crash": "something falls",
    "door_creak": "a door creaks",
    "shutter": "a shutter rattles",
    "laugh": "a laugh",
    "step_other": "footsteps",
    "breath": "breath on the neck",
    "pop_glass": "a light bursts",
    "steps": "footsteps",
}


def band(off):
    for key, _, lo, hi in BANDS:
        if lo <= off < hi:
            return key
    return "behind"


def load():
    return [json.loads(line) for line in RUNS.read_text().splitlines() if line.strip()]


def summarise(runs, player):
    R = [r for r in runs if r["player"] == player]
    n = len(R)
    pre = lambda e: e["stage"] != "chase"
    mins = st.mean(sum(s[1] for s in r["stages"] if s[0] != "chase") for r in R) / 600
    scares = []  # each director decision, with the sound it made
    for r in R:
        ev = r["events"]
        for i, e in enumerate(ev):
            if e["what"] != "scare":
                continue
            kind = next(
                (x["what"] for x in ev[i + 1 : i + 6] + ev[max(0, i - 5) : i] if x["t"] == e["t"] and x["source"] == "director" and x["sense"] == "sound"),
                "steps",
            )
            scares.append({**e, "kind": "steps" if kind == "step_other" else kind, "seed": r["seed"]})
    gaps = []
    firsts = []
    for r in R:
        ts = [e["t"] for e in r["events"] if e["what"] == "scare"]
        if ts:
            firsts.append(ts[0] / 10)
        gaps += [(b - a) / 10 for a, b in zip(ts, ts[1:])]
    # where things were, before the chase: sights and sounds from the
    # director, the spirit and possessions (the cult is seen head on)
    where = {"sight": Counter(), "sound": Counter()}
    for r in R:
        for e in r["events"]:
            if pre(e) and e["source"] in ("director", "spirit", "possession") and e["sense"] in where:
                where[e["sense"]][band(e["off"])] += 1
    # director scares a minute at each menace
    at = [st.mean(r["at_menace"][k] for r in R) / 600 for k in range(5)]
    per_menace = Counter(s["menace"] for s in scares)
    rate = [(per_menace[k] / n) / at[k] if at[k] > 0 else 0 for k in range(5)]
    kinds = Counter(s["kind"] for s in scares)
    glimpses = [e for r in R for e in r["events"] if e["what"] == "glimpse"]
    pre_counts = Counter((e["source"], e["what"]) for r in R for e in r["events"] if pre(e))
    return {
        "player": player,
        "runs": n,
        "minutes_before_chase": round(mins, 2),
        "scares_per_run": round(len(scares) / n, 2),
        "scares_per_minute": round(len(scares) / n / mins, 2),
        "first_scare_s": round(st.mean(firsts), 1) if firsts else None,
        "gap_median_s": round(st.median(gaps), 1) if gaps else None,
        "gap_p90_s": round(sorted(gaps)[int(0.9 * len(gaps))], 1) if gaps else None,
        "gap_longest_s": round(max(gaps), 1) if gaps else None,
        "gaps": gaps,
        "busy_share": round(sum(s["busy"] for s in scares) / max(1, len(scares)), 2),
        "busy_minutes": round(st.mean(r["busy"] for r in R) / 600, 2),
        "minutes_at_menace": [round(a, 2) for a in at],
        "rate_at_menace": [round(x, 2) for x in rate],
        "kinds": dict(kinds.most_common()),
        "where": {k: dict(v) for k, v in where.items()},
        "glimpses_per_run": round(len(glimpses) / n, 2),
        "glimpse_off": sorted(Counter(g["off"] for g in glimpses).items())[:6],
        "glimpse_dist_m": round(st.mean(g["dist"] for g in glimpses) / 10, 1) if glimpses else None,
        "mannequin_turns_per_run": round(pre_counts[("director", "mannequin_turn")] / n, 2),
        "possession_steps_per_run": round(pre_counts[("possession", "step_closer")] / n, 2),
        "possession_knocks_per_run": round(pre_counts[("possession", "plastic_knock")] / n, 2),
        "cult_caught_share": round(sum(any(e["what"] == "shout" and e["stage"] != "chase" and e["stage"] != "ouija" for e in r["events"]) for r in R) / n, 2),
        "hurt_ticks": round(st.mean(r["hurt"] for r in R), 1),
        "escaped": sum(r["escaped"] for r in R),
    }


# ---------- drawing ----------

W = 640
E = html.escape


def svg(h, body, label):
    return f'<svg class="viz" viewBox="0 0 {W} {h}" role="img" aria-label="{E(label)}">{body}</svg>'


def hbars(rows, label, unit="", cls="s1", fmt=lambda v: f"{v:g}", ref=None):
    """rows: (name, value[, tip]); one series, labelled at each bar's end."""
    top = max([r[1] for r in rows] + ([max(ref)] if ref else [])) or 1
    left, right, row = 200, 70, 30
    h = row * len(rows) + 12
    out = []
    for i, r in enumerate(rows):
        name, v = r[0], r[1]
        tip = r[2] if len(r) > 2 else f"{name}: {fmt(v)}{unit}"
        y = 6 + i * row
        w = max(4, (W - left - right) * v / top) if v > 0 else 0
        out.append(f'<text x="{left - 10}" y="{y + 17}" text-anchor="end" class="lab">{E(name)}</text>')
        if w:
            out.append(f'<rect x="{left}" y="{y + 5}" width="{w:.1f}" height="16" rx="4" class="{cls}"><title>{E(tip)}</title></rect>')
        out.append(f'<text x="{left + w + 6:.1f}" y="{y + 17}" class="val">{E(fmt(v))}{E(unit)}</text>')
        if ref:
            x = left + (W - left - right) * ref[i] / top
            out.append(f'<line x1="{x:.1f}" x2="{x:.1f}" y1="{y + 1}" y2="{y + 25}" class="ref"><title>designed ceiling {ref[i]} a minute</title></line>')
    return svg(h, "".join(out), label)


def histogram(values, edges, label):
    counts = [sum(1 for v in values if lo <= v < hi) for lo, hi in zip(edges, edges[1:])]
    top = max(counts) or 1
    left, bottom, h = 40, 34, 200
    bw = (W - left - 10) / len(counts)
    out = [f'<line x1="{left}" x2="{W - 10}" y1="{h - bottom}" y2="{h - bottom}" class="axis"/>']
    for i, c in enumerate(counts):
        bh = (h - bottom - 20) * c / top
        x = left + i * bw + 1
        tip = f"{edges[i]:g} to {edges[i + 1]:g} s: {c} gaps ({100 * c / max(1, len(values)):.0f}%)"
        if c:
            out.append(f'<rect x="{x:.1f}" y="{h - bottom - bh:.1f}" width="{bw - 2:.1f}" height="{bh:.1f}" rx="3" class="s1"><title>{E(tip)}</title></rect>')
        out.append(f'<text x="{x + bw / 2 - 1:.1f}" y="{h - bottom + 16}" text-anchor="middle" class="tick">{edges[i]:g}</text>')
    out.append(f'<text x="{W - 10}" y="{h - 4}" text-anchor="end" class="tick">seconds of quiet between one director scare and the next</text>')
    return svg(h, "".join(out), label), counts


def timeline(run, label):
    """One run: a row per source, a dot per event, the stages as bands."""
    end = run["length"] / 10
    left, h = 92, 40 + 30 * len(SOURCES) + 30
    sx = lambda t: left + (W - left - 10) * t / end
    out = []
    t0 = 0
    for i, (stage, d) in enumerate(run["stages"]):
        x0, x1 = sx(t0 / 10), sx((t0 + d) / 10)
        out.append(f'<rect x="{x0:.1f}" y="18" width="{max(0, x1 - x0 - 2):.1f}" height="{30 * len(SOURCES) + 6}" class="band{i % 2}"/>')
        if d > 150:
            out.append(f'<text x="{(x0 + x1) / 2:.1f}" y="13" text-anchor="middle" class="tick">{E(stage)}</text>')
        t0 += d
    for k, src in enumerate(SOURCES):
        y = 36 + 30 * k
        out.append(f'<text x="{left - 8}" y="{y + 4}" text-anchor="end" class="lab">{E(src)}</text>')
        seen = set()
        for e in run["events"]:
            if e["source"] != src or e["sense"] == "decision":
                continue
            t = e["t"] / 10
            key = (round(t), e["what"])
            if key in seen:
                continue
            seen.add(key)
            shape = "s%d" % (k + 1)
            tip = f'{t:.0f} s, {e["what"].replace("_", " ")} ({e["sense"]}), {e["off"]}° off the view, {e["stage"]}'
            if e["sense"] == "sight":
                out.append(f'<rect x="{sx(t) - 5:.1f}" y="{y - 5}" width="10" height="10" rx="2" class="{shape} ring"><title>{E(tip)}</title></rect>')
            else:
                out.append(f'<circle cx="{sx(t):.1f}" cy="{y}" r="5" class="{shape} ring"><title>{E(tip)}</title></circle>')
    ticks = range(0, int(end) + 1, 60)
    yb = 30 + 30 * len(SOURCES) + 4
    for t in ticks:
        out.append(f'<text x="{sx(t):.1f}" y="{yb + 16}" text-anchor="middle" class="tick">{t // 60} min</text>')
    return svg(h, "".join(out), label)


def table(head, rows):
    th = "".join(f"<th>{E(str(x))}</th>" for x in head)
    tr = "".join("<tr>" + "".join(f"<td>{E(str(x))}</td>" for x in r) + "</tr>" for r in rows)
    return f'<details class="tv"><summary>Table</summary><div class="tbl"><table><tr>{th}</tr>{tr}</table></div></details>'


STYLE = """
<style>
#scary { --s1: #2a78d6; --s2: #eb6834; --s3: #1baf7a; --s4: #eda100; }
@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) #scary { --s1: #3987e5; --s2: #d95926; --s3: #199e70; --s4: #c98500; } }
:root[data-theme="dark"] #scary { --s1: #3987e5; --s2: #d95926; --s3: #199e70; --s4: #c98500; }
#scary svg.viz { width: 100%; height: auto; display: block; margin: 8px 0 4px; font: 12px var(--body); }
#scary .viz .lab { fill: var(--fg); }
#scary .viz .val, #scary .viz .tick { fill: var(--muted); }
#scary .viz .axis { stroke: var(--line); }
#scary .viz .ref { stroke: var(--fg); stroke-width: 2; stroke-dasharray: 3 2; }
#scary .viz .band0 { fill: color-mix(in srgb, var(--line) 35%, transparent); }
#scary .viz .band1 { fill: color-mix(in srgb, var(--line) 15%, transparent); }
#scary .viz .s1 { fill: var(--s1); } #scary .viz .s2 { fill: var(--s2); }
#scary .viz .s3 { fill: var(--s3); } #scary .viz .s4 { fill: var(--s4); }
#scary .viz .ring { stroke: var(--panel); stroke-width: 2; }
#scary .viz rect:hover, #scary .viz circle:hover { stroke: var(--fg); }
#scary .tiles { display: grid; grid-template-columns: repeat(auto-fit, minmax(140px, 1fr)); gap: 10px; margin: 10px 0; }
#scary .tile { background: var(--panel); border: 1px solid var(--line); border-radius: 8px; padding: 10px 12px; }
#scary .tile b { display: block; font-size: 26px; line-height: 1.1; }
#scary .tile span { color: var(--muted); font-size: 13px; }
#scary .key { display: flex; flex-wrap: wrap; gap: 6px 16px; font-size: 13px; color: var(--muted); }
#scary .key i { display: inline-block; width: 10px; height: 10px; border-radius: 50%; margin-right: 5px; vertical-align: -1px; }
#scary .key i.sq { border-radius: 2px; }
#scary details.tv { margin: 0 0 14px; font-size: 13px; }
</style>
"""


def page(m, b, runs):
    gaps_svg, gap_counts = histogram(m["gaps"], [0, 15, 30, 45, 60, 75, 90, 120, 180], "How long the quiet lasts between director scares, methodical player")
    edges = [0, 15, 30, 45, 60, 75, 90, 120, 180]
    # a run near the median count, for the timeline
    meth = [r for r in runs if r["player"] == "methodical"]
    target = m["scares_per_run"]
    typical = min(meth, key=lambda r: (abs(sum(e["what"] == "scare" for e in r["events"]) - target), r["seed"]))
    tiles = [
        (f'{m["scares_per_run"]:g}', "director scares in a methodical run, before the chase"),
        (f'{m["scares_per_minute"]:g}', "director scares a minute (about one every %.0f s)" % (60 / m["scares_per_minute"])),
        (f'{m["first_scare_s"]:g} s', "until the first: it comes during the first load"),
        (f'{m["gap_median_s"]:g} s', "median quiet between scares (longest %g s)" % m["gap_longest_s"]),
        (f'{100 * m["busy_share"]:.0f}%', "of scares land while your hands are busy (%.0f%% of the time they are)" % (100 * m["busy_minutes"] / m["minutes_before_chase"])),
        (f'{m["glimpses_per_run"]:g}', "glimpses of the spirit at the edge of the view"),
    ]
    tile_html = '<div class="tiles">' + "".join(f"<div class=tile><b>{E(v)}</b><span>{E(t)}</span></div>" for v, t in tiles) + "</div>"
    where_rows = []
    for sense in ("sight", "sound"):
        c = m["where"][sense]
        tot = sum(c.values()) or 1
        for key, name, *_ in BANDS:
            where_rows.append((sense, name, c.get(key, 0), round(100 * c.get(key, 0) / tot)))
    sight_svg = hbars([(r[1], r[3], f"{r[0]}: {r[1]}, {r[3]}% ({r[2]} events)") for r in where_rows if r[0] == "sight"], "Where the things you could see happened, relative to the view", "%", "s2")
    sound_svg = hbars([(r[1], r[3], f"{r[0]}: {r[1]}, {r[3]}% ({r[2]} events)") for r in where_rows if r[0] == "sound"], "Where the sounds came from, relative to the view", "%", "s1")
    menace_rows = [(f"menace {k}", m["rate_at_menace"][k], f"menace {k}: {m['rate_at_menace'][k]} a minute measured over {m['minutes_at_menace'][k]} min a run; ceiling {CAP[k]}") for k in range(5)]
    menace_svg = hbars(menace_rows, "Director scares a minute at each menace, against the designed ceiling", "", "s1", ref=CAP)
    kinds = m["kinds"]
    ktot = sum(kinds.values()) or 1
    kind_rows = [(NAMES.get(k, k), round(100 * v / ktot), f"{NAMES.get(k, k)}: {v} in {m['runs']} runs ({100 * v / ktot:.0f}%)") for k, v in kinds.items()]
    kind_svg = hbars(kind_rows, "Which scare the director picks", "%", "s1")
    tl = timeline(typical, "One methodical run, event by event")
    key = '<p class="key">' + "".join(f'<span><i class="" style="background:var(--s{k + 1})"></i>{E(s)}</span>' for k, s in enumerate(SOURCES)) + '<span>● a sound</span><span>■ something seen</span></p>'
    cmp_rows = [
        ("minutes before the chase", m["minutes_before_chase"], b["minutes_before_chase"]),
        ("director scares a run", m["scares_per_run"], b["scares_per_run"]),
        ("director scares a minute", m["scares_per_minute"], b["scares_per_minute"]),
        ("share while hands busy", m["busy_share"], b["busy_share"]),
        ("median quiet (s)", m["gap_median_s"], b["gap_median_s"]),
        ("glimpses a run", m["glimpses_per_run"], b["glimpses_per_run"]),
        ("mannequins turned to you a run", m["mannequin_turns_per_run"], b["mannequin_turns_per_run"]),
        ("possessed steps closer a run", m["possession_steps_per_run"], b["possession_steps_per_run"]),
        ("runs the cult caught you", f'{100 * m["cult_caught_share"]:.0f}%', f'{100 * b["cult_caught_share"]:.0f}%'),
        ("escaped", f'{m["escaped"]} of {m["runs"]}', f'{b["escaped"]} of {b["runs"]}'),
    ]
    parts = {
        "tiles": tile_html,
        "timeline": key + tl + table(["time (s)", "source", "what", "sense", "degrees off view", "stage"], [(e["t"] / 10, e["source"], e["what"], e["sense"], e["off"], e["stage"]) for e in typical["events"] if e["source"] in SOURCES and e["sense"] != "decision"]),
        "typical_seed": typical["seed"],
        "where": "<p><b>What you could see</b> (glimpses, possessed props moving, mannequins turning)</p>" + sight_svg + "<p><b>What you could hear</b> (the director, possessions, the spirit)</p>" + sound_svg + table(["sense", "where", "events", "share %"], where_rows),
        "menace": menace_svg + table(["menace", "minutes a run", "measured a minute", "ceiling a minute"], [(k, m["minutes_at_menace"][k], m["rate_at_menace"][k], CAP[k]) for k in range(5)]),
        "gaps": gaps_svg + table(["quiet (s)", "gaps"], [(f"{lo:g} to {hi:g}", c) for lo, hi, c in zip(edges, edges[1:], gap_counts)]),
        "kinds": kind_svg + table(["scare", "share %"], [(r[0], r[1]) for r in kind_rows]),
        "compare": table(["", "methodical", "brisk"], cmp_rows).replace("<details class=\"tv\">", "<details class=\"tv\" open>"),
    }
    return parts


def main():
    runs = load()
    m = summarise(runs, "methodical")
    b = summarise(runs, "brisk")
    OUT.mkdir(parents=True, exist_ok=True)
    slim = {k: {kk: vv for kk, vv in v.items() if kk != "gaps"} for k, v in (("methodical", m), ("brisk", b))}
    (OUT / "summary.json").write_text(json.dumps(slim, indent=1) + "\n")
    parts = page(m, b, runs)
    (OUT / "parts.json").write_text(json.dumps({"style": STYLE, **parts}))
    print(json.dumps(slim, indent=1))


if __name__ == "__main__":
    main()
