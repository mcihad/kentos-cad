#!/usr/bin/env python3
"""Independent reference of Yakınlık analizi (docs/adr/0215), written from the ADR alone, no KentOS code.

Writes fixtures/processing/v1/proximity.kcad (the drawing) and fixtures/processing/v1/proximity.json (the five tools
run on it, in the processing cases' format, fixtures/processing/README.md): En yakını bul, Uzaklık matrisi (list,
matrix, summary), En yakın merkeze bağla, Komşu alanlar and En kısa çizgi.

The rules (ADR 0215 §2):

1. Edge to edge distance: 0 for objects that meet (edges within 1 mm, a point on or in the other, one inside the
   other's area); else the least of: point to point, point to edge, and for each pair of edges the ends of the first
   to the second, then the ends of the second to the first. The nearest points are the first candidate with the least
   distance in that order (points of the input with the target's points then edges, the target's points with the
   input's edges, then the input's edges with the target's in order).
2. Centre to centre: the areas' centroids, the points themselves.
3. A target nearer comes first; equal ones in the targets' order; an object is never its own target; a maximum keeps
   only those not farther.
4. Neighbours: the length two areas' boundaries share (straight edges on one line within the tolerance, over the part
   their projections share; arcs of one circle over the angles they share); an edge neighbour shares more than the
   tolerance, a corner neighbour only meets; an overlap shares inside (its area); both ways round, in order.
5. Written: lengths with the project's 3 decimals, areas with 2, bearings (clockwise from north, grad) with 4, by the
   display rule (scripts/fixtures/numeric_display.py); names from the name field, else the label, else the place.

The drawing's distances are made to have one nearest pair each (no ties between different points), so the nearest
points do not hang on the last bits of a sum.
"""

import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from numeric_display import shown  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
PROC_DIR = ROOT / "fixtures" / "processing" / "v1"
E, N = 487000, 4420000
TOL = 1e-3


def P(x, y):
    return {"x": E + x, "y": N + y}


def rect(x0, y0, x1, y1):
    return [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]


