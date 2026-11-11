#!/usr/bin/env python3
"""The shared cases of `cad.scenarios.impact` (Senaryo ölçüleri; plan/map-cad.md MAP-10, docs/adr/1440), without
KentOS code.

    python3 scripts/fixtures/scenarios_impact_cases.py           # writes fixtures/commands/v1/cad.scenarios.impact.json
    python3 scripts/fixtures/scenarios_impact_cases.py --check   # writes nothing; compares

The rules, by hand:
- the pairs: each layer of the scenario group that stands for (`replaces`) a base layer, in the tree's order;
- each side over its layers, in the drawing's order: objects; areas (rectangles here: exact shoelace); lines' length
  (hypot); capacity Σ area × KAKS over the areas whose use is not park, yeşil, meydan or yol, an area without a KAKS
  counted as a gap; the areas by use (the field's trimmed value, “” for none), sorted by code;
- the changes, matched by the key (first object of a key on its side): the base's keys in order (removed, or geometry
  when the vertices differ, decision when the non-empty trimmed attributes differ, both), then the alternative's new
  keys (added);
- the stamp: SHA-256 over the base layers' objects, “id|layer|kind|k=v;…|xbits,ybits;…”, its first 8 bytes in hex;
  `stale` is whether a given stamp differs from it;
- asked for (docs/adr/2040), the user's values: the impervious area Σ area × the share of its use (a use with area and
  no share a gap), and the area reached, a `from` area whose centroid (a rectangle's middle) lies within the distance
  of a `to` area (the distance to a rectangle: 0 inside, else to its box), with its share of the `from` areas (0 with
  none); a side with `from` areas and no `to` area a gap; and on each side, by persistent id in the drawing's order,
  the `from` areas reached, those not and the `to` areas (`reach`, docs/adr/2260);
- asked for (docs/adr/2660), the area near an infrastructure layer: a measured area (of the `from` uses, or every
  area) whose centroid lies within the distance of an object of the layer (lines here: the distance to their
  segments), with its share; the layer the scenario stands in for is each side's own; a side with measured areas and
  no object on its layer a gap;
- asked for (docs/adr/4260), the slope on the terrain surface “Zemin” (four points, its two triangles the Delaunay
  ones, decided here by the empty circle): each triangle's slope from its plane, 100 · √(a² + b²) for z = c + a·x + b·y;
  each measured area clipped by each triangle (Sutherland–Hodgman, exact fractions), the parts' area-weighted mean
  slope, the steep part (slope ≥ the threshold) and its share, the part off the surface (a gap above 1e-6 m²);
- docs/adr/4660: with the user's rising breaks, the area of each
  class (from 0 to the first break, between two, from the last up, each holding its lower break); with `aspect`, the
  area by the direction each triangle's slope faces: the upward normal's horizontal part (−a, −b) for z = c + a·x + b·y,
  clockwise from north, in the eight directions (north 337.5°–22.5° …, each holding its lower bound), a level triangle
  (a = b = 0, exactly) flat.
- docs/adr/5060: with `slope.map`, each measured area's parts on the triangles (the same exact clipping), each with its
  class, aspect, slope, size and ring (counter-clockwise from its lowest vertex, the westmost of equals; repeated and
  straight-through vertices dropped), an area's parts by their first vertex (south, then west); with `access.network`,
  the access along the project's network: the network's lines cut at their ends and where another's end lies on them,
  each area's centroid placed at the nearest point of the lines (off the network when farther than `connect`), the
  shortest way between two places by Dijkstra over the cut lines, a `from` area reaching when its leg + the way + the
  `to` area's leg is the distance at most (of equal totals the first `to` area), and each way as its points; the
  scenario layers' stamp as the base's; the tables written in the drawing (`SenaryoKimlik`) read back with whether
  either stamp changed.
"""

import copy
import hashlib
import json
import math
import struct
import sys
from fractions import Fraction
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.scenarios.impact.json"
X0, Y0 = 487000.0, 4420000.0
OPEN = {"park", "yesil", "meydan", "yol"}


def uid(n):
    return f"0192f5c0-1440-7000-8000-{n:012x}"


def node(id_, name, kind="layer", children=None, **more):
    return {"id": id_, "name": name, "type": kind, "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, **more,
            "children": children or []}


def tree(infra=False, ground=False):
    extra = [node("altyapi", "Kanalizasyon hattı")] if infra else []
    extra += [node("zemin", "Zemin noktaları")] if ground else []
    return extra + [node("0", "0"),
            node("imar", "İmar planı", "group", [node("karar", "Karar alanları"), node("yol", "Yollar")]),
            node("alt-a", "Alternatif A", "group", [node("alt-a-karar", "Karar alanları", replaces="karar"),
                                                     node("alt-a-yol", "Yollar", replaces="yol")],
                 scenario={"note": "Okul yanına yeşil alan, ticaret konuta"})]


def P(x, y):
    return {"x": X0 + x, "y": Y0 + y}


def area(layer, r, **attrs):
    x0, y0, x1, y1 = r
    return {"kind": "polygon", "layerId": layer, "attrs": attrs, "pts": [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]}


def line(layer, a, b, **attrs):
    return {"kind": "line", "layerId": layer, "attrs": attrs, "a": P(*a), "b": P(*b)}


BASE = [
    area("karar", (0, 0, 40, 50), KararKimlik="K1", Kullanim="konut", KAKS="1.20"),
    area("karar", (40, 0, 70, 50), KararKimlik="K2", Kullanim="ticaret-konut", KAKS="1.75"),
    area("karar", (70, 0, 100, 30), KararKimlik="K3", Kullanim="park"),
    area("karar", (70, 30, 100, 50), KararKimlik="K4", Kullanim="egitim", KAKS="1.00"),
    area("karar", (100, 0, 120, 50), KararKimlik="K6", Kullanim="sanayi", KAKS="1.00"),
    line("yol", (0, -5), (100, -5), YolKimlik="Y1"),
]
ALT = [
    area("alt-a-karar", (0, 0, 40, 50), KararKimlik="K1", Kullanim="konut", KAKS="1.20"),
    area("alt-a-karar", (40, 0, 70, 50), KararKimlik="K2", Kullanim="konut", KAKS="1.50"),
    area("alt-a-karar", (70, 0, 100, 20), KararKimlik="K3", Kullanim="park"),
    area("alt-a-karar", (70, 30, 100, 50), KararKimlik="K4", Kullanim="egitim", KAKS="1.00"),
    area("alt-a-karar", (70, 20, 100, 30), KararKimlik="K5", Kullanim="yesil"),
    area("alt-a-karar", (120, 0, 130, 10), Kullanim="konut"),
    line("alt-a-yol", (0, -5), (100, -5), YolKimlik="Y1"),
    line("alt-a-yol", (100, -5), (100, 60), YolKimlik="Y2"),
]


