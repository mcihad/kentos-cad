#!/usr/bin/env python3
"""Doğrusal referanslama (docs/adr/0224): the shared cases of the core's
routes, written from the ADR's rules without KentOS code, at 50 digits with
mpmath (exact fractions for the events' intervals).

A route is parts walked in order, each a path of vertices, bulges (an arc
edge: tan of a quarter of its sweep, counter-clockwise positive) and a
measure (M) a vertex, none where it has none. On an edge with both ends
measured, M runs linearly along it: by length on a straight edge, by angle
on an arc. The measure direction is the route's: from its first measured
vertex to its last (walked order); when the last is smaller, against the
drawing. An offset or a distance is positive to the right of the measure
direction (KentOS's way, Km yaz's; a tool may turn it).

- locate(route, p): the closest point of the route (the first in walked
  order of equally close ones), its M (none on an edge with an end without
  one), the signed distance (± the distance, + when p is right of the
  measure direction there), the length along the route walked to it (the
  parts' gaps not counted), its part and edge.
- point_at(route, m, offset): the first place in walked order whose M is m
  (on an edge of equal ends, its start), moved square to the route by the
  offset; a measure outside the route's is refused with its range.
- stretch(route, from, to, offset): the route's places with M between the
  two (each edge's piece; touching pieces one path), their measures,
  offset as the core offsets a path (mitred corners; an arc grows or
  shrinks about its centre); the pieces run from `from` to `to`: in walked
  order when that is the measure direction's (rising measures for from
  below to), else reversed; equal measures are refused.
- create(pieces, rules): pieces of one route id joined (Uzunluk, Tek alan:
  from the endpoint nearest the priority corner of the route's box, each
  next piece the first one, in input order, touching the chain's end;
  Uzunluk the length, Tek alan each piece's field value, scaled by the
  factor and from the start; the gaps between chains counted or not;
  İki alan: each piece's from and to along it, pieces in the order of
  their from, a piece starting where the last ended joined).
- calibrate(route, points, rules): the points within the radius, at their
  closest places; at least two; vertices put where they fall (not within
  10⁻⁶ m of one; none with `split` false, a line keeping its two vertices);
  each vertex's M linear in the length along the route
  between the points around it, before the first and after the last by the
  first's and the last's interval when asked, else as it was.
- overlay(a, b, union): line events of a route cut where any begins or
  ends; Kesişim the pieces both cover, Birleşim those either covers, each
  pair of events covering a piece one row, neighbouring pieces of one pair
  one row.
- summary(route): the first, last, least and most M and how many vertices
  have one.
- locate_line(route, pts, radius): when every vertex is within the radius,
  the measures at the first's and the last's closest places and the
  farthest vertex's distance.
- locate_area(route, rings): the route's pieces inside an area (its outer
  ring and holes, straight edges here): each edge cut where it crosses a
  ring, each stretch between two cuts inside when its middle is (a stretch
  of 10⁻⁶ m or less neither opens nor closes a piece), touching inside
  stretches of one part one piece, with the measures at its ends (none on
  an edge with an end without one) and the lengths walked to them.

    python3 scripts/fixtures/lrs_cases.py          # write
    python3 scripts/fixtures/lrs_cases.py --check  # compare
"""

import json
import sys
from fractions import Fraction
from pathlib import Path

import mpmath as mp

mp.mp.dps = 50
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "lrs" / "v1" / "cases.json"
SOURCE = "scripts/fixtures/lrs_cases.py (docs/adr/0224)"
ON = mp.mpf("1e-6")
TOLERANCE = 1e-7


# ── Geometry ─────────────────────────────────────────────────────────────


def P(x, y):
    return (mp.mpf(x), mp.mpf(y))


class Edge:
    """An edge from a to b with a bulge (0: straight) and its ends' measures."""

    def __init__(self, a, b, bulge, ma, mb):
        self.a, self.b = a, b
        self.bulge = mp.mpf(bulge)
        self.ma, self.mb = ma, mb
        if self.bulge != 0:
            dx, dy = b[0] - a[0], b[1] - a[1]
            c = mp.sqrt(dx * dx + dy * dy)
            self.sweep = 4 * mp.atan(self.bulge)
            mx, my = (a[0] + b[0]) / 2, (a[1] + b[1]) / 2
            k = (c / 2) / mp.tan(self.sweep / 2)
            # Left normal of a→b, unit.
            nx, ny = -dy / c, dx / c
            self.c = (mx + nx * k, my + ny * k)
            self.r = mp.sqrt((a[0] - self.c[0]) ** 2 + (a[1] - self.c[1]) ** 2)
            self.a0 = mp.atan2(a[1] - self.c[1], a[0] - self.c[0])

    def arc(self):
        return self.bulge != 0

    def length(self):
        if self.arc():
            return abs(self.sweep) * self.r
        return mp.sqrt((self.b[0] - self.a[0]) ** 2 + (self.b[1] - self.a[1]) ** 2)

    def at(self, t):
        """The point at fraction t and the unit direction of travel there."""
        if self.arc():
            ang = self.a0 + self.sweep * t
            # The ends are the vertices themselves.
            p = self.a if t == 0 else self.b if t == 1 else (self.c[0] + self.r * mp.cos(ang), self.c[1] + self.r * mp.sin(ang))
            s = 1 if self.sweep > 0 else -1
            return p, (-mp.sin(ang) * s, mp.cos(ang) * s)
        dx, dy = self.b[0] - self.a[0], self.b[1] - self.a[1]
        l = mp.sqrt(dx * dx + dy * dy)
        return (self.a[0] + dx * t, self.a[1] + dy * t), (dx / l, dy / l)

    def closest(self, p):
        """The fraction of the closest point of the edge to p."""
        if self.arc():
            phi = mp.atan2(p[1] - self.c[1], p[0] - self.c[0])
            two = 2 * mp.pi
            if self.sweep > 0:
                rel = (phi - self.a0) % two
            else:
                rel = (self.a0 - phi) % two
            if rel <= abs(self.sweep):
                return rel / abs(self.sweep)
            # Outside the sweep: the nearer end.
            da = dist(p, self.a)
            db = dist(p, self.b)
            return mp.mpf(0) if da <= db else mp.mpf(1)
        dx, dy = self.b[0] - self.a[0], self.b[1] - self.a[1]
        l2 = dx * dx + dy * dy
        t = ((p[0] - self.a[0]) * dx + (p[1] - self.a[1]) * dy) / l2
        return min(mp.mpf(1), max(mp.mpf(0), t))

    def m(self, t):
        if self.ma is None or self.mb is None:
            return None
        return self.ma + (self.mb - self.ma) * t

    def piece(self, t0, t1):
        """The sub-edge from t0 to t1: its ends and bulge."""
        p0, _ = self.at(t0)
        p1, _ = self.at(t1)
        bulge = mp.tan(self.sweep * (t1 - t0) / 4) if self.arc() else mp.mpf(0)
        return p0, p1, bulge


