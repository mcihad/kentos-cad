#!/usr/bin/env python3
"""Independent reference of Kenar eşleme (docs/adr/0159).

Writes fixtures/fit/v1/edgematch.json from the ADR's rules alone, with
mpmath at 50 digits and no KentOS code. Both platforms find every case's
links with the shared core (`ops::edgematch`, WASM `edgematchLinks`) and put
the lines together (`edgematchApply`); they must give the same links, the
same counts, the same geometry and the same elevations.

The rules (docs/adr/0159 §3–§7):

1. An end: a line's a or b, an open polyline's first or last vertex. Its
   outward direction u: along its edge towards the end; an arc edge's
   tangent there (the chord turned by half the sweep). A polyline whose ends
   meet (1 µm) is closed and takes no part, as no other kind does.
2. A junction: another object of the same set has a vertex within 1 µm of
   the end. A connected end: an end of the other set is within 1 µm.
   Neither is linked.
3. A candidate: a source end s and an adjacent end t with
   1 µm < |t − s| ≤ d; the angle θ between u and −v (v the adjacent end's
   outward direction) at most α; the same key when a criterion is given
   (trimmed, Turkish lower case); both within d of the border when there is
   one. Its score is θ/α + |t − s|/d.
4. Links: the candidates by score, then by the source end's place (object,
   its first end before its last), then the adjacent end's; taken in turn
   when both ends are still free.
5. Unmatched: free source ends (no junction, not connected, not linked)
   within d of the border, or without a border within d of an adjacent end.
   Junction ends are counted the same way.
6. Where they meet: the adjacent end; the middle (s + t)/2; or the point of
   the border nearest to the middle.
7. Move: the end goes there. Segment: a straight edge from the end to there
   (a line becomes a polyline; the new vertex takes the end's elevation);
   within 1 µm the end moves instead. Adjust: every vertex moves by
   Δ₁·(1 − sᵢ/L) + Δ₂·sᵢ/L, sᵢ its distance along the path from the first
   vertex, L the path's length, an arc edge by its arc. In the middle and on
   the border the adjacent lines are put right the same way.
8. A result with two vertices in a row within 1 µm: that object's links are
   not written, at either end, and the rest is worked out again without
   them, until none is left.

What is compared: the links (source, its end, adjacent, its end; the gap
within 1e-9 m, the angle within 1e-9°, the score within 1e-12), the
unmatched ends, the junctions and the others; every changed object's shape
(within 1e-9 m) and elevations; the links not written. Every decision is
written only when it is clear of its threshold (`margin`).
"""

import argparse
import json
import sys
from pathlib import Path

import mpmath as mp

mp.mp.dps = 50

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "fit" / "v1" / "edgematch.json"
SAME = mp.mpf("1e-6")
FIRST, LAST = "first", "last"


def M(v):
    return mp.mpf(v)


def P(x, y):
    return {"x": x, "y": y}


def xy(p):
    return (M(p["x"]), M(p["y"]))


def line(a, b):
    return {"kind": "line", "a": P(*a), "b": P(*b)}


def polyline(pts, bulges=None):
    out = {"kind": "polyline", "pts": [P(*p) for p in pts]}
    out["bulges"] = list(bulges) if bulges is not None else [0.0] * (len(pts) - 1)
    return out


# ── Paths and ends ──


def path_of(shape):
    """A line's or an open polyline's vertices and bulges (mpmath), or None."""
    if shape["kind"] == "line":
        a, b = xy(shape["a"]), xy(shape["b"])
        return ([a, b], [M(0)]) if dist(a, b) > SAME else None
    if shape["kind"] == "polyline":
        pts = [xy(p) for p in shape["pts"]]
        if len(pts) < 2 or dist(pts[0], pts[-1]) <= SAME:
            return None
        bulges = [M(b) for b in (shape.get("bulges") or [])] + [M(0)] * len(pts)
        return pts, bulges[: len(pts) - 1]
    return None