def setup(base=None, alt=None, layers=None, extra=(), work="map", surfaces=None):
    ents = [copy.deepcopy(e) for e in (base if base is not None else BASE) + (alt if alt is not None else ALT)
            + list(extra)]
    for i, e in enumerate(ents, start=1):
        e["id"] = i
    settings = {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": work}
    if surfaces is not None:
        settings["road"] = {"surfaces": surfaces}
    return {"format": "kentos.document", "version": 2, "name": "Senaryo ölçüleri",
            "settings": settings,
            "origin": {"x": X0, "y": Y0}, "layers": layers or tree(), "activeLayer": "karar", "entities": ents,
            "uids": [uid(i + 1) for i in range(len(ents))], "styles": {"items": [], "categories": []}}


# ── The measures ─────────────────────────────────────────────────────────────────────────────────────────────────


def bits(v):
    return format(struct.unpack(">Q", struct.pack(">d", v))[0], "016x")


def verts(e):
    if e["kind"] == "line":
        return [(e["a"]["x"], e["a"]["y"]), (e["b"]["x"], e["b"]["y"])]
    return [(p["x"], p["y"]) for p in e["pts"]]


def shoelace(e):
    q = [(Fraction(x), Fraction(y)) for x, y in verts(e)]
    return float(abs(sum(q[i][0] * q[(i + 1) % len(q)][1] - q[(i + 1) % len(q)][0] * q[i][1]
                         for i in range(len(q)))) / 2)


def length(e):
    (ax, ay), (bx, by) = verts(e)
    return ((bx - ax) ** 2 + (by - ay) ** 2) ** 0.5


def pairs(s):
    out = []

    def walk(ns):
        for n in ns:
            if n["type"] == "layer" and n.get("replaces"):
                out.append((n["replaces"], n["id"]))
            walk(n["children"])
    walk(next(n for n in s["layers"] if n["id"] == "alt-a")["children"])
    return out


def kaks(e):
    try:
        v = float(e["attrs"].get("KAKS", "").strip().replace(",", "."))
        return v
    except ValueError:
        return None


def side(s, layers, field, key):
    out = {"objects": 0, "area": 0.0, "length": 0.0, "capacity": 0.0, "noKaks": 0, "noUse": 0, "noKey": 0,
           "uses": {}, "keyed": [], "areas": []}
    for layer in layers:
        for e in [e for e in s["entities"] if e["layerId"] == layer]:
            out["objects"] += 1
            get = lambda k: e["attrs"].get(k, "").strip()  # noqa: E731
            if e["kind"] == "polygon":
                a = shoelace(e)
                out["area"] += a
                u = get(field)
                out["areas"].append((e, u, a))
                if not u:
                    out["noUse"] += 1
                out["uses"][u] = out["uses"].get(u, 0.0) + a
                if u not in OPEN:
                    k = kaks(e)
                    if k is None:
                        out["noKaks"] += 1
                    else:
                        out["capacity"] += a * k
                kk = get(key)
                if not kk:
                    out["noKey"] += 1
                elif kk not in [x[0] for x in out["keyed"]]:
                    out["keyed"].append((kk, e, a))
            elif e["kind"] == "line":
                out["length"] += length(e)
    return out


def capacity(e, a, field):
    if e["attrs"].get(field, "").strip() in OPEN:
        return 0.0
    k = kaks(e)
    return None if k is None else a * k


def stamp(s, layers):
    h = hashlib.sha256()
    for layer in layers:
        for e in [e for e in s["entities"] if e["layerId"] == layer]:
            attrs = ";".join(f"{k}={v}" for k, v in sorted(e["attrs"].items()))
            pts = ";".join(f"{bits(x)},{bits(y)}" for x, y in verts(e))
            h.update(f"{e['id']}|{layer}|{e['kind']}|{attrs}|{pts}\n".encode())
    return h.digest()[:8].hex()


def box(e):
    xs, ys = [p["x"] for p in e["pts"]], [p["y"] for p in e["pts"]]
    return min(xs), min(ys), max(xs), max(ys)


def impervious(sd, shares):
    total, missing = 0.0, []
    for _, u, a in sd["areas"]:
        if u in shares:
            total += a * shares[u]
        elif a > 0 and u not in missing:
            missing.append(u)
    return total, missing


def reached(sd, access):
    """The area reached, the `from` areas' whole, and by persistent id the `from` areas reached, those not and the `to`
    areas (docs/adr/2260), in the drawing's order."""
    to = [e for e, u, _ in sd["areas"] if u in access["to"]]
    targets = [box(e) for e in to]
    got = whole = 0.0
    yes, no = [], []
    for e, u, a in sd["areas"]:
        if u not in access["from"]:
            continue
        whole += a
        x0, y0, x1, y1 = box(e)
        cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
        near = lambda t: (max(t[0] - cx, 0, cx - t[2]) ** 2 + max(t[1] - cy, 0, cy - t[3]) ** 2) ** 0.5  # noqa: E731
        if any(near(t) <= access["distance"] for t in targets):
            got += a
            yes.append(uid(e["id"]))
        else:
            no.append(uid(e["id"]))
    return got, whole, bool(targets), yes, no, [uid(e["id"]) for e in to]


def to_segment(p, a, b):
    (px, py), (ax, ay), (bx, by) = p, a, b
    dx, dy = bx - ax, by - ay
    t = max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)))
    return ((px - ax - t * dx) ** 2 + (py - ay - t * dy) ** 2) ** 0.5


def nearest_on_segment(p, a, b):
    """The segment's nearest place to p and its distance (the foot of the perpendicular, held within the ends)."""
    (px, py), (ax, ay), (bx, by) = p, a, b
    dx, dy = bx - ax, by - ay
    t = max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)))
    q = (ax + t * dx, ay + t * dy)
    return q, ((px - q[0]) ** 2 + (py - q[1]) ** 2) ** 0.5


def near(s, sd, layer, infra):
    """The area near the layer's objects (docs/adr/2660), the measured areas' whole, the layer's objects; and the
    measured areas near and beyond and the layer's objects by persistent id (docs/adr/3060); and each measured area
    joined to its nearest object (docs/adr/3460): the first of equals in the drawing's order, its nearest place."""
    objs = [e for e in s["entities"] if e["layerId"] == layer]
    lines = [verts(e) for e in objs]
    uses = [u for u in infra.get("from", []) if u.strip()]
    got = whole = 0.0
    yes, no, links = [], [], []
    for e, u, a in sd["areas"]:
        if uses and u not in uses:
            continue
        whole += a
        x0, y0, x1, y1 = box(e)
        c = ((x0 + x1) / 2, (y0 + y1) / 2)
        if any(to_segment(c, p, q) <= infra["distance"] for pts in lines for p, q in zip(pts, pts[1:])):
            got += a
            yes.append(uid(e["id"]))
        else:
            no.append(uid(e["id"]))
        tried = [(k,) + nearest_on_segment(c, p, q) for k, pts in enumerate(lines) for p, q in zip(pts, pts[1:])]
        if tried:
            best = min(tried, key=lambda x: (x[2], x[0]))
            # Two objects at exactly one distance (axis-parallel roads, whole metres: both sides compute it exactly)
            # take the first in the drawing's order; none at nearly one distance, which would hang on the last digit.
            assert all(x[0] == best[0] or x[2] == best[2] or x[2] - best[2] > 1e-6 for x in tried), (e["id"], tried)
            links.append({"area": uid(e["id"]), "line": uid(objs[best[0]]["id"]), "from": [c[0], c[1]],
                          "to": [best[1][0], best[1][1]], "distance": best[2]})
    return got, whole, len(lines), yes, no, [uid(e["id"]) for e in objs], links


def canonical_ring(pts):
    """A ring as the commands write it: repeated and straight-through vertices dropped, counter-clockwise, from its
    lowest vertex (the westmost of equals)."""
    pts = [p for i, p in enumerate(pts) if p != pts[i - 1]] if len(pts) > 1 else list(pts)
    changed = True
    while changed and len(pts) > 3:
        changed = False
        for i in range(len(pts)):
            a, b, c = pts[i - 1], pts[i], pts[(i + 1) % len(pts)]
            if (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]) == 0:
                del pts[i]
                changed = True
                break
    area2 = sum(pts[i][0] * pts[(i + 1) % len(pts)][1] - pts[(i + 1) % len(pts)][0] * pts[i][1] for i in range(len(pts)))
    if area2 < 0:
        pts = pts[::-1]
    k = min(range(len(pts)), key=lambda i: (pts[i][1], pts[i][0]))
    return pts[k:] + pts[:k]