def dist(p, q):
    return mp.sqrt((p[0] - q[0]) ** 2 + (p[1] - q[1]) ** 2)


def mval(v):
    return None if v is None else mp.mpf(v)


def route_of(parts):
    """A route as its parts, each [(pts, bulges, ms)] read to numbers."""
    out = []
    for part in parts:
        pts = [P(x, y) for x, y in part["pts"]]
        bulges = [mp.mpf(b) for b in part.get("bulges", [0] * len(pts))]
        ms = [mval(m) for m in part["ms"]]
        out.append((pts, bulges, ms))
    return out


def edges(part):
    pts, bulges, ms = part
    return [Edge(pts[i], pts[i + 1], bulges[i] if i < len(bulges) else 0, ms[i], ms[i + 1]) for i in range(len(pts) - 1)]


def direction(route):
    """+1 when the route's measures grow along the drawing, -1 when they fall."""
    measured = [m for _, _, ms in route for m in ms if m is not None]
    if len(measured) >= 2 and measured[-1] < measured[0]:
        return -1
    return 1


def right_of(tangent):
    return (tangent[1], -tangent[0])


# ── The core's rules ─────────────────────────────────────────────────────


def locate(route, p):
    sign = direction(route)
    best = None
    along = mp.mpf(0)
    for k, part in enumerate(route):
        for i, e in enumerate(edges(part)):
            t = e.closest(p)
            foot, tangent = e.at(t)
            d = dist(p, foot)
            if best is None or d < best["d"]:
                best = {"d": d, "foot": foot, "tangent": tangent, "t": t, "m": e.m(t), "along": along + e.length() * t, "part": k, "edge": i}
            along += e.length()
    if best is None:
        return None
    v = (p[0] - best["foot"][0], p[1] - best["foot"][1])
    rn = right_of(best["tangent"])
    side = (v[0] * rn[0] + v[1] * rn[1]) * sign
    signed = best["d"] if side >= 0 else -best["d"]
    return {
        "m": best["m"],
        "distance": signed,
        "foot": best["foot"],
        "along": best["along"],
        "part": best["part"],
        "edge": best["edge"],
    }


def measure_range(route):
    ms = [m for _, _, ms in route for m in ms if m is not None]
    return (min(ms), max(ms)) if ms else None


def point_at(route, m, offset):
    m = mp.mpf(m)
    sign = direction(route)
    for part in route:
        for e in edges(part):
            if e.ma is None or e.mb is None:
                continue
            lo, hi = min(e.ma, e.mb), max(e.ma, e.mb)
            if lo <= m <= hi:
                t = mp.mpf(0) if e.ma == e.mb else (m - e.ma) / (e.mb - e.ma)
                foot, tangent = e.at(t)
                rn = right_of(tangent)
                d = mp.mpf(offset) * sign
                return {"p": (foot[0] + rn[0] * d, foot[1] + rn[1] * d)}
    rng = measure_range(route)
    if rng is None:
        return {"fault": "noMeasures"}
    return {"fault": "outside", "least": rng[0], "most": rng[1]}


