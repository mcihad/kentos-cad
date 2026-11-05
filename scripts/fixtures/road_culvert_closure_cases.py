#!/usr/bin/env python3
"""The shared cases of a flood's closure of the road over a culvert (docs/adr/0948, plan/road.md Y42; ROAD-DRAIN-08).

Written without KentOS code, and by another way than KentOS': KentOS bisects the total flow on the criterion itself;
here each criterion is first a limit on the flow per metre of road. On the crest the flow passes critical depth, so the
depth is (q²/g)^(1/3) and the speed √(g·depth): a limit on the depth d, the speed v or their product p is the limit
q* = min(p, √g·d^1.5, v³/g) on q. The road carries L·q* at the head h* = (q*/Cd)^(2/3), so the threshold total flow is
the culvert's at the crest + h* plus L·q* — one root of the culvert's headwater, no search over the flow. The
hydraulics are road_culvert_hydraulics_cases.py's (HDS-5 in mpmath at 40 digits); the vehicles' limits Shand et al.
(2011), the hazard classes AIDR 7-3's.

    uv run --no-project --python 3.11 --with mpmath python3 scripts/fixtures/road_culvert_closure_cases.py [--check]
"""

import json
import sys
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_culvert_hydraulics_cases as hy  # noqa: E402

mp.mp.dps = 40
OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "culvert_closure.json"
G = hy.G
VEHICLES = [("smallPassenger", "0.3", "3.0", "0.3"), ("largePassenger", "0.4", "3.0", "0.45"),
            ("large4wd", "0.5", "3.0", "0.6")]
HAZARD = [("0.3", "0.3", "2.0"), ("0.6", "0.5", "2.0"), ("0.6", "1.2", "2.0"), ("1.0", "2.0", "2.0"), ("4.0", "4.0", "4.0")]


def q_limit(d, v, p):
    """The largest flow per metre within the depth d, speed v and product p on a crest at critical flow."""
    d, v, p = mp.mpf(d), mp.mpf(v), mp.mpf(p)
    return min(p, mp.sqrt(G) * d ** mp.mpf("1.5"), v ** 3 / G)


def threshold(c, weir, tw, qstar, peak):
    """The least total flow putting more than q* per metre over the road; None above the peak. q* None: the
    overtopping's (the culvert's flow with the water at the crest)."""
    crest, length, cd = (mp.mpf(weir[k]) for k in ("crest", "length", "cd"))
    if qstar is None:
        q = hy.flow_at(c, crest, tw)
    else:
        h = (qstar / cd) ** (mp.mpf(2) / 3)
        q = hy.flow_at(c, crest + h, tw) + length * qstar
    return q if q < mp.mpf(peak) else None


def above(points, q):
    spans = []
    for (t0, q0), (t1, q1) in zip(points, points[1:]):
        t0, q0, t1, q1 = map(mp.mpf, (t0, q0, t1, q1))
        cross = lambda a, b: t0 + (t1 - t0) * (q - a) / (b - a)  # noqa: E731
        a, b = q0 > q, q1 > q
        piece = (t0, t1) if a and b else (t0, cross(q0, q1)) if a else (cross(q0, q1), t1) if b else None
        if piece and piece[1] > piece[0]:
            if spans and abs(spans[-1][1] - piece[0]) <= mp.mpf("1e-30"):
                spans[-1] = (spans[-1][0], piece[1])
            else:
                spans.append(piece)
    return sum((b - a for a, b in spans), mp.mpf(0)), spans


def flood(c, weir, tw, points, factor, cd):
    scaled = [(t, mp.mpf(q) * mp.mpf(factor)) for t, q in points]
    w = {**weir, "cd": cd}
    peak = max(q for _, q in scaled)

    def span(qstar):
        q = threshold(c, w, tw, qstar, peak)
        if q is None:
            return {"flow": None, "hours": 0.0, "spans": []}
        hours, spans = above(scaled, q)
        return {"flow": float(q), "hours": float(hours), "spans": [[float(a), float(b)] for a, b in spans]}
    p = hy.perform(c, w, peak, tw)
    return {"factor": factor, "cd": cd, "peakFlow": float(peak), "peakHeadwater": float(p["headwater"]),
            "peakOvertopping": float(p["overtopping"]), "peakHazard": p["hazard"],
            "submerged": span(None),
            "vehicles": [span(q_limit(d, v, pp)) for _, d, v, pp in VEHICLES],
            "hazards": [span(q_limit(d, v, pp)) for pp, d, v in HAZARD]}


