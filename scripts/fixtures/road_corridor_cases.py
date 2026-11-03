#!/usr/bin/env python3
"""A road corridor's shared cases (docs/adr/0905, plan/road.md Y7.1), from an independent reference: the regions'
rules; a region's stations from the alignment's elements (scripts/fixtures/road_alignment_cases.py, mpmath at 40
digits) by the ADR's text: the region's ends, the tangent interval's multiples from station 0, each curve and clothoid
divided equally by min(curve interval, 2·√(2Rt − t²)) at its smallest radius, the elements' ends and the events given,
those within 1 mm one, the strongest reason kept; and the feature lines, the strip of triangles and the stretches not
solved from sections written here by hand. No KentOS code.

    uv run --no-project --python 3.11 --with mpmath python3 scripts/fixtures/road_corridor_cases.py
    ... --check   # fails when fixtures/road/v1/corridor.json is not what this writes
"""

import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_alignment_cases as al  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "corridor.json"
P = al.pi
SAME = 1e-3
MAX_STATIONS = 20_000
WHY = ["region", "geometry", "critical", "profile", "user", "curve", "interval"]


# ---- regions ------------------------------------------------------------------------------------------------------

def regions(rs, length):
    for i, (a, b) in enumerate(rs):
        if not (b - a >= SAME):
            return {"error": {"kind": "EmptyRegion", "index": i}}
        if a < -1e-9 or b > length + 1e-9:
            return {"error": {"kind": "Outside", "index": i}}
    order = sorted(range(len(rs)), key=lambda i: (rs[i][0], i))
    gaps = []
    for x, y in zip(order, order[1:]):
        a, b = rs[x], rs[y]
        if b[0] < a[1] - SAME:
            return {"error": {"kind": "Overlap", "first": min(x, y), "second": max(x, y)}}
        if b[0] > a[1] + SAME:
            gaps.append([a[1], b[0]])
    return {"gaps": gaps}


# ---- stations -----------------------------------------------------------------------------------------------------

def step(curve, sag, r):
    if sag is None or not math.isfinite(r) or r <= 0:
        return curve
    chord = 2 * r if sag >= r else 2 * math.sqrt(2 * r * sag - sag * sag)
    return min(curve, chord)


def stations(els, region, tangent, curve, sag, events):
    if not (tangent > 0 and curve > 0 and (sag is None or sag > 0)):
        return {"error": {"kind": "BadFrequency"}}
    a, b = region
    inside = lambda s: a - 1e-9 <= s <= b + 1e-9  # noqa: E731
    out = [(a, 0), (b, 0)]
    for e in els:
        e0, length = float(e.station), float(e.length)
        e1 = e0 + length
        if e1 < a - 1e-9 or e0 > b + 1e-9:
            continue
        out += [(s, 1) for s in (e0, e1) if inside(s)]
        if e.kind == "tangent":
            lo, hi = max(e0, a), min(e1, b)
            out += [(k * tangent, 6) for k in range(math.ceil(lo / tangent), math.floor(hi / tangent) + 1)]
        else:
            k = max(abs(float(e.k0)), abs(float(e.k1)))
            st = step(curve, sag, 1 / k if k > 0 else math.inf)
            n = max(math.ceil(length / st - 1e-9), 1)
            out += [(e0 + length * i / n, 5) for i in range(1, n) if inside(e0 + length * i / n)]
    out += [(s, WHY.index(w)) for s, w in events if inside(s)]
    out = sorted(((min(max(s, a), b), w) for s, w in out if inside(s)))
    kept, anchor = [], -math.inf
    for s, w in out:
        if s - anchor < SAME:
            if w < kept[-1][1]:
                kept[-1] = (s, w)
        else:
            anchor = s
            kept.append((s, w))
    if len(kept) > MAX_STATIONS:
        return {"error": {"kind": "TooMany"}}
    return {"stations": [[s, WHY[w]] for s, w in kept]}


# ---- sections joined ----------------------------------------------------------------------------------------------