def offset_path(pts, bulges, d):
    """The core's path offset (positive left of travel): straight edges moved
    square, meeting at their lines' crossing; an arc's radius grown or shrunk
    about its centre. The cases keep to corners that meet."""
    n = len(pts)
    moved = []
    for i in range(n - 1):
        e = Edge(pts[i], pts[i + 1], bulges[i], None, None)
        if e.arc():
            # The arc about its centre: left of travel is inward on a CCW arc.
            s = 1 if e.sweep > 0 else -1
            r = e.r - d * s
            a1 = e.a0 + e.sweep
            pa = (e.c[0] + r * mp.cos(e.a0), e.c[1] + r * mp.sin(e.a0))
            pb = (e.c[0] + r * mp.cos(a1), e.c[1] + r * mp.sin(a1))
            moved.append(("arc", pa, pb, bulges[i]))
        else:
            _, t = e.at(0)
            ln = (-t[1], t[0])
            pa = (pts[i][0] + ln[0] * d, pts[i][1] + ln[1] * d)
            pb = (pts[i + 1][0] + ln[0] * d, pts[i + 1][1] + ln[1] * d)
            moved.append(("seg", pa, pb, mp.mpf(0)))
    out = [moved[0][1]]
    for i in range(len(moved) - 1):
        k0, a0, b0, _ = moved[i]
        k1, a1, b1, _ = moved[i + 1]
        if k0 == "seg" and k1 == "seg":
            out.append(cross_lines(a0, b0, a1, b1))
        else:
            # An arc meets its neighbour at the moved shared vertex (the cases' arcs are tangent to it).
            out.append(b0)
    out.append(moved[-1][2])
    return out, [b for _, _, _, b in moved]


def cross_lines(a, b, c, d):
    r = (b[0] - a[0], b[1] - a[1])
    s = (d[0] - c[0], d[1] - c[1])
    den = r[0] * s[1] - r[1] * s[0]
    t = ((c[0] - a[0]) * s[1] - (c[1] - a[1]) * s[0]) / den
    return (a[0] + r[0] * t, a[1] + r[1] * t)


def stretch(route, frm, to, offset):
    frm, to = mp.mpf(frm), mp.mpf(to)
    if frm == to:
        return {"fault": "same"}
    lo, hi = min(frm, to), max(frm, to)
    sign = direction(route)
    parts = []
    for part in route:
        open_path = None
        last_end = None
        for i, e in enumerate(edges(part)):
            if e.ma is None or e.mb is None:
                open_path = None
                continue
            if e.ma == e.mb:
                if not lo <= e.ma <= hi:
                    open_path = None
                    continue
                t0, t1 = mp.mpf(0), mp.mpf(1)
            else:
                ta = (lo - e.ma) / (e.mb - e.ma)
                tb = (hi - e.ma) / (e.mb - e.ma)
                t0, t1 = max(mp.mpf(0), min(ta, tb)), min(mp.mpf(1), max(ta, tb))
                if t1 - t0 <= 0:
                    open_path = None
                    continue
            p0, p1, bulge = e.piece(t0, t1)
            m0, m1 = e.m(t0), e.m(t1)
            if open_path is not None and last_end == (i - 1) and t0 == 0:
                open_path["pts"].append(p1)
                open_path["bulges"].append(bulge)
                open_path["ms"].append(m1)
            else:
                open_path = {"pts": [p0, p1], "bulges": [bulge], "ms": [m0, m1]}
                parts.append(open_path)
            last_end = i if t1 == 1 else None
            if t1 != 1:
                open_path = None
    if not parts:
        rng = measure_range(route)
        if rng is None:
            return {"fault": "noMeasures"}
        return {"fault": "outside", "least": rng[0], "most": rng[1]}
    if offset != 0:
        for q in parts:
            q["pts"], q["bulges"] = offset_path(q["pts"], q["bulges"], -mp.mpf(offset) * sign)
    if (frm > to) != (sign < 0):
        parts = [
            {
                "pts": list(reversed(q["pts"])),
                "bulges": [-b for b in reversed(q["bulges"])],
                "ms": list(reversed(q["ms"])),
            }
            for q in reversed(parts)
        ]
    return {"parts": parts}


# ── Create, calibrate, overlay, summary, lines ──────────────────────────


CORNERS = {"lowerLeft": (0, 0), "upperLeft": (0, 1), "lowerRight": (1, 0), "upperRight": (1, 1)}


def piece_length(pts, bulges):
    return sum((Edge(pts[i], pts[i + 1], bulges[i], None, None).length() for i in range(len(pts) - 1)), mp.mpf(0))


def walk_lengths(pts, bulges):
    """The length walked to each vertex."""
    out = [mp.mpf(0)]
    for i in range(len(pts) - 1):
        out.append(out[-1] + Edge(pts[i], pts[i + 1], bulges[i], None, None).length())
    return out


def reversed_piece(pts, bulges):
    return list(reversed(pts)), [-b for b in reversed(bulges)]


def create(pieces, rules):
    """Each route id's route, in the order the ids first come, and the faults of the pieces left out."""
    method = rules["method"]
    factor, start = mp.mpf(rules.get("factor", 1)), mp.mpf(rules.get("start", 0))
    ignore = rules.get("ignoreGaps", True)
    order, by_id, faults = [], {}, []
    for k, piece in enumerate(pieces):
        rid = piece.get("id", "")
        if rid.strip() == "":
            faults.append({"piece": k, "why": "noId"})
            continue
        if method == "oneField" and piece.get("value") is None:
            faults.append({"piece": k, "why": "noValue"})
            continue
        if method == "twoFields" and (piece.get("from") is None or piece.get("to") is None):
            faults.append({"piece": k, "why": "noFromTo"})
            continue
        if rid not in by_id:
            by_id[rid] = []
            order.append(rid)
        pts = [P(x, y) for x, y in piece["pts"]]
        bulges = [mp.mpf(b) for b in piece.get("bulges", [0] * (len(pts) - 1))]
        by_id[rid].append({"k": k, "pts": pts, "bulges": bulges, "value": piece.get("value"), "from": piece.get("from"), "to": piece.get("to")})
    routes = []
    for rid in order:
        group = by_id[rid]
        if method == "twoFields":
            parts = two_fields(group)
        else:
            parts = chained(group, rules.get("priority", "upperLeft"), method, factor, start, ignore)
        routes.append({"id": rid, "parts": parts})
    return {"routes": routes, "faults": faults}