def cases():
    out = []
    pipe = {"kind": "circle", "diameter": 1.2}
    box = {"kind": "box", "width": 2.0, "height": 1.5}
    for (_, d, v, p), want in zip(VEHICLES, ("0.3", "0.45", "0.6")):
        assert q_limit(d, v, p) == mp.mpf(want), "on a crest the vehicles' product governs"

    def case(name, c, weir, tw, points, factors=(1.0,), cds=None):
        cds = cds or (weir["cd"],)
        floods = [flood(c, weir, tw, points, f, cd) for f, cd in zip(factors, cds)]
        for f in floods:
            hs = [f["submerged"]["hours"]] + [v["hours"] for v in f["vehicles"]]
            assert all(a >= b - 1e-12 for a, b in zip(hs, hs[1:])), (name, "a sturdier vehicle is closed no longer")
        out.append({"name": name, "culvert": c, "weir": weir, "tailwater": tw, "hydrograph": points, "floods": floods})

    road = {"crest": 2.2, "length": 40.0, "cd": 1.66}
    tri = lambda qp, tp, tb, base=0.0: [[0.0, base], [tp, qp], [tb, base]]  # noqa: E731
    pipe_c = hy.culvert(pipe, "pipe-square-headwall")
    box_c = hy.culvert(box, "box-wingwall-30-75", ke=0.4)
    case("Boru, üçgen hidrograf, yol kısa süre su altında", pipe_c, road, 0.5, tri(8.0, 2.0, 6.0))
    case("Boru, büyük taşkın: binek araçlar kapalı", pipe_c, road, 0.5, tri(30.0, 2.0, 8.0))
    case("Boru, dar çukur: 4×4 de kapalı", pipe_c, {**road, "length": 8.0}, 0.5, tri(15.0, 1.5, 6.0))
    case("Aşmayan taşkın", pipe_c, road, 0.5, tri(3.0, 1.0, 4.0))
    case("Taban debili hidrograf", pipe_c, road, 0.5, tri(20.0, 3.0, 12.0, 1.0))
    case("İki tepeli hidrograf", pipe_c, road, 0.5, [[0.0, 0.0], [1.0, 18.0], [2.0, 4.0], [3.0, 25.0], [6.0, 0.0]])
    case("Tepe düzlüğü", pipe_c, road, 0.5, [[0.0, 0.0], [1.0, 22.0], [3.0, 22.0], [5.0, 0.0]])
    case("Kutu menfez, uzun taşkın", box_c, {"crest": 2.6, "length": 80.0, "cd": 1.66}, 1.0, tri(60.0, 6.0, 24.0))
    case("Debi ve Cd aralığı", pipe_c, road, 0.5, tri(30.0, 2.0, 8.0), (0.8, 1.0, 1.2), (1.5, 1.66, 1.8))
    case("Dar aralık, kutu", box_c, {"crest": 2.6, "length": 80.0, "cd": 1.66}, 1.0, tri(60.0, 6.0, 24.0),
         (0.9, 1.0, 1.1), (1.6, 1.66, 1.7))
    case("Yüksek kuyruk suyu", pipe_c, road, 1.8, tri(20.0, 2.0, 8.0))
    case("Başta yüksek: ilk nokta eşiğin üstünde", pipe_c, road, 0.5, [[0.0, 20.0], [4.0, 0.0]])
    return out


def main():
    data = {"vehicles": [{"name": n, "depth": float(d), "speed": float(v), "product": float(p)}
                         for n, d, v, p in VEHICLES], "cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            sys.exit(f"{OUT} is not what {Path(__file__).name} writes")
        print(f"{OUT}: {len(data['cases'])} cases")
        return
    OUT.write_text(text)
    print(f"wrote {OUT}: {len(data['cases'])} cases")
    for c in data["cases"]:
        for f in c["floods"]:
            v = " ".join(f"{x['hours']:.2f}" for x in f["vehicles"])
            hz = " ".join(f"{x['hours']:.2f}" for x in f["hazards"])
            print(f"  {c['name'][:40]:40} ×{f['factor']} Cd {f['cd']} tepe {f['peakFlow']:.1f} H{f['peakHazard']} "
                  f"su altında {f['submerged']['hours']:.2f} sa (Q≥{f['submerged']['flow'] or 0:.2f}) araçlar {v} | H2–H6 {hz}")


if __name__ == "__main__":
    main()
