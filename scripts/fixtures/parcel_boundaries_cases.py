#!/usr/bin/env python3
"""The shared cases of `cad.parcels.boundaries` (Sınır grafiği; plan/map-cad.md MAP-08, docs/adr/2000), without KentOS
code.

    uv run --with mpmath==1.4.1 python scripts/fixtures/parcel_boundaries_cases.py           # writes fixtures/commands/v1/cad.parcels.boundaries.json
    uv run --with mpmath==1.4.1 python scripts/fixtures/parcel_boundaries_cases.py --check   # writes nothing; compares

The rules, by hand (the ADR's):

- a parcel's rings: its outer ring, its holes, then each further part's ring and holes; each edge with its bulge
  (none: straight); an edge whose ends are one place is left out;
- a corner is a place where a ring has a vertex, the same coordinates exactly; the corners by east, then north, are
  K1, K2 …;
- a boundary is an edge between two corners with one bulge, taken from the lower corner to the higher (the bulge
  negated when the edge runs the other way), the same within 1e-9; the boundaries by their corners (then bulge) are
  S1 …; a boundary's length is its chord's, or c·θ / (2 sin(θ/2)) with θ = 4 atan b;
- a ring's parcel lies left of its edges when the ring runs counter-clockwise (its signed area, the shoelace (exact on
  the stored values) plus each
  arc's segment r²/2 (θ − sin θ) signed as its bulge, positive), right otherwise; a hole's the other way; a boundary's
  left and right are its first parcel on each side; a further one is an overlap at the edge's middle on its curve;
- missingVertex: a corner that is no end of a straight boundary, the foot of its perpendicular strictly inside it and
  within 1 µm; or no end of an arc boundary, within 1 µm of the arc's circle (the circle through its ends and its
  middle, its centre where the chords' perpendicular bisectors meet, in mpmath at 50 digits) and strictly on its
  middle's side of the chord (docs/adr/2220); splitCorner: two corners within 1 µm; problems by kind (missingVertex,
  splitCorner, overlap), then by their corner, then by the boundary;
- a parcel: its identity's parts written (İl/İlçe/Mahalle/Ada/Parsel), its net area (outer rings less their holes),
  its boundaries in ring order, its neighbours across them in the drawing's order.
"""

import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

import mpmath as mp

mp.mp.dps = 50

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.parcels.boundaries.json"
X0, Y0 = 487000.0, 4420000.0
W = 1e-9
SAME = {"revision": "same", "canUndo": False}
NEAR = 1e-6


def uid(n):
    return f"0192f800-2000-7000-8000-{n:012x}"


def layer(lid, name):
    return {"id": lid, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "#E5484D", "lineType": "continuous", "lineWeight": 0.35}, "children": []}


def P(x, y):
    return {"x": X0 + x, "y": Y0 + y}


def parcel(pts, attrs=None, bulges=None, holes=None, parts=None):
    e = {"kind": "polygon", "layerId": "parsel", "attrs": attrs or {}, "pts": [P(*p) for p in pts]}
    if bulges:
        e["bulges"] = bulges
    if holes:
        e["holes"] = [{"pts": [P(*p) for p in h]} for h in holes]
    if parts:
        e["parts"] = [{"pts": [P(*p) for p in part]} for part in parts]
    return e


def ident(n, ada="101"):
    return {"İl": "Ankara", "İlçe": "Çankaya", "Mahalle": "Kızılay", "Ada": ada, "Parsel": str(n)}


