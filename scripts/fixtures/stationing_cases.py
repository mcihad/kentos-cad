#!/usr/bin/env python3
"""Hat boyunca kilometre, Km yaz (docs/adr/0189): the shared cases, written
from the ADR's rules without KentOS code, at 50 digits with mpmath.

- The stations: the km multiples of the interval along the route (the km
  of its first point is the start; walked from the end when reversed), its
  ends when asked; a station within 10⁻⁹ m of another is one (the
  multiple's text), and round a closed route the end is the start's.
- The km text: the multiples with the interval's decimals (its shortest
  writing), the ends with the project's; the display rule (ADR 0149).
- At each station: a tick square to the route, the km text past it on its
  side square to the route and upright (past a right angle either way half
  a turn, hung on its baseline's right), beside the tick's line (its
  baseline a quarter of the height off it), a cross-section, a point at an
  offset (the right of the way positive).

The route's edges and the km's writing are the point calculator's
reference's (`point_calc_cases.py`, ADR 0188). The core runs the cases
natively and through WASM (crates/shared/geometry-core/tests/all/stationing.rs,
apps/web/src/tools/stationing.wasm.test.ts).

    python3 scripts/fixtures/stationing_cases.py          # write
    python3 scripts/fixtures/stationing_cases.py --check  # compare
"""

import json
import math
import sys
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import point_calc_cases as pc  # noqa: E402

mp.mp.dps = 50
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "stationing" / "v1" / "cases.json"
SOURCE = "scripts/fixtures/stationing_cases.py (docs/adr/0189)"
PI = mp.pi
SLACK = mp.mpf("1e-9")
MOST = 20000
NO_ROUTE = "Bu nesnenin üzerinde yürünecek tek bir yolu yok: çizgi, çoklu çizgi, yay, daire, elips, eğri ya da tek parçalı alan seçin."
BAD_INTERVAL = "Aralık sıfırdan büyük bir uzunluk olmalı."


def interval_decimals(interval):
    """The fewest decimals that write the interval (its shortest writing), at most six."""
    text = repr(float(interval))
    if "e" in text or "E" in text:
        return 6
    whole, _, frac = text.partition(".")
    frac = frac.rstrip("0")
    return min(len(frac), 6)


def frame(edges, closed, s):
    """The point s along the walked edges and the unit direction there; at a vertex the next edge's."""
    total = sum(pc.length(ed) for ed in edges)
    if closed and s >= total:
        s = mp.mpf(0)
    acc = mp.mpf(0)
    for i, ed in enumerate(edges):
        l = pc.length(ed)
        if s < acc + l or i == len(edges) - 1:
            return pc.at_edge(ed, min(mp.mpf(1), (s - acc) / l))
        acc += l


def stations(e, rules):
    edges = pc.walk(e, rules["reverse"])
    if not edges:
        return {"problem": NO_ROUTE}
    _, closed = pc.edges_of(e)
    interval, start = mp.mpf(rules["interval"]), mp.mpf(rules["start"])
    if not interval > 0:
        return {"problem": BAD_INTERVAL}
    length = sum(pc.length(ed) for ed in edges)
    end = start + length
    first = int(mp.ceil((start - SLACK) / interval))
    last = int(mp.floor((end + SLACK) / interval))
    count = last - first + 1 if last >= first else 0
    if count > MOST:
        return {"problem": f"Bu aralıkla {count} istasyon olur, en çok {MOST} yazılır; aralığı büyütün."}
    step = interval_decimals(rules["interval"])
    at = []
    if rules["ends"]:
        at.append((mp.mpf(0), float(start), rules["decimals"]))
    for k in range(first, last + 1):
        km = float(k) * rules["interval"]  # the multiple as the float it is
        s = min(max(mp.mpf(km) - start, mp.mpf(0)), length)
        at.append((s, km, step))
    if rules["ends"]:
        at.append((length, float(end), rules["decimals"]))
    at.sort(key=lambda a: a[0])
    kept = []
    for s, km, d in at:
        if kept and abs(s - kept[-1][0]) <= SLACK:
            if d == step and kept[-1][2] != step:
                kept[-1] = (kept[-1][0], km, d)
            continue
        kept.append((s, km, d))
    if closed and len(kept) > 1 and kept[0][0] <= SLACK:
        kept = [k for k in kept if k[0] < length - SLACK]
    out = []
    for s, km, d in kept:
        p, t = frame(edges, closed, s)
        out.append({"s": s, "km": km, "text": pc.km_text(km, d), "point": p, "tangent": t})
    return {"stations": out}