# ── docs/adr/5060: the access along a network ─────────────────────────────────────────────────────────────────────

def net_segments(s, layer):
    return [((e["a"]["x"] - X0, e["a"]["y"] - Y0), (e["b"]["x"] - X0, e["b"]["y"] - Y0))
            for e in s["entities"] if e["layerId"] == layer and e["kind"] == "line"]


def on_segment(p, a, b):
    """Where p lies on ab (0..1), exactly, or None."""
    cross = (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
    if cross != 0:
        return None
    dx, dy = b[0] - a[0], b[1] - a[1]
    t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)
    return t if 0 <= t <= 1 else None


def net_place(segs, c, connect):
    """The nearest point of the lines to c within `connect`: (segment, t, point, distance), or None."""
    tried = []
    for k, (a, b) in enumerate(segs):
        q, d = nearest_on_segment(c, a, b)
        dx, dy = b[0] - a[0], b[1] - a[1]
        t = ((q[0] - a[0]) * dx + (q[1] - a[1]) * dy) / (dx * dx + dy * dy)
        tried.append((d, k, t, q))
    if not tried:
        return None
    best = min(tried)
    # The cases keep one nearest line: a tie would hang on the graph's own order of pieces.
    assert all(x is best or x[0] - best[0] > 1e-6 or x[3] == best[3] for x in tried), (c, tried)
    return None if best[0] > connect else (best[1], best[2], best[3], best[0])


def net_way(segs, start, end):
    """The shortest way from place `start` to place `end` (segment, t, point, d) over the lines cut at their ends and
    where another's end lies on them: (length, points), or None."""
    cuts = [[(0.0, a), (1.0, b)] for a, b in segs]
    for i, (a, b) in enumerate(segs):
        for j, (c, d) in enumerate(segs):
            if i != j:
                for p in (c, d):
                    t = on_segment(p, a, b)
                    if t is not None and 0 < t < 1:
                        cuts[i].append((t, p))
    for k, t, q, _ in (start, end):
        cuts[k].append((t, q))
    graph = {}
    for pts in cuts:
        pts = sorted(set(pts))
        for (_, p), (_, q) in zip(pts, pts[1:]):
            if p != q:
                w = math.hypot(q[0] - p[0], q[1] - p[1])
                graph.setdefault(p, []).append((q, w))
                graph.setdefault(q, []).append((p, w))
    src, dst = start[2], end[2]
    if src == dst:
        return 0.0, [src]
    dist, prev, done = {src: 0.0}, {}, set()
    while True:
        open_ = [(d, p) for p, d in dist.items() if p not in done]
        if not open_:
            return None
        d, p = min(open_)
        if p == dst:
            break
        done.add(p)
        for q, w in graph.get(p, []):
            if d + w < dist.get(q, math.inf):
                dist[q], prev[q] = d + w, p
    way = [dst]
    while way[-1] != src:
        way.append(prev[way[-1]])
    return dist[dst], way[::-1]


def along(s, sd, access, layer):
    """The access along the side's network layer `layer` (docs/adr/5060)."""
    segs = net_segments(s, layer)
    connect = access["connect"]
    centre = lambda e: ((box(e)[0] + box(e)[2]) / 2 - X0, (box(e)[1] + box(e)[3]) / 2 - Y0)  # noqa: E731
    ends, targets, off_targets = [], [], 0
    for e, u, _ in sd["areas"]:
        if u in access["to"]:
            targets.append(uid(e["id"]))
            at = net_place(segs, centre(e), connect)
            if at is None:
                off_targets += 1
            else:
                ends.append((uid(e["id"]), centre(e), at))
    got = whole = 0.0
    yes, no, routes, off = [], [], [], []
    for e, u, a in sd["areas"]:
        if u not in access["from"]:
            continue
        whole += a
        c = centre(e)
        at = net_place(segs, c, connect)
        if at is None:
            off.append(uid(e["id"]))
            no.append(uid(e["id"]))
            continue
        found = []
        for k, (tid, tc, tat) in enumerate(ends):
            w = net_way(segs, at, tat)
            if w is not None:
                found.append((at[3] + w[0] + tat[3], k, w[1]))
        if not found:
            no.append(uid(e["id"]))
            continue
        total, k, way = min(found, key=lambda x: (x[0], x[1]))
        assert all(x[1] == k or x[0] - total > 1e-6 for x in found), found
        tid, tc, tat = ends[k]
        pts = [c, at[2]] + way + [tat[2], tc]
        pts = [p for i, p in enumerate(pts) if i == 0 or p != pts[i - 1]]
        routes.append({"area": uid(e["id"]), "target": tid, "distance": total, "legs": [at[3], tat[3]],
                       "path": [[X0 + x, Y0 + y] for x, y in pts]})
        if total <= access["distance"]:
            got += a
            yes.append(uid(e["id"]))
        else:
            no.append(uid(e["id"]))
    return {"got": got, "whole": whole, "yes": yes, "no": no, "targets": targets, "routes": routes, "off": off,
            "offTargets": off_targets}


def rs(v):
    """A number as Rust writes an f64 (40.0 as 40)."""
    return str(int(v)) if float(v).is_integer() else repr(float(v))