# The clean block: P1, P2 side by side; P3 above them with a corner where they meet; P4 east of P2 with an arc for its
# east side; P5 with a hole, P6 filling the hole; P7 drawn clockwise; P8 of two parts; P9 and P10 sharing an arc.
CLEAN = [
    parcel([(0, 0), (20, 0), (20, 15), (0, 15)], ident(1)),
    parcel([(20, 0), (40, 0), (40, 15), (20, 15)], ident(2)),
    parcel([(0, 15), (20, 15), (40, 15), (40, 30), (0, 30)], ident(3)),
    parcel([(40, 0), (55, 0), (55, 15), (40, 15)], ident(4), bulges=[0, -0.2, 0, 0]),
    parcel([(70, 0), (100, 0), (100, 30), (70, 30)], ident(5), holes=[[(80, 10), (80, 20), (90, 20), (90, 10)]]),
    parcel([(80, 10), (90, 10), (90, 20), (80, 20)], ident(6)),
    parcel([(120, 0), (120, 10), (130, 10), (130, 0)], ident(7)),
    parcel([(140, 0), (150, 0), (150, 10), (140, 10)], ident(8), parts=[[(150, 0), (160, 0), (160, 10), (150, 10)]]),
    parcel([(170, 0), (180, 0), (180, 10), (170, 10)], {"Ada": "102", "Parsel": "9"}, bulges=[0, 0.25, 0, 0]),
    parcel([(180, 0), (190, 0), (190, 10), (180, 10)], {"Parsel": "10"}, bulges=[0, 0, 0, -0.25]),
]
# The block with problems: Q1, Q2 under Q3 whose bottom has no corner where they meet; Q4 and Q5 whose corners are
# 0.4 µm apart; Q7 inside Q6 on its bottom edge.
SPLIT = 0.0000004
PROBLEMS = [
    parcel([(0, 0), (20, 0), (20, 10), (0, 10)], ident(21)),
    parcel([(20, 0), (30, 0), (30, 10), (20, 10)], ident(22)),
    parcel([(0, 10), (30, 10), (30, 20), (0, 20)], ident(23)),
    parcel([(40, 0), (50, 0), (50, 10), (40, 10)], ident(24)),
    parcel([(50 + SPLIT, 0), (60, 0), (60, 10), (50 + SPLIT, 10)], ident(25)),
    parcel([(70, 0), (80, 0), (80, 10), (70, 10)], ident(26)),
    parcel([(70, 0), (80, 0), (80, 5), (70, 5)], ident(27)),
]


# Arcs (docs/adr/2220): R1's top is an arc 2 m high over its 20 m chord (bulge 0.2: radius 26, centre 24 m under the
# chord's middle); R2 and R3 above it follow the same circle up to its top (110, 12), where R1 has no corner; R4's corner
# (110, −40) is on the circle but across the chord; R5's corner is 0.4 µm over the arc's top, R6's 3 µm.
HALF = -math.tan(math.atan(0.2) / 2)
ARCS = [
    parcel([(100, 0), (120, 0), (120, 10), (100, 10)], ident(31), bulges=[0, 0, 0.2, 0]),
    parcel([(100, 10), (110, 12), (110, 20), (100, 20)], ident(32), bulges=[HALF, 0, 0, 0]),
    parcel([(110, 12), (120, 10), (120, 20), (110, 20)], ident(33), bulges=[HALF, 0, 0, 0]),
    parcel([(105, -45), (115, -45), (110, -40)], ident(34)),
    parcel([(110, 12 + 0.0000004), (115, 20), (105, 20)], ident(35)),
    parcel([(110, 12 + 0.000003), (115, 20), (105, 20)], ident(36)),
]


def arc_near(p, a, b, bulge):
    """Whether `p` is within 1 µm of the arc from `a` to `b`: of the circle through its ends and middle, and strictly
    on the middle's side of the chord."""
    P_, A, B = ([mp.mpf(v) for v in q] for q in (p, a, b))
    M = [mp.mpf(v) for v in middle(a, b, bulge)]
    cross = lambda o, u, v: (u[0] - o[0]) * (v[1] - o[1]) - (u[1] - o[1]) * (v[0] - o[0])  # noqa: E731
    if cross(A, B, P_) * cross(A, B, M) <= 0:
        return False
    # The circumcentre of A, B, M, relative to A.
    bx, by, mx, my = B[0] - A[0], B[1] - A[1], M[0] - A[0], M[1] - A[1]
    d = 2 * (bx * my - by * mx)
    ox = (my * (bx * bx + by * by) - by * (mx * mx + my * my)) / d
    oy = (bx * (mx * mx + my * my) - mx * (bx * bx + by * by)) / d
    r = mp.sqrt(ox * ox + oy * oy)
    return abs(mp.sqrt((P_[0] - A[0] - ox) ** 2 + (P_[1] - A[1] - oy) ** 2) - r) <= NEAR


