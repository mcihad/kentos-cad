#!/usr/bin/env python3
"""The shared cases of `cad.pipes.check` (Boru hattı denetimi; plan/map-cad.md MAP-16, docs/adr/1640), without KentOS
code.

    python3 scripts/fixtures/pipes_check_cases.py           # writes fixtures/commands/v1/cad.pipes.check.json
    python3 scripts/fixtures/pipes_check_cases.py --check   # writes nothing; compares

The drawing is a Harita project with a ground lattice (corners every 10 m over [0, 200] × [0, 100] from X0, Y0):

    G(x, y) = 100 − 0.02 x + k(x) + 0.01 |y − 50|,   k(x) = max(0, 0.5 − 0.01 |x − 120|)

Every break of G is on a lattice line, so each square is one plane and the answers do not depend on how a square is
cut into triangles: along a straight pipe the ground is linear between the places the pipe crosses a lattice line, and
so is the crown, and the least and greatest cover are at those places or at the pipe's vertices (a curved pipe: at its
vertices only). Two networks:

- Atıksu (sewer): pipes on `atiksu` with their inverts in `Akar baş` / `Akar son` (metres) and `Çap` (mm), manholes on
  `baca`; drawn with the flow. P1–P4 along y = 50, P5 and P6 branches from the north written in centimetres and as a
  depth, P7 a pipe alone, P8 flat, P9 leaving the ground.
- Yağmursuyu (storm): 3D polylines on `yagmur` whose vertices' heights are the inverts, `Çap` in metres, structures on
  `izgara` with `Kapak kotu`; the direction field `Akış` = `ters` flows against the drawing; Q3 is an arc.

The rules, by hand: a pipe's path length (an arc segment c·θ / (2 sin(θ/2)), θ = 4·atan|b|); the invert linear by
length between the ends' attributes or the vertices' heights; upstream the drawn start unless `ters`; slope (up −
down) / length; rising is reverseSlope, zero flat, below the least lowSlope, above the greatest steepSlope; the crown
the invert plus the diameter (none asked: the invert); cover the ground less the crown; below zero aboveGround, below
the least shallow, above the greatest deep; an end's invert that misses the cover limits but fits them read /100 (cm)
or /1000 (mm) invertUnit, as a depth (cover = value − diameter) invertIsDepth; a diameter not in the catalogue that is
there in another unit diameterUnit, else notInCatalog; parts are the pipes joined at shared ends, all but the longest
detached; an end at no structure noStructure; an outlet invert above the lowest inlet invert at a node outletAbove.
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.pipes.check.json"
X0, Y0 = 487000.0, 4420000.0
W = 1e-6
KINDS = ["detached", "noStructure", "noInvert", "invertUnit", "invertIsDepth", "reverseSlope", "flat", "lowSlope",
         "steepSlope", "outletAbove", "noDiameter", "diameterUnit", "notInCatalog", "noGround", "offSurface",
         "aboveGround", "shallow", "deep", "curved"]
UNITS = {"mm": 0.001, "cm": 0.01, "m": 1.0}


def uid(n):
    return f"0192f640-1640-7000-8000-{n:012x}"


def layer(id_, name):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def k_bump(x):
    return max(0.0, 0.5 - 0.01 * abs(x - 120.0))


def ground(x, y):
    """The ground at local (x, y), none off the lattice."""
    if not (0.0 <= x <= 200.0 and 0.0 <= y <= 100.0):
        return None
    return 100.0 - 0.02 * x + k_bump(x) + 0.01 * abs(y - 50.0)


# ── The drawing ───────────────────────────────────────────────────────────────────────────────────────────────────
# Manholes of the sewer (local places).
M = {1: (0, 50), 2: (50, 50), 3: (100, 50), 4: (150, 50), 5: (200, 50), 6: (50, 80), 7: (100, 80), 8: (200, 20),
     9: (230, 20)}
# Sewer pipes: (name, from, to, Akar baş, Akar son, Çap).
SEWER = [
    ("P1", M[1], M[2], "98.0", "97.5", "300"),
    ("P2", M[2], M[3], "97.5", "97.6", "300"),
    ("P3", M[3], M[4], "97.0", "96.6", "300"),
    ("P4", M[4], M[5], "96.7", "92.0", "400"),
    ("P5", M[6], M[2], "9800", "97.6", "0.2"),
    ("P6", M[7], M[3], "1.5", "97.1", "250"),
    ("P7", (180, 90), (200, 90), None, None, "300"),
    ("P8", M[5], M[8], "92.0", "92.0", "300"),
    ("P9", M[8], M[9], "91.9", "91.6", "300"),
]
# Storm structures: (place, Kapak kotu).
S = [((0, 20), "100.3"), ((60, 20), "99.1"), ((100, 20), "98.6"), ((140, 20), None)]
# Storm pipes: (name, points, heights, bulges, Akış, Çap in metres).
STORM = [
    ("Q1", [(0, 20), (30, 20), (60, 20)], [99.0, 98.7, 98.5], None, None, "0.4"),
    ("Q2", [(100, 20), (60, 20)], [98.0, 98.3], None, "ters", "0.4"),
    ("Q3", [(100, 20), (140, 20)], [97.9, 97.5], [0.2], None, "0.5"),
]


def P(p):
    return {"x": X0 + p[0], "y": Y0 + p[1]}


def setup(work="map"):
    layers = [layer("0", "0"), layer("zemin", "Zemin"), layer("atiksu", "Atıksu"), layer("baca", "Baca"),
              layer("yagmur", "Yağmursuyu"), layer("izgara", "Izgara"), layer("yol", "Yol"), layer("bos", "Boş")]
    ents, uids, n = [], [], 0

    def add(e):
        nonlocal n
        n += 1
        e["id"] = n
        ents.append(e)
        uids.append(uid(n))
        return n

    for j in range(11):
        for i in range(21):
            x, y = 10.0 * i, 10.0 * j
            add({"kind": "point", "layerId": "zemin", "attrs": {}, "p": P((x, y)), "z": ground(x, y)})
    for m in sorted(M):
        add({"kind": "point", "layerId": "baca", "attrs": {"Ad": f"B{m}"}, "p": P(M[m])})
    for name, a, b, i0, i1, d in SEWER:
        attrs = {"Ad": name, "Çap": d}
        if i0 is not None:
            attrs["Akar baş"] = i0
            attrs["Akar son"] = i1
        add({"kind": "line", "layerId": "atiksu", "attrs": attrs, "a": P(a), "b": P(b)})
    for k, (p, cover) in enumerate(S):
        attrs = {"Ad": f"I{k + 1}"}
        if cover is not None:
            attrs["Kapak kotu"] = cover
        add({"kind": "point", "layerId": "izgara", "attrs": attrs, "p": P(p)})
    for name, pts, zs, bulges, flow, d in STORM:
        attrs = {"Ad": name, "Çap": d}
        if flow:
            attrs["Akış"] = flow
        e = {"kind": "polyline", "layerId": "yagmur", "attrs": attrs, "pts": [P(p) for p in pts],
             "zs": zs}
        if bulges:
            e["bulges"] = bulges + [0.0]
        add(e)
    add({"kind": "line", "layerId": "yol", "attrs": {}, "a": P((0, 0)), "b": P((50, 0))})
    networks = [
        {"id": "atiksu", "name": "Atıksu", "kind": "utility", "edges": [{"layer": "atiksu"}],
         "junctions": [{"layer": "baca", "role": "junction"}], "connect": "ends", "tolerance": 0.01,
         "direction": {"kind": "digitized"}},
        {"id": "yagmur", "name": "Yağmursuyu", "kind": "utility", "edges": [{"layer": "yagmur"}],
         "junctions": [{"layer": "izgara", "role": "junction"}], "connect": "ends", "tolerance": 0.01,
         "direction": {"kind": "field", "field": "Akış", "forward": [], "backward": ["ters"], "closed": []}},
        {"id": "yollar", "name": "Yollar", "kind": "road", "edges": [{"layer": "yol"}], "connect": "ends",
         "tolerance": 0.01, "direction": {"kind": "both"}},
    ]
    settings = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": work, "drawingFont": "barlow",
                "networks": networks}
    if work in ("map", "road"):
        settings["road"] = {"surfaces": [{"id": "t-1", "name": "Zemin", "points": {"layers": ["zemin"]}},
                                         {"id": "t-2", "name": "Boş", "points": {"layers": ["bos"]}}]}
    if work == "gis":
        settings["workspace"] = "gis"
        settings.pop("cadWork")
    return {"format": "kentos.document", "version": 2, "name": "Altyapı", "settings": settings,
            "origin": {"x": X0, "y": Y0}, "layers": layers, "activeLayer": "0", "entities": ents, "uids": uids,
            "styles": {"items": [], "categories": []}}


# ── The answers ───────────────────────────────────────────────────────────────────────────────────────────────────
def number(text):
    if text is None:
        return None
    t = text.strip()
    body = t[1:] if t.startswith("-") else t
    whole, sep, frac = body.replace(",", ".").partition(".")
    if not whole.isdigit() or (sep and not frac.isdigit()):
        return None
    return float(t.replace(",", "."))


def arc(chord, bulge):
    if bulge == 0.0:
        return chord
    th = 4.0 * math.atan(abs(bulge))
    return chord * th / (2.0 * math.sin(th / 2.0))


class Pipe:
    def __init__(self, n, name, pts, bulges, attrs, heights, layer_):
        self.n, self.name, self.pts, self.attrs, self.heights, self.layer = n, name, pts, attrs, heights, layer_
        self.bulges = bulges or [0.0] * (len(pts) - 1)
        self.curved = any(b != 0.0 for b in self.bulges)
        self.st = [0.0]
        for i in range(len(pts) - 1):
            c = math.hypot(pts[i + 1][0] - pts[i][0], pts[i + 1][1] - pts[i][1])
            self.st.append(self.st[-1] + arc(c, self.bulges[i]))
        self.length = self.st[-1]

    def invert(self, ends, s):
        if ends is not None:
            i0, i1 = ends
            return i0 + (i1 - i0) * s / self.length
        if self.heights is None:
            return None
        for k in range(len(self.st) - 1):
            if s <= self.st[k + 1] or k == len(self.st) - 2:
                s0, s1 = self.st[k], self.st[k + 1]
                h = self.heights
                return h[k] + (h[k + 1] - h[k]) * (s - s0) / (s1 - s0)

    def samples(self):
        """(station, place) at the vertices and, for a straight pipe, every lattice line crossing on the lattice."""
        if self.curved:
            return list(zip(self.st, self.pts))
        out = []
        for i in range(len(self.pts) - 1):
            (ax, ay), (bx, by) = self.pts[i], self.pts[i + 1]
            ts = {0.0, 1.0}
            for v in range(0, 201, 10):
                if ax != bx:
                    t = (v - ax) / (bx - ax)
                    y = ay + t * (by - ay)
                    if 0 < t < 1 and 0 <= y <= 100:
                        ts.add(t)
            for v in range(0, 101, 10):
                if ay != by:
                    t = (v - ay) / (by - ay)
                    x = ax + t * (bx - ax)
                    if 0 < t < 1 and 0 <= x <= 200:
                        ts.add(t)
            for t in sorted(ts):
                if out and t == 0.0:
                    continue
                out.append((self.st[i] + t * (self.st[i + 1] - self.st[i]), (ax + t * (bx - ax), ay + t * (by - ay))))
        return out


def drawing():
    """The two networks' pipes and structures, in the drawing's order."""
    n = 21 * 11
    baca = {}
    for m in sorted(M):
        n += 1
        baca[M[m]] = n
    sewer = []
    for name, a, b, i0, i1, d in SEWER:
        n += 1
        attrs = {"Ad": name, "Çap": d}
        if i0 is not None:
            attrs.update({"Akar baş": i0, "Akar son": i1})
        sewer.append(Pipe(n, name, [a, b], None, attrs, None, "atiksu"))
    izgara = {}
    for p, cover in S:
        n += 1
        izgara[p] = cover
    storm = []
    for name, pts, zs, bulges, flow, d in STORM:
        n += 1
        attrs = {"Ad": name, "Çap": d}
        if flow:
            attrs["Akış"] = flow
        storm.append(Pipe(n, name, pts, bulges, attrs, zs, "yagmur"))
    return {"atiksu": (sewer, {p: {} for p in baca}), "yagmur": (storm, {p: {"Kapak kotu": c} for p, c in
                                                                          izgara.items()})}


def report(net, inverts, diameter=None, ground_=None, lo=None, hi=None, smin=None, smax=None):
    pipes, structures = drawing()[net]
    found, rows = [], []
    key = lambda p: (round(p[0], 6), round(p[1], 6))
    # Parts: pipes joined at shared ends; all but the longest detached.
    parent = list(range(len(pipes)))

    def root(i):
        while parent[i] != i:
            i = parent[i]
        return i

    ends = {}
    for i, p in enumerate(pipes):
        for q in (p.pts[0], p.pts[-1]):
            ends.setdefault(key(q), []).append(i)
    for lst in ends.values():
        for j in lst[1:]:
            a, b = root(lst[0]), root(j)
            if a != b:
                parent[max(a, b)] = min(a, b)
    part_len = {}
    for i, p in enumerate(pipes):
        part_len[root(i)] = part_len.get(root(i), 0.0) + p.length
    longest = max(part_len, key=lambda r: (part_len[r], -r))
    at_node_in, at_node_out = {}, {}

    def mark(i, kind, at, value=None, unit=None):
        f = {"kind": kind, "uid": uid(pipes[i].n), "at": [X0 + at[0], Y0 + at[1]]}
        if value is not None:
            f["value"] = value
        if unit is not None:
            f["unit"] = unit
        found.append((i, KINDS.index(kind), len(found), f))

    for i, p in enumerate(pipes):
        start, end = p.pts[0], p.pts[-1]
        rev = p.attrs.get("Akış") == "ters"
        L = p.length
        if root(i) != longest:
            mark(i, "detached", start, part_len[root(i)])
        for q in (start, end):
            if key(q) not in {key(s) for s in structures}:
                mark(i, "noStructure", q)
        if inverts == "fields":
            a, b = number(p.attrs.get("Akar baş")), number(p.attrs.get("Akar son"))
            ends_ = (a, b) if a is not None and b is not None else None
            known = ends_ is not None
        else:
            ends_ = None
            known = p.heights is not None
        if not known:
            mark(i, "noInvert", start)
        dm = None
        dread = None
        if diameter:
            v = number(p.attrs.get(diameter["field"]))
            dread = v if v is not None and v > 0 else None
            dm = dread * UNITS[diameter["unit"]] if dread is not None else None
        crown = 0.0 if not diameter else dm

        def g_at(q):
            if ground_ and ground_["from"] == "surface":
                return ground(*q)
            if ground_ and ground_["from"] == "structures":
                return number(structures.get(key(q), {}).get(ground_["field"])) if key(q) in structures else None
            return None

        eg = [g_at(start), g_at(end)]
        if ground_ and ground_["from"] == "surface":
            samples = [(s, q, ground(*q)) for s, q in p.samples()]
        elif ground_:
            samples = [(0.0, start, eg[0]), (L, end, eg[1])]
        else:
            samples = []
        if ends_ and lo is not None and hi is not None and crown is not None:
            for v, g0, q in ((ends_[0], eg[0], start), (ends_[1], eg[1], end)):
                if g0 is None:
                    continue
                fits = lambda c: lo <= c <= hi
                if fits(g0 - (v + crown)):
                    continue
                unit = next((u for kk, u in ((100.0, "cm"), (1000.0, "mm")) if fits(g0 - (v / kk + crown))), None)
                if unit:
                    mark(i, "invertUnit", q, v, unit)
                elif fits(v - crown):
                    mark(i, "invertIsDepth", q, v)
        i0, i1 = p.invert(ends_, 0.0), p.invert(ends_, L)
        up, down = (i1, i0) if rev else (i0, i1)
        slope = (up - down) / L if up is not None and down is not None else None
        if slope is not None:
            if slope < 0:
                mark(i, "reverseSlope", start, slope)
            elif slope == 0:
                mark(i, "flat", start, slope)
            elif smin is not None and slope < smin:
                mark(i, "lowSlope", start, slope)
            if smax is not None and slope > smax:
                mark(i, "steepSlope", start, slope)
        if up is not None and down is not None:
            frm, to = (end, start) if rev else (start, end)
            at_node_out.setdefault(key(frm), []).append((i, up, frm))
            at_node_in.setdefault(key(to), []).append((i, down))
        if diameter:
            if dread is None:
                mark(i, "noDiameter", start)
            elif diameter.get("catalog"):
                cat = diameter["catalog"]
                inc = lambda v: any(abs(v - c) <= 1e-9 * max(abs(c), 1.0) for c in cat)
                if not inc(dread):
                    other = next((u for u in ("mm", "cm", "m") if u != diameter["unit"]
                                  and inc(dread * UNITS[u] / UNITS[diameter["unit"]])), None)
                    if other:
                        mark(i, "diameterUnit", start, dread, other)
                    else:
                        mark(i, "notInCatalog", start, dread)
        mn = mx = None
        if ground_:
            on = [s for s in samples if s[2] is not None]
            if not on:
                mark(i, "noGround", start)
            else:
                off = next((s for s in samples if s[2] is None), None)
                if off:
                    mark(i, "noGround" if ground_["from"] == "structures" else "offSurface", off[1])
            if crown is not None:
                for s, q, gz in on:
                    inv = p.invert(ends_, s)
                    if inv is None:
                        continue
                    c = gz - (inv + crown)
                    if mn is None or c < mn[0]:
                        mn = (c, q)
                    if mx is None or c > mx[0]:
                        mx = (c, q)
            if mn:
                if mn[0] < 0:
                    mark(i, "aboveGround", mn[1], mn[0])
                elif lo is not None and mn[0] < lo:
                    mark(i, "shallow", mn[1], mn[0])
            if mx and hi is not None and mx[0] > hi:
                mark(i, "deep", mx[1], mx[0])
        if p.curved and ground_:
            mark(i, "curved", start)
        row = {"uid": uid(p.n), "layer": p.layer, "length": L}
        if rev:
            row["reversed"] = True
        for k_, v in (("upstream", up), ("downstream", down), ("slope", slope), ("diameter", dm)):
            if v is not None:
                row[k_] = v
        if mn:
            row["minCover"] = mn[0]
            row["minCoverAt"] = [X0 + mn[1][0], Y0 + mn[1][1]]
        if mx:
            row["maxCover"] = mx[0]
        rows.append(row)
    for node, outs in at_node_out.items():
        ins = at_node_in.get(node)
        if not ins:
            continue
        low = min(d for _, d in ins)
        for i, o, at in outs:
            if o > low:
                mark(i, "outletAbove", at, o - low)
    found.sort(key=lambda t: (t[0], t[1], t[2]))
    for row in rows:
        kinds = []
        for i, _, _, f in found:
            if f["uid"] == row["uid"] and f["kind"] not in kinds:
                kinds.append(f["kind"])
        row["findings"] = kinds
    counts = [{"kind": kk, "count": sum(1 for t in found if t[3]["kind"] == kk)} for kk in KINDS]
    return {"network": net, "coverTo": "crown" if diameter else "invert", "pipes": rows,
            "findings": [t[3] for t in found], "counts": [c for c in counts if c["count"]],
            "parts": len(part_len), "length": sum(p.length for p in pipes)}


# ── The cases ─────────────────────────────────────────────────────────────────────────────────────────────────────
SAME = {"revision": "same", "canUndo": False}
SEWER_IN = {"from": "fields", "start": "Akar baş", "end": "Akar son"}
SEWER_D = {"field": "Çap", "unit": "mm", "catalog": [200.0, 300.0, 400.0, 500.0]}
ZEMIN = {"from": "surface", "surface": "Zemin"}
LIMITS = {"minCover": 0.8, "maxCover": 3.0, "minSlope": 0.009, "maxSlope": 0.05}
STORM_D = {"field": "Çap", "unit": "m"}
KAPAK = {"from": "structures", "field": "Kapak kotu"}
NOT_MAP = ("not_a_road_project", "Arazi yüzeyi Harita ve Yol projelerinde olur (KentOS Harita, KentOS Yol); bu çizim "
                                 "ikisinden biri değil.")


def ok(name, input_, out, op="plan", setup_=None):
    step = {"op": op, "input": input_, "result": {"status": "completed", "output": None if op == "validate" else out,
                                                 "warnings": []}, "expect": SAME, "within": W}
    case = {"name": name, "steps": [step]}
    if setup_ is not None:
        case["setup"] = setup_
    return case


def refused(name, input_, code, message, path_=None, setup_=None):
    err = {"code": code, "message": message}
    if path_ is not None:
        err["path"] = path_
    case = {"name": name, "steps": [{"op": "execute", "input": input_, "result": {"status": "failed", "error": err},
                                     "expect": SAME}]}
    if setup_ is not None:
        case["setup"] = setup_
    return case


def sewer_input(**over):
    inp = {"network": "atiksu", "inverts": SEWER_IN, "diameter": SEWER_D, "ground": ZEMIN, **LIMITS}
    for k_, v in over.items():
        if v is None:
            inp.pop(k_, None)
        else:
            inp[k_] = v
    return inp


def sewer(**over):
    inp = sewer_input(**over)
    return inp, report("atiksu", "fields", inp.get("diameter"), inp.get("ground"), inp.get("minCover"),
                       inp.get("maxCover"), inp.get("minSlope"), inp.get("maxSlope"))


def storm(**over):
    inp = {"network": "yagmur", "inverts": {"from": "heights"}, "diameter": STORM_D, "ground": KAPAK, **over}
    inp = {k_: v for k_, v in inp.items() if v is not None}
    inv = inp["inverts"]["from"]
    return inp, report("yagmur", inv, inp.get("diameter"), inp.get("ground"), inp.get("minCover"),
                       inp.get("maxCover"), inp.get("minSlope"), inp.get("maxSlope"))


def cases():
    full_in, full = sewer()
    kinds = {f["kind"] for f in full["findings"]}
    want = {"detached", "noStructure", "noInvert", "invertUnit", "invertIsDepth", "reverseSlope", "flat", "lowSlope",
            "steepSlope", "outletAbove", "diameterUnit", "notInCatalog", "offSurface", "aboveGround", "shallow", "deep"}
    assert want <= kinds, want - kinds
    st_in, st = storm(minCover=0.5, maxCover=3.0)
    assert not any(f["kind"] == "reverseSlope" for f in st["findings"]), st["findings"]
    assert {"noGround"} <= {f["kind"] for f in st["findings"]}, st["findings"]
    curved_in, curved = storm(ground=ZEMIN)
    assert "curved" in {f["kind"] for f in curved["findings"]}
    out = [
        ok("atıksu: akar kotları öznitelikte, çap mm ve katalog, örtü Zemin yüzeyinden, dört sınır", full_in, full),
        ok("uygula: salt okunur, çizim değişmez", full_in, full, op="execute"),
        ok("doğrulama", full_in, None, op="validate"),
        ok("sınırsız: yalnız her projede yanlış olan (ters eğim, düz, zeminin üstü, bağlantı)", *sewer(
            minCover=None, maxCover=None, minSlope=None, maxSlope=None)),
        ok("çapsız: örtü akar kotuna", *sewer(diameter=None)),
        ok("katalogsuz çap: birim önerisi yok", *sewer(diameter={"field": "Çap", "unit": "mm"})),
        ok("zeminsiz: örtü denetlenmez", *sewer(ground=None)),
        ok("yalnız en az örtü", *sewer(maxCover=None, minSlope=None, maxSlope=None)),
        ok("ağ adıyla, büyük küçük harf ayrımsız", {**full_in, "network": "ATIKSU"}, full),
        ok("yağmursuyu: köşe kotları, Akış = ters akışın yönünü çevirir, örtü ızgaraların kapak kotundan", st_in, st),
        ok("yağmursuyu yüzeyle: yaylı boruda örtü yalnız köşelerde", curved_in, curved),
        ok("yağmursuyu öznitelikten akar kotu yok", *storm(inverts=SEWER_IN)),
        ok("CBS projesinde kapak kotuyla", st_in, st, setup_=setup("gis")),
        ok("Yol projesinde yüzeyle", full_in, full, setup_=setup("road")),
    ]
    bad = "invalid_pipe_check"
    out += [
        refused("boş baş akar kotu özniteliği", sewer_input(inverts={"from": "fields", "start": " ", "end": "x"}),
                bad, "Baş akar kotunun özniteliği boş; özniteliğin adını yazın.", "inverts.start"),
        refused("boş son akar kotu özniteliği", sewer_input(inverts={"from": "fields", "start": "x", "end": ""}),
                bad, "Son akar kotunun özniteliği boş; özniteliğin adını yazın.", "inverts.end"),
        refused("boş çap özniteliği", sewer_input(diameter={"field": "", "unit": "mm"}), bad,
                "Çapın özniteliği boş; özniteliğin adını yazın.", "diameter.field"),
        refused("katalogda sıfır çap", sewer_input(diameter={"field": "Çap", "unit": "mm", "catalog": [200.0, 0.0]}),
                bad, "Katalogdaki çaplar 0'dan büyük olmalı.", "diameter.catalog[1]"),
        refused("201 çaplık katalog", sewer_input(diameter={"field": "Çap", "unit": "mm",
                                                           "catalog": [float(i + 1) for i in range(201)]}),
                bad, "Çap kataloğunda en çok 200 çap olur.", "diameter.catalog"),
        refused("boş yüzey", sewer_input(ground={"from": "surface", "surface": ""}), bad,
                "Zemin yüzeyi boş; yüzeyin adını ya da kimliğini yazın.", "ground.surface"),
        refused("boş kapak kotu özniteliği", sewer_input(ground={"from": "structures", "field": " "}), bad,
                "Kapak kotunun özniteliği boş; özniteliğin adını yazın.", "ground.field"),
        refused("sıfır en az örtü", sewer_input(minCover=0.0), bad,
                "En az örtü 0'dan büyük bir sayı olmalı; denetlenmeyecekse boş bırakın.", "minCover"),
        refused("eksi en çok eğim", sewer_input(maxSlope=-0.01), bad,
                "En çok eğim 0'dan büyük bir sayı olmalı; denetlenmeyecekse boş bırakın.", "maxSlope"),
        refused("en az örtü en çoktan büyük", sewer_input(minCover=3.5), bad,
                "En az örtü en çok örtüden büyük olamaz.", "maxCover"),
        refused("en az eğim en çoktan büyük", sewer_input(minSlope=0.06), bad,
                "En az eğim en çok eğimden büyük olamaz.", "maxSlope"),
        refused("olmayan ağ", sewer_input(network="kanal"), "unknown_network",
                "“kanal” kimlikli ya da adlı ağ projede yok; Ağlar'da tanımlı bir ağı seçin.", "network"),
        refused("yol ağı", sewer_input(network="yollar"), "not_a_pipe_network",
                "“Yollar” bir yol ağı; boru hattı denetimi altyapı ağlarını denetler.", "network"),
        refused("olmayan yüzey", sewer_input(ground={"from": "surface", "surface": "Kazı"}), "unknown_terrain",
                "“Kazı” kimlikli ya da adlı yüzey projede yok.", "ground.surface"),
        refused("kurulamayan yüzey", sewer_input(ground={"from": "surface", "surface": "Boş"}), "terrain_unbuildable",
                "$any", None),
        refused("CBS projesinde yüzey", sewer_input(), NOT_MAP[0], NOT_MAP[1], None, setup_=setup("gis")),
    ]
    return out


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.pipes.check", "commandVersion": 1,
           "title": "Boru hattı denetimi",
           "note": "ADR 1640. Bağımsız başvuru scripts/fixtures/pipes_check_cases.py: zemin kapalı biçimli kafes "
                   "(her kare tek düzlem), örtü köşelerde ve kafes çizgisi kesişimlerinde, eğim, birim ve bağlantı "
                   "kuralları elle; KentOS kodu yok. Sınırlar durumların kendi değerleridir, yönetmelik değeri değil.",
           "setup": setup(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok_ = OUT.exists() and OUT.read_text("utf-8") == text
        print("Boru hattı denetimi ortak durumları güncel." if ok_ else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok_ else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
