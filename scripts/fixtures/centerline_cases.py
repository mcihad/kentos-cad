#!/usr/bin/env python3
"""Orta hat (docs/adr/0190): the shared cases, written from the ADR's rules
without KentOS code, at 50 digits with mpmath.

- The second side runs the first's way: walked from its end when its ends
  meet the first's crossed nearer than straight.
- Matched sides (as many edges, each pair parallel lines or concentric
  arcs of the same sweep): the centreline edge by edge, its vertices the
  middles of the sides' vertices, its arcs their bulges.
- Otherwise the sides are matched by the share of their lengths and the
  centreline goes through the middles: at the shares of both sides'
  vertices, its ends and every step along the longer side.

The routes' edges are the point calculator's reference's
(`point_calc_cases.py`, ADR 0188). The core runs the cases natively and
through WASM (crates/shared/geometry-core/tests/all/centerline.rs,
apps/web/src/tools/centerline.wasm.test.ts).

    python3 scripts/fixtures/centerline_cases.py          # write
    python3 scripts/fixtures/centerline_cases.py --check  # compare
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
OUT = ROOT / "fixtures" / "centerline" / "v1" / "cases.json"
SOURCE = "scripts/fixtures/centerline_cases.py (docs/adr/0190)"
PI = mp.pi
MOST = 100000
NO_ROUTE = "Bu nesnenin üzerinde yürünecek tek bir yolu yok: çizgi, çoklu çizgi, yay, daire, elips, eğri ya da tek parçalı alan seçin."
CLOSED = "Kapalı yolun orta hattı çizilmez; iki açık kenar seçin."
BAD_STEP = "Adım sıfırdan büyük bir uzunluk olmalı."


def walk(e, from_end):
    edges = pc.walk(e, from_end)
    return edges, sum(pc.length(ed) for ed in edges)


def at(edges, s):
    acc = mp.mpf(0)
    for i, ed in enumerate(edges):
        l = pc.length(ed)
        if s < acc + l or i == len(edges) - 1:
            return pc.at_edge(ed, min(mp.mpf(1), max(mp.mpf(0), (s - acc) / l)))[0]
        acc += l


def dist(p, q):
    return mp.sqrt((p[0] - q[0]) ** 2 + (p[1] - q[1]) ** 2)


def ends(edges):
    first = pc.at_edge(edges[0], mp.mpf(0))[0]
    last = pc.at_edge(edges[-1], mp.mpf(1))[0]
    return first, last


def matched(ea, eb):
    """The centreline edge by edge, or None when the sides do not match."""
    if len(ea) != len(eb):
        return None
    bulges = []
    for a, b in zip(ea, eb):
        if a[0] == "seg" and b[0] == "seg":
            da = (a[2][0] - a[1][0], a[2][1] - a[1][1])
            db = (b[2][0] - b[1][0], b[2][1] - b[1][1])
            la, lb = mp.sqrt(da[0] ** 2 + da[1] ** 2), mp.sqrt(db[0] ** 2 + db[1] ** 2)
            cross = (da[0] * db[1] - da[1] * db[0]) / (la * lb)
            dot = (da[0] * db[0] + da[1] * db[1]) / (la * lb)
            if abs(cross) > mp.mpf("1e-9") or dot <= 0:
                return None
            bulges.append(mp.mpf(0))
        elif a[0] == "arc" and b[0] == "arc":
            _, ca, ra, a0a, sa = a
            _, cb, rb, a0b, sb = b
            if dist(ca, cb) > mp.mpf("1e-3") or abs(sa - sb) > mp.mpf("1e-9"):
                return None
            bulges.append(mp.tan(sa / 4))
        else:
            return None
    pts = []
    for k in range(len(ea) + 1):
        pa = pc.at_edge(ea[min(k, len(ea) - 1)], mp.mpf(0 if k < len(ea) else 1))[0]
        pb = pc.at_edge(eb[min(k, len(eb) - 1)], mp.mpf(0 if k < len(eb) else 1))[0]
        pts.append(((pa[0] + pb[0]) / 2, (pa[1] + pb[1]) / 2))
    return pts, bulges


def sampled(ea, la, eb, lb, step):
    lmax = max(la, lb)
    ts = {mp.mpf(0), mp.mpf(1)}
    acc = mp.mpf(0)
    for ed in ea[:-1]:
        acc += pc.length(ed)
        ts.add(acc / la)
    acc = mp.mpf(0)
    for ed in eb[:-1]:
        acc += pc.length(ed)
        ts.add(acc / lb)
    k = 1
    while k * step < lmax:
        ts.add(k * step / lmax)
        k += 1
    kept = []
    for t in sorted(ts):
        if kept and t - kept[-1] <= mp.mpf("1e-12"):
            continue
        kept.append(t)
    pts = []
    for t in kept:
        pa, pb = at(ea, t * la), at(eb, t * lb)
        pts.append(((pa[0] + pb[0]) / 2, (pa[1] + pb[1]) / 2))
    return pts


def centerline(a, b, step):
    ea, la = walk(a, False)
    eb, lb = walk(b, False)
    if not ea or not eb:
        return {"problem": NO_ROUTE}
    if pc.edges_of(a)[1] or pc.edges_of(b)[1]:
        return {"problem": CLOSED}
    step = mp.mpf(step)
    if not step > 0:
        return {"problem": BAD_STEP}
    a0, a1 = ends(ea)
    b0, b1 = ends(eb)
    if dist(a0, b1) + dist(a1, b0) < dist(a0, b0) + dist(a1, b1):
        eb, lb = walk(b, True)
    if max(la, lb) / step > MOST:
        return {"problem": "Bu adımla çok nokta olur; adımı büyütün."}
    m = matched(ea, eb)
    if m:
        pts, bulges = m
        return {"method": "matched", "pts": [[float(p[0]), float(p[1])] for p in pts], "bulges": [float(x) for x in bulges]}
    pts = sampled(ea, la, eb, lb, step)
    return {"method": "sampled", "pts": [[float(p[0]), float(p[1])] for p in pts], "bulges": None}


# ── The cases ───────────────────────────────────────────────────────────────

T8 = math.tan(math.pi / 8)
R = 10 * math.sqrt(2)
def side(d):
    """A road's side d to the left of the axis (-40,-10) → (-20,-10) ⌒ (0,-10): its arc about (-10, 0) of
    radius 10√2 − d, its straight level with the arc's start."""
    k = (R - d) / R
    c = (-10.0, 0.0)
    p1 = (c[0] + (-20.0 - c[0]) * k, c[1] + (-10.0 - c[1]) * k)
    p2 = (c[0] + (0.0 - c[0]) * k, c[1] + (-10.0 - c[1]) * k)
    return pc.polyline([(-40.0, p1[1]), p1, p2], [0, T8])