def setup(entities, work="map", more=()):
    ents = []
    for e in list(entities) + list(more):
        ents.append({**e, "id": len(ents) + 1})
    settings = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": work, "drawingFont": "barlow"}
    if work == "gis":
        settings["workspace"] = "gis"
        settings.pop("cadWork")
    return {"format": "kentos.document", "version": 2, "name": "Sınırlar", "settings": settings,
            "origin": {"x": X0, "y": Y0}, "layers": [layer("0", "0"), layer("parsel", "Parsel"), layer("yol", "Yol")],
            "activeLayer": "parsel", "entities": ents, "uids": [uid(e["id"]) for e in ents],
            "styles": {"items": [], "categories": []}}


# ── The graph by hand ──────────────────────────────────────────────────────────────────────────────────────────────
def xy(p):
    return (p["x"], p["y"])


def rings(e):
    """(points, bulges, hole) of each ring: the outer, its holes, then each part's ring and holes."""
    def ring(pts, bulges, hole):
        return ([xy(p) for p in pts], [(bulges[i] if bulges and i < len(bulges) else 0.0) for i in range(len(pts))], hole)
    out = [ring(e["pts"], e.get("bulges"), False)]
    out += [ring(h["pts"], h.get("bulges"), True) for h in e.get("holes", [])]
    for part in e.get("parts", []):
        out.append(ring(part["pts"], part.get("bulges"), False))
        out += [ring(h["pts"], h.get("bulges"), True) for h in part.get("holes", [])]
    return out


def segment(c, b):
    theta = 4 * math.atan(abs(b))
    r = c / (2 * math.sin(theta / 2))
    return r * r / 2 * (theta - math.sin(theta))


def signed_area(pts, bulges):
    """The shoelace exactly on the stored values (map coordinates would lose millimetres squared in floats), plus each
    arc's segment."""
    n = len(pts)
    q = [(F(x), F(y)) for x, y in pts]
    a = float(sum(q[i][0] * q[(i + 1) % n][1] - q[(i + 1) % n][0] * q[i][1] for i in range(n)) / 2)
    for i, b in enumerate(bulges):
        if b:
            p, r = pts[i], pts[(i + 1) % n]
            a += math.copysign(segment(math.hypot(r[0] - p[0], r[1] - p[1]), b), b)
    return a


def length(a, b, bulge):
    c = math.hypot(b[0] - a[0], b[1] - a[1])
    if bulge == 0 or c == 0:
        return c
    theta = 4 * math.atan(bulge)
    return c * theta / (2 * math.sin(theta / 2))


def middle(a, b, bulge):
    dx, dy = b[0] - a[0], b[1] - a[1]
    return [(a[0] + b[0]) / 2 + dy * bulge / 2, (a[1] + b[1]) / 2 - dx * bulge / 2]