def chained(group, priority, method, factor, start, ignore):
    xs = [p[0] for g in group for p in g["pts"]]
    ys = [p[1] for g in group for p in g["pts"]]
    cx, cy = CORNERS[priority]
    corner = (max(xs) if cx else min(xs), max(ys) if cy else min(ys))
    used = [False] * len(group)
    chains = []
    while not all(used):
        # The unused endpoint nearest the corner (input order, start before end, on a tie).
        best = None
        for i, g in enumerate(group):
            if used[i]:
                continue
            for end in (0, 1):
                q = g["pts"][0] if end == 0 else g["pts"][-1]
                d = dist(q, corner)
                if best is None or d < best[0]:
                    best = (d, i, end)
        _, i, end = best
        used[i] = True
        pts, bulges = group[i]["pts"], group[i]["bulges"]
        if end == 1:
            pts, bulges = reversed_piece(pts, bulges)
        chain = [(pts, bulges, group[i])]
        while True:
            tail = chain[-1][0][-1]
            nxt = None
            for j, g in enumerate(group):
                if used[j]:
                    continue
                if dist(g["pts"][0], tail) <= ON:
                    nxt = (j, False)
                    break
                if dist(g["pts"][-1], tail) <= ON:
                    nxt = (j, True)
                    break
            if nxt is None:
                break
            j, rev = nxt
            used[j] = True
            p2, b2 = group[j]["pts"], group[j]["bulges"]
            if rev:
                p2, b2 = reversed_piece(p2, b2)
            chain.append((p2, b2, group[j]))
        chains.append(chain)
    parts = []
    cum = mp.mpf(0)
    last_end = None
    for chain in chains:
        if last_end is not None and not ignore:
            cum += dist(last_end, chain[0][0][0])
        pts, bulges, ms = [], [], []
        for n, (p, b, g) in enumerate(chain):
            walked = walk_lengths(p, b)
            total = walked[-1]
            scale = (mp.mpf(g["value"]) / total) if method == "oneField" else mp.mpf(1)
            for v in range(len(p)):
                if n > 0 and v == 0:
                    continue
                pts.append(p[v])
                ms.append(start + factor * (cum + walked[v] * scale))
            bulges.extend(b)
            cum += total * scale
        last_end = pts[-1]
        parts.append({"pts": pts, "bulges": bulges, "ms": ms})
    return parts


def two_fields(group):
    ordered = sorted(group, key=lambda g: (mp.mpf(g["from"]), g["k"]))
    parts = []
    for g in ordered:
        walked = walk_lengths(g["pts"], g["bulges"])
        total = walked[-1]
        f, t = mp.mpf(g["from"]), mp.mpf(g["to"])
        ms = [f + (t - f) * w / total for w in walked]
        if parts and dist(parts[-1]["pts"][-1], g["pts"][0]) <= ON:
            parts[-1]["pts"].extend(g["pts"][1:])
            parts[-1]["bulges"].extend(g["bulges"])
            parts[-1]["ms"].extend(ms[1:])
        else:
            parts.append({"pts": list(g["pts"]), "bulges": list(g["bulges"]), "ms": ms})
    return parts


def split_at(route, places):
    """The route with a vertex put at each place (part, edge, t) not within ON of a vertex."""
    out = []
    for k, (pts, bulges, ms) in enumerate(route):
        npts, nb, nms = [pts[0]], [], [ms[0]]
        for i in range(len(pts) - 1):
            e = Edge(pts[i], pts[i + 1], bulges[i], ms[i], ms[i + 1])
            cuts = sorted({t for (pk, pe, t) in places if pk == k and pe == i})
            ts = [mp.mpf(0)]
            for t in cuts:
                p, _ = e.at(t)
                if dist(p, e.at(ts[-1])[0]) > ON and dist(p, e.b) > ON:
                    ts.append(t)
            ts.append(mp.mpf(1))
            for a, b in zip(ts, ts[1:]):
                p1, _ = e.at(b) if b != 1 else (pts[i + 1], None)
                npts.append(p1)
                nb.append(mp.tan(e.sweep * (b - a) / 4) if e.arc() else mp.mpf(0))
                nms.append(e.m(b) if b != 1 else ms[i + 1])
        out.append((npts, nb, nms))
    return out


def calibrate(route, points, rules):
    radius = mp.mpf(rules["radius"])
    ignore = rules.get("ignoreGaps", True)
    found = []
    used = []
    for k, (p, m) in enumerate(points):
        q = locate_walk(route, P(*p), ignore)
        if q is None or q["d"] > radius:
            used.append(False)
            continue
        used.append(True)
        found.append((q["s"], mp.mpf(m), q["part"], q["edge"], q["t"], k))
    found.sort(key=lambda f: (f[0], f[5]))
    # One measure a place: a later point at the same place is left out.
    kept = []
    for f in found:
        if kept and kept[-1][0] == f[0]:
            used[f[5]] = False
            continue
        kept.append(f)
    if len(kept) < 2:
        return {"fault": "fewPoints", "used": used}
    # Without split only the route's own vertices take measures (a line keeps its two).
    split = split_at(route, [(f[2], f[3], f[4]) for f in kept] if rules.get("split", True) else [])
    lengths = walk_route(split, ignore)
    first, last = kept[0], kept[-1]
    out = []
    for (pts, bulges, ms), ss in zip(split, lengths):
        nms = []
        for m_old, s in zip(ms, ss):
            if s < first[0] - ON:
                nms.append(slope_value(kept[0], kept[1], s) if rules.get("before", True) else m_old)
            elif s > last[0] + ON:
                nms.append(slope_value(kept[-2], kept[-1], s) if rules.get("after", True) else m_old)
            else:
                k = 0
                while k + 2 < len(kept) and s > kept[k + 1][0]:
                    k += 1
                nms.append(slope_value(kept[k], kept[k + 1], s))
        out.append({"pts": pts, "bulges": bulges, "ms": nms})
    return {"parts": out, "used": used}