def dist(a, b):
    return mp.sqrt((b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2)


def unit(v):
    n = mp.sqrt(v[0] ** 2 + v[1] ** 2)
    return (v[0] / n, v[1] / n)


def turn(v, phi):
    return (v[0] * mp.cos(phi) - v[1] * mp.sin(phi), v[0] * mp.sin(phi) + v[1] * mp.cos(phi))


def outward(pts, bulges, end):
    """The end's outward direction: along its edge towards it, an arc edge's tangent."""
    n = len(pts)
    if end == FIRST:
        k = next((k for k in range(1, n) if dist(pts[k], pts[0]) > SAME), None)
        if k is None:
            return None
        if k == 1 and abs(bulges[0]) > M("1e-12"):
            chord = unit((pts[1][0] - pts[0][0], pts[1][1] - pts[0][1]))
            t = turn(chord, -2 * mp.atan(bulges[0]))
            return (-t[0], -t[1])
        return unit((pts[0][0] - pts[k][0], pts[0][1] - pts[k][1]))
    k = next((k for k in range(n - 2, -1, -1) if dist(pts[k], pts[-1]) > SAME), None)
    if k is None:
        return None
    if k == n - 2 and abs(bulges[n - 2]) > M("1e-12"):
        chord = unit((pts[-1][0] - pts[-2][0], pts[-1][1] - pts[-2][1]))
        return turn(chord, 2 * mp.atan(bulges[n - 2]))
    return unit((pts[-1][0] - pts[k][0], pts[-1][1] - pts[k][1]))


def ends_of(members):
    """Every end of the members that take part: (member, end, point, outward)."""
    out = []
    for i, m in enumerate(members):
        path = path_of(m["shape"])
        if path is None:
            continue
        pts, bulges = path
        for end in (FIRST, LAST):
            u = outward(pts, bulges, end)
            if u is not None:
                out.append((i, end, pts[0] if end == FIRST else pts[-1], u))
    return out


def others(members):
    return sum(1 for m in members if path_of(m["shape"]) is None)


def vertices(members, skip):
    """Every vertex of the other lines, polylines (open or closed) and areas (rings and holes) of a set."""
    for j, m in enumerate(members):
        if j == skip:
            continue
        shape = m["shape"]
        if shape["kind"] == "line":
            yield from (xy(shape["a"]), xy(shape["b"]))
        elif shape["kind"] in ("polyline", "polygon"):
            rings = [shape] + list(shape.get("parts") or [])
            for r in rings:
                yield from (xy(p) for p in r["pts"])
                for h in r.get("holes") or []:
                    yield from (xy(p) for p in h["pts"])


def key_of(m):
    """A key as the criterion compares it: trimmed, Turkish lower case."""
    k = m.get("key")
    if k is None:
        return None
    def fold(c):
        if c == "I":
            return "ı"
        if c == "İ":
            return "i"
        low = c.lower()
        return low if len(low) == 1 else c

    return "".join(fold(c) for c in k.strip())


# ── The border ──


def border_segments(border):
    if border is None:
        return []
    if border["kind"] == "line":
        return [(xy(border["a"]), xy(border["b"]))]
    pts = [xy(p) for p in border["pts"]]
    assert all(b == 0 for b in border.get("bulges") or []), "the reference's borders are straight"
    segs = list(zip(pts, pts[1:]))
    if border["kind"] == "polygon":
        segs.append((pts[-1], pts[0]))
    return segs


def closest(seg, p):
    a, b = seg
    d = (b[0] - a[0], b[1] - a[1])
    t = ((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / (d[0] ** 2 + d[1] ** 2)
    t = max(M(0), min(M(1), t))
    q = (a[0] + d[0] * t, a[1] + d[1] * t)
    return q, dist(p, q)


def to_border(segs, p):
    """The nearest point of the border to p and its distance; the first edge on a tie."""
    best = None
    for s in segs:
        q, dd = closest(s, p)
        if best is None or dd < best[1]:
            best = (q, dd)
    return best


# ── Links ──


def margin(value, line, what, room=M("1e-6")):
    assert abs(value - line) > room, f"{what}: {value} is at its threshold {line}"


def find(case):
    src, adj, s = case["sources"], case["adjacent"], case["settings"]
    d, alpha = M(s["distance"]), M(s["angle"]) * mp.pi / 180
    segs = border_segments(s.get("border"))
    keyed = s.get("keyed", False)
    src_ends, adj_ends = ends_of(src), ends_of(adj)

    def junction(members, i, p):
        return any(dist(v, p) <= SAME for v in vertices(members, i))

    def connected(p, others_ends):
        return any(dist(q, p) <= SAME for (_, _, q, _) in others_ends)

    free_src = [e for e in src_ends if not junction(src, e[0], e[2]) and not connected(e[2], adj_ends)]
    free_adj = [e for e in adj_ends if not junction(adj, e[0], e[2]) and not connected(e[2], src_ends)]

    def near(p):
        if segs:
            dd = to_border(segs, p)[1]
            margin(dd, d, "border distance")
            return dd <= d
        for (_, _, q, _) in adj_ends:
            g = dist(p, q)
            margin(g, d, "gap to an adjacent end")
            if g <= d:
                return True
        return False

    cands = []
    for (i, ie, sp, u) in free_src:
        for (j, je, tp, v) in free_adj:
            gap = dist(sp, tp)
            if gap <= SAME:
                continue
            margin(gap, d, "gap")
            if gap > d:
                continue
            w = (-v[0], -v[1])
            theta = mp.atan2(abs(u[0] * w[1] - u[1] * w[0]), u[0] * w[0] + u[1] * w[1])
            margin(theta, alpha, "angle", M("1e-9"))
            if theta > alpha:
                continue
            if keyed and (key_of(src[i]) is None or key_of(src[i]) != key_of(adj[j])):
                continue
            if segs:
                if not (near(sp) and near(tp)):
                    continue
            score = theta / alpha + gap / d
            cands.append((score, (i, 0 if ie == FIRST else 1), (j, 0 if je == FIRST else 1), ie, je, sp, tp, gap, theta))
    cands.sort(key=lambda c: (c[0], c[1], c[2]))
    for k, a in enumerate(cands):
        for b in cands[k + 1 :]:
            if a[1] == b[1] or a[2] == b[2]:
                margin(a[0], b[0], "two scores of one end", M("1e-9"))
    taken_s, taken_a, links = set(), set(), []
    for (score, ks, ka, ie, je, sp, tp, gap, theta) in cands:
        if ks in taken_s or ka in taken_a:
            continue
        taken_s.add(ks)
        taken_a.add(ka)
        links.append({"source": ks[0], "sourceEnd": ie, "adjacent": ka[0], "adjacentEnd": je, "from": sp, "to": tp, "gap": gap, "angle": theta * 180 / mp.pi, "score": score})
    unmatched = []
    for (i, ie, sp, _) in free_src:
        if (i, 0 if ie == FIRST else 1) in taken_s:
            continue
        if near(sp):
            unmatched.append({"source": i, "end": ie})
    junctions = sum(1 for (i, ie, sp, _) in src_ends if junction(src, i, sp) and near(sp))
    return links, unmatched, junctions, others(src)


# ── Putting them together ──


def arc_length(a, b, bulge):
    chord = dist(a, b)
    if abs(bulge) <= M("1e-12") or chord < M("1e-12"):
        return chord
    r = chord * (1 + bulge**2) / (4 * abs(bulge))
    return r * 4 * abs(mp.atan(bulge))


def meet_point(link, meet, segs):
    s, t = link["from"], link["to"]
    if meet == "adjacent":
        return t
    mid = ((s[0] + t[0]) / 2, (s[1] + t[1]) / 2)
    if meet == "middle":
        return mid
    return to_border(segs, mid)[0]


def put(member, shifts, method):
    """A member with its ends' targets ({first: m, last: m}): its new path and elevations, or None if it collapses."""
    shape = member["shape"]
    pts, bulges = path_of(shape)
    pts, bulges = list(pts), list(bulges)
    zs = [list(z) for z in member["zs"]]
    path_z = zs[0] if zs and len(zs[0]) == len(pts) else None
    if method == "adjust":
        lengths = [arc_length(pts[k], pts[k + 1], bulges[k]) for k in range(len(pts) - 1)]
        total = sum(lengths)
        acc, along = M(0), []
        for k in range(len(pts)):
            along.append(acc)
            if k < len(lengths):
                acc += lengths[k]
        d1 = (shifts[FIRST][0] - pts[0][0], shifts[FIRST][1] - pts[0][1]) if FIRST in shifts else (M(0), M(0))
        d2 = (shifts[LAST][0] - pts[-1][0], shifts[LAST][1] - pts[-1][1]) if LAST in shifts else (M(0), M(0))
        moved = []
        for k, p in enumerate(pts):
            f = along[k] / total
            moved.append((p[0] + d1[0] * (1 - f) + d2[0] * f, p[1] + d1[1] * (1 - f) + d2[1] * f))
        # The ends land exactly where they meet.
        if FIRST in shifts:
            moved[0] = shifts[FIRST]
        if LAST in shifts:
            moved[-1] = shifts[LAST]
        pts = moved
    else:
        for end in (FIRST, LAST):
            if end not in shifts:
                continue
            m = shifts[end]
            at = 0 if end == FIRST else len(pts) - 1
            if method == "segment" and dist(pts[at], m) > SAME:
                if end == FIRST:
                    pts.insert(0, m)
                    bulges.insert(0, M(0))
                    if path_z is not None:
                        path_z.insert(0, path_z[0])
                else:
                    pts.append(m)
                    bulges.append(M(0))
                    if path_z is not None:
                        path_z.append(path_z[-1])
            else:
                pts[at] = m
    if any(dist(a, b) <= SAME for a, b in zip(pts, pts[1:])):
        return None
    if shape["kind"] == "line" and len(pts) == 2:
        out = {"kind": "line", "a": {"x": pts[0][0], "y": pts[0][1]}, "b": {"x": pts[1][0], "y": pts[1][1]}}
    else:
        out = {"kind": "polyline", "pts": [{"x": p[0], "y": p[1]} for p in pts], "bulges": bulges}
    return out, ([path_z] if path_z is not None else zs)


def apply(case, links, use, meet, method):
    src, adj = case["sources"], case["adjacent"]
    segs = border_segments(case["settings"].get("border"))
    live = list(use)
    while True:
        targets = {}
        for li in live:
            link = links[li]
            m = meet_point(link, meet, segs)
            targets.setdefault(("s", link["source"]), {})[link["sourceEnd"]] = m
            if meet != "adjacent":
                targets.setdefault(("a", link["adjacent"]), {})[link["adjacentEnd"]] = m
        results, collapsed = {}, set()
        for key, shifts in targets.items():
            members = src if key[0] == "s" else adj
            r = put(members[key[1]], shifts, method)
            if r is None:
                collapsed.add(key)
            else:
                results[key] = r
        if not collapsed:
            break
        live = [li for li in live if ("s", links[li]["source"]) not in collapsed and ("a", links[li]["adjacent"]) not in collapsed]
    refused = [li for li in use if li not in live]
    return {
        "sources": [results.get(("s", i), (None, None))[0] for i in range(len(src))],
        "sourceZs": [results.get(("s", i), (None, None))[1] for i in range(len(src))],
        "adjacent": [results.get(("a", j), (None, None))[0] for j in range(len(adj))],
        "adjacentZs": [results.get(("a", j), (None, None))[1] for j in range(len(adj))],
        "refused": refused,
    }


# ── Cases ──


def member(shape, zs=None, key=None):
    if zs is None:
        n = 2 if shape["kind"] == "line" else len(shape.get("pts", []))
        zs = [[None] * n] if shape["kind"] in ("line", "polyline") else []
    out = {"shape": shape, "zs": zs}
    if key is not None:
        out["key"] = key
    return out


def settings(distance=0.5, angle=30.0, border=None, keyed=False):
    out = {"distance": distance, "angle": angle, "keyed": keyed}
    if border is not None:
        out["border"] = border
    return out


EDGE = 1000.0
BORDER = line((EDGE, 0.0), (EDGE, 500.0))


def cases():
    out = []

    # Five roads cross a sheet edge at x = 1000: each piece ends a few centimetres short, over or beside its continuation.
    west = [
        member(line((900.0, 100.0), (999.97, 100.02))),
        member(polyline([(880.0, 170.0), (950.0, 182.0), (999.95, 180.0)])),
        member(line((920.0, 250.0), (1000.04, 250.0))),
        member(polyline([(910.0, 300.0), (960.0, 320.0), (999.9, 331.5)])),
        member(line((930.0, 420.0), (999.98, 419.99))),
    ]
    east = [
        member(line((1000.01, 100.05), (1100.0, 101.0))),
        member(polyline([(1000.03, 180.04), (1050.0, 178.0), (1120.0, 190.0)])),
        member(line((999.96, 250.03), (1080.0, 250.0))),
        member(polyline([(1000.12, 331.62), (1060.0, 349.0)])),
        member(line((1000.0, 420.0), (1090.0, 425.0))),
    ]
    applies = [
        {"meet": "adjacent", "method": "move"},
        {"meet": "middle", "method": "move"},
        {"meet": "border", "method": "adjust"},
        {"meet": "adjacent", "method": "segment"},
        {"meet": "adjacent", "method": "move", "use": [0, 2, 4]},
    ]
    out.append(("Pafta kenarı: beş yol santimetrelerle kısa, taşan ya da yana kaymış", west, east, settings(border=BORDER), applies))

    # Two adjacent ends: the nearer one turns away (25°), the farther one goes straight on (2°): the straight one is the continuation.
    a = [member(line((900.0, 50.0), (999.9, 50.0)))]
    b = [
        member(line((999.95, 50.04), (1045.3154, 71.1706))),
        member(line((1000.2, 50.0105), (1060.1634, 52.1045))),
    ]
    out.append(("İki aday: yakın olan döner, uzak olan doğru gider; devamı uzak olan", a, b, settings(), [{"meet": "adjacent", "method": "move"}]))

    # A crossing street: within reach but at right angles; the end stays unmatched.
    a = [member(line((900.0, 60.0), (999.9, 60.0)))]
    b = [member(line((1000.1, 60.2), (1000.1, 160.0)))]
    out.append(("Dik açıyla gelen çizgi aday değil; uç eşsiz kalır", a, b, settings(border=BORDER), []))

    # A junction at the edge: two source lines end at one point, a third ends on a parcel's corner; none is linked, all three are counted.
    a = [
        member(line((900.0, 80.0), (999.95, 80.0))),
        member(line((999.95, 80.0), (980.0, 120.0))),
        member(line((960.0, 150.0), (999.93, 150.0))),
        member({"kind": "polygon", "pts": [P(999.93, 150.0), P(990.0, 150.0), P(990.0, 160.0)]}),
    ]
    b = [member(line((1000.05, 80.02), (1100.0, 80.0))), member(line((1000.06, 150.01), (1100.0, 150.0)))]
    out.append(("Kavşak uçları eşlenmez, sayılır: iki çizginin ve bir alan köşesinin ucu", a, b, settings(border=BORDER), []))

    # An arc edge reaching the edge: its outward direction is the arc's tangent; Köşeleri ayarla counts the arc's length.
    a = [member(polyline([(950.0, 200.0), (980.0, 200.0), (999.92, 205.0)], [0.0, 0.08]), [[101.0, 102.0, 103.0]])]
    b = [member(line((999.97, 205.02), (1036.7904, 220.6492)), [[103.5, 104.0]])]
    out.append(("Yaylı kenar: dışa yön yayın teğeti; Köşeleri ayarla yayın boyuyla", a, b, settings(), [{"meet": "adjacent", "method": "adjust"}, {"meet": "middle", "method": "adjust"}]))

    # A line and a polyline with elevations: Parça ekle adds a vertex at each end's elevation; Ucu taşı keeps them.
    a = [member(line((900.0, 30.0), (999.8, 30.1)), [[100.0, 100.5]]), member(polyline([(900.0, 10.0), (950.0, 12.0), (999.85, 11.0)]), [[99.0, 99.5, 100.25]])]
    b = [member(line((1000.15, 30.12), (1100.0, 31.0)), [[100.7, 101.0]]), member(line((1000.1, 11.05), (1100.0, 12.0)))]
    out.append(("Kotlar: Parça ekle yeni köşeye ucun kotunu verir, öbür yöntemler kotları korur", a, b, settings(), [{"meet": "adjacent", "method": "segment"}, {"meet": "adjacent", "method": "move"}, {"meet": "middle", "method": "adjust"}]))

    # A short source line between two neighbours: both its ends are linked; Köşeleri ayarla adds the two shifts.
    a = [member(polyline([(1000.05, 400.0), (1010.0, 400.5), (1019.9, 400.0)]))]
    b = [member(line((950.0, 400.1), (999.98, 400.02))), member(line((1020.07, 399.97), (1080.0, 400.0)))]
    out.append(("İki ucu da bağlı nesne: kaymalar toplanır", a, b, settings(), [{"meet": "adjacent", "method": "adjust"}, {"meet": "middle", "method": "move"}]))

    # Two keyed candidates: the better one is another kind of line (a fence, not a road); with the key the road wins.
    a = [member(line((900.0, 140.0), (999.95, 140.0)), key=" Yol ")]
    b = [member(line((1000.03, 140.01), (1100.0, 140.0)), key="ÇİT"), member(line((1000.2, 140.06), (1100.0, 140.5)), key="YOL")]
    out.append(("Eşleşme ölçütü: daha iyi aday başka türse aynı tür alınır", a, b, settings(keyed=True), [{"meet": "adjacent", "method": "move"}]))

    # A polyline whose end would land on its own last-but-one vertex: a zero edge; its link is not written, the other one is.
    a = [member(polyline([(990.0, 75.0), (999.8, 75.0), (999.95, 75.0)])), member(line((900.0, 90.0), (999.9, 90.0)))]
    b = [member(line((1000.1, 75.0), (999.8000002, 75.0))), member(line((1000.1, 90.05), (1100.0, 90.0)))]
    out.append(("Sıfır boylu kenar kalacaksa nesnenin bağı yazılmaz; öbürü yazılır", a, b, settings(), [{"meet": "adjacent", "method": "move"}]))

    # Kinds that take no part: an area, a closed polyline, a circle; counted.
    a = [
        member({"kind": "polygon", "pts": [P(990.0, 0.0), P(999.0, 0.0), P(999.0, 5.0)]}),
        member(polyline([(990.0, 20.0), (999.0, 20.0), (995.0, 25.0), (990.0, 20.0)])),
        member({"kind": "circle", "c": P(995.0, 40.0), "r": 2.0}),
        member(line((900.0, 45.0), (999.9, 45.0))),
    ]
    b = [member(line((1000.05, 45.02), (1100.0, 45.0)))]
    out.append(("Katılmayan türler sayılır", a, b, settings(border=BORDER), [{"meet": "border", "method": "move"}]))
    return out


def num(v):
    return float(v)


def shape_json(s):
    if s is None:
        return None
    if s["kind"] == "line":
        return {"kind": "line", "a": {"x": num(s["a"]["x"]), "y": num(s["a"]["y"])}, "b": {"x": num(s["b"]["x"]), "y": num(s["b"]["y"])}}
    return {"kind": "polyline", "pts": [{"x": num(p["x"]), "y": num(p["y"])} for p in s["pts"]], "bulges": [num(b) for b in s["bulges"]]}


def zs_json(zs):
    if zs is None:
        return None
    return [[None if z is None else num(z) for z in path] for path in zs]


def build():
    out = []
    for name, src, adj, s, applies in cases():
        case = {"sources": src, "adjacent": adj, "settings": s}
        links, unmatched, junctions, others_n = find(case)
        expected = {
            "links": [
                {"source": l["source"], "sourceEnd": l["sourceEnd"], "adjacent": l["adjacent"], "adjacentEnd": l["adjacentEnd"], "gap": num(l["gap"]), "angle": num(l["angle"]), "score": num(l["score"])}
                for l in links
            ],
            "unmatched": unmatched,
            "junctions": junctions,
            "others": others_n,
        }
        puts = []
        for a in applies:
            use = a.get("use", list(range(len(links))))
            r = apply(case, links, use, a["meet"], a["method"])
            puts.append(
                {
                    "meet": a["meet"],
                    "method": a["method"],
                    "use": use,
                    "expected": {
                        "sources": [shape_json(x) for x in r["sources"]],
                        "sourceZs": [zs_json(x) for x in r["sourceZs"]],
                        "adjacent": [shape_json(x) for x in r["adjacent"]],
                        "adjacentZs": [zs_json(x) for x in r["adjacentZs"]],
                        "refused": r["refused"],
                    },
                }
            )
        out.append({"name": name, "sources": src, "adjacent": adj, "settings": s, "expected": expected, "apply": puts})
    return {
        "format": "kentos.fit-edgematch",
        "version": 1,
        "description": "Kenar eşleme (docs/adr/0159): bağlar (aday, puan, eşleme, eşsiz ve kavşak uçları, katılmayan türler) ve üç buluşma yeriyle üç yöntemin geometrisi, kotlarıyla. scripts/fixtures/edgematch_cases.py mpmath ile 50 basamakta, kurallardan yazar.",
        "tolerance": {"metres": 1e-9, "degrees": 1e-9, "score": 1e-12},
        "cases": out,
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="compare with the written file instead of writing it")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT} güncel değil: python3 scripts/fixtures/edgematch_cases.py", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT}: güncel")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f"{OUT}: {len(json.loads(text)['cases'])} durum")


if __name__ == "__main__":
    main()