def report(s, field="Kullanim", key="KararKimlik", since=None, impervious_=None, access=None, infrastructure=None,
           slope=None):
    ps = pairs(s)
    bases, alts = [b for b, _ in ps], [l for _, l in ps]
    b, a = side(s, bases, field, key), side(s, alts, field, key)

    def m(id_, label, unit, x, y, method):
        return {"id": id_, "label": label, "unit": unit, "base": x, "alternative": y, "difference": y - x,
                "method": method}
    measures = [
        m("objects", "Nesne sayısı", "count", b["objects"], a["objects"], "Karşılaştırılan katmanlardaki bütün nesneler."),
        m("area", "Alanların toplamı", "m2", b["area"], a["area"],
          "Alanların düzlemsel alanı, koordinatlardan (yay ve delikler dahil)."),
        m("length", "Çizgilerin uzunluğu", "m", b["length"], a["length"], "Çizgi ve çoklu çizgilerin düzlemsel uzunluğu."),
        m("capacity", "İnşaat alanı (alan × KAKS)", "m2", b["capacity"], a["capacity"],
          "Her yapılaşan alanın alanı × KAKS; KAKS'ı olmayan alan sayılmaz (Veri eksikleri)."),
    ]
    asked_gaps = []
    if impervious_ is not None:
        (x, mb), (y, ma) = impervious(b, impervious_), impervious(a, impervious_)
        given = ", ".join(f"{c} {rs(k)}" for c, k in sorted(impervious_.items()))
        measures.append(m("impervious", "Geçirimsiz alan", "m2", x, y,
                          f"Her alanın alanı × kullanımının verilen geçirimsizlik payı ({given}); payı verilmeyen kullanım "
                          "sayılmaz (Veri eksikleri)."))
        missing = sorted(set(mb) | set(ma))
        if missing:
            words = ", ".join(w or "(kullanımı yok)" for w in missing)
            asked_gaps.append(f"Geçirimsizlik payı verilmemiş kullanım: {words}; geçirimsiz alan onlarsız hesaplandı.")
    reach = None
    near_out = None
    slope_map = None
    if access is not None and access.get("network"):
        net = s["settings"]["networks"][0]
        layer = net["edges"][0]["layer"]
        theirs = next((l for b_, l in ps if b_ == layer), layer)
        x, y = along(s, b, access, layer), along(s, a, access, theirs)
        xg, xw, xt, yg, yw, yt = x["got"], x["whole"], bool(x["targets"]), y["got"], y["whole"], bool(y["targets"])
        reach = {"baseReached": x["yes"], "baseUnreached": x["no"], "baseTargets": x["targets"],
                 "alternativeReached": y["yes"], "alternativeUnreached": y["no"], "alternativeTargets": y["targets"]}
        for k, v in (("baseRoutes", x["routes"]), ("alternativeRoutes", y["routes"]), ("baseOff", x["off"]),
                     ("alternativeOff", y["off"])):
            if v:
                reach[k] = v
        fr, to, d, cn, nn = (", ".join(access["from"]), ", ".join(access["to"]), rs(access["distance"]),
                             rs(access["connect"]), net["name"])
        if x["off"] or y["off"]:
            asked_gaps.append(f"Ağa bağlanamayan ({nn} ağına {cn} m'den uzak) {fr} alanı: mevcut durumda {len(x['off'])}, "
                              f"senaryoda {len(y['off'])}; erişmez sayıldı.")
        if x["offTargets"] + y["offTargets"]:
            asked_gaps.append(f"Ağa bağlanamayan ({nn} ağına {cn} m'den uzak) {to} alanı: mevcut durumda "
                              f"{x['offTargets']}, senaryoda {y['offTargets']}; ona erişilmez sayıldı.")
        measures.append(m("access", f"Erişen alan ({fr} → {to}, {d} m, {nn} ağı boyunca)", "m2", xg, yg,
                          f"Kullanımı {fr} olan alanların ağırlık merkezinden kullanımı {to} olan en yakın alanın ağırlık "
                          f"merkezine {nn} ağı boyunca uzaklık en çok {d} m ise alanı sayılır: merkezden ağın en yakın yerine "
                          "bağlanma, ağda tanımının izin verdiği en kısa yol ve ağdan hedefin merkezine bağlanma; merkezi ağa "
                          f"{cn} m'den uzak alan ağa bağlanamaz."))
    elif access is not None:
        (xg, xw, xt, xr, xu, xs), (yg, yw, yt, yr, yu, ys) = reached(b, access), reached(a, access)
        reach = {"baseReached": xr, "baseUnreached": xu, "baseTargets": xs,
                 "alternativeReached": yr, "alternativeUnreached": yu, "alternativeTargets": ys}
        fr, to, d = ", ".join(access["from"]), ", ".join(access["to"]), rs(access["distance"])
        measures.append(m("access", f"Erişen alan ({fr} → {to}, {d} m)", "m2", xg, yg,
                          f"Kullanımı {fr} olan alanların ağırlık merkezinden kullanımı {to} olan en yakın alana düz uzaklık "
                          f"en çok {d} m ise alanı sayılır; merkez alanın içindeyse uzaklık 0."))
    if access is not None:
        share = lambda g_, w_: g_ / w_ * 100 if w_ > 0 else 0.0  # noqa: E731
        measures.append(m("accessShare", "Erişen alanın payı", "%", share(xg, xw), share(yg, yw),
                          f"Erişen alanın {fr} alanlarının toplamına oranı; {fr} alanı olmayan tarafta 0."))
        for has_from, has_to, where in ((xw > 0, xt, "mevcut durumda"), (yw > 0, yt, "senaryoda")):
            if has_from and not has_to:
                asked_gaps.append(f"Erişilecek kullanım ({to}) {where} yok; erişen alan 0.")
    if infrastructure is not None:
        lid = infrastructure["layer"]
        theirs = next((l for b_, l in ps if b_ == lid), lid)
        (xg, xw, xn, xy, xf, xl, xk), (yg, yw, yn, yy, yf, yl, yk) = (near(s, b, lid, infrastructure),
                                                                      near(s, a, theirs, infrastructure))
        near_out = {"baseNear": xy, "baseFar": xf, "baseLines": xl,
                    "alternativeNear": yy, "alternativeFar": yf, "alternativeLines": yl}
        if xk:
            near_out["baseLinks"] = xk
        if yk:
            near_out["alternativeLinks"] = yk
        name = {"altyapi": "Kanalizasyon hattı", "yol": "Yollar", "karar": "Karar alanları"}[lid]
        d = rs(infrastructure["distance"])
        uses_ = [u for u in infrastructure.get("from", []) if u.strip()]
        if uses_:
            label, which = f"Altyapıya yakın alan ({', '.join(uses_)} → {name}, {d} m)", f"Kullanımı {', '.join(uses_)} olan alanların"
        else:
            label, which = f"Altyapıya yakın alan ({name}, {d} m)", "Alanların"
        measures.append(m("infrastructure", label, "m2", xg, yg,
                          f"{which} ağırlık merkezinden {name} katmanındaki en yakın nesneye düz uzaklık en çok {d} m ise "
                          "alanı sayılır; hattın kapasitesi ya da bağlantı hakkı değildir."))
        share = lambda g_, w_: g_ / w_ * 100 if w_ > 0 else 0.0  # noqa: E731
        measures.append(m("infrastructureShare", "Altyapıya yakın alanın payı", "%", share(xg, xw), share(yg, yw),
                          "Altyapıya yakın alanın ölçülen alanların toplamına oranı; ölçülen alanı olmayan tarafta 0."))
        for whole_, objects, where in ((xw, xn, "mevcut durumda"), (yw, yn, "senaryoda")):
            if whole_ > 0 and objects == 0:
                asked_gaps.append(f"Altyapı katmanında ({name}) {where} nesne yok; yakın alan 0.")
    if slope is not None:
        name = "Zemin"
        ground = {e["attrs"]["Ad"]: (e["p"]["x"] - X0, e["p"]["y"] - Y0, e["z"]) for e in s["entities"]
                  if e["layerId"] == "zemin"}
        triangles = ground_triangles({k: (round(x), round(y), z) for k, (x, y, z) in ground.items()})
        (xw, xo, xs_, xd, xc, xa, xp), (yw, yo, ys_, yd, yc, ya, yp) = (slopes(b, slope, triangles),
                                                                        slopes(a, slope, triangles))
        if slope.get("map"):
            slope_map = {"base": xp, "alternative": yp}
        uses_ = [u for u in slope.get("from", []) if u.strip()]
        whose = f"{', '.join(uses_)}, {name}" if uses_ else name
        th = rs(slope["threshold"])
        mean = lambda on, ws: ws / on if on > 0 else 0.0  # noqa: E731
        share = lambda on, st: st / on * 100 if on > 0 else 0.0  # noqa: E731
        measures.append(m("slopeMean", f"Ortalama eğim ({whose})", "%", mean(xo, xs_), mean(yo, ys_),
                          f"Ölçülen alanların {name} yüzeyinin üçgenlerine düşen parçalarının alan ağırlıklı ortalaması; "
                          "üçgenin eğimi düzleminden (normali, iki kenarının vektörel çarpımı), düşeyin yataya oranı, yüzde. "
                          "Yüzeyin dışındaki kısım sayılmaz."))
        measures.append(m("slopeSteep", f"Dik alan (en az {th} %, {whose})", "m2", xd, yd,
                          f"Eğimi en az {th} % olan üçgenlere düşen alan."))
        measures.append(m("slopeSteepShare", "Dik alanın payı", "%", share(xo, xd), share(yo, yd),
                          "Dik alanın yüzeyin üstündeki ölçülen alana oranı; yüzeyde alanı olmayan tarafta 0."))
        measures.append(m("slopeOff", "Yüzeyin dışındaki alan", "m2", max(xw - xo, 0.0), max(yw - yo, 0.0),
                          f"Ölçülen alanların {name} yüzeyinin üçgenlerine düşmeyen kısmı; eğimi bilinmez."))
        breaks = [rs(b_) for b_ in slope.get("classes", [])]
        for k in range(len(breaks) + 1 if breaks else 0):
            if k == 0:
                label_, note = f"Eğimi {breaks[0]} %'den az ({whose})", f"Eğimi {breaks[0]} %'den az olan üçgenlere düşen alan."
            elif k == len(breaks):
                label_, note = (f"Eğimi en az {breaks[k - 1]} % ({whose})",
                                f"Eğimi en az {breaks[k - 1]} % olan üçgenlere düşen alan.")
            else:
                label_, note = (f"Eğimi {breaks[k - 1]}–{breaks[k]} % ({whose})",
                                f"Eğimi en az {breaks[k - 1]} %, {breaks[k]} %'den az olan üçgenlere düşen alan.")
            measures.append(m(f"slopeClass{k + 1}", label_, "m2", xc[k], yc[k], note))
        if slope.get("aspect"):
            for k, (id_, word, bounds) in enumerate(ASPECTS):
                note = ("Eğimi 0 olan (yatay) üçgenlere düşen alan; bakısı yok." if k == 0 else
                        f"Bakısı (eğimin indiği yön, kuzeyden saat yönünde) {bounds} olan üçgenlere düşen alan; alt "
                        "sınır dahil.")
                measures.append(m(id_, f"Bakı: {word} ({whose})", "m2", xa[k], ya[k], note))
        for whole_, on, where in ((xw, xo, "mevcut durumda"), (yw, yo, "senaryoda")):
            if whole_ - on > 1e-6:
                asked_gaps.append(f"Ölçülen alanların bir kısmı {where} {name} yüzeyinin dışında; eğim yalnız yüzeyin "
                                  "üstündeki kısımdan.")
    uses = [{"code": c, "base": b["uses"].get(c, 0.0), "alternative": a["uses"].get(c, 0.0),
             "difference": a["uses"].get(c, 0.0) - b["uses"].get(c, 0.0)}
            for c in sorted(set(b["uses"]) | set(a["uses"]))]
    attr = lambda e, k: e["attrs"].get(k, "").strip() or None  # noqa: E731
    decided = lambda e: {k: v.strip() for k, v in e["attrs"].items() if v.strip()}  # noqa: E731

    def change(k, kind, eb=None, ab=None, ea=None, aa=None):
        c = {"key": k, "change": kind}
        for name, v in (("baseUse", attr(eb, field) if eb else None), ("alternativeUse", attr(ea, field) if ea else None),
                        ("baseArea", ab), ("alternativeArea", aa),
                        ("baseCapacity", capacity(eb, ab, field) if eb else None),
                        ("alternativeCapacity", capacity(ea, aa, field) if ea else None)):
            if v is not None:
                c[name] = v
        return c
    changes = []
    for k, eb, ab in b["keyed"]:
        other = [x for x in a["keyed"] if x[0] == k]
        if not other:
            changes.append(change(k, "removed", eb, ab))
            continue
        _, ea, aa = other[0]
        g, d = verts(eb) != verts(ea), decided(eb) != decided(ea)
        if g or d:
            changes.append(change(k, {(True, False): "geometry", (False, True): "decision", (True, True): "both"}[(g, d)],
                                  eb, ab, ea, aa))
    for k, ea, aa in a["keyed"]:
        if k not in [x[0] for x in b["keyed"]]:
            changes.append(change(k, "added", ea=ea, aa=aa))
    gaps = []
    if b["noKaks"] + a["noKaks"]:
        gaps.append(f"KAKS'ı olmayan yapılaşan alan: mevcut durumda {b['noKaks']}, senaryoda {a['noKaks']}; inşaat "
                    "alanı onlarsız hesaplandı.")
    if b["noUse"] + a["noUse"]:
        gaps.append(f"Kullanımı ({field}) olmayan alan: mevcut durumda {b['noUse']}, senaryoda {a['noUse']}.")
    if b["noKey"] + a["noKey"]:
        gaps.append(f"Anahtarı ({key}) olmayan alan: mevcut durumda {b['noKey']}, senaryoda {a['noKey']}; "
                    "değişiklikleri izlenemez.")
    gaps += asked_gaps
    now = stamp(s, bases)
    names = {"karar": "Karar alanları", "yol": "Yollar"}
    out = {"scenario": "alt-a", "name": "Alternatif A",
           "layers": [{"base": x, "layer": y, "name": names[x]} for x, y in ps], "field": field, "key": key,
           "measures": measures, "uses": uses, "changes": changes, "gaps": gaps,
           "assumptions": [f"Alanlar ve uzunluklar düzlemsel, projenin koordinatlarından (EPSG:{s['settings']['srid']}).",
                           "İnşaat alanı = alan × KAKS: brüt alan, terk ve DOP düşülmeden.",
                           "Park, yeşil alan, meydan ve yol inşaat alanı taşımaz.",
                           "Tek puan yok: ölçüler yan yana verilir; senaryoyu kullanıcı seçer."],
           "stamp": now}
    if since is not None:
        out["stale"] = since != now
    if reach is not None:
        out["reach"] = reach
    if near_out is not None:
        out["near"] = near_out
    if slope_map is not None:
        out["slopeMap"] = slope_map
    out["scenarioStamp"] = stamp(s, alts)
    recs = [{"table": uid(e["id"]), "stamp": e["attrs"]["TabanDamgasi"], "scenarioStamp": e["attrs"]["SenaryoDamgasi"],
             "baseChanged": e["attrs"]["TabanDamgasi"] != now,
             "scenarioChanged": e["attrs"]["SenaryoDamgasi"] != out["scenarioStamp"]}
            for e in s["entities"] if e["attrs"].get("SenaryoKimlik") == "alt-a" and "TabanDamgasi" in e["attrs"]
            and "SenaryoDamgasi" in e["attrs"]]
    if recs:
        out["records"] = recs
    out["revision"] = "$current"
    return out