def slope_value(a, b, s):
    return a[1] + (b[1] - a[1]) * (s - a[0]) / (b[0] - a[0])


def walk_route(route, ignore):
    """The length walked to each vertex of each part (gaps counted unless ignored)."""
    out = []
    cum = mp.mpf(0)
    last = None
    for pts, bulges, _ in route:
        if last is not None and not ignore:
            cum += dist(last, pts[0])
        w = walk_lengths(pts, bulges)
        out.append([cum + x for x in w])
        cum += w[-1]
        last = pts[-1]
    return out


def locate_walk(route, p, ignore):
    """The closest place with the length walked to it, gaps as `walk_route` counts them."""
    best = None
    cum = mp.mpf(0)
    last = None
    for k, part in enumerate(route):
        if last is not None and not ignore:
            cum += dist(last, part[0][0])
        for i, e in enumerate(edges(part)):
            t = e.closest(p)
            foot, _ = e.at(t)
            d = dist(p, foot)
            if best is None or d < best["d"]:
                best = {"d": d, "s": cum + e.length() * t, "part": k, "edge": i, "t": t}
            cum += e.length()
        last = part[0][-1]
    return best


def overlay(a, b, union):
    rows = []
    routes = []
    for ev in a + b:
        if ev["route"] not in routes:
            routes.append(ev["route"])
    for r in routes:
        ea = [(i, Fraction(min(e["from"], e["to"])), Fraction(max(e["from"], e["to"]))) for i, e in enumerate(a) if e["route"] == r]
        eb = [(i, Fraction(min(e["from"], e["to"])), Fraction(max(e["from"], e["to"]))) for i, e in enumerate(b) if e["route"] == r]
        cuts = sorted({x for _, lo, hi in ea + eb for x in (lo, hi)})
        pieces = []
        for lo, hi in zip(cuts, cuts[1:]):
            ca = [i for i, x, y in ea if x <= lo and hi <= y] or ([None] if union else [])
            cb = [i for i, x, y in eb if x <= lo and hi <= y] or ([None] if union else [])
            for i in ca:
                for j in cb:
                    if i is None and j is None:
                        continue
                    pieces.append((lo, hi, i, j))
        # Neighbouring pieces of one pair are one row.
        for lo, hi, i, j in pieces:
            same = [q for q in rows if q["route"] == r and q["a"] == i and q["b"] == j and q["to"] == lo]
            if same:
                same[-1]["to"] = hi
            else:
                rows.append({"route": r, "from": lo, "to": hi, "a": i, "b": j})
    rows.sort(key=lambda q: (routes.index(q["route"]), q["from"], -1 if q["a"] is None else q["a"], -1 if q["b"] is None else q["b"]))
    return rows


def summary(route):
    ms = [m for _, _, ms in route for m in ms if m is not None]
    if not ms:
        return {"count": 0}
    return {"first": ms[0], "last": ms[-1], "least": min(ms), "most": max(ms), "count": len(ms)}


def locate_line(route, pts, radius):
    places = [locate(route, P(*p)) for p in pts]
    if any(q is None or abs(q["distance"]) > mp.mpf(radius) for q in places):
        return {"none": True}
    if places[0]["m"] is None or places[-1]["m"] is None:
        return {"none": True}
    return {"from": places[0]["m"], "to": places[-1]["m"], "distance": max(abs(q["distance"]) for q in places)}


def line_cuts(e, a, b):
    """The fractions (0 < t < 1) along edge e where it crosses the straight edge a→b."""
    out = []
    dx, dy = b[0] - a[0], b[1] - a[1]
    if e.arc():
        # (a + s·d − c)² = r², s in [0, 1]; the point's angle within the arc's sweep.
        fx, fy = a[0] - e.c[0], a[1] - e.c[1]
        qa = dx * dx + dy * dy
        qb = 2 * (fx * dx + fy * dy)
        qc = fx * fx + fy * fy - e.r * e.r
        disc = qb * qb - 4 * qa * qc
        if disc < 0:
            return out
        for sgn in (-1, 1):
            s_ = (-qb + sgn * mp.sqrt(disc)) / (2 * qa)
            if s_ < 0 or s_ > 1:
                continue
            x, y = a[0] + dx * s_, a[1] + dy * s_
            ang = mp.atan2(y - e.c[1], x - e.c[0])
            # The fraction of the sweep from the start angle, turning the arc's way.
            turn = (ang - e.a0) if e.sweep > 0 else (e.a0 - ang)
            turn = turn % (2 * mp.pi)
            t = turn / abs(e.sweep)
            if 0 < t < 1:
                out.append(t)
        return out
    ex, ey = e.b[0] - e.a[0], e.b[1] - e.a[1]
    den = ex * dy - ey * dx
    if den == 0:
        return out
    wx, wy = a[0] - e.a[0], a[1] - e.a[1]
    t = (wx * dy - wy * dx) / den
    u = (wx * ey - wy * ex) / den
    if 0 < t < 1 and 0 <= u <= 1:
        out.append(t)
    return out