def graph(entities, chosen):
    """`chosen`: the entities' indices in the order they are taken."""
    ents = [entities[i] for i in chosen]
    uids = [uid(i + 1) for i in chosen]
    places = sorted({p for e in ents for pts, _, _ in rings(e) for p in pts})
    index = {p: i for i, p in enumerate(places)}
    at_corner = [[] for _ in places]
    edges = []  # [lo, hi, bulge, [(parcel, left)]]
    ring_edges = [[] for _ in ents]
    for k, e in enumerate(ents):
        for pts, bulges, hole in rings(e):
            n = len(pts)
            ccw = signed_area(pts, bulges) > 0
            for i in range(n):
                a, b = pts[i], pts[(i + 1) % n]
                ia, ib = index[a], index[b]
                if k not in at_corner[ia]:
                    at_corner[ia].append(k)
                if ia == ib:
                    continue
                forward = ia < ib
                lo, hi = (ia, ib) if forward else (ib, ia)
                bulge = bulges[i] if forward else -bulges[i]
                left_of_way = ccw != hole
                left = left_of_way if forward else not left_of_way
                j = next((j for j, x in enumerate(edges) if x[0] == lo and x[1] == hi and abs(x[2] - bulge) <= 1e-9), None)
                if j is None:
                    edges.append([lo, hi, bulge, []])
                    j = len(edges) - 1
                edges[j][3].append((k, left))
                ring_edges[k].append(j)
    for c in at_corner:
        c.sort()
    order = sorted(range(len(edges)), key=lambda j: (edges[j][0], edges[j][1], edges[j][2]))
    bid = {j: n for n, j in enumerate(order)}
    problems = []
    boundaries = []
    shared = outer = 0
    for j in order:
        lo, hi, bulge, sides = edges[j]
        left = next((k for k, s in sides if s), None)
        right = next((k for k, s in sides if not s), None)
        used = (left is not None) + (right is not None)
        if used == 2:
            shared += 1
        else:
            outer += 1
        if len(sides) > used:
            on = sorted({k for k, _ in sides})
            problems.append((2, bid[j], {"kind": "overlap", "at": middle(places[lo], places[hi], bulge),
                                         "parcels": [uids[k] for k in on]}))
        b = {"id": f"S{bid[j] + 1}", "from": f"K{lo + 1}", "to": f"K{hi + 1}", "bulge": bulge,
             "length": length(places[lo], places[hi], bulge)}
        if left is not None:
            b["left"] = uids[left]
        if right is not None:
            b["right"] = uids[right]
        boundaries.append(b)
    for c, p in enumerate(places):
        for j in order:
            lo, hi, bulge, sides = edges[j]
            if c in (lo, hi):
                continue
            a, b = places[lo], places[hi]
            if bulge != 0:
                near = arc_near(p, a, b, bulge)
            else:
                dx, dy = b[0] - a[0], b[1] - a[1]
                l2 = dx * dx + dy * dy
                t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2
                near = 0 < t < 1 and abs((p[0] - a[0]) * dy - (p[1] - a[1]) * dx) / math.sqrt(l2) <= NEAR
            if near:
                on = sorted(set(at_corner[c]) | {k for k, _ in sides})
                problems.append((0, c, {"kind": "missingVertex", "at": [p[0], p[1]], "parcels": [uids[k] for k in on]}))
    for i in range(len(places)):
        for j in range(i + 1, len(places)):
            if places[j][0] - places[i][0] > NEAR:
                break
            if math.hypot(places[j][0] - places[i][0], places[j][1] - places[i][1]) <= NEAR:
                on = sorted(set(at_corner[i]) | set(at_corner[j]))
                problems.append((1, i, {"kind": "splitCorner", "at": [places[i][0], places[i][1]],
                                        "parcels": [uids[k] for k in on]}))
    problems.sort(key=lambda t: (t[0], t[1]))
    parcels = []
    for k, e in enumerate(ents):
        name = "/".join(v for v in (e["attrs"].get(f, "").strip() for f in ("İl", "İlçe", "Mahalle", "Ada", "Parsel")) if v)
        area = 0.0
        for pts, bulges, hole in rings(e):
            area += -abs(signed_area(pts, bulges)) if hole else abs(signed_area(pts, bulges))
        neighbours = sorted({o for j in ring_edges[k] for o, _ in edges[j][3] if o != k})
        parcels.append({"uid": uids[k], "name": name, "area": area, "boundaries": [f"S{bid[j] + 1}" for j in ring_edges[k]],
                        "neighbours": [uids[o] for o in neighbours]})
    return {"corners": [{"id": f"K{i + 1}", "at": [p[0], p[1]], "parcels": [uids[k] for k in at_corner[i]]}
                        for i, p in enumerate(places)],
            "boundaries": boundaries, "parcels": parcels, "problems": [p for _, _, p in problems],
            "shared": shared, "outer": outer}


# ── The cases ──────────────────────────────────────────────────────────────────────────────────────────────────────
def ok(name, entities, chosen=None, op="execute", given=None, setup_=None):
    all_ = [i for i, e in enumerate(entities) if e.get("kind") == "polygon" and e.get("layerId") == "parsel"]
    picked = chosen if chosen is not None else all_
    inp = {} if chosen is None else {"uids": [uid(i + 1) for i in chosen]}
    if given is not None:
        inp = given
    out = None if op == "validate" else graph(entities, picked)
    case = {"name": name, "steps": [{"op": op, "input": inp, "result": {"status": "completed", "output": out, "warnings": []},
                                     "expect": SAME, "within": W}]}
    if setup_ is not None:
        case["setup"] = setup_
    return case


def refused(name, inp, code, message, path, setup_=None):
    case = {"name": name, "steps": [{"op": "execute", "input": inp,
                                     "result": {"status": "failed", "error": {"code": code, "message": message, "path": path}},
                                     "expect": SAME}]}
    if setup_ is not None:
        case["setup"] = setup_
    return case


