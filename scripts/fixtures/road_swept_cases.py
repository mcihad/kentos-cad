#!/usr/bin/env python3
"""The shared cases of `road.swept.query` and `.draw` (plan/road.md Y17, docs/adr/0915; a design vehicle's swept path).

The drawing is road_junction_cases.py's (its junctions; a junction's curb returns are that reference's own answer in
road.junctions.query.json) with a few lines, polylines, an arc, a circle and a closed island of this script.

The kinematics are the ADR's, written here again without KentOS code:

- the front axle's centre runs along the path's chords, placed every `step` along each chord (n = ⌈len/step⌉ equal
  pieces); arcs are taken as chords: n = ⌈|θ|·r/step⌉ equal angles (a polyline's bulge b has θ = 4·atan b, its centre
  (c/2)(1 − b²)/(2b) left of the chord's middle);
- the rear axle follows each chord by the exact tractrix: with u the chord's direction, φ the bar's angle to u,
  tan(φ/2) ← tan(φ/2)·e^(−s/L); a trailer's axle follows the kingpin (on the tractor's axis, `kingpin` behind the rear
  axle) the same way over the kingpin's chords; at the start everything is aligned with the first chord;
- the bodies are rectangles from `rear` behind the rear axle (a trailer's axle) to `front` ahead of the front axle (the
  kingpin), `width` wide; their corners front left, front right, rear right, rear left;
- off-tracking: an axle's distance to the path with its lead-in (the first chord's line `reach` = L (+ |kingpin| +
  trailer's wheelbase) behind the start), the stretch [d − 2·reach − step, d + step] of it;
- clearance: the least distance of a body's edges to an obstacle's segments, zero where they meet or a point of the
  obstacle is inside; the first least in sample, body, obstacle order.

A junction's quadrant: the front at the return's radius + offset round its centre from the first tangent point's angle
through the sweep, `approach` straight before and after along the arc's tangents; the curb the same at the radius.

    python3 scripts/fixtures/road_swept_cases.py [--check]
"""

import bisect
import copy
import json
import math
import sys
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
W = 1e-6
ANY = "$any"
UID = "0192f5d0-7c3e-7d4a-9b1e-{:012d}"


def completed(output, warns=()):
    return {"status": "completed", "output": output, "warnings": warns if warns == ANY else list(warns)}


def failed(code, message, path=None):
    e = {"code": code, "message": message}
    if path:
        e["path"] = path
    return {"status": "failed", "error": e}