def inside_rings(rings, p):
    """Even-odd over the area's straight rings: inside its outer ring and outside its holes."""
    count = 0
    for ring in rings:
        n = len(ring)
        for i in range(n):
            a, b = ring[i], ring[(i + 1) % n]
            if (a[1] > p[1]) != (b[1] > p[1]):
                x = a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1])
                if p[0] < x:
                    count += 1
    return count % 2 == 1


def locate_area(route, rings):
    rings = [[P(x, y) for x, y in ring] for ring in rings]
    pieces = []
    along = mp.mpf(0)
    for k, part in enumerate(route):
        open_ = False
        for e in edges(part):
            ts = {mp.mpf(0), mp.mpf(1)}
            for ring in rings:
                for i in range(len(ring)):
                    ts.update(line_cuts(e, ring[i], ring[(i + 1) % len(ring)]))
            ts = sorted(ts)
            length = e.length()
            for t0, t1 in zip(ts, ts[1:]):
                if (t1 - t0) * length <= ON:
                    continue
                mid, _ = e.at((t0 + t1) / 2)
                if inside_rings(rings, mid):
                    if open_:
                        pieces[-1]["to"] = e.m(t1)
                        pieces[-1]["end"] = along + length * t1
                    else:
                        pieces.append({"from": e.m(t0), "to": e.m(t1), "start": along + length * t0, "end": along + length * t1})
                        open_ = True
                else:
                    open_ = False
            along += length
    return pieces


# ── The cases ────────────────────────────────────────────────────────────

BULGE_HALF = 1  # a half turn, counter-clockwise
BULGE_QUARTER = float(mp.tan(mp.pi / 8))

STRAIGHT = [{"pts": [[0, 0], [100, 0], [100, 50]], "ms": [0, 100, 150]}]
FALLING = [{"pts": [[0, 0], [100, 0], [100, 50]], "ms": [150, 50, 0]}]
ARC = [{"pts": [[0, 0], [100, 0]], "bulges": [BULGE_HALF], "ms": [0, 100]}]
GAP_M = [{"pts": [[0, 0], [100, 0], [200, 0]], "ms": [0, None, 200]}]
TWO_PARTS = [{"pts": [[0, 0], [100, 0]], "ms": [0, 100]}, {"pts": [[0, 50], [100, 50]], "ms": [100, 200]}]
FLAT = [{"pts": [[0, 0], [100, 0], [100, 50], [200, 50]], "ms": [0, 100, 100, 200]}]
BACK = [{"pts": [[0, 0], [100, 0], [100, 50]], "ms": [0, 100, 50]}]
MAP = [{"pts": [[487120.25, 4420310.5], [487220.25, 4420310.5], [487220.25, 4420410.5]], "ms": [1000, 1100, 1200]}]
UNMEASURED = [{"pts": [[0, 0], [100, 0], [100, 100]], "ms": [None, None, None]}]


def num(v):
    if v is None:
        return None
    return float(v)


def pt(p):
    return {"x": float(p[0]), "y": float(p[1])}


def part_json(q):
    out = {"pts": [pt(p) for p in q["pts"]]}
    if any(b != 0 for b in q["bulges"]):
        out["bulges"] = [float(b) for b in q["bulges"]]
    out["ms"] = [num(m) for m in q["ms"]]
    return out


def route_json(parts):
    out = []
    for q in parts:
        o = {"pts": [{"x": x, "y": y} for x, y in q["pts"]]}
        if "bulges" in q:
            o["bulges"] = q["bulges"]
        o["ms"] = q["ms"]
        out.append(o)
    return {"parts": out}


def fault_json(r):
    out = {"fault": r["fault"]}
    for k in ("least", "most"):
        if k in r:
            out[k] = float(r[k])
    return out