def place(cross, o):
    t = cross["direction"]
    return [cross["at"][0] + math.sin(t) * o, cross["at"][1] - math.cos(t) * o]


def codes(comps, ditch, side):
    out = ["axis"] + [f"c{i}" for i in range(1, comps + 1)]
    if side["kind"] == "cut" and ditch is not None:
        out.append("ditchToe")
        if ditch > 0:
            out.append("ditchBottom")
    if len(out) < len(side["points"]):
        out.append("daylight")
    return out[: len(side["points"])]


def coded(t, side, word):
    comps, ditch = t[word]
    pts = [(f"{word}.{c}", p[0], p[1]) for c, p in list(zip(codes(comps, ditch, side), side["points"]))[1:]]
    pts.append((f"{word}.hinge", side["hinge"][0], side["hinge"][1]))
    if side["daylight"] is not None and not any(c.endswith(".daylight") for c, _, _ in pts):
        pts.append((f"{word}.daylight", side["daylight"][0], side["daylight"][1]))
    return pts


def top(cross, t):
    sec = cross["section"]
    def upto(side, n):
        return side["points"][: n + 1] if side["kind"] == "open" else side["points"]
    left = upto(sec["left"], t["left"][0])
    right = upto(sec["right"], t["right"][0])
    return [place(cross, p[0]) + [p[1]] for p in list(reversed(left)) + right[1:]]


SAME_DIAGONAL = 1e-9  # two plan diagonals within this (m) are a tie: a advances


def strip(a, b):
    d = lambda p, q: math.hypot(q[0] - p[0], q[1] - p[1])  # noqa: E731
    i = j = 0
    out = []
    while i + 1 < len(a) or j + 1 < len(b):
        if i + 1 == len(a):
            take = False
        elif j + 1 == len(b):
            take = True
        else:
            take = d(a[i + 1], b[j]) <= d(a[i], b[j + 1]) + SAME_DIAGONAL
        if take:
            out.append([a[i], a[i + 1], b[j]])
            i += 1
        else:
            out.append([a[i], b[j + 1], b[j]])
            j += 1
    return out


def area(t):
    a, b, c = t
    return 0.5 * ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]))


def assemble(regions_):
    features, tris, unsolved, folded = [], [], [], []
    for ri, reg in enumerate(regions_):
        t = reg["template"]
        runs = {}
        for cross in reg["crosses"]:
            s, sec = cross["station"], cross["section"]
            here = {}
            if sec is not None:
                xy = place(cross, 0)
                here["axis"] = [s, xy[0], xy[1], sec["axis"]]
                for word in ("left", "right"):
                    for code, o, z in coded(t, sec[word], word):
                        xy = place(cross, o)
                        here[code] = [s, xy[0], xy[1], z]
            for code in list(runs):
                if code not in here:
                    features.append({"code": code, "region": ri, "points": runs.pop(code)})
            for code, p in here.items():
                runs.setdefault(code, []).append(p)
        features += [{"code": c, "region": ri, "points": p} for c, p in runs.items()]
        for right in (False, True):
            run = None
            for cross in reg["crosses"]:
                s, sec = cross["station"], cross["section"]
                why = "noSection" if sec is None else ("open" if sec["right" if right else "left"]["kind"] == "open" else None)
                if run and why == run[0]:
                    run[2] = s
                    continue
                if run:
                    unsolved.append({"region": ri, "right": right, "why": run[0], "from": run[1], "to": run[2]})
                run = [why, s, s] if why else None
            if run:
                unsolved.append({"region": ri, "right": right, "why": run[0], "from": run[1], "to": run[2]})
        for x, y in zip(reg["crosses"], reg["crosses"][1:]):
            if x["section"] is None or y["section"] is None:
                continue
            got = strip(top(x, t), top(y, t))
            if any(area(g) <= 0 for g in got):
                if folded and folded[-1][0] == ri and abs(folded[-1][2] - x["station"]) < SAME:
                    folded[-1][2] = y["station"]
                else:
                    folded.append([ri, x["station"], y["station"]])
                continue
            tris += got
    key = lambda f: (f["region"], f["code"], f["points"][0][0])  # noqa: E731
    return {"features": sorted(features, key=key), "triangles": tris,
            "unsolved": sorted(unsolved, key=lambda u: (u["region"], u["right"], u["from"])), "folded": folded}