SAME = {"revision": "same", "canUndo": False}


# docs/adr/4260: the terrain surface “Zemin”, four points on its own layer; the blocks east of x = 125 lie off it.
GROUND = {"A": (-10, -10, 100), "B": (125, -10, 103), "C": (125, 70, 121), "D": (-10, 80, 106)}


def in_circle(a, b, c, d):
    """Whether d lies inside the circle through a, b, c (counter-clockwise), exactly."""
    rows = [[Fraction(p[0] - d[0]), Fraction(p[1] - d[1])] for p in (a, b, c)]
    rows = [[x, y, x * x + y * y] for x, y in rows]
    det = (rows[0][0] * (rows[1][1] * rows[2][2] - rows[1][2] * rows[2][1])
           - rows[0][1] * (rows[1][0] * rows[2][2] - rows[1][2] * rows[2][0])
           + rows[0][2] * (rows[1][0] * rows[2][1] - rows[1][1] * rows[2][0]))
    return det > 0


def ground_triangles(ground=None):
    """The surface's two triangles (counter-clockwise, local x, y) with their slopes, percent, and their aspects (ADR
    4660: 0 flat, 1–8 north … north-west)."""
    ground = ground or GROUND
    a, b, c, d = (ground[k] for k in "ABCD")
    tris = [(a, b, c), (a, c, d)] if not in_circle(a, b, c, d) else [(a, b, d), (b, c, d)]
    out = []
    for p, q, r in tris:
        (x1, y1, z1), (x2, y2, z2), (x3, y3, z3) = p, q, r
        det = Fraction((x2 - x1) * (y3 - y1) - (x3 - x1) * (y2 - y1))
        ga = ((z2 - z1) * (y3 - y1) - (z3 - z1) * (y2 - y1)) / det
        gb = ((x2 - x1) * (z3 - z1) - (x3 - x1) * (z2 - z1)) / det
        if ga == 0 and gb == 0:
            aspect = 0
        else:
            direction = math.degrees(math.atan2(-ga, -gb)) % 360
            aspect = 1 + int(math.floor((direction + 22.5) / 45)) % 8
        out.append(([(Fraction(p[0]), Fraction(p[1])) for p in (p, q, r)], 100 * math.sqrt(ga * ga + gb * gb), aspect))
    return out