UNCHANGED = {"canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
STEPPED = {"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}
CONFLICT = {"status": "conflict", "error": {
    "code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu "
    "çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision", "revision": "$current"}}
BAD_REVISION = failed("invalid_revision", "Beklenen sürüm “on” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı "
                      "yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision")
NOT_ROAD = failed("not_a_road_project", "Kavşak yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")


def step(op, inp, result, expect):
    return {"op": op, "input": inp, "result": result, "expect": expect, "within": W}


# ── Geometry ──────────────────────────────────────────────────────────────────────────────────────────────────────
def sub(a, b):
    return [a[0] - b[0], a[1] - b[1]]


def norm(a):
    return math.hypot(a[0], a[1])


def unit(a):
    n = norm(a)
    return [a[0] / n, a[1] / n]


def arc_places(c, r, start, sweep, step_):
    n = max(1, math.ceil(abs(sweep) * r / step_))
    return [[c[0] + r * math.cos(start + sweep * k / n), c[1] + r * math.sin(start + sweep * k / n)] for k in range(n + 1)]


def bulge_places(a, b, bulge, step_):
    if bulge == 0.0:
        return [a, b]
    dx, dy = b[0] - a[0], b[1] - a[1]
    c = math.hypot(dx, dy)
    h = c / 2.0 * (1.0 - bulge * bulge) / (2.0 * bulge)
    centre = [(a[0] + b[0]) / 2.0 - dy / c * h, (a[1] + b[1]) / 2.0 + dx / c * h]
    r = math.hypot(a[0] - centre[0], a[1] - centre[1])
    start = math.atan2(a[1] - centre[1], a[0] - centre[0])
    out = arc_places(centre, r, start, 4.0 * math.atan(bulge), step_)
    out[-1] = b
    return out


def polyline_places(pts, bulges, closed, step_):
    out = [pts[0]]
    n = len(pts)
    for i in range(n if closed else n - 1):
        b = bulges[i] if i < len(bulges) else 0.0
        out += bulge_places(pts[i], pts[(i + 1) % n], b, step_)[1:]
    return out


def follow(r, f, g, l):
    d = sub(g, f)
    s = norm(d)
    if s == 0.0:
        return r
    u = [d[0] / s, d[1] / s]
    n = [-u[1], u[0]]
    b = sub(f, r)
    c, sn = (b[0] * u[0] + b[1] * u[1]) / l, (b[0] * n[0] + b[1] * n[1]) / l
    phi = math.atan2(sn, c)
    half = math.tan(phi / 2.0) * math.exp(-s / l)
    phi2 = 2.0 * math.atan(half) if math.isfinite(half) else phi
    return [g[0] - l * (math.cos(phi2) * u[0] + math.sin(phi2) * n[0]),
            g[1] - l * (math.cos(phi2) * u[1] + math.sin(phi2) * n[1])]


def body(back, ahead, width):
    h = unit(sub(ahead, back))
    n = [-h[1] * width / 2.0, h[0] * width / 2.0]
    return [[ahead[0] + n[0], ahead[1] + n[1]], [ahead[0] - n[0], ahead[1] - n[1]],
            [back[0] - n[0], back[1] - n[1]], [back[0] + n[0], back[1] + n[1]]]


def resample(path, step_):
    out = [(0.0, path[0])]
    at = 0.0
    for a, b in zip(path, path[1:]):
        ln = norm(sub(b, a))
        if ln == 0.0:
            continue
        n = max(1, math.ceil(ln / step_))
        for k in range(1, n + 1):
            t = k / n
            out.append((at + t * ln, [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]))
        at += ln
    return out


def seg_point(a, b, p):
    d, w = sub(b, a), sub(p, a)
    l2 = d[0] * d[0] + d[1] * d[1]
    t = min(1.0, max(0.0, (w[0] * d[0] + w[1] * d[1]) / l2)) if l2 > 0 else 0.0
    return math.hypot(p[0] - a[0] - t * d[0], p[1] - a[1] - t * d[1])


def dedup(ps):
    out = []
    for p in ps:
        if not out or out[-1] != p:
            out.append(p)
    return out


def drive(v, path, step_):
    places = dedup(path)
    L = v["wheelbase"]
    tr = v.get("trailer")
    first = unit(sub(places[1], places[0]))
    f = places[0]
    r = [f[0] - L * first[0], f[1] - L * first[1]]

    def kingpin(f_, r_, k):
        h = unit(sub(f_, r_))
        return [r_[0] - k * h[0], r_[1] - k * h[1]]

    ta = None
    if tr:
        k = kingpin(f, r, tr["kingpin"])
        ta = [k[0] - tr["wheelbase"] * first[0], k[1] - tr["wheelbase"] * first[1]]

    def sample(d, f_, r_, ta_):
        h = unit(sub(f_, r_))
        bodies = [body([r_[0] - v["rear"] * h[0], r_[1] - v["rear"] * h[1]],
                       [f_[0] + v["front"] * h[0], f_[1] + v["front"] * h[1]], v["width"])]
        if tr:
            k = kingpin(f_, r_, tr["kingpin"])
            ht = unit(sub(k, ta_))
            bodies.append(body([ta_[0] - tr["rear"] * ht[0], ta_[1] - tr["rear"] * ht[1]],
                               [k[0] + tr["front"] * ht[0], k[1] + tr["front"] * ht[1]], tr["width"]))
        return {"d": d, "front": f_, "rear": r_, "trailer": ta_, "bodies": bodies}

    samples = [sample(0.0, f, r, ta)]
    for d, g in resample(places, step_)[1:]:
        r2 = follow(r, f, g, L)
        if tr:
            k1, k2 = kingpin(f, r, tr["kingpin"]), kingpin(g, r2, tr["kingpin"])
            ta = follow(ta, k1, k2, tr["wheelbase"])
        f, r = g, r2
        samples.append(sample(d, f, r, ta))
    reach = L + (abs(tr["kingpin"]) + tr["wheelbase"] if tr else 0.0)
    line = [[places[0][0] - reach * first[0], places[0][1] - reach * first[1]]] + places
    along = [-reach, 0.0]
    for a, b in zip(places, places[1:]):
        along.append(along[-1] + norm(sub(b, a)))

    def off(p, d):
        lo, hi = d - 2.0 * reach - step_, d + step_
        start = max(0, bisect.bisect_left(along, lo) - 1)
        best = math.inf
        for k in range(start, len(line) - 1):
            if along[k] > hi:
                break
            best = min(best, seg_point(line[k], line[k + 1], p))
        return best

    offt = max([0.0] + [off(s["rear"], s["d"]) for s in samples])
    toff = max([0.0] + [off(s["trailer"], s["d"]) for s in samples]) if tr else None
    return samples, offt, toff


def orient(p, q, r):
    return (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])


def on(p, q, r):
    return min(p[0], q[0]) <= r[0] <= max(p[0], q[0]) and min(p[1], q[1]) <= r[1] <= max(p[1], q[1])


def meet(a, b, c, d):
    d1, d2, d3, d4 = orient(c, d, a), orient(c, d, b), orient(a, b, c), orient(a, b, d)
    if d1 != 0 and d2 != 0 and d3 != 0 and d4 != 0:
        return (d1 > 0) != (d2 > 0) and (d3 > 0) != (d4 > 0)
    return (d1 == 0 and on(c, d, a)) or (d2 == 0 and on(c, d, b)) or (d3 == 0 and on(a, b, c)) or \
        (d4 == 0 and on(a, b, d))


def seg_seg(a, b, c, d):
    if meet(a, b, c, d):
        return 0.0
    return min(seg_point(a, b, c), seg_point(a, b, d), seg_point(c, d, a), seg_point(c, d, b))


def inside(rect, p):
    return all(orient(rect[i], rect[(i + 1) % 4], p) <= 0 for i in range(4))


def clearance(samples, lines):
    best = None
    for k, s in enumerate(samples):
        for j, b in enumerate(s["bodies"]):
            for o, line in enumerate(lines):
                d = math.inf
                for a, c in zip(line, line[1:]):
                    for i in range(4):
                        d = min(d, seg_seg(b[i], b[(i + 1) % 4], a, c))
                if any(inside(b, p) for p in line):
                    d = 0.0
                if math.isfinite(d) and (best is None or d < best[0]):
                    best = (d, k, j, o)
    return best


# ── The drawing ──────────────────────────────────────────────────────────────────────────────────────────────────
JUNCTIONS = json.loads((OUT / "road.junctions.query.json").read_text())
RETURNS = {j["id"]: j for j in JUNCTIONS["cases"][0]["steps"][0]["result"]["output"]["junctions"]}

QB = math.tan(math.pi / 8.0)
OBJECTS = {
    "line": {"kind": "line", "a": {"x": 1000.0, "y": 0.0}, "b": {"x": 1030.0, "y": 0.0}},
    "ell": {"kind": "polyline", "pts": [{"x": 1000.0, "y": 100.0}, {"x": 1030.0, "y": 100.0}, {"x": 1030.0, "y": 130.0}],
            "bulges": [0.0, 0.0, 0.0]},
    "bend": {"kind": "polyline", "pts": [{"x": 1000.0, "y": 200.0}, {"x": 1020.0, "y": 200.0}, {"x": 1030.0, "y": 210.0},
                                         {"x": 1030.0, "y": 230.0}], "bulges": [0.0, QB, 0.0, 0.0]},
    "arc": {"kind": "arc", "c": {"x": 1100.0, "y": 0.0}, "r": 20.0, "a0": 0.0, "a1": math.pi / 2.0},
    "curb": {"kind": "line", "a": {"x": 995.0, "y": 3.0}, "b": {"x": 1040.0, "y": 3.0}},
    "post": {"kind": "circle", "c": {"x": 1033.0, "y": 96.0}, "r": 0.5},
    "island": {"kind": "polyline", "pts": [{"x": 1026.0, "y": 104.0}, {"x": 1028.0, "y": 104.0},
                                           {"x": 1028.0, "y": 106.0}, {"x": 1026.0, "y": 106.0}],
               "bulges": [0.0, 0.0, 0.0, 0.0], "closed": True},
    "ring": {"kind": "polyline", "pts": [{"x": 900.0, "y": 0.0}, {"x": 910.0, "y": 0.0}, {"x": 910.0, "y": 10.0}],
             "bulges": [0.0, 0.0, 0.0], "closed": True},
    "dot": {"kind": "point", "p": {"x": 950.0, "y": 0.0}},
    "round": {"kind": "circle", "c": {"x": 950.0, "y": 50.0}, "r": 5.0},
}
UIDS = {}


def setup(work="road"):
    s = copy.deepcopy(JUNCTIONS["setup"])
    base = {"type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}
    s["layers"].append({"id": "arac", "name": "Araç izi", **copy.deepcopy(base)})
    s["layers"].append({"id": "cizim", "name": "Çizim", **copy.deepcopy(base)})
    k = max(e["id"] for e in s["entities"])
    for name, e in OBJECTS.items():
        k += 1
        s["entities"].append({**copy.deepcopy(e), "id": k, "layerId": "cizim", "attrs": {}})
        UIDS[name] = UID.format(k)
        s["uids"].append(UIDS[name])
    s["settings"]["cadWork"] = work
    if work != "road":
        del s["settings"]["road"]
    return s


def obj_path(name, step_):
    e = OBJECTS[name]
    if e["kind"] == "line":
        return [[e["a"]["x"], e["a"]["y"]], [e["b"]["x"], e["b"]["y"]]]
    if e["kind"] == "polyline":
        return polyline_places([[p["x"], p["y"]] for p in e["pts"]], e["bulges"], e.get("closed", False), step_)
    if e["kind"] == "arc":
        sw = (e["a1"] - e["a0"]) % (2 * math.pi) or 2 * math.pi
        return arc_places([e["c"]["x"], e["c"]["y"]], e["r"], e["a0"], sw, step_)
    if e["kind"] == "circle":
        return arc_places([e["c"]["x"], e["c"]["y"]], e["r"], 0.0, 2 * math.pi, step_)
    raise ValueError(name)


def quadrant(jid, q, offset, approach, step_):
    r = RETURNS[jid]["returns"][q]
    c = [r["cx"], r["cy"]]
    start = math.atan2(r["onA"]["y"] - r["cy"], r["onA"]["x"] - r["cx"])
    sweep = math.radians(r["sweep"])
    turn = math.copysign(1.0, sweep)

    def along(a):
        return [-turn * math.sin(a), turn * math.cos(a)]

    ta, tb = along(start), along(start + sweep)

    def run(radius):
        arc = arc_places(c, radius, start, sweep, step_)
        out = [[arc[0][0] - ta[0] * approach, arc[0][1] - ta[1] * approach]] + arc + \
              [[arc[-1][0] + tb[0] * approach, arc[-1][1] + tb[1] * approach]]
        return dedup(out)

    return run(r["radius"] + offset), run(r["radius"])


CAR = {"wheelbase": 2.7, "front": 0.9, "rear": 1.0, "width": 1.8}
TRUCK = {"name": "Kamyon", "wheelbase": 6.1, "front": 1.2, "rear": 1.8, "width": 2.6}
SEMI = {"name": "Çekici römork", "wheelbase": 3.8, "front": 1.4, "rear": 0.9, "width": 2.5,
        "trailer": {"kingpin": 0.3, "wheelbase": 10.0, "front": 1.0, "rear": 3.0, "width": 2.5}}
AHEAD = {"wheelbase": 3.6, "front": 1.2, "rear": 1.2, "width": 2.5,
         "trailer": {"kingpin": -0.5, "wheelbase": 7.5, "front": 1.5, "rear": 2.0, "width": 2.5}}


def answer(inp):
    """The output and warnings of a query that reads."""
    step_ = inp["step"]
    curb = None
    if "junction" in inp:
        j = inp["junction"]
        jid = next(i for i, x in RETURNS.items() if i == j["junction"] or x["name"].lower() == j["junction"].lower())
        path, cline = quadrant(jid, j["quadrant"], j["offset"], j["approach"], step_)
        curb = (jid, cline)
    elif "uid" in inp:
        name = next(n for n, u in UIDS.items() if u == inp["uid"])
        path = obj_path(name, step_)
    else:
        path = inp["path"]
    if inp.get("reverse"):
        path = path[::-1]
    lines, owner = [], []
    for uid in inp.get("obstacles", []):
        name = next(n for n, u in UIDS.items() if u == uid)
        lines.append(obj_path(name, step_))
        owner.append((uid, False))
    if curb:
        lines.append(curb[1])
        owner.append((curb[0], True))
    samples, offt, toff = drive(inp["vehicle"], path, step_)
    units = len(samples[0]["bodies"])
    out = {"length": samples[-1]["d"], "samples": len(samples), "front": [s["front"] for s in samples],
           "rear": [s["rear"] for s in samples]}
    if inp["vehicle"].get("trailer"):
        out["trailer"] = [s["trailer"] for s in samples]
    out["corners"] = [[s["bodies"][j][i] for s in samples] for j in range(units) for i in range(4)]
    out["offtracking"] = offt
    if toff is not None:
        out["trailerOfftracking"] = toff
    warns = []
    c = clearance(samples, lines)
    if c:
        d, k, j, o = c
        obstacle, is_curb = owner[o]
        s = samples[k]
        if d == 0.0:
            who = "kavşağın bordürüne" if is_curb else f"“{obstacle}” engeline"
            warns.append({"code": "swept_touches", "message": f"Aracın {'çekici' if j == 0 else 'römork'} gövdesi, yolun "
                          f"{s['d']:.2f} m'sinde {who} değiyor.", "path": "junction" if is_curb else "obstacles"})
        out["clearance"] = {"distance": d, "along": s["d"], "at": s["front"], "body": j, "obstacle": obstacle,
                            "curb": is_curb}
    return out, warns


def invalid(message, path=None):
    return failed("invalid_swept", message, path)


def cases():
    out = []

    def q(name, inp, result=None, setup_=None):
        if result is None:
            o, w = answer(inp)
            result = completed(o, w)
        c = {"name": name, "steps": [step("execute", inp, result, UNCHANGED)]}
        if setup_:
            c["setup"] = setup_
        out.append(c)

    J = {"junction": "j-1", "quadrant": 0, "offset": 1.5, "approach": 10.0}
    q("Düz çizgi ve bordür", {"vehicle": CAR, "uid": UIDS["line"], "step": 1.0, "obstacles": [UIDS["curb"]]})
    q("Noktalarla dik dönüş", {"vehicle": TRUCK, "path": [[0.0, 0.0], [20.0, 0.0], [20.0, 20.0]], "step": 1.0})
    q("Çoklu çizgi ve direk", {"vehicle": TRUCK, "uid": UIDS["ell"], "step": 1.0, "obstacles": [UIDS["post"]]})
    q("Yaylı çoklu çizgi", {"vehicle": CAR, "uid": UIDS["bend"], "step": 1.0})
    q("Yay", {"vehicle": TRUCK, "uid": UIDS["arc"], "step": 0.5})
    q("Sondan sürülür", {"vehicle": TRUCK, "uid": UIDS["ell"], "step": 2.0, "reverse": True})
    q("Çekici ve römork", {"vehicle": SEMI, "path": [[0.0, 0.0], [30.0, 0.0], [30.0, 30.0]], "step": 1.0})
    q("Mafsalı arka dingilin önünde", {"vehicle": AHEAD, "uid": UIDS["ell"], "step": 2.0, "obstacles": [UIDS["post"]]})
    q("Kavşağın birinci çeyreği", {"vehicle": TRUCK, "junction": J, "step": 1.0})
    q("Geniş dönüşte açıklık", {"vehicle": TRUCK, "junction": {**J, "quadrant": 1, "offset": 3.5}, "step": 1.0})
    q("Üçüncü çeyrek ters yönde", {"vehicle": CAR, "junction": {**J, "quadrant": 2}, "step": 1.0, "reverse": True})
    q("Bordüre değen araç", {"vehicle": SEMI, "junction": {**J, "quadrant": 3, "offset": 0.5}, "step": 1.0})
    q("Kavşak adıyla", {"vehicle": CAR, "junction": {**J, "junction": "ana kavşak"}, "step": 2.0})
    q("Ada engeline değer", {"vehicle": TRUCK, "uid": UIDS["ell"], "step": 2.0, "obstacles": [UIDS["island"], UIDS["post"]]})
    q("Kapalı çoklu çizgi ve daire engel", {"vehicle": CAR, "uid": UIDS["line"], "step": 1.0,
                                            "obstacles": [UIDS["ring"], UIDS["round"]]})
    q("Yol projesi olmadan çizgiyle", {"vehicle": CAR, "uid": UIDS["line"], "step": 2.0}, setup_="$map")
    # Refusals.
    one = "Tek bir yol verin: path (noktalar), uid (çizimdeki nesne) ya da junction (kavşağın çeyreği)."
    q("Yol yok", {"vehicle": CAR, "step": 1.0}, invalid(one, "path"))
    q("İki yol", {"vehicle": CAR, "uid": UIDS["line"], "path": [[0.0, 0.0], [1.0, 0.0]], "step": 1.0}, invalid(one, "path"))
    q("Genişliği sıfır araç", {"vehicle": {**CAR, "width": 0.0}, "uid": UIDS["line"], "step": 1.0},
      invalid("Aracın dingil aralığı ve genişliği sıfırdan büyük, çıkıntıları eksi olmayan sonlu sayılar olmalı (römorkun "
              "da).", "vehicle"))
    q("Römorkun dingil aralığı eksi", {"vehicle": {**SEMI, "trailer": {**SEMI["trailer"], "wheelbase": -1.0}},
                                       "uid": UIDS["line"], "step": 1.0},
      invalid("Aracın dingil aralığı ve genişliği sıfırdan büyük, çıkıntıları eksi olmayan sonlu sayılar olmalı (römorkun "
              "da).", "vehicle"))
    q("Aralık sıfır", {"vehicle": CAR, "uid": UIDS["line"], "step": 0.0},
      invalid("Örnekleme aralığı (step) sıfırdan büyük olmalı.", "step"))
    q("Beşinci çeyrek", {"vehicle": CAR, "junction": {**J, "quadrant": 4}, "step": 1.0},
      invalid("Kavşağın çeyreği 0 ile 3 arası olmalı.", "junction.quadrant"))
    q("Eksi uzaklık", {"vehicle": CAR, "junction": {**J, "offset": -1.0}, "step": 1.0},
      invalid("Bordürden uzaklık (offset) eksi olmayan bir sayı olmalı.", "junction.offset"))
    q("Tek yerli yol", {"vehicle": CAR, "path": [[5.0, 5.0], [5.0, 5.0]], "step": 1.0},
      invalid("Yolun en az iki ayrı yeri olmalı.", "path"))
    q("Kavşak yol projesinde", {"vehicle": CAR, "junction": J, "step": 1.0}, NOT_ROAD, setup_="$map")
    q("Olmayan kavşak", {"vehicle": CAR, "junction": {**J, "junction": "j-9"}, "step": 1.0},
      failed("unknown_junction", "“j-9” kimlikli ya da adlı kavşak projede yok.", "junction.junction"))
    q("Katlı kavşakta dönüş yok", {"vehicle": CAR, "junction": {**J, "junction": "j-2"}, "step": 1.0},
      failed("invalid_junction", "“Üst geçit” kavşağının bordür dönüşü yok (katlı kavşak); araç izi çeyreğin dönüşünde "
             "sürülür.", "junction.junction"))
    q("Olmayan nesne", {"vehicle": CAR, "uid": "0192f5d0-7c3e-7d4a-9b1e-999999999999", "step": 1.0},
      failed("entity_not_found", "“0192f5d0-7c3e-7d4a-9b1e-999999999999” kimlikli nesne çizimde yok.", "uid"))
    q("Olmayan engel", {"vehicle": CAR, "uid": UIDS["line"], "step": 1.0,
                        "obstacles": [UIDS["curb"], "0192f5d0-7c3e-7d4a-9b1e-999999999998"]},
      failed("entity_not_found", "“0192f5d0-7c3e-7d4a-9b1e-999999999998” kimlikli nesne çizimde yok.", "obstacles[1]"))
    q("Daire yol olamaz", {"vehicle": CAR, "uid": UIDS["round"], "step": 1.0},
      invalid(f"“{UIDS['round']}” yol olamaz: bir çizgi, açık tek parçalı çoklu çizgi ya da yay olmalı.", "uid"))
    q("Kapalı çoklu çizgi yol olamaz", {"vehicle": CAR, "uid": UIDS["ring"], "step": 1.0},
      invalid(f"“{UIDS['ring']}” yol olamaz: bir çizgi, açık tek parçalı çoklu çizgi ya da yay olmalı.", "uid"))
    q("Nokta engel olamaz", {"vehicle": CAR, "uid": UIDS["line"], "step": 1.0, "obstacles": [UIDS["dot"]]},
      invalid(f"“{UIDS['dot']}” engel olamaz: bir çizgi, çoklu çizgi, yay ya da daire olmalı.", "obstacles[0]"))
    q("Çok örnek", {"vehicle": CAR, "uid": UIDS["line"], "step": 0.0001},
      invalid("Yol bu aralıkla 100000 örnekten çok tutuyor; aralığı büyütün.", "step"))
    return out


def draw_cases():
    out = []
    base = {"vehicle": TRUCK, "uid": UIDS["ell"], "step": 2.0, "obstacles": [UIDS["post"]]}

    def count(query, every, labels):
        o, _ = answer(query)
        units = len(o["corners"]) // 4
        n = o["samples"]
        bodies = 0 if every is None else sum(1 for k in range(n) if k % every == 0 or k + 1 == n) * units
        return 1 + 4 * units + bodies + (1 if labels else 0)

    def ok(name, inp, warns=(), plan=False):
        n = count(inp["query"], inp.get("every"), inp.get("labels", False))
        result = completed({"uids": [ANY] * n, "revision": "$current"}, answer(inp["query"])[1] + list(warns))
        out.append({"name": name, "steps": [step("plan" if plan else "execute", inp, result,
                                                 UNCHANGED if plan else STEPPED)]})

    def no(name, inp, result, setup_=None):
        c = {"name": name, "steps": [step("execute", inp, result, UNCHANGED)]}
        if setup_:
            c["setup"] = setup_
        out.append(c)

    J = {"junction": "j-1", "quadrant": 0, "offset": 1.5, "approach": 10.0}
    ok("İzler ve yazı", {"query": base, "layerId": "arac", "labels": True})
    ok("Her beşinci örnekte gövde", {"query": base, "layerId": "arac", "every": 5})
    ok("Her örnekte gövde", {"query": {**base, "step": 3.0}, "layerId": "arac", "every": 1})
    ok("Yalnız izler", {"query": base, "layerId": "arac"})
    ok("Römorklu araç gövdeleriyle", {"query": {"vehicle": SEMI, "path": [[0.0, 0.0], [30.0, 0.0], [30.0, 30.0]],
                                                "step": 2.0}, "layerId": "arac", "every": 4, "labels": True})
    ok("Kavşağın çeyreği", {"query": {"vehicle": TRUCK, "junction": {**J, "offset": 3.5}, "step": 1.0}, "layerId": "arac", "every": 10,
                            "labels": True})
    ok("Değen araç uyarısıyla", {"query": {"vehicle": SEMI, "junction": {**J, "quadrant": 3, "offset": 0.5}, "step": 1.0},
                                 "layerId": "arac", "labels": True})
    ok("Planı yazmaz", {"query": base, "layerId": "arac", "every": 5, "labels": True}, plan=True)
    ok("Başka katmana", {"query": base, "layerId": "korkuluk"})
    hidden = {"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}
    ok("Gizli katmana uyarıyla", {"query": base, "layerId": "gizli"}, [hidden])
    ok("Yol projesi olmadan", {"query": {"vehicle": CAR, "uid": UIDS["line"], "step": 2.0}, "layerId": "arac"})
    out[-1]["setup"] = "$map"
    no("Gövde aralığı sıfır", {"query": base, "layerId": "arac", "every": 0},
       invalid("Gövdeler her kaç örnekte bir çizilecek (every) sıfırdan büyük olmalı.", "every"))
    no("Kilitli katman", {"query": base, "layerId": "kilitli"},
       failed("layer_locked", "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı "
                              "etkinleştirin.", "layerId"))
    no("Katman grubu", {"query": base, "layerId": "grup"},
       {"status": "failed", "error": {"code": "not_a_layer", "message": ANY, "path": "layerId"}})
    no("Katman yok", {"query": base, "layerId": "yok"},
       failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
              "layerId"))
    no("Sorunun reddi", {"query": {**base, "step": 0.0}, "layerId": "arac"},
       invalid("Örnekleme aralığı (step) sıfırdan büyük olmalı.", "step"))
    no("Olmayan kavşak", {"query": {"vehicle": CAR, "junction": {**J, "junction": "j-9"}, "step": 1.0}, "layerId": "arac"},
       failed("unknown_junction", "“j-9” kimlikli ya da adlı kavşak projede yok.", "junction.junction"))
    no("Geçersiz beklenen sürüm", {"query": base, "layerId": "arac", "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {"query": base, "layerId": "arac", "expectedRevision": "41"}, CONFLICT)
    no("Kavşak yol projesinde", {"query": {"vehicle": CAR, "junction": J, "step": 1.0}, "layerId": "arac"}, NOT_ROAD,
       setup_="$map")
    return out


def document(command, title, found):
    road = setup()
    for c in found:
        if c.get("setup") == "$map":
            c["setup"] = setup(work="map")
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "scripts/fixtures/road_swept_cases.py'den; kirişlerde kesin traktris, gövde dikdörtgenleri, iç kayma "
                    "ve açıklık ADR 0915'in kuralıyla; kavşağın dönüşleri road_junction_cases.py'nin cevabından; "
                    "KentOS kodu olmadan.",
            "setup": road, "cases": found}


def main():
    setup()  # the uids
    docs = {"road.swept.query.json": document("road.swept.query", "Araç izini sorgula: iz, iç kayma, açıklık", cases()),
            "road.swept.draw.json": document("road.swept.draw", "Araç izini çiz: izler ve gövdeler", draw_cases())}
    stale = []
    for name, doc in docs.items():
        text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
        path = OUT / name
        if "--check" in sys.argv:
            if not path.exists() or path.read_text() != text:
                stale.append(name)
        else:
            path.write_text(text)
        print(f"{name}: {len(doc['cases'])} durum")
    if stale:
        sys.exit(f"güncel değil: {stale}")


if __name__ == "__main__":
    main()