def build():
    locates = []
    for title, parts, p in [
        ("İlk kenarın solu (eksi)", STRAIGHT, (40, 10)),
        ("İlk kenarın sağı", STRAIGHT, (40, -5)),
        ("İkinci kenarın doğusu: kuzeye giden rotanın sağı", STRAIGHT, (120, 20)),
        ("Azalan ölçüde yan ölçünün yönüne göre", FALLING, (40, 10)),
        ("Yay: açıyla doğrusal ölçü", ARC, (50, -60)),
        ("Yayın dışında, yakın uç", ARC, (-20, 5)),
        ("Bir ucu ölçüsüz kenar: ölçü yok", GAP_M, (150, 5)),
        ("İki parça: ikincisi daha yakın", TWO_PARTS, (50, 40)),
        ("Harita koordinatları", MAP, (487170.75, 4420305.25)),
    ]:
        r = locate(route_of(parts), P(*p))
        locates.append({
            "title": title,
            "route": route_json(parts),
            "p": pt(P(*p)),
            "expect": {"m": num(r["m"]), "distance": float(r["distance"]), "foot": pt(r["foot"]), "along": float(r["along"]), "part": r["part"], "edge": r["edge"]},
        })
    points = []
    for title, parts, m, off in [
        ("İlk kenarda", STRAIGHT, 40, 0),
        ("İkinci kenarda, sağa 5", STRAIGHT, 125, 5),
        ("Rotanın dışında", STRAIGHT, 200, 0),
        ("Azalan ölçü: sağ ölçünün yönüne göre", FALLING, 110, 10),
        ("Yayın ortası", ARC, 50, 0),
        ("Yayın ortasından sağa 10", ARC, 50, 10),
        ("Değişmeyen ölçülü kenar: ilk yer", FLAT, 100, 0),
        ("Geri dönen ölçü: ilk geçtiği yer", BACK, 75, 0),
        ("Ölçüsüz rota", UNMEASURED, 10, 0),
        ("Harita koordinatları, sola 2,5", MAP, 1150.5, -2.5),
    ]:
        r = point_at(route_of(parts), m, off)
        expect = {"p": pt(r["p"])} if "p" in r else fault_json(r)
        points.append({"title": title, "route": route_json(parts), "m": m, "offset": off, "expect": expect})
    stretches = []
    for title, parts, frm, to, off in [
        ("İki kenar boyunca", STRAIGHT, 40, 125, 0),
        ("Ters: baş sondan büyük", STRAIGHT, 125, 40, 0),
        ("Sağa 5: köşe birleşir", STRAIGHT, 40, 125, 5),
        ("Yayın ortası", ARC, 25, 75, 0),
        ("Geri dönen ölçü: iki parça", BACK, 60, 80, 0),
        ("Bütün rota", STRAIGHT, 0, 150, 0),
        ("Aynı ölçüler", STRAIGHT, 50, 50, 0),
        ("Rotanın dışında", STRAIGHT, 200, 300, 0),
        ("İki parçalı rota", TWO_PARTS, 50, 150, 0),
    ]:
        r = stretch(route_of(parts), frm, to, off)
        expect = {"parts": [part_json(q) for q in r["parts"]]} if "parts" in r else fault_json(r)
        stretches.append({"title": title, "route": route_json(parts), "from": frm, "to": to, "offset": off, "expect": expect})
    creates = []
    for title, pieces, rules in [
        (
            "Uzunluk: parçalar uç uca, ters çizilen çevrilir",
            [
                {"id": "A", "pts": [[100, 0], [100, 50]]},
                {"id": "A", "pts": [[0, 0], [100, 0]]},
                {"id": "B", "pts": [[300, 0], [200, 0]]},
            ],
            {"method": "length", "priority": "lowerLeft"},
        ),
        (
            "Uzunluk: boşluk sayılmaz ve sayılır",
            [{"id": "A", "pts": [[0, 0], [10, 0]]}, {"id": "A", "pts": [[20, 0], [30, 0]]}],
            {"method": "length", "priority": "lowerLeft", "ignoreGaps": True},
        ),
        (
            "Uzunluk: boşluk sayılır",
            [{"id": "A", "pts": [[0, 0], [10, 0]]}, {"id": "A", "pts": [[20, 0], [30, 0]]}],
            {"method": "length", "priority": "lowerLeft", "ignoreGaps": False},
        ),
        (
            "Uzunluk: çarpan ve başlangıç (km)",
            [{"id": "Y1", "pts": [[0, 0], [1000, 0]]}],
            {"method": "length", "priority": "lowerLeft", "factor": 0.001, "start": 5},
        ),
        (
            "Sağ üst öncelik: ölçü öbür uçtan",
            [{"id": "A", "pts": [[0, 0], [100, 0], [100, 50]]}],
            {"method": "length", "priority": "upperRight"},
        ),
        (
            "Tek alan: parçanın ölçü uzunluğu",
            [{"id": "A", "pts": [[0, 0], [100, 0]], "value": 2.5}, {"id": "A", "pts": [[100, 0], [300, 0]], "value": 5}],
            {"method": "oneField", "priority": "lowerLeft"},
        ),
        (
            "İki alan: baş ve son, değen parçalar birleşir; eksik olan atlanır",
            [
                {"id": "A", "pts": [[100, 0], [100, 100]], "from": 20, "to": 40},
                {"id": "A", "pts": [[0, 0], [100, 0]], "from": 10, "to": 20},
                {"id": "A", "pts": [[500, 0], [600, 0]], "from": 50},
                {"id": "", "pts": [[0, 0], [1, 0]], "from": 0, "to": 1},
            ],
            {"method": "twoFields"},
        ),
        (
            "Yaylı parça: uzunluk yay boyunca",
            [{"id": "A", "pts": [[0, 0], [100, 0]], "bulges": [BULGE_HALF]}],
            {"method": "length", "priority": "lowerLeft"},
        ),
    ]:
        r = create(pieces, rules)
        creates.append({
            "title": title,
            "pieces": [{**p, "pts": [{"x": x, "y": y} for x, y in p["pts"]]} for p in pieces],
            "rules": rules,
            "expect": {"routes": [{"id": q["id"], "parts": [part_json(x) for x in q["parts"]]} for q in r["routes"]], "faults": r["faults"]},
        })
    calibrations = []
    base = [{"pts": [[0, 0], [100, 0], [100, 100]], "ms": [None, None, None]}]
    measured = [{"pts": [[0, 0], [100, 0], [100, 100]], "ms": [0, 100, 200]}]
    for title, parts, pts, rules in [
        ("İki nokta: köşeler konur, dışarısı uzatılır", base, [((20, 1), 1000), ((101, 50), 1100)], {"radius": 2}),
        ("Dışarısı uzatılmaz: eski ölçüler kalır", measured, [((20, 1), 1000), ((101, 50), 1100)], {"radius": 2, "before": False, "after": False}),
        ("Yarıçapın dışındaki nokta alınmaz: tek nokta yetmez", base, [((20, 1), 1000), ((150, 50), 1100)], {"radius": 2}),
        ("Köşedeki nokta yeni köşe konmaz", measured, [((100, 0), 500), ((100, 100), 600)], {"radius": 1}),
        ("Yaylı kenar bölünür", [{"pts": [[0, 0], [100, 0]], "bulges": [BULGE_HALF], "ms": [None, None]}], [((0, 0), 0), ((50, -50), 10), ((100, 0), 30)], {"radius": 1}),
        ("Bölmeden: çizgi iki köşesini korur, uçları noktalardan", [{"pts": [[0, 0], [100, 0]], "ms": [None, None]}], [((20, 1), 1000), ((60, -1), 1040)], {"radius": 2, "split": False}),
    ]:
        r = calibrate(route_of(parts), pts, rules)
        expect = {"parts": [part_json(q) for q in r["parts"]], "used": r["used"]} if "parts" in r else {"fault": r["fault"], "used": r["used"]}
        calibrations.append({"title": title, "route": route_json(parts), "points": [{"p": {"x": p[0], "y": p[1]}, "m": m} for p, m in pts], "rules": rules, "expect": expect})
    overlays = []
    ev_a = [{"route": "A", "from": 0, "to": 100}, {"route": "A", "from": 150, "to": 200}, {"route": "B", "from": 10, "to": 20}]
    ev_b = [{"route": "A", "from": 50, "to": 175}, {"route": "B", "from": 20, "to": 30}, {"route": "A", "from": 60, "to": 40}]
    for title, union in [("Kesişim", False), ("Birleşim", True)]:
        rows = overlay(ev_a, ev_b, union)
        overlays.append({
            "title": title,
            "a": ev_a,
            "b": ev_b,
            "union": union,
            "expect": [{"route": q["route"], "from": float(q["from"]), "to": float(q["to"]), "a": q["a"], "b": q["b"]} for q in rows],
        })
    summaries = []
    for title, parts in [("Artan", STRAIGHT), ("Azalan", FALLING), ("Ölçüsüz köşeli", GAP_M), ("Ölçüsüz", UNMEASURED), ("İki parça", TWO_PARTS)]:
        s = summary(route_of(parts))
        summaries.append({"title": title, "route": route_json(parts), "expect": {k: (float(v) if k != "count" else v) for k, v in s.items()}})
    lines = []
    for title, parts, pts, radius in [
        ("Rotanın üstünde", STRAIGHT, [(10, 0.5), (60, -0.5), (100, 30)], 1),
        ("Bir köşesi uzak", STRAIGHT, [(10, 0.5), (60, 5)], 1),
        ("Ters yönde", STRAIGHT, [(90, 0), (20, 0)], 1),
    ]:
        r = locate_line(route_of(parts), pts, radius)
        expect = {"none": True} if "none" in r else {"from": float(r["from"]), "to": float(r["to"]), "distance": float(r["distance"])}
        lines.append({"title": title, "route": route_json(parts), "pts": [{"x": x, "y": y} for x, y in pts], "radius": radius, "expect": expect})
    areas = []
    for title, parts, rings in [
        ("İlk kenarı kesen kare", STRAIGHT, [[(20, -10), (60, -10), (60, 10), (20, 10)]]),
        ("Delikli alan: iki parça", STRAIGHT, [[(10, -10), (90, -10), (90, 10), (10, 10)], [(40, -5), (40, 5), (50, 5), (50, -5)]]),
        ("Köşeyi saran alan: tek parça", STRAIGHT, [[(80, -10), (120, -10), (120, 20), (80, 20)]]),
        ("Yay: açıyla doğrusal ölçü", ARC, [[(40, -60), (60, -60), (60, -40), (40, -40)]]),
        ("İki rota parçası", TWO_PARTS, [[(40, -10), (60, -10), (60, 60), (40, 60)]]),
        ("Ölçüsüz uçlu kenar", GAP_M, [[(50, -10), (150, -10), (150, 10), (50, 10)]]),
        ("Rotanın dışında", STRAIGHT, [[(200, 200), (300, 200), (300, 300), (200, 300)]]),
    ]:
        r = locate_area(route_of(parts), rings)
        areas.append({
            "title": title,
            "route": route_json(parts),
            "rings": [[{"x": x, "y": y} for x, y in ring] for ring in rings],
            "expect": [{"from": num(q["from"]), "to": num(q["to"]), "start": float(q["start"]), "end": float(q["end"])} for q in r],
        })
    return {
        "format": "kentos.lrs-cases",
        "version": 1,
        "generatedBy": SOURCE,
        "tolerance": TOLERANCE,
        "locate": locates,
        "pointAt": points,
        "stretch": stretches,
        "create": creates,
        "calibrate": calibrations,
        "overlay": overlays,
        "summary": summaries,
        "locateLine": lines,
        "locateArea": areas,
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)}: diskteki dosya kurallardan üretilenle aynı değil", file=sys.stderr)
            return 1
        print("doğrusal referanslamanın durumları tutarlı")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"written: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