def clip(subject, triangle):
    """Sutherland–Hodgman: the convex subject polygon inside the counter-clockwise triangle, exactly."""
    out = list(subject)
    for k in range(3):
        a, b = triangle[k], triangle[(k + 1) % 3]
        side = lambda p: (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])  # noqa: E731
        given, out = out, []
        for i, p in enumerate(given):
            q = given[(i + 1) % len(given)]
            sp, sq = side(p), side(q)
            if sp >= 0:
                out.append(p)
            if (sp >= 0) != (sq >= 0):
                t_ = sp / (sp - sq)
                out.append((p[0] + (q[0] - p[0]) * t_, p[1] + (q[1] - p[1]) * t_))
        if not out:
            return []
    return out


def exact_area(poly):
    n = len(poly)
    return abs(sum(poly[i][0] * poly[(i + 1) % n][1] - poly[(i + 1) % n][0] * poly[i][1] for i in range(n))) / 2


def slopes(sd, slope, triangles):
    """A side's measured areas (rectangles) on the triangles: whole, on the surface, Σ area × slope, steep; ADR 4660's
    areas by class and by aspect."""
    every = all(not u.strip() for u in slope.get("from", []))
    whole = on = weighted = steep = 0.0
    breaks = slope.get("classes", [])
    classes, aspects = [0.0] * (len(breaks) + 1), [0.0] * 9
    pieces = []
    for e, usage, a_ in sd["areas"]:
        if not every and usage not in slope["from"]:
            continue
        whole += a_
        rect = [(Fraction(p["x"]) - Fraction(X0), Fraction(p["y"]) - Fraction(Y0)) for p in e["pts"]]
        mine = []
        for tri, s_, aspect in triangles:
            got = float(exact_area(clip(rect, tri))) if len(clip(rect, tri)) >= 3 else 0.0
            if got > 0:
                ring = canonical_ring(clip(rect, tri))
                mine.append({"area": uid(e["id"]), "class": sum(1 for b_ in breaks if s_ >= b_), "aspect": aspect,
                             "slope": s_, "size": got,
                             "ring": [[float(Fraction(X0) + x), float(Fraction(Y0) + y)] for x, y in ring]})
                on += got
                weighted += got * s_
                if s_ >= slope["threshold"]:
                    steep += got
                classes[sum(1 for b_ in breaks if s_ >= b_)] += got
                aspects[aspect] += got
        pieces += sorted(mine, key=lambda p: (p["ring"][0][1], p["ring"][0][0]))
    return whole, on, weighted, steep, classes, aspects, pieces


ASPECTS = [("aspectFlat", "Düz", ""), ("aspectN", "Kuzey", "337,5°–22,5°"), ("aspectNE", "Kuzeydoğu", "22,5°–67,5°"),
           ("aspectE", "Doğu", "67,5°–112,5°"), ("aspectSE", "Güneydoğu", "112,5°–157,5°"),
           ("aspectS", "Güney", "157,5°–202,5°"), ("aspectSW", "Güneybatı", "202,5°–247,5°"),
           ("aspectW", "Batı", "247,5°–292,5°"), ("aspectNW", "Kuzeybatı", "292,5°–337,5°")]


def ground_points(ground=None):
    return [{"kind": "point", "layerId": "zemin", "attrs": {"Ad": k}, "p": P(x, y), "z": float(z)}
            for k, (x, y, z) in (ground or GROUND).items()]


ZEMIN = [{"id": "t-1", "name": "Zemin", "points": {"layers": ["zemin"]}}]
SLOPE_SETUP = setup(layers=tree(ground=True), extra=ground_points(), surfaces=ZEMIN)
# ADR 4660: the same corners level, at 100 m: every triangle flat.
FLAT_SETUP = setup(layers=tree(ground=True), extra=ground_points({k: (x, y, 100) for k, (x, y, _) in GROUND.items()}),
                   surfaces=ZEMIN)


def ok(name, s=None, op="plan", **kw):
    s = s or setup()
    input_ = {"scenario": "alt-a", **{k: v for k, v in kw.items() if v is not None}}
    given = {k: v for k, v in kw.items() if v is not None}
    for k, first in (("field", "Kullanim"), ("key", "KararKimlik")):
        if k in given:
            given[k] = given[k].strip() or first
    if "impervious" in given:
        given["impervious_"] = given.pop("impervious")
    out = report(s, **given)
    # The slope's parts come from an overlay of floats near 4 420 000 m: within a micrometre, not 1e-9.
    case = {"name": name, "steps": [{"op": op, "input": input_, "result": {
        "status": "completed", "output": None if op == "validate" else out, "warnings": []}, "expect": SAME,
        "within": 1e-6 if "slope" in given else 1e-9}]}
    if s is not BASE_SETUP:
        case["setup"] = s
    return case


def refused(name, input_, code, message, s=None, path="scenario"):
    case = {"name": name, "steps": [{"op": "execute", "input": input_, "result": {"status": "failed", "error": {
        "code": code, "message": message, "path": path}}, "expect": SAME}]}
    if s is not None:
        case["setup"] = s
    return case


BASE_SETUP = setup()
# docs/adr/2040: the user's impervious shares by use and an access question (the cases' own values, not KentOS's).
SHARES = {"konut": 0.6, "ticaret-konut": 0.8, "park": 0.1, "egitim": 0.5, "sanayi": 0.9, "yesil": 0.05}
ACCESS = {"distance": 40.0, "from": ["konut"], "to": ["park", "yesil"]}


# docs/adr/2660: a sewer line along y = 60 north of the blocks, on its own layer (not the scenario's).
SEWER = line("altyapi", (0, 60), (130, 60), HatKimlik="K-1")
INFRA_SETUP = setup(layers=tree(infra=True), extra=[SEWER])
EMPTY_INFRA = setup(layers=tree(infra=True))


# docs/adr/5060: the project's road network “Yollar” on the roads' layer, which the scenario stands in for; the
# scenario's roads run on to x = 105 and north along it (no area's centroid as near one line as another).
NET_ALT = [e for e in ALT if e["kind"] != "line"] + [line("alt-a-yol", (0, -5), (105, -5), YolKimlik="Y1"),
                                                     line("alt-a-yol", (105, -5), (105, 60), YolKimlik="Y2")]
NET_SETUP = setup(alt=NET_ALT)
NET_SETUP["settings"]["networks"] = [{"id": "yollar", "name": "Yollar", "kind": "road", "edges": [{"layer": "yol"}],
                                      "connect": "ends", "tolerance": 0.01, "direction": {"kind": "both"}}]
NET_ACCESS = {"distance": 100.0, "from": ["konut"], "to": ["park", "yesil"], "network": "yollar", "connect": 40.0}


def table(attrs):
    """A one-cell table on layer 0, east of the blocks, with the attributes a written comparison keeps."""
    return {"kind": "table", "layerId": "0", "attrs": attrs, "p": P(140, 50), "rotation": 0.0, "height": 2.5,
            "rows": [5.0], "columns": [20.0], "cells": [["Ölçü"]]}


def records_setup():
    """Two tables written in the drawing: one with the stamps of now, one whose scenario has changed since."""
    first = report(BASE_SETUP)
    return setup(extra=[table({"SenaryoKimlik": "alt-a", "TabanDamgasi": first["stamp"],
                               "SenaryoDamgasi": first["scenarioStamp"]}),
                        table({"SenaryoKimlik": "alt-a", "TabanDamgasi": first["stamp"],
                               "SenaryoDamgasi": "0000000000000000"}),
                        table({"SenaryoKimlik": "alt-b", "TabanDamgasi": first["stamp"],
                               "SenaryoDamgasi": first["scenarioStamp"]})])