LAYERS = [
    {"id": "cizim", "name": "Çizim", "type": "layer", "visible": True, "locked": False, "expanded": True,
     "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []},
    {"id": "parsel", "name": "Parsel", "type": "layer", "visible": True, "locked": False, "expanded": True,
     "style": {"color": "#E5484D", "lineType": "continuous", "lineWeight": 0.35, "fill": "#E5484D1F"}, "children": []},
    {"id": "durak", "name": "Durak", "type": "layer", "visible": True, "locked": False, "expanded": True,
     "style": {"color": "#3E63DD", "lineType": "continuous", "lineWeight": 0.25}, "children": []},
    {"id": "okul", "name": "Okul", "type": "layer", "visible": True, "locked": False, "expanded": True,
     "style": {"color": "#30A46C", "lineType": "continuous", "lineWeight": 0.25}, "children": []},
    {"id": "yol", "name": "Yol", "type": "layer", "visible": True, "locked": False, "expanded": True,
     "style": {"color": "#8E8C99", "lineType": "continuous", "lineWeight": 0.5}, "children": []},
]


def ent(i, kind, layer, attrs, **geom):
    e = {"kind": kind, "id": i, "layerId": layer, "attrs": attrs}
    e.update(geom)
    return e


# Parcels 6 and 7 share the arc from (20, 40) to (20, 60) bulging east: bulge 0.5 (6 going north, −0.5 seen from its
# left: east is its right) and 0.5 for 7 going south; chord 20, radius 12.5, centre (12.5, 50), sweep 4·atan(0.5).
ENTITIES = [
    ent(1, "polygon", "parsel", {"Ad": "P1"}, pts=rect(0, 0, 20, 30)),
    ent(2, "polygon", "parsel", {"Ad": "P2"}, pts=rect(20, 0, 45, 30)),
    ent(3, "polygon", "parsel", {"Ad": "P3"}, pts=rect(45, 0, 60, 20)),
    ent(4, "polygon", "parsel", {"Ad": "P4"}, pts=rect(60, 20, 80, 40)),
    ent(5, "polygon", "parsel", {"Ad": "P5"}, pts=rect(75, 30, 95, 50)),
    ent(6, "polygon", "parsel", {"Ad": "P6"}, pts=[P(0, 40), P(20, 40), P(20, 60), P(0, 60)], bulges=[0, -0.5, 0, 0]),
    ent(7, "polygon", "parsel", {"Ad": "P7"}, pts=[P(20, 40), P(40, 40), P(40, 60), P(20, 60)], bulges=[0, 0, 0, 0.5]),
    ent(11, "point", "durak", {"Ad": "Çarşı"}, p=P(10, -5)),
    ent(12, "point", "durak", {"Ad": "Okul önü"}, p=P(50, 35)),
    ent(13, "point", "durak", {"Ad": "Sanayi"}, p=P(100, 10)),
    ent(14, "point", "durak", {"Ad": "Pazar"}, p=P(30, 33)),
    ent(21, "point", "okul", {"Ad": "Atatürk"}, p=P(10, 50)),
    ent(22, "point", "okul", {"Ad": "Cumhuriyet"}, p=P(70, 10)),
    ent(31, "polyline", "yol", {"Ad": "Ana cadde"}, pts=[P(-10, -10), P(110, -10)]),
    ent(32, "polyline", "yol", {"Ad": "Dere yolu"}, pts=[P(100, -10), P(100, 60)]),
]

DOCUMENT = {
    "format": "kentos.document",
    "version": 1,
    "name": "İşlem durumları: yakınlık",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                 "workspace": "gis", "drawingFont": "barlow"},
    "origin": {"x": E, "y": N},
    "layers": LAYERS,
    "activeLayer": "cizim",
    "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}

BY_ID = {e["id"]: e for e in ENTITIES}


def on_layer(layer):
    return [e["id"] for e in ENTITIES if e["layerId"] == layer]


# ── Geometry: straight objects (the arcs only in the neighbours' shared length) ──

def xy(q):
    return (q["x"], q["y"])


def shape(i):
    """(points, edges, ring or None) of a straight object; coordinates absolute."""
    e = BY_ID[i]
    if e["kind"] == "point":
        return [xy(e["p"])], [], None
    if e["kind"] == "polyline":
        pts = [xy(q) for q in e["pts"]]
        return [], list(zip(pts, pts[1:])), None
    if e["kind"] == "polygon":
        pts = [xy(q) for q in e["pts"]]
        return [], list(zip(pts, pts[1:] + pts[:1])), pts
    raise ValueError(e["kind"])


def dist(p, q):
    return math.hypot(p[0] - q[0], p[1] - q[1])


def closest_on_seg(seg, p):
    (ax, ay), (bx, by) = seg
    dx, dy = bx - ax, by - ay
    l2 = dx * dx + dy * dy
    t = 0.0 if l2 == 0 else max(0.0, min(1.0, ((p[0] - ax) * dx + (p[1] - ay) * dy) / l2))
    q = (ax + t * dx, ay + t * dy)
    return q, dist(p, q)


def segs_cross(s1, s2):
    def orient(a, b, c):
        return (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    a, b = s1
    c, d = s2
    o1, o2, o3, o4 = orient(a, b, c), orient(a, b, d), orient(c, d, a), orient(c, d, b)
    return (o1 > 0) != (o2 > 0) and (o3 > 0) != (o4 > 0) and o1 != 0 and o2 != 0 and o3 != 0 and o4 != 0


def inside_ring(ring, p):
    x, y = p
    c = False
    for (ax, ay), (bx, by) in zip(ring, ring[1:] + ring[:1]):
        if (ay > y) != (by > y) and x < (bx - ax) * (y - ay) / (by - ay) + ax:
            c = not c
    return c


def seg_gap(s1, s2):
    return min(min(closest_on_seg(s2, p)[1] for p in s1), min(closest_on_seg(s1, q)[1] for q in s2))


def meet(i, j):
    pa, ea, ra = shape(i)
    pb, eb, rb = shape(j)
    for s1 in ea:
        for s2 in eb:
            if segs_cross(s1, s2) or seg_gap(s1, s2) <= TOL:
                return True
    for p in pa:
        if any(closest_on_seg(s, p)[1] <= TOL for s in eb) or (rb and inside_ring(rb, p)) or any(dist(p, q) <= TOL for q in pb):
            return True
    for q in pb:
        if any(closest_on_seg(s, q)[1] <= TOL for s in ea) or (ra and inside_ring(ra, q)):
            return True
    return bool((ra and rb and (inside_ring(rb, ra[0]) or inside_ring(ra, rb[0]))))


def nearest(i, j):
    """(d, a, b): rule 1's nearest points from i to j."""
    if meet(i, j):
        return 0.0, None, None
    pa, ea, _ = shape(i)
    pb, eb, _ = shape(j)
    best = [math.inf, None, None]

    def take(d, a, b):
        if d < best[0]:
            best[:] = [d, a, b]
    for p in pa:
        for q in pb:
            take(dist(p, q), p, q)
        for s in eb:
            c, d = closest_on_seg(s, p)
            take(d, p, c)
    for q in pb:
        for s in ea:
            c, d = closest_on_seg(s, q)
            take(d, c, q)
    for s1 in ea:
        for s2 in eb:
            for p in s1:
                c, d = closest_on_seg(s2, p)
                take(d, p, c)
            for q in s2:
                c, d = closest_on_seg(s1, q)
                take(d, c, q)
    return tuple(best)


def centre(i):
    p, _, ring = shape(i)
    if ring is None:
        return p[0]
    a = cx = cy = 0.0
    for (x0, y0), (x1, y1) in zip(ring, ring[1:] + ring[:1]):
        cross = x0 * y1 - x1 * y0
        a += cross
        cx += (x0 + x1) * cross
        cy += (y0 + y1) * cross
    return (cx / (3 * a), cy / (3 * a))


def found(inputs, targets, k, max_d, measure):
    """Rule 3: each input's targets as (input place, target place, d, a, b)."""
    out = []
    for i, a in enumerate(inputs):
        rows = []
        for j, b in enumerate(targets):
            if a == b:
                continue
            if measure == "centers":
                ca, cb = centre(a), centre(b)
                rows.append((dist(ca, cb), j, ca, cb))
            else:
                d, pa, pb = nearest(a, b)
                rows.append((d, j, pa, pb))
        rows = [r for r in rows if r[0] <= max_d]
        rows.sort(key=lambda r: (r[0], r[1]))
        if k:
            rows = rows[:k]
        out.extend((i, j, d, pa, pb) for d, j, pa, pb in rows)
    return out


def bearing(a, b):
    t = math.atan2(b[0] - a[0], b[1] - a[1])
    return t + 2 * math.pi if t < 0 else t


def length(d):
    return shown(d, 3)


def name(i, field, place):
    v = BY_ID[i]["attrs"].get(field, "").strip() if field else ""
    return v or (BY_ID[i].get("label") or "").strip() or str(place + 1)


# ── Neighbours ──

ARC = {"c": (E + 12.5, N + 50), "r": 12.5, "sweep": 4 * math.atan(0.5)}


def boundary(i):
    """The edges of an area: ("seg", a, b) or ("arc", centre, radius, start angle, extent counter-clockwise)."""
    e = BY_ID[i]
    pts = [xy(q) for q in e["pts"]]
    bulges = e.get("bulges", [0] * len(pts))
    out = []
    for k, (a, b) in enumerate(zip(pts, pts[1:] + pts[:1])):
        if bulges[k]:
            c = ARC["c"]
            # Both arcs go round from (20, 40) to (20, 60) counter-clockwise.
            start = math.atan2(N + 40 - c[1], E + 20 - c[0])
            out.append(("arc", c, ARC["r"], start, ARC["sweep"]))
        else:
            out.append(("seg", a, b))
    return out


def edge_overlap(e1, e2):
    if e1[0] == "seg" and e2[0] == "seg":
        (p, q), (r, s) = (e1[1], e1[2]), (e2[1], e2[2])
        if dist(r, s) > dist(p, q):
            (p, q), (r, s) = (r, s), (p, q)
        L = dist(p, q)
        ux, uy = (q[0] - p[0]) / L, (q[1] - p[1]) / L
        off = lambda v: abs((v[0] - p[0]) * uy - (v[1] - p[1]) * ux)  # noqa: E731
        if off(r) > TOL or off(s) > TOL:
            return 0.0
        along = lambda v: (v[0] - p[0]) * ux + (v[1] - p[1]) * uy  # noqa: E731
        t0, t1 = along(r), along(s)
        return max(0.0, min(L, max(t0, t1)) - max(0.0, min(t0, t1)))
    if e1[0] == "arc" and e2[0] == "arc":
        if dist(e1[1], e2[1]) > TOL or abs(e1[2] - e2[2]) > TOL:
            return 0.0
        lo, hi = max(e1[3], e2[3]), min(e1[3] + e1[4], e2[3] + e2[4])
        return max(0.0, hi - lo) * e1[2]
    return 0.0


def rect_of(i):
    xs = [q["x"] for q in BY_ID[i]["pts"]]
    ys = [q["y"] for q in BY_ID[i]["pts"]]
    return min(xs), min(ys), max(xs), max(ys)


def overlap_area(i, j):
    """The drawing's overlapping areas are rectangles."""
    a, b = rect_of(i), rect_of(j)
    w = min(a[2], b[2]) - max(a[0], b[0])
    h = min(a[3], b[3]) - max(a[1], b[1])
    return w * h if w > 0 and h > 0 else 0.0


def neighbours(ids, corners, overlaps):
    out = []
    for i, a in enumerate(ids):
        for j, b in enumerate(ids):
            if i == j:
                continue
            shared = sum(edge_overlap(e1, e2) for e1 in boundary(a) for e2 in boundary(b))
            if overlaps and overlap_area(a, b) > 0:
                out.append((i, j, "Örtüşme", shared, overlap_area(a, b)))
            elif shared > TOL:
                out.append((i, j, "Kenar", shared, 0.0))
            elif corners and touch(a, b):
                out.append((i, j, "Köşe", shared, 0.0))
    return out


def touch(a, b):
    """Rectangles that meet: their closures share a point."""
    if BY_ID[a].get("bulges") or BY_ID[b].get("bulges"):
        return False
    x, y = rect_of(a), rect_of(b)
    return x[0] <= y[2] + TOL and y[0] <= x[2] + TOL and x[1] <= y[3] + TOL and y[1] <= x[3] + TOL


# ── The cases ──

def updated(changes):
    out = []
    for i in sorted(changes):
        e = BY_ID[i]
        u = {"id": i, "attrs": changes[i]}
        if "label" in e:
            u["label"] = e["label"]
        out.append(u)
    return out


def case_nearest(cid, title, inputs, max_d):
    targets = on_layer("durak")
    rows = {r[0]: r for r in found(inputs, targets, 1, max_d if max_d > 0 else math.inf, "edges")}
    changes = {}
    none = 0
    for i, a in enumerate(inputs):
        attrs = dict(BY_ID[a]["attrs"])
        r = rows.get(i)
        if r is None:
            none += 1
        else:
            t = BY_ID[targets[r[1]]]
            attrs["Yakın uzaklık"] = length(r[2])
            attrs["Yakın Ad"] = t["attrs"]["Ad"]
            if r[2] > 0:
                attrs["Yakın semt"] = shown(bearing(r[3], r[4]) * 200 / math.pi, 4)
        if attrs != BY_ID[a]["attrs"]:
            changes[a] = attrs
    log = [{"level": "info", "text": f"{none} nesnenin {'en çok uzaklık içinde ' if max_d > 0 else ''}hedefi yok; alanları boşaltıldı."}] if none else []
    expect = {"status": "ok", "summary": f"{len(inputs) - none} nesneye en yakın hedef yazıldı (“Yakın uzaklık”).", "undo": "En yakını bul",
              "updated": updated(changes), "outputs": {"count": len(inputs) - none, "changed": sorted(changes)}}
    if log:
        expect["log"] = log
    return {"id": cid, "title": title, "document": "proximity.kcad", "selection": inputs, "run": {"tool": "proximity.nearest"},
            "values": {"input": {"scope": "selection"}, "targets": {"scope": "layer", "layerId": "durak"}, "measure": "edges",
                       "max": max_d, "fields": "Ad", "prefix": "Yakın ", "bearing": True},
            "expect": expect}


def case_matrix(cid, title, form, k):
    inputs = [1, 2, 3]
    targets = on_layer("durak")
    rows_found = found(inputs, targets, k, math.inf, "edges")
    src = lambda i: name(inputs[i], "Ad", i)  # noqa: E731
    tgt = lambda j: name(targets[j], "Ad", j)  # noqa: E731
    rows = []
    if form == "list":
        columns = ["Kaynak", "Hedef", "Sıra", "Uzaklık"]
        last, rank = -1, 0
        for i, j, d, _, _ in rows_found:
            rank = rank + 1 if i == last else 1
            last = i
            rows.append([src(i), tgt(j), str(rank), length(d)])
    elif form == "matrix":
        columns = ["Kaynak"] + [tgt(j) for j in range(len(targets))]
        cells = [[""] * len(targets) for _ in inputs]
        for i, j, d, _, _ in rows_found:
            cells[i][j] = length(d)
        rows = [[src(i)] + cells[i] for i in range(len(inputs))]
    else:
        columns = ["Kaynak", "Hedef sayısı", "En az", "Ortalama", "En çok"]
        for i in range(len(inputs)):
            ds = [d for ii, _, d, _, _ in rows_found if ii == i]
            s = 0.0
            for d in ds:
                s += d
            rows.append([src(i), str(len(ds)), length(ds[0]), length(s / len(ds)), length(ds[-1])])
    return {"id": cid, "title": title, "document": "proximity.kcad", "selection": inputs, "run": {"tool": "proximity.matrix"},
            "values": {"input": {"scope": "selection"}, "targets": {"scope": "layer", "layerId": "durak"}, "measure": "edges", "k": k,
                       "max": 0, "form": form, "name": "Ad", "targetName": "Ad"},
            "expect": {"status": "ok", "summary": f"{len(inputs)} kaynaktan {len(rows_found)} uzaklık ölçüldü.", "undo": None,
                       "outputs": {"count": len(rows_found), "table": {"columns": columns, "rows": rows}}}}


OUTPUT_STYLE = lambda c: {"color": c, "lineType": "continuous", "lineWeight": 0.25, "fill": f"{c}26"}  # noqa: E731


def case_hub():
    inputs = on_layer("durak")
    hubs = on_layer("okul")
    added = []
    for i, j, d, a, b in found(inputs, hubs, 1, math.inf, "centers"):
        attrs = dict(BY_ID[inputs[i]]["attrs"])
        attrs["Merkez"] = name(hubs[j], "Ad", j)
        attrs["Uzaklık"] = length(d)
        added.append({"kind": "line", "layerId": "islem-merkeze-baglantilar", "attrs": attrs, "a": {"x": a[0], "y": a[1]}, "b": {"x": b[0], "y": b[1]}})
    return {"id": "hub-stops-schools", "title": "En yakın merkeze bağla: her durak en yakın okula, merkezden merkeze; çizgiler durağın özniteliklerini, Merkez'i ve Uzaklık'ı taşır",
            "document": "proximity.kcad", "run": {"tool": "proximity.hub"},
            "values": {"input": {"scope": "layer", "layerId": "durak"}, "hubs": {"scope": "layer", "layerId": "okul"}, "hubName": "Ad", "max": 0,
                       "layer": {"newName": "Merkeze bağlantılar"}},
            "expect": {"status": "ok", "summary": f"{len(added)} nesne en yakın merkeze bağlandı.", "undo": "En yakın merkeze bağla",
                       "layers": [{"id": "islem-merkeze-baglantilar", "name": "Merkeze bağlantılar", "style": OUTPUT_STYLE("#E5732E")}],
                       "added": added, "outputs": {"count": len(added)}}}


def case_neighbours():
    ids = on_layer("parsel")
    rows_found = neighbours(ids, True, True)
    rows = [[name(ids[i], "Ad", i), name(ids[j], "Ad", j), kind, length(L), shown(area, 2) if kind == "Örtüşme" else ""]
            for i, j, kind, L, area in rows_found]
    changes = {}
    for i, a in enumerate(ids):
        names = [name(ids[j], "Ad", j) for ii, j, *_ in rows_found if ii == i]
        attrs = dict(BY_ID[a]["attrs"])
        attrs["Komşu sayısı"] = str(len(names))
        if names:
            attrs["Komşular"] = ", ".join(names)
        changes[a] = attrs
    overlapping = sum(1 for r in rows_found if r[2] == "Örtüşme") // 2
    return {"id": "neighbours-parcels", "title": "Komşu alanlar: ortak kenarlar (biri yaylı), köşeden değen ve örtüşen parseller; komşu sayısı ve adları yazılır",
            "document": "proximity.kcad", "run": {"tool": "proximity.neighbors"},
            "values": {"input": {"scope": "layer", "layerId": "parsel"}, "tolerance": 0.001, "corners": True, "overlaps": True, "name": "Ad",
                       "write": True, "countField": "Komşu sayısı", "listField": "Komşular"},
            "expect": {"status": "ok", "summary": f"{len(ids)} alanda {len(rows_found) // 2} komşuluk bulundu.", "undo": "Komşu alanlar",
                       "log": [{"level": "warn", "text": f"{overlapping} alan çiftinin içleri örtüşüyor."}],
                       "updated": updated(changes),
                       "outputs": {"count": len(rows_found), "table": {"columns": ["Alan", "Komşu", "Komşuluk", "Ortak kenar", "Örtüşen alan"], "rows": rows}}}}


def case_shortest():
    inputs = [1, 2]
    targets = on_layer("yol")
    added = []
    last, rank = -1, 0
    for i, j, d, a, b in found(inputs, targets, 2, math.inf, "edges"):
        rank = rank + 1 if i == last else 1
        last = i
        attrs = {"Kaynak": name(inputs[i], "Ad", i), "Hedef": name(targets[j], "Ad", j), "Sıra": str(rank), "Uzaklık": length(d)}
        added.append({"kind": "line", "layerId": "islem-en-kisa-cizgiler", "attrs": attrs, "a": {"x": a[0], "y": a[1]}, "b": {"x": b[0], "y": b[1]}})
    return {"id": "shortest-lines-roads", "title": "En kısa çizgi: iki parselden en yakın iki yola, en yakın noktalar arası; Kaynak, Hedef, Sıra, Uzaklık",
            "document": "proximity.kcad", "selection": inputs, "run": {"tool": "proximity.shortestLine"},
            "values": {"input": {"scope": "selection"}, "targets": {"scope": "layer", "layerId": "yol"}, "k": 2, "max": 0, "name": "Ad", "targetName": "Ad",
                       "layer": {"newName": "En kısa çizgiler"}},
            "expect": {"status": "ok", "summary": f"{len(added)} en kısa çizgi yazıldı.", "undo": "En kısa çizgi",
                       "layers": [{"id": "islem-en-kisa-cizgiler", "name": "En kısa çizgiler", "style": OUTPUT_STYLE("#8A3FFC")}],
                       "added": added, "outputs": {"count": len(added)}}}


def case_matrix_centres():
    return {"id": "matrix-centres", "title": "Uzaklık matrisi, merkezden merkeze: bir parselin ağırlık merkezinden iki okula (hepsi), sırasıyla",
            "document": "proximity.kcad", "selection": [1], "run": {"tool": "proximity.matrix"},
            "values": {"input": {"scope": "selection"}, "targets": {"scope": "layer", "layerId": "okul"}, "measure": "centers", "k": 0, "max": 0,
                       "form": "list", "name": "Ad", "targetName": "Ad"},
            "expect": {"status": "ok", "summary": "1 kaynaktan 2 uzaklık ölçüldü.", "undo": None,
                       "outputs": {"count": 2, "table": {"columns": ["Kaynak", "Hedef", "Sıra", "Uzaklık"], "rows": [
                           ["P1", name(21, "Ad", 0), "1", length(dist(centre(1), centre(21)))],
                           ["P1", name(22, "Ad", 1), "2", length(dist(centre(1), centre(22)))]]}}}}


def build():
    cases = [
        case_nearest("nearest-stops", "En yakını bul: beş parselin en yakın durağı, uzaklığı, adı ve semti (grad); eşitsiz en yakın noktalar", [1, 2, 3, 4, 5], 0),
        case_nearest("nearest-stops-within", "En yakını bul, en çok 12 m: iki parselin hedefi yok, alanları boşaltılır ve sayısı söylenir", [1, 2, 3, 4, 5], 12),
        case_matrix("matrix-list", "Uzaklık matrisi, Liste: üç parselden en yakın iki durağa, sıralarıyla", "list", 2),
        case_matrix("matrix-matrix", "Uzaklık matrisi, Matris: üç parsel × dört durak", "matrix", 0),
        case_matrix("matrix-summary", "Uzaklık matrisi, Özet: her parselin dört durağa en az, ortalama ve en çok uzaklığı", "summary", 0),
        case_matrix_centres(),
        case_hub(),
        case_neighbours(),
        case_shortest(),
    ]
    return {"format": "kentos.processing-cases", "version": 1, "tolerance": 1e-9, "documents": {}, "cases": cases}


def main():
    proc = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    doc = json.dumps(DOCUMENT, ensure_ascii=False, indent=1) + "\n"
    outs = [(PROC_DIR / "proximity.json", proc), (PROC_DIR / "proximity.kcad", doc)]
    if "--check" in sys.argv[1:]:
        bad = [p for p, t in outs if not p.exists() or p.read_text() != t]
        for p in bad:
            print(f"{p.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
        if bad:
            return 1
        print("yakınlık durumları tutarlı: fixtures/processing/v1/proximity.*")
        return 0
    for p, t in outs:
        p.write_text(t)
        print(f"yazıldı: {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