# ---- the cases ----------------------------------------------------------------------------------------------------

STRAIGHT = [P("a", 0, 0), P("b", 300, 0)]
CURVED = [P("a", 0, 0), P("p1", 200, 0, 150, 40, 40), P("b", 330, 180)]
ARC = [P("a", 0, 0), P("p1", 200, 0, 60), P("b", 260, 200)]

REGION_CASES = [
    ("Tek bölge", [[0, 300]], 300),
    ("Bitişik bölgeler boşluksuz", [[0, 100], [100, 300]], 300),
    ("Aradaki boşluk", [[0, 100], [140, 300]], 300),
    ("Sırasız yazılmış bölgeler", [[200, 300], [0, 120]], 300),
    ("Kesişen bölgeler", [[0, 150], [140, 300]], 300),
    ("Boş bölge", [[50, 50]], 300),
    ("Aksın dışına taşan", [[0, 320]], 300),
    ("Ters yazılmış bölge", [[100, 40]], 300),
]

STATION_CASES = [
    ("Aliyman, 20 m", STRAIGHT, [0, 300], 20, 10, None, []),
    ("Aliyman, bölge katların arasında başlar", STRAIGHT, [15, 95], 20, 10, None, []),
    ("Klotoidli kurp, 20 ve 10 m", CURVED, None, 20, 10, None, []),
    ("Klotoidli kurp, sapma toleransı 0,02 m", CURVED, None, 20, 10, 0.02, []),
    ("Dairesel kurp, sapma toleransı kurp aralığından büyük", ARC, None, 25, 25, 1.0, []),
    ("Dairesel kurp, sapma yarıçaptan büyük", ARC, None, 25, 25, 100.0, []),
    ("Olaylar: dever, profil, kullanıcı", CURVED, None, 25, 10, None,
     [[57.3, "critical"], [120.0004, "profile"], [140.0, "user"], [999, "user"]]),
    ("Aralıkla çakışan güçlü neden kalır", STRAIGHT, [0, 100], 20, 10, None, [[40.0005, "profile"], [60.0, "user"]]),
    ("Bölge sınırı ile geometri noktası", CURVED, None, 50, 50, None, []),
    ("Sıfır aralık", STRAIGHT, [0, 100], 0, 10, None, []),
    ("Sıfır tolerans", ARC, None, 20, 10, 0, []),
]


def side(points, hinge, daylight, kind):
    return {"points": points, "hinge": hinge, "daylight": daylight, "kind": kind}


def section(axis, left, right):
    return {"axis": axis, "left": left, "right": right}


def flat(axis, w=3.5, slope=-0.025, fill_to=None):
    """Two lanes and a shoulder per side (3,5 + 3,5 + 1); the fill slope to `fill_to` metres out."""
    def one(sign):
        pts = [[0, axis], [sign * w, axis + w * slope], [sign * 2 * w, axis + 2 * w * slope],
               [sign * (2 * w + 1), axis + 2 * w * slope - 0.04]]
        hinge = pts[-1]
        day = None if fill_to is None else [sign * fill_to, hinge[1] - (fill_to - (2 * w + 1)) / 1.5]
        return side(pts + ([day] if day else []), hinge, day, "fill" if day else "open")
    return section(axis, one(-1), one(1))


def cut_ditch(axis):
    w = 3.5
    def one(sign):
        z1, z2, z3 = axis - 0.0875, axis - 0.175, axis - 0.215
        pts = [[0, axis], [sign * w, z1], [sign * 2 * w, z2], [sign * 8, z3], [sign * 9, z3 - 0.5],
               [sign * 10, z3 - 0.5], [sign * 14, z3 + 3.5]]
        return side(pts, [sign * 8, z3], [sign * 14, z3 + 3.5], "cut")
    return section(axis, one(-1), one(1))