def moved_base():
    """The field state changed after the comparison: K1's corner moved."""
    base = copy.deepcopy(BASE)
    base[0] = area("karar", (0, 0, 41, 50), KararKimlik="K1", Kullanim="konut", KAKS="1.20")
    return setup(base=base)


def cases():
    first = report(BASE_SETUP)
    old = first["stamp"]
    assert [c["change"] for c in first["changes"]] == ["decision", "geometry", "removed", "added"], first["changes"]
    no_pairs = tree()
    for c in no_pairs[2]["children"]:
        c.pop("replaces")
    return [
        ok("plan: ölçüler, kullanımlar, değişen kararlar, eksikler, varsayımlar", BASE_SETUP),
        ok("uygula: salt okunur", BASE_SETUP, op="execute"),
        ok("doğrulama", BASE_SETUP, op="validate"),
        ok("aynı tabanla damga: güncel", BASE_SETUP, since=old),
        ok("taban değişince eski karşılaştırma eskir (stale)", moved_base(), since=old),
        ok("taban değişince K1 değişikliği de görünür", moved_base()),
        ok("damga başka karşılaştırmanın: eskimiş sayılır", BASE_SETUP, since="0000000000000000"),
        ok("başka kullanım alanı ve anahtar", BASE_SETUP, field="YolKimlik", key="Kullanim"),
        ok("boş alan adı ilk değere döner", BASE_SETUP, field="  ", key=" "),
        ok("aynı senaryo: değişiklik yok", setup(alt=[dict(copy.deepcopy(e), layerId={"karar": "alt-a-karar",
                                                                                          "yol": "alt-a-yol"}[e["layerId"]])
                                                     for e in BASE])),
        ok("boş senaryo: hepsi kaldırılmış", setup(alt=[])),
        ok("boş taban: hepsi eklenmiş", setup(base=[])),
        ok("virgüllü KAKS okunur", setup(alt=[dict(copy.deepcopy(e), attrs=dict(e["attrs"], KAKS="1,50"))
                                              if e["attrs"].get("KararKimlik") == "K2" else e for e in ALT])),
        ok("KAKS yazı değilse eksik sayılır", setup(alt=[dict(copy.deepcopy(e), attrs=dict(e["attrs"], KAKS="yok"))
                                                       if e["attrs"].get("KararKimlik") == "K2" else e for e in ALT])),
        ok("yalnız geometri değişimi", setup(alt=[area("alt-a-karar", (0, 0, 45, 50), KararKimlik="K1", Kullanim="konut",
                                                       KAKS="1.20")], base=[BASE[0]])),
        ok("geometri ve karar birlikte", setup(alt=[area("alt-a-karar", (0, 0, 45, 50), KararKimlik="K1",
                                                         Kullanim="ticaret", KAKS="2.00")], base=[BASE[0]])),
        ok("anahtar yinelenirse ilki eşlenir", setup(alt=ALT + [area("alt-a-karar", (200, 0, 210, 10),
                                                                     KararKimlik="K1", Kullanim="konut", KAKS="1")])),
        ok("park, yeşil, meydan, yol inşaat alanı taşımaz", setup(
            base=[area("karar", (0, 0, 10, 10), KararKimlik="A", Kullanim=u) for u in ("park", "yesil", "meydan", "yol")],
            alt=[])),
        # docs/adr/2040: the impervious area and the access, the user's values.
        ok("geçirimsizlik payları verilmiş: geçirimsiz alan", BASE_SETUP, impervious=SHARES),
        ok("payı verilmeyen kullanım eksik sayılır", BASE_SETUP,
           impervious={k: v for k, v in SHARES.items() if k not in ("sanayi", "yesil")}),
        ok("erişim: konuttan parka ve yeşil alana 40 m", BASE_SETUP, access=ACCESS),
        ok("erişim: 50 m'de mevcut durumdaki konut da erişir", BASE_SETUP, access=dict(ACCESS, distance=50.0)),
        ok("erişilecek kullanım hiçbir tarafta yok: iki eksik", BASE_SETUP,
           access={"distance": 40.0, "from": ["konut"], "to": ["meydan"]}),
        ok("geçirimsizlik ve erişim birlikte", BASE_SETUP, impervious=SHARES, access=ACCESS),
        # docs/adr/2660: the area near an infrastructure layer, the user's distance.
        ok("altyapıya yakınlık: kanalizasyon hattına 40 m", INFRA_SETUP, infrastructure={"layer": "altyapi", "distance": 40.0}),
        ok("altyapıya yakınlık yalnız konut alanlarında", INFRA_SETUP,
           infrastructure={"layer": "altyapi", "distance": 40.0, "from": ["konut"]}),
        ok("senaryonun yerine geçtiği katman altyapı: her taraf kendi yollarıyla", BASE_SETUP,
           infrastructure={"layer": "yol", "distance": 30.0}),
        ok("altyapı katmanı boş: yakın alan 0, iki tarafta da eksik", EMPTY_INFRA,
           infrastructure={"layer": "altyapi", "distance": 40.0}),
        refused("altyapı uzaklığı 0", {"scenario": "alt-a", "infrastructure": {"layer": "altyapi", "distance": 0.0}},
                "invalid_input", "Altyapı uzaklığı 0'dan büyük olmalı (yazılan 0).", s=INFRA_SETUP,
                path="infrastructure.distance"),
        refused("altyapı katmanı çizimde yok", {"scenario": "alt-a", "infrastructure": {"layer": "su", "distance": 40.0}},
                "invalid_input", "“su” kimlikli altyapı katmanı çizimde yok; bir katmanın kimliğini verin.", s=INFRA_SETUP,
                path="infrastructure.layer"),
        refused("altyapı katmanı yerine grup verilmiş", {"scenario": "alt-a", "infrastructure": {"layer": "imar",
                                                                                                "distance": 40.0}},
                "invalid_input", "“imar” kimlikli altyapı katmanı çizimde yok; bir katmanın kimliğini verin.",
                path="infrastructure.layer"),
        refused("geçirimsizlik payı 1'den büyük", {"scenario": "alt-a", "impervious": {"konut": 1.5}}, "invalid_input",
                "“konut” kullanımının geçirimsizlik payı 0 ile 1 arasında olmalı (yazılan 1.5).", path="impervious.konut"),
        refused("erişim uzaklığı 0", {"scenario": "alt-a", "access": {"distance": 0.0, "from": ["konut"], "to": ["park"]}},
                "invalid_input", "Erişim uzaklığı 0'dan büyük olmalı (yazılan 0).", path="access.distance"),
        refused("erişen kullanım yok", {"scenario": "alt-a", "access": {"distance": 40.0, "from": [], "to": ["park"]}},
                "invalid_input", "Erişen kullanım verilmemiş; en az bir kullanım kodu yazın.", path="access.from"),
        # docs/adr/4260: the slope on the terrain surface “Zemin”.
        ok("eğim: Zemin yüzeyinde, dik eşik 10 %; senaryonun doğudaki konutu yüzeyden taşar", SLOPE_SETUP,
           slope={"surface": "t-1", "threshold": 10.0}),
        ok("eğim yalnız konut alanlarında, yüzey adıyla", SLOPE_SETUP,
           slope={"surface": "zemin", "threshold": 10.0, "from": ["konut"]}),
        ok("eğim: eşik yüksek, dik alan yok", SLOPE_SETUP, slope={"surface": "Zemin", "threshold": 50.0}),
        ok("eğim ve altyapı birlikte", setup(layers=tree(infra=True, ground=True), extra=[SEWER] + ground_points(),
                                            surfaces=ZEMIN),
           infrastructure={"layer": "altyapi", "distance": 40.0}, slope={"surface": "t-1", "threshold": 12.5}),
        refused("dik eğim eşiği 0", {"scenario": "alt-a", "slope": {"surface": "t-1", "threshold": 0.0}},
                "invalid_input", "Dik eğim eşiği 0'dan büyük bir yüzde olmalı (yazılan 0).", s=SLOPE_SETUP,
                path="slope.threshold"),
        # docs/adr/4660: the areas by slope class and by aspect.
        ok("eğim sınıfları ve bakı: iki üçgen ayrı sınıflarda, bakıları güney ve güneybatı", SLOPE_SETUP,
           slope={"surface": "t-1", "threshold": 10.0, "classes": [5.0, 15.0, 30.0], "aspect": True}),
        ok("eğim sınıfları konutta, tek sınır: 20 %'den az ve en az", SLOPE_SETUP,
           slope={"surface": "t-1", "threshold": 10.0, "from": ["konut"], "classes": [20.0]}),
        ok("düz yüzey: bütün alan Düz, eğim 0 sınıfında", FLAT_SETUP,
           slope={"surface": "t-1", "threshold": 10.0, "classes": [2.5], "aspect": True}),
        refused("eğim sınıfı sınırları artmıyor", {"scenario": "alt-a", "slope": {"surface": "t-1", "threshold": 10.0,
                                                                                "classes": [15.0, 5.0]}},
                "invalid_input", "Eğim sınıflarının sınırları 0'dan büyük, artan yüzdeler olmalı, en çok 20 sınır "
                "(yazılan 15; 5).", s=SLOPE_SETUP, path="slope.classes"),
        refused("eğim sınıfı sınırı 0", {"scenario": "alt-a", "slope": {"surface": "t-1", "threshold": 10.0,
                                                                       "classes": [0.0, 5.0]}},
                "invalid_input", "Eğim sınıflarının sınırları 0'dan büyük, artan yüzdeler olmalı, en çok 20 sınır "
                "(yazılan 0; 5).", s=SLOPE_SETUP, path="slope.classes"),
        refused("eğim için yüzey projede yok", {"scenario": "alt-a", "slope": {"surface": "yok", "threshold": 10.0}},
                "unknown_terrain", "“yok” kimlikli ya da adlı yüzey projede yok.", s=SLOPE_SETUP, path="slope.surface"),
        {"name": "eğim: arazi yüzeyi Harita ve Yol projelerinde", "setup": setup(
            layers=tree(ground=True), extra=ground_points(), work="architecture"), "steps": [{
            "op": "execute", "input": {"scenario": "alt-a", "slope": {"surface": "t-1", "threshold": 10.0}},
            "result": {"status": "failed", "error": {
                "code": "not_a_road_project",
                "message": "Arazi yüzeyi Harita ve Yol projelerinde olur (KentOS Harita, KentOS Yol); bu çizim ikisinden "
                           "biri değil."}}, "expect": SAME}]},
        {"name": "eğim: tek noktalı yüzey kurulamaz", "setup": setup(
            layers=tree(ground=True), extra=ground_points()[:1], surfaces=ZEMIN), "steps": [{
            "op": "execute", "input": {"scenario": "alt-a", "slope": {"surface": "t-1", "threshold": 10.0}},
            "result": {"status": "failed", "error": {"code": "terrain_unbuildable", "message": "$any"}},
            "expect": SAME}]},
        # docs/adr/5060: the slope map, the access along the project's network, the tables written in the drawing.
        ok("eğim haritası: alanların üçgenlere düşen parçaları sınıf ve bakıyla", SLOPE_SETUP,
           slope={"surface": "t-1", "threshold": 10.0, "classes": [5.0, 15.0, 30.0], "aspect": True, "map": True}),
        ok("eğim haritası konutta, sınıfsız: hepsi ilk sınıfta", SLOPE_SETUP,
           slope={"surface": "t-1", "threshold": 10.0, "from": ["konut"], "map": True}),
        ok("ağ üzerinden erişim: konuttan parka ve yeşil alana Yollar ağı boyunca 100 m", NET_SETUP, access=NET_ACCESS),
        ok("ağ üzerinden erişim adıyla, 120 m: mevcut durumdaki konut da erişir", NET_SETUP,
           access=dict(NET_ACCESS, network="Yollar", distance=120.0)),
        ok("ağa 25 m'den uzak konut ağa bağlanamaz: erişmez, eksik", NET_SETUP, access=dict(NET_ACCESS, connect=25.0)),
        ok("ağa 18 m'den uzak hedef de bağlanamaz", NET_SETUP, access=dict(NET_ACCESS, connect=18.0)),
        refused("ağ üzerinden erişimde bağlanma uzaklığı yok", {"scenario": "alt-a", "access": {
            "distance": 100.0, "from": ["konut"], "to": ["park"], "network": "yollar"}}, "invalid_input",
                "Ağ üzerinden erişimde ağa bağlanma uzaklığı verilmemiş; alanın ağırlık merkezinin ağa en çok kaç metre uzak "
                "olabileceğini yazın.", s=NET_SETUP, path="access.connect"),
        refused("ağa bağlanma uzaklığı 0", {"scenario": "alt-a", "access": dict(NET_ACCESS, connect=0.0)}, "invalid_input",
                "Ağa bağlanma uzaklığı 0'dan büyük olmalı (yazılan 0).", s=NET_SETUP, path="access.connect"),
        refused("bağlanma uzaklığı ağsız", {"scenario": "alt-a", "access": dict(ACCESS, connect=20.0)}, "invalid_input",
                "Ağa bağlanma uzaklığı yalnız ağ üzerinden erişimde kullanılır; bir ağ seçin ya da uzaklığı boş bırakın.",
                s=NET_SETUP, path="access.network"),
        refused("ağ projede yok", {"scenario": "alt-a", "access": dict(NET_ACCESS, network="patika")}, "unknown_network",
                "“patika” kimlikli ya da adlı ağ projede yok; Ağlar'da tanımlı bir ağı seçin.", s=NET_SETUP,
                path="access.network"),
        ok("çizimdeki ölçü tabloları: biri güncel, birinin senaryosu değişmiş", records_setup()),
        refused("senaryo değil", {"scenario": "imar"}, "scenario_not_found",
                "“imar” kimlikli senaryo çizimde yok. Bir senaryo grubunun kimliğini verin."),
        refused("çizimde olmayan senaryo", {"scenario": "alt-z"}, "scenario_not_found",
                "“alt-z” kimlikli senaryo çizimde yok. Bir senaryo grubunun kimliğini verin."),
        refused("yerine geçen katmanı olmayan senaryo", {"scenario": "alt-a"}, "nothing_to_compare",
                "“Alternatif A” senaryosunun hiçbir katmanı bir ana katmanın yerine geçmiyor; karşılaştırılacak bir şey "
                "yok.", setup(layers=no_pairs)),
    ]


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.scenarios.impact", "commandVersion": 1,
           "title": "Senaryo ölçüleri",
           "note": "ADR 1440. Bağımsız başvuru scripts/fixtures/scenarios_impact_cases.py: ölçüler, eşleme ve damga "
                   "elle (rasyonel shoelace, SHA-256); KentOS kodu yok.",
           "setup": BASE_SETUP, "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok_ = OUT.exists() and OUT.read_text("utf-8") == text
        print("Senaryo ölçüleri ortak durumları güncel." if ok_ else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok_ else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