def cases():
    line = {"kind": "line", "layerId": "yol", "attrs": {}, "a": P(0, -5), "b": P(40, -5)}
    with_line = CLEAN + [line]
    sp = setup(PROBLEMS)
    sa = setup(ARCS)
    return [
        ok("temiz ada: ortak sınırlar, köşede birleşen üç parsel, yaylı dış sınır, delik, saat yönü, iki parça, yaylı ortak sınır",
           CLEAN),
        ok("iki komşu parsel", CLEAN, [0, 1]),
        ok("tek parsel: sınırlarının hepsi dış", CLEAN, [0]),
        ok("komşusu seçilmeyen parselin sınırı dış kalır", CLEAN, [0, 2]),
        ok("seçim sırası grafiği değiştirmez, kimlikler köşelerden", CLEAN, [1, 0]),
        ok("delikli parsel ve deliği dolduran parsel: deliğin kenarları ortak", CLEAN, [4, 5]),
        ok("delikli parsel tek başına: deliğin kenarlarında parsel dışarıda", CLEAN, [4]),
        ok("saat yönünde çizilmiş parsel: sol ve sağ yine doğru", CLEAN, [6]),
        ok("iki parçalı parsel: iki halkanın ortak kenarı kendi kendine", CLEAN, [7]),
        ok("yaylı ortak sınır: iki parselin aynı yayı bir sınır", CLEAN, [8, 9]),
        ok("yaylı dış sınır: yayın boyu", CLEAN, [3]),
        ok("kimliği eksik parselin adı yazılan kısımlarıyla", CLEAN, [8]),
        ok("parsel katmanındaki çizgi alınmaz", with_line, setup_=setup(with_line)),
        ok("sorunlu ada: T birleşimi, 0,4 µm ayrık köşeler, bir yanda iki parsel", PROBLEMS, setup_=sp),
        ok("T birleşimi tek başına: alttaki iki parselin köşesi üstteki kenarın içinde", PROBLEMS, [0, 1, 2], setup_=sp),
        ok("ayrık köşeler tek başına: ortak sınır yok, iki splitCorner", PROBLEMS, [3, 4], setup_=sp),
        ok("bir yanda iki parsel: overlap ve iki eksik köşe", PROBLEMS, [5, 6], setup_=sp),
        # docs/adr/2220: a corner on an arc boundary that does not end there.
        ok("yaylı sınırda eksik köşe: üstteki iki parselin köşesi alttakinin yayının tepesinde", ARCS, [0, 1, 2],
           setup_=sa),
        ok("yayın çemberinde ama kirişin öbür yanındaki köşe yayın üstünde değildir", ARCS, [0, 3], setup_=sa),
        ok("yaydan 0,4 µm uzaktaki köşe eksik köşedir", ARCS, [0, 4], setup_=sa),
        ok("yaydan 3 µm uzaktaki köşe eksik köşe değildir", ARCS, [0, 5], setup_=sa),
        ok("plan aynı grafiği verir", CLEAN, op="plan"),
        ok("doğrulama", CLEAN, op="validate"),
        ok("CBS projesinde de", CLEAN, [0, 1, 2], setup_=setup(CLEAN, "gis")),
        ok("parsel katmanı boş: grafik boş", [], setup_=setup([])),
        refused("geçersiz kimlik", {"uids": ["P1"]}, "invalid_uid", "“P1” geçerli bir nesne kimliği değil.", "uids[0]"),
        refused("olmayan nesne", {"uids": [uid(1), uid(99)]}, "entity_not_found", f"“{uid(99)}” kimlikli nesne çizimde yok.",
                "uids[1]"),
        refused("alan olmayan nesne", {"uids": [uid(11)]}, "not_a_parcel",
                f"“{uid(11)}” bir alan değil; sınır grafiği yalnız alanlardan kurulur.", "uids[0]", setup_=setup(with_line)),
    ]


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.parcels.boundaries", "commandVersion": 1,
           "title": "Sınır grafiği",
           "note": "ADR 2000, 2220. Bağımsız başvuru scripts/fixtures/parcel_boundaries_cases.py: köşeler, sınırlar, iki yanın "
                   "parselleri, alanlar, boylar ve sorunlar ADR'nin kurallarıyla elle (yayın çemberi uçlarından ve "
                   "ortasından, mpmath); KentOS kodu yok.",
           "setup": setup(CLEAN), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        same = OUT.exists() and OUT.read_text("utf-8") == text
        print("Sınır grafiği ortak durumları güncel." if same else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if same else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