def objects(stations, look):
    texts, ticks, sections, points = [], [], [], []
    for st in stations:
        p, t = st["point"], st["tangent"]
        left = (-t[1], t[0])
        off = lambda d: (p[0] + left[0] * d, p[1] + left[1] * d)  # noqa: E731
        if look["tick"] > 0:
            ticks.append({"a": off(mp.mpf(look["tick"])), "b": off(-mp.mpf(look["tick"])), "km": st["text"]})
        if look["text"] is not None:
            way = 1 if look["text"] == "left" else -1
            n = (left[0] * way, left[1] * way)
            start = max(mp.mpf(look["tick"]), 0) + mp.mpf(look["height"]) / 2
            a = mp.atan2(n[1], n[0])
            upright = a > PI / 2 or a < -PI / 2
            turn, align = (a + PI, "baselineRight") if upright else (a, None)
            # Beside the tick's line: the baseline a quarter of the height off it, on the letters' side.
            up = (-mp.sin(turn), mp.cos(turn))
            gap = mp.mpf(look["height"]) / 4
            rotation = turn * 180 / PI
            if rotation > 180:
                rotation -= 360
            at = (p[0] + n[0] * start + up[0] * gap, p[1] + n[1] * start + up[1] * gap)
            texts.append({"p": at, "rotation": rotation, "align": align, "text": st["text"]})
        if look["section"] > 0:
            sections.append({"a": off(mp.mpf(look["section"])), "b": off(-mp.mpf(look["section"])), "km": st["text"]})
        if look["point"] is not None:
            points.append({"p": off(-mp.mpf(look["point"])), "km": st["text"]})
    return {"texts": texts, "ticks": ticks, "sections": sections, "points": points}


def xy(v):
    return [float(v[0]), float(v[1])]


def written(e, rules, look):
    got = stations(e, rules)
    if "problem" in got:
        return {"problem": got["problem"]}
    o = objects(got["stations"], look)
    return {
        "stations": [
            {"s": float(s["s"]), "km": s["km"], "text": s["text"], "point": xy(s["point"]), "tangent": xy(s["tangent"])}
            for s in got["stations"]
        ],
        "texts": [{"p": xy(t["p"]), "rotation": float(t["rotation"]), "align": t["align"], "text": t["text"]} for t in o["texts"]],
        "ticks": [{"a": xy(t["a"]), "b": xy(t["b"]), "km": t["km"]} for t in o["ticks"]],
        "sections": [{"a": xy(t["a"]), "b": xy(t["b"]), "km": t["km"]} for t in o["sections"]],
        "points": [{"p": xy(t["p"]), "km": t["km"]} for t in o["points"]],
    }


# ── The cases ───────────────────────────────────────────────────────────────

LINE = pc.line((0, 0), (45, 0))
ROUTE = pc.polyline([(-40, -10), (-20, -10), (0, -10)], [0, math.tan(math.pi / 8)])
SQUARE = pc.polygon([(0, 0), (10, 0), (10, 10), (0, 10)])
CIRCLE = pc.circle((0, 0), 10)
FAR = pc.polyline([(486990.125, 4419990.25), (487012.5, 4419995.75), (487030.0, 4420011.0)], [0, -0.3])
NORTH = pc.line((5, 0), (5, 30))


def rules(interval=20.0, start=0.0, reverse=False, ends=True, decimals=3):
    return {"interval": interval, "start": start, "reverse": reverse, "ends": ends, "decimals": decimals}


def look(text="left", height=2.0, tick=1.0, section=0.0, point=None):
    return {"text": text, "height": height, "tick": tick, "section": section, "point": point}


CASES = [
    ("a line: the multiples and both ends, every object", LINE, rules(), look(section=5.0, point=3.0)),
    ("a start between multiples, no ends: the end a multiple", LINE, rules(start=15.0, ends=False), look()),
    ("walked from the end: the left is south, the text reads down", LINE, rules(reverse=True), look()),
    ("on the right of an eastward line", LINE, rules(), look(text="right", tick=0.0)),
    ("northward: the left is west, the text turns upright and hangs on its right", NORTH, rules(interval=10.0), look()),
    ("a route with a quarter-turn arc, every 7 m", ROUTE, rules(interval=7.0), look(section=4.0)),
    ("a half-metre interval writes its decimal", LINE, rules(interval=12.5, start=0.25), look(text=None, tick=0.5)),
    ("a closed square: its end is its start", SQUARE, rules(interval=10.0), look()),
    ("a circle every 15 m, the end the start", CIRCLE, rules(interval=15.0, start=1000.0), look(text="right")),
    ("map coordinates, a km beyond the first", FAR, rules(interval=10.0, start=1234.5, decimals=2), look(point=-2.5)),
    ("too many stations", LINE, rules(interval=0.001), look()),
    ("no interval", LINE, rules(interval=0.0), look()),
    ("no route: a point", pc.point((1, 1)), rules(), look()),
]


def cases():
    return {
        "format": "kentos.stationing-cases",
        "version": 1,
        "source": SOURCE,
        "cases": [{"name": n, "shape": e, "rules": r, "look": lk, "expect": written(e, r, lk)} for n, e, r, lk in CASES],
    }


def main():
    text = json.dumps(cases(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            sys.exit("fixtures/stationing/v1/cases.json güncel değil; yeniden yazmak için --check'siz çalıştırın.")
        print("fixtures/stationing/v1/cases.json güncel.")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print("fixtures/stationing/v1/cases.json yazıldı.")


if __name__ == "__main__":
    main()