def cross(s, at, direction, sec):
    return {"station": s, "at": at, "direction": direction, "section": sec}


EAST = 0.0
ASSEMBLE_CASES = [
    ("İki kesit, dolgu", [{"template": {"left": [3, None], "right": [3, None]},
      "crosses": [cross(0, [0, 0], EAST, flat(100, fill_to=12)), cross(20, [20, 0], EAST, flat(100.4, fill_to=13))]}]),
    ("Açık yan menteşede biter", [{"template": {"left": [3, None], "right": [3, None]},
      "crosses": [cross(0, [0, 0], EAST, flat(100, fill_to=12)), cross(20, [20, 0], EAST, flat(100, fill_to=None)),
                  cross(40, [40, 0], EAST, flat(100, fill_to=None)), cross(60, [60, 0], EAST, flat(100, fill_to=12))]}]),
    ("Kesitsiz km çizgileri kırar", [{"template": {"left": [3, None], "right": [3, None]},
      "crosses": [cross(0, [0, 0], EAST, flat(100, fill_to=12)), cross(20, [20, 0], EAST, None),
                  cross(40, [40, 0], EAST, flat(100, fill_to=12)), cross(60, [60, 0], EAST, flat(100, fill_to=12))]}]),
    ("Yarmada hendek", [{"template": {"left": [3, 1.0], "right": [3, 1.0]},
      "crosses": [cross(0, [0, 0], EAST, cut_ditch(90)), cross(25, [25, 0], EAST, cut_ditch(90.5)),
                  cross(50, [50, 0], EAST, flat(91, fill_to=12))]}]),
    ("Kuzeye dönen aks", [{"template": {"left": [3, None], "right": [3, None]},
      "crosses": [cross(0, [0, 0], EAST, flat(50, fill_to=11)),
                  cross(20, [19.9, 1.99], 0.2, flat(50.2, fill_to=11)),
                  cross(40, [39.2, 7.9], 0.4, flat(50.4, fill_to=11))]}]),
    ("Dar kurpta kesitler kesişir", [{"template": {"left": [3, None], "right": [3, None]},
      "crosses": [cross(0, [0, 0], EAST, flat(50, fill_to=30)), cross(5, [4.9, 0.5], 0.6, flat(50, fill_to=30)),
                  cross(10, [9, 2.5], 1.2, flat(50, fill_to=30)), cross(15, [11.5, 6], 1.8, flat(50, fill_to=30))]}]),
    ("İki bölge", [{"template": {"left": [3, None], "right": [3, None]},
      "crosses": [cross(0, [0, 0], EAST, flat(100, fill_to=12)), cross(20, [20, 0], EAST, flat(100, fill_to=12))]},
     {"template": {"left": [3, 1.0], "right": [3, 1.0]},
      "crosses": [cross(20, [20, 0], EAST, cut_ditch(100)), cross(40, [40, 0], EAST, cut_ditch(100))]}]),
]


def make():
    out = {"regions": [], "stations": [], "assemble": []}
    for name, rs, length in REGION_CASES:
        out["regions"].append({"name": name, "regions": rs, "length": length, **regions(rs, length)})
    for name, pis, region, t, c, sag, events in STATION_CASES:
        els, length, _, _ = al.build(pis)
        region = region or [0, float(length)]
        out["stations"].append({"name": name, "pis": pis, "region": region, "tangent": t, "curve": c, "sag": sag,
                                "events": events, **stations(els, region, t, c, sag, events)})
    for name, rs in ASSEMBLE_CASES:
        out["assemble"].append({"name": name, "regions": rs, **assemble(rs)})
    return out


def main():
    text = json.dumps(make(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            sys.exit(f"{OUT} is not what {Path(__file__).name} writes")
        print("corridor cases are current")
        return
    OUT.write_text(text)
    print(f"written {OUT}")


if __name__ == "__main__":
    main()