CASES = [
    ("parallel lines: their middle", pc.line((0, 0), (40, 0)), pc.line((0, 6), (40, 6)), 1.0),
    ("the second side drawn the other way", pc.line((0, 0), (40, 0)), pc.line((40, 6), (0, 6)), 1.0),
    ("parallel lines of different lengths: still on the middle line", pc.line((0, 0), (40, 0)), pc.line((5, 6), (35, 6)), 1.0),
    ("a road's sides: parallel straights and concentric arcs, edge by edge", side(3.0), side(-3.0), 1.0),
    ("a converging bank: sampled every 5 m", pc.line((0, 0), (40, 0)), pc.polyline([(0, 8), (20, 6), (40, 4)]), 5.0),
    ("a line and an arc: sampled", pc.line((-10, 0), (10, 0)), pc.arc((0, 0), 10, 0.2, 2.9), 2.0),
    ("map coordinates", pc.polyline([(487000.0, 4420000.0), (487030.0, 4420010.0)]), pc.polyline([(487001.0, 4420004.0), (487029.5, 4420013.5)]), 2.5),
    ("a closed side is refused", pc.line((0, 0), (40, 0)), pc.polygon([(0, 6), (40, 6), (40, 12)]), 1.0),
    ("no route", pc.line((0, 0), (40, 0)), pc.point((1, 1)), 1.0),
    ("no step", pc.line((0, 0), (40, 0)), pc.line((0, 6), (40, 6)), 0.0),
    ("too many points", pc.line((0, 0), (40, 0)), pc.polyline([(0, 8), (40, 4)]), 0.0001),
]


def cases():
    return {
        "format": "kentos.centerline-cases",
        "version": 1,
        "source": SOURCE,
        "cases": [{"name": n, "a": a, "b": b, "step": st, "expect": centerline(a, b, st)} for n, a, b, st in CASES],
    }


def main():
    text = json.dumps(cases(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            sys.exit("fixtures/centerline/v1/cases.json güncel değil; yeniden yazmak için --check'siz çalıştırın.")
        print("fixtures/centerline/v1/cases.json güncel.")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print("fixtures/centerline/v1/cases.json yazıldı.")


if __name__ == "__main__":
    main()
