#!/usr/bin/env python3
"""Independent reference of Topolojik temizlik (docs/adr/0148).

Writes fixtures/topology/v1/clean.json from the rules alone, with Python's
standard library and no KentOS code. The geometry core
(`ops::topology::topology_clean`, crates/shared/geometry-core/tests/all/topology.rs)
and the web through its WASM must give the same within 1e-9 m.

Input: objects in the drawing's order, each a kind, whether it is fixed and
its paths ({pts, bulges, closed, zs}); a tolerance; the four works.

- Kinds: line, polyline and arc (one open path; an arc is two vertices and
  a bulge), area (closed rings), point (one vertex), edges (a boundary
  only: a circle, an ellipse's or a curve's chords; no vertices).
- A vertex is mobile when its object is not fixed, not a point, not edges,
  and Uçlar is on and it is an open path's end, or Köşeler is on.
- TOUCH = 1e-6 m is "the same place" and "on".

Joining (Uçlar, Köşeler):
1. Vertices within TOUCH of one another (transitively) are a node, at its
   first member's place (drawing order, then path, then vertex). A node
   is fixed when a member is not mobile; it holds a point when a member's
   object is a point.
2. Priority: nodes holding a point, then other fixed nodes, then mobile
   ones; more objects meeting first; then the first member's order.
3. Representatives: every fixed node; then each mobile node in priority
   order that has no representative within the tolerance.
4. Every other mobile node, in priority order, goes to the nearest
   representative within the tolerance (equal distance: priority) that it
   may join, else stays. It may not join when, for an object it shares
   with the representative's cluster, the vertices of one path would not
   be a run in that path (cyclically for a ring, and through the ends for
   an open path that closes), or two paths of one object would meet, or a
   path would keep fewer distinct vertices than two (open), three (a ring,
   or an open path that closes).
5. Its vertices move onto the representative's place; each takes the
   representative's elevation (its first member that has one), else keeps
   its own. A changed path drops a vertex that falls on the one before it
   (and a ring's last on its first), keeping the next edge's bulge.

Free ends (Uzat, Buda, Uçlar onto an edge), on the joined drawing: an open
path's end is free when no vertex or edge of another object, and no other
vertex of its path, lies within TOUCH of it. The boundaries are the other
objects' edges.
- Trim: from the end back along the path, the first crossing (a touch
  counts) whose distance along the path is over TOUCH and at most the
  tolerance.
- Extend: when the end edge is straight, along it from the end, the
  first boundary hit over TOUCH and at most the tolerance away.
- Both: the smaller change; equal: trim. Neither, with Uçlar on: the
  nearest point of a boundary within the tolerance (over TOUCH).
- A path's two ends are worked out alike; when both are trimmed past each
  other, neither is.
- Elevations: an extended end carries its end edge's grade when both its
  ends have one, else keeps its own; a trimmed end takes the cut edge's
  elevation by length (an arc by angle) when both ends have one, else its
  own; an end moved onto an edge keeps its own.

Counts: ends and vertices moved by joining, extended, trimmed, moved onto
an edge; the largest move of any vertex. Changes in order: joining's
vertex by vertex, then the ends object by object, the start first.

    python3 scripts/fixtures/topology_cases.py           # write the fixture
    python3 scripts/fixtures/topology_cases.py --check   # compare
"""

import json
import math
import random
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/topology/v1/clean.json"
TOUCH = 1e-6


# --- primitives ----------------------------------------------------------------------------


def xy(p):
    return {"x": p[0], "y": p[1]}


def dist(a, b):
    return math.hypot(b[0] - a[0], b[1] - a[1])


def arc_of(a, b, bulge):
    """An arc edge from a to b: centre, radius, start angle, signed sweep."""
    theta = 4 * math.atan(bulge)
    dx, dy = b[0] - a[0], b[1] - a[1]
    chord = math.hypot(dx, dy)
    h = (chord / 2) / math.tan(theta / 2)
    c = ((a[0] + b[0]) / 2 - dy / chord * h, (a[1] + b[1]) / 2 + dx / chord * h)
    r = (chord / 2) / abs(math.sin(theta / 2))
    return c, r, math.atan2(a[1] - c[1], a[0] - c[0]), theta


class Edge:
    def __init__(self, a, b, bulge=0.0):
        self.a, self.b = a, b
        self.arc = arc_of(a, b, bulge) if bulge and abs(bulge) > 1e-12 and dist(a, b) > 0 else None
        self.bulge = bulge if self.arc else 0.0

    def length(self):
        if self.arc:
            return self.arc[1] * abs(self.arc[3])
        return dist(self.a, self.b)

    def at(self, t):
        if self.arc:
            c, r, a0, sw = self.arc
            th = a0 + sw * t
            return (c[0] + r * math.cos(th), c[1] + r * math.sin(th))
        return (self.a[0] + (self.b[0] - self.a[0]) * t, self.a[1] + (self.b[1] - self.a[1]) * t)

    def param_of_angle(self, th):
        """The fraction of the sweep at angle th, or None when off the arc."""
        c, r, a0, sw = self.arc
        d = (th - a0) % (2 * math.pi)
        if sw >= 0:
            t = d / sw if sw else 0.0
            if t > 1 + 1e-12:
                d2 = d - 2 * math.pi
                t2 = d2 / sw
                return t2 if t2 >= -1e-12 else None
            return t
        d = (a0 - th) % (2 * math.pi)
        t = d / -sw
        if t > 1 + 1e-12:
            t2 = (d - 2 * math.pi) / -sw
            return t2 if t2 >= -1e-12 else None
        return t

    def closest(self, p):
        """(point, distance, t) of the edge's nearest point to p."""
        if self.arc:
            c, r, a0, sw = self.arc
            th = math.atan2(p[1] - c[1], p[0] - c[0])
            t = self.param_of_angle(th)
            cands = [0.0, 1.0] + ([t] if t is not None and 0 <= t <= 1 else [])
            best = None
            for u in cands:
                q = self.at(u)
                d = dist(p, q)
                if best is None or d < best[1]:
                    best = (q, d, u)
            return best
        dx, dy = self.b[0] - self.a[0], self.b[1] - self.a[1]
        l2 = dx * dx + dy * dy
        t = 0.0 if l2 == 0 else max(0.0, min(1.0, ((p[0] - self.a[0]) * dx + (p[1] - self.a[1]) * dy) / l2))
        q = self.at(t)
        return q, dist(p, q), t


def seg_seg(e1, e2):
    (ax, ay), (bx, by) = e1.a, e1.b
    (cx, cy), (dx_, dy_) = e2.a, e2.b
    rx, ry, sx, sy = bx - ax, by - ay, dx_ - cx, dy_ - cy
    den = rx * sy - ry * sx
    if abs(den) < 1e-15 * (abs(rx * sy) + abs(ry * sx) + 1e-300):
        return []
    qx, qy = cx - ax, cy - ay
    t = (qx * sy - qy * sx) / den
    u = (qx * ry - qy * rx) / den
    eps = 1e-12
    if -eps <= t <= 1 + eps and -eps <= u <= 1 + eps:
        t, u = min(1.0, max(0.0, t)), min(1.0, max(0.0, u))
        return [(e1.at(t), t, u)]
    return []


def line_circle(a, b, c, r):
    """Parameters t along a→b where the line meets the circle."""
    dx, dy = b[0] - a[0], b[1] - a[1]
    fx, fy = a[0] - c[0], a[1] - c[1]
    A = dx * dx + dy * dy
    B = 2 * (fx * dx + fy * dy)
    C = fx * fx + fy * fy - r * r
    disc = B * B - 4 * A * C
    if A == 0 or disc < 0:
        return []
    s = math.sqrt(disc)
    return sorted({(-B - s) / (2 * A), (-B + s) / (2 * A)})


def seg_arc(seg, arc):
    c, r, a0, sw = arc.arc
    out = []
    for t in line_circle(seg.a, seg.b, c, r):
        if -1e-12 <= t <= 1 + 1e-12:
            t = min(1.0, max(0.0, t))
            p = seg.at(t)
            u = arc.param_of_angle(math.atan2(p[1] - c[1], p[0] - c[0]))
            if u is not None and -1e-12 <= u <= 1 + 1e-12:
                out.append((p, t, min(1.0, max(0.0, u))))
    return out


def arc_arc(e1, e2):
    c1, r1, _, _ = e1.arc
    c2, r2, _, _ = e2.arc
    d = dist(c1, c2)
    if d == 0 or d > r1 + r2 or d < abs(r1 - r2):
        return []
    a = (r1 * r1 - r2 * r2 + d * d) / (2 * d)
    h2 = r1 * r1 - a * a
    h = math.sqrt(max(0.0, h2))
    mx, my = c1[0] + a * (c2[0] - c1[0]) / d, c1[1] + a * (c2[1] - c1[1]) / d
    pts = {(mx + h * (c2[1] - c1[1]) / d, my - h * (c2[0] - c1[0]) / d), (mx - h * (c2[1] - c1[1]) / d, my + h * (c2[0] - c1[0]) / d)}
    out = []
    for p in pts:
        t = e1.param_of_angle(math.atan2(p[1] - c1[1], p[0] - c1[0]))
        u = e2.param_of_angle(math.atan2(p[1] - c2[1], p[0] - c2[0]))
        if t is not None and u is not None and -1e-12 <= t <= 1 + 1e-12 and -1e-12 <= u <= 1 + 1e-12:
            out.append((p, min(1.0, max(0.0, t)), min(1.0, max(0.0, u))))
    return out


def crossings(e1, e2):
    """(point, t on e1, u on e2) where two edges meet."""
    if e1.arc and e2.arc:
        return arc_arc(e1, e2)
    if e1.arc:
        return [(p, u, t) for p, t, u in seg_arc(e2, e1)]
    if e2.arc:
        return seg_arc(e1, e2)
    return seg_seg(e1, e2)


def ray_hits(o, d, e):
    """Distances along the unit direction d from o where the ray meets e."""
    far = (o[0] + d[0] * 1e9, o[1] + d[1] * 1e9)
    ray = Edge(o, far)
    return [t * 1e9 for _, t, _ in crossings(ray, e)]


# --- objects -------------------------------------------------------------------------------


def path_edges(path):
    pts, closed = path["pts"], path["closed"]
    bulges = path.get("bulges") or [0.0] * len(pts)
    n = len(pts)
    count = n if closed else n - 1
    return [Edge(tuple(pts[i]), tuple(pts[(i + 1) % n]), bulges[i]) for i in range(count)]


def all_edges(obj):
    return [e for p in obj["paths"] for e in path_edges(p)]


# --- the rules -----------------------------------------------------------------------------


def clean(objects, tol, works):
    objs = [dict(o, paths=[dict(p, pts=[(q["x"], q["y"]) for q in p["pts"]], zs=list(p["zs"]), **({"bulges": list(p["bulges"])} if p.get("bulges") is not None else {})) for p in o["paths"]]) for o in objects]
    opened = ("line", "polyline", "arc")
    verts = []  # (o, k, i)
    for o, ob in enumerate(objs):
        if ob["kind"] == "edges":
            continue
        for k, p in enumerate(ob["paths"]):
            for i in range(len(p["pts"])):
                verts.append((o, k, i))

    def pos(v):
        o, k, i = v
        return objs[o]["paths"][k]["pts"][i]

    def is_end(v):
        o, k, i = v
        p = objs[o]["paths"][k]
        return objs[o]["kind"] in opened and not p["closed"] and i in (0, len(p["pts"]) - 1)

    def mobile(v):
        ob = objs[v[0]]
        if ob["fixed"] or ob["kind"] in ("point", "edges"):
            return False
        return (works["ends"] and is_end(v)) or (works["vertices"] and ob["kind"] in opened + ("area",))

    # 1. Nodes.
    parent = list(range(len(verts)))

    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    for i in range(len(verts)):
        for j in range(i + 1, len(verts)):
            if dist(pos(verts[i]), pos(verts[j])) <= TOUCH:
                a, b = find(i), find(j)
                if a != b:
                    parent[max(a, b)] = min(a, b)
    groups = {}
    for i in range(len(verts)):
        groups.setdefault(find(i), []).append(verts[i])
    nodes = []
    for members in groups.values():
        members.sort()
        fixed = any(not mobile(v) for v in members)
        point = any(objs[v[0]]["kind"] == "point" for v in members)
        degree = len({v[0] for v in members})
        nodes.append({"members": members, "at": pos(members[0]), "fixed": fixed, "point": point, "degree": degree})

    def priority(n):
        return (0 if n["point"] else 1 if n["fixed"] else 2, -n["degree"], n["members"][0])

    nodes.sort(key=priority)
    rank = {id(n): r for r, n in enumerate(nodes)}

    # 3. Representatives.
    reps = [n for n in nodes if n["fixed"]]
    for n in nodes:
        if n["fixed"]:
            continue
        if not any(dist(n["at"], r["at"]) <= tol for r in reps):
            reps.append(n)
    rep_ids = {id(r) for r in reps}

    # Clusters: per representative, its members; per path, the clusters' index sets.
    cluster = {id(r): list(r["members"]) for r in reps}
    for n in nodes:
        if id(n) not in rep_ids:
            cluster[id(n)] = list(n["members"])  # a node standing alone

    def path_sets(path_key, clusters):
        out = []
        for members in clusters:
            s = sorted({v[2] for v in members if (v[0], v[1]) == path_key})
            if s:
                out.append(s)
        return out

    def run_ok(s, n, closed, open_closing):
        if len(s) < 2:
            return True
        if not closed and not open_closing:
            return s[-1] - s[0] == len(s) - 1
        gaps = sum(1 for j in range(len(s) - 1) if s[j + 1] - s[j] > 1) + (1 if (s[0] + n) - s[-1] > 1 else 0)
        return gaps <= 1

    def may_join(node, rep_key):
        joined = cluster[rep_key] + node["members"]
        objects_in = {}
        for v in joined:
            objects_in.setdefault(v[0], set()).add(v[1])
        for o, paths in objects_in.items():
            if len(paths) > 1:
                return False
        touched = {(v[0], v[1]) for v in node["members"]}
        others = [m for key, m in cluster.items() if key not in (rep_key, id(node))]
        for path_key in touched:
            o, k = path_key
            p = objs[o]["paths"][k]
            n = len(p["pts"])
            closed = p["closed"]
            sets = path_sets(path_key, others + [joined])
            s_new = sorted({v[2] for v in joined if (v[0], v[1]) == path_key})
            closing = (not closed) and 0 in s_new and (n - 1) in s_new
            if not run_ok(s_new, n, closed, closing):
                return False
            any_closing = (not closed) and any(0 in s and (n - 1) in s for s in sets)
            distinct = n - sum(len(s) - 1 for s in sets)
            if distinct < (3 if closed or any_closing else 2):
                return False
        return True

    target = {}
    for n in nodes:
        if id(n) in rep_ids or n["fixed"]:
            continue
        cands = sorted((dist(n["at"], r["at"]), rank[id(r)], id(r)) for r in reps if dist(n["at"], r["at"]) <= tol)
        for _, _, rk in cands:
            if may_join(n, rk):
                cluster[rk] = cluster[rk] + n["members"]
                del cluster[id(n)]
                target[id(n)] = rk
                break

    # 5. Moves.
    rep_of = {id(r): r for r in reps}
    changes, counts, shift = [], {"ends": 0, "vertices": 0, "extended": 0, "trimmed": 0, "edges": 0}, 0.0
    moved = {}
    for n in nodes:
        rk = target.get(id(n))
        if rk is None:
            continue
        r = rep_of[rk]
        rz = next((objs[v[0]]["paths"][v[1]]["zs"][v[2]] for v in r["members"] if objs[v[0]]["paths"][v[1]]["zs"][v[2]] is not None), None)
        for v in n["members"]:
            moved[v] = (r["at"], rz)
    changed = set()
    for v in sorted(moved):
        o, k, i = v
        p = objs[o]["paths"][k]
        old = p["pts"][i]
        to, rz = moved[v]
        if old != to:
            changes.append({"kind": "end" if is_end(v) else "vertex", "from": xy(old), "to": xy(to), "object": o})
            counts["ends" if is_end(v) else "vertices"] += 1
            shift = max(shift, dist(old, to))
        p["pts"][i] = to
        if rz is not None:
            p["zs"][i] = rz
        changed.add(o)
    # Vertices that fall on the one before them in a changed path.
    for o in changed:
        for p in objs[o]["paths"]:
            pts, zs = p["pts"], p["zs"]
            bulges = p.get("bulges")
            keep = [0]
            for i in range(1, len(pts)):
                if pts[i] == pts[keep[-1]]:
                    if bulges is not None:
                        bulges[keep[-1]] = bulges[i]
                    continue
                keep.append(i)
            if p["closed"] and len(keep) > 1 and pts[keep[-1]] == pts[keep[0]]:
                keep.pop()
            p["pts"] = [pts[i] for i in keep]
            p["zs"] = [zs[i] for i in keep]
            if bulges is not None:
                p["bulges"] = [bulges[i] for i in keep]

    # Free ends.
    def others_vertices(o):
        return [q for oo, ob in enumerate(objs) if oo != o and ob["kind"] != "edges" for p in ob["paths"] for q in p["pts"]]

    def others_edges(o):
        return [e for oo, ob in enumerate(objs) if oo != o for e in all_edges(ob)]

    ends_work = []
    for o, ob in enumerate(objs):
        if ob["fixed"] or ob["kind"] not in opened:
            continue
        p = ob["paths"][0]
        if p["closed"] or len(p["pts"]) < 2:
            continue
        verts_o, edges_o = others_vertices(o), others_edges(o)
        plan = {}
        for which in ("start", "end"):
            i = 0 if which == "start" else len(p["pts"]) - 1
            e_pt = p["pts"][i]
            if any(dist(e_pt, q) <= TOUCH for q in verts_o):
                continue
            if any(ed.closest(e_pt)[1] <= TOUCH for ed in edges_o):
                continue
            if any(dist(e_pt, q) <= TOUCH for j, q in enumerate(p["pts"]) if j != i):
                continue
            own = path_edges(p)
            lengths = [ed.length() for ed in own]
            total = sum(lengths)
            best = None
            if works["trim"]:
                cuts = []
                for j, ed in enumerate(own):
                    before = sum(lengths[:j])
                    for b in edges_o:
                        for q, t, _ in crossings(ed, b):
                            cuts.append((before + t * lengths[j], j, t, q))
                if which == "end":
                    cands = [c for c in cuts if TOUCH < total - c[0] <= tol]
                    if cands:
                        c = max(cands, key=lambda c: c[0])
                        best = ("trim", total - c[0], c)
                else:
                    cands = [c for c in cuts if TOUCH < c[0] <= tol]
                    if cands:
                        c = min(cands, key=lambda c: c[0])
                        best = ("trim", c[0], c)
            if works["extend"]:
                edge = own[-1] if which == "end" else own[0]
                if not edge.arc:
                    a, b = (edge.a, edge.b) if which == "end" else (edge.b, edge.a)
                    l = dist(a, b)
                    d = ((b[0] - a[0]) / l, (b[1] - a[1]) / l)
                    hits = [t for bnd in edges_o for t in ray_hits(b, d, bnd) if TOUCH < t <= tol]
                    if hits:
                        t = min(hits)
                        if best is None or t < best[1]:
                            best = ("extend", t, (b[0] + d[0] * t, b[1] + d[1] * t))
            if best is None and works["ends"]:
                near = [(ed.closest(e_pt)[1], ed.closest(e_pt)[0]) for ed in edges_o]
                near = [n for n in near if TOUCH < n[0] <= tol]
                if near:
                    dmin, q = min(near, key=lambda n: n[0])
                    best = ("edge", dmin, q)
            if best:
                plan[which] = best
        if "start" in plan and "end" in plan and plan["start"][0] == "trim" and plan["end"][0] == "trim":
            if plan["start"][2][0] >= plan["end"][2][0] - TOUCH:
                del plan["start"], plan["end"]
        if plan:
            ends_work.append((o, plan))

    def cut_point(p, s_data, own, which):
        """A cut's place and elevation: the crossing, and its edge's elevation by length (an arc by
        angle) when both ends have one, else the cut end's own."""
        _, j, t, q = s_data
        zs = p["zs"]
        n = len(p["pts"])
        za, zb = zs[j], zs[(j + 1) % n]
        if za is not None and zb is not None:
            return q, za + (zb - za) * t
        return q, zs[0] if which == "start" else zs[-1]

    for o, plan in ends_work:
        p = objs[o]["paths"][0]
        pts, zs = p["pts"], p["zs"]
        bulges = p.get("bulges")
        first, last = pts[0], pts[-1]
        own = path_edges(p)
        # The cuts, on the path as it was: a sub-path from the start's cut to the end's.
        if any(plan.get(w, ("",))[0] == "trim" for w in ("start", "end")):
            n = len(pts)
            j0, t0 = (plan["start"][2][1], plan["start"][2][2]) if plan.get("start", ("",))[0] == "trim" else (0, 0.0)
            j1, t1 = (plan["end"][2][1], plan["end"][2][2]) if plan.get("end", ("",))[0] == "trim" else (n - 2, 1.0)
            new_pts, new_zs, new_b = [], [], []
            if plan.get("start", ("",))[0] == "trim":
                q, zq = cut_point(p, plan["start"][2], own, "start")
            else:
                q, zq = pts[0], zs[0]
            new_pts.append(q)
            new_zs.append(zq)
            for i in range(j0 + 1, j1 + 1):
                new_pts.append(pts[i])
                new_zs.append(zs[i])
            if plan.get("end", ("",))[0] == "trim":
                q, zq = cut_point(p, plan["end"][2], own, "end")
            else:
                q, zq = pts[-1], zs[-1]
            new_pts.append(q)
            new_zs.append(zq)
            if bulges is not None:
                for i in range(j0, j1 + 1):
                    ed = own[i]
                    lo = t0 if i == j0 else 0.0
                    hi = t1 if i == j1 else 1.0
                    new_b.append(math.tan(ed.arc[3] * (hi - lo) / 4) if ed.arc else 0.0)
                new_b.append(0.0)
                p["bulges"] = new_b
            p["pts"], p["zs"] = new_pts, new_zs
        for which in ("start", "end"):
            if which not in plan:
                continue
            kind, amount, data = plan[which]
            pts, zs = p["pts"], p["zs"]
            i = 0 if which == "start" else len(pts) - 1
            old = first if which == "start" else last
            if kind == "extend":
                a = pts[1] if which == "start" else pts[-2]
                za, zb = (zs[1], zs[0]) if which == "start" else (zs[-2], zs[-1])
                end_at = pts[i]
                pts[i] = data
                if za is not None and zb is not None:
                    zs[i] = zb + (zb - za) * (amount / dist(a, end_at))
                counts["extended"] += 1
            elif kind == "edge":
                pts[i] = data
                counts["edges"] += 1
            else:
                counts["trimmed"] += 1
            new = pts[i]
            changes.append({"kind": {"extend": "extended", "edge": "edge", "trim": "trimmed"}[kind], "from": xy(old), "to": xy(new), "object": o})
            shift = max(shift, dist(old, new))
            changed.add(o)

    out = []
    for o in sorted(changed):
        out.append({"object": o, "paths": [{"pts": [xy(q) for q in p["pts"]], **({"bulges": p["bulges"]} if p.get("bulges") is not None else {}), "closed": p["closed"], "zs": p["zs"]} for p in objs[o]["paths"]]})
    return {"changed": out, "changes": changes, "counts": counts, "maxShift": shift}


# --- cases ---------------------------------------------------------------------------------

E0, N0 = 487000.0, 4420000.0


def obj(kind, paths, fixed=False):
    return {"kind": kind, "fixed": fixed, "paths": paths}


def path(pts, closed=False, bulges=None, zs=None, at=(0.0, 0.0)):
    out = {"pts": [{"x": at[0] + x, "y": at[1] + y} for x, y in pts], "closed": closed, "zs": zs or [None] * len(pts)}
    if bulges is not None:
        out["bulges"] = bulges
    return out


ALL = {"ends": True, "vertices": False, "extend": True, "trim": True}


def hand(at):
    c = []

    def case(name, objects, tol=0.05, works=ALL):
        c.append({"name": name, "objects": objects, "tolerance": tol, "works": works})

    P = lambda pts, **kw: path(pts, at=at, **kw)  # noqa: E731
    case("iki uç buluşur", [obj("line", [P([(0, 0), (10, 0)])]), obj("line", [P([(10.03, 0.01), (20, 5)])])])
    case("uç noktaya gider", [obj("line", [P([(0, 0), (10, 0)])]), obj("point", [P([(10.02, -0.01)], zs=[101.25])])])
    case("uç iç köşeye gider", [obj("polyline", [P([(0, 0), (10, 0), (10, 10)])]), obj("line", [P([(10.02, 0.03), (20, 0)])])])
    case("zincirlenmez", [obj("line", [P([(0, 0), (1, 0)])]), obj("line", [P([(1.03, 0), (2, 1)])]), obj("line", [P([(1.06, 0), (2, -1)])])])
    case("en yakın temsilci", [obj("point", [P([(0, 0)])]), obj("point", [P([(0.08, 0)])]), obj("line", [P([(0.05, 0.001), (5, 5)])])], tol=0.06)
    case("çizginin iki ucu birleşmez", [obj("line", [P([(0, 0), (0.03, 0)])])])
    case("yol kapanır", [obj("polyline", [P([(0, 0), (10, 0), (10, 10), (0, 10), (0.02, 0.03)])])])
    case("üç köşeli yol kapanmaz", [obj("polyline", [P([(0, 0), (0.5, 0.04), (0.01, 0.02)])])], tol=0.05)
    case("kısa kalan uzar", [obj("line", [P([(0, 0), (10, 0)])]), obj("line", [P([(5, 0.03), (5, 10)])])])
    case("taşan kısalır", [obj("line", [P([(0, 0), (10, 0)])]), obj("line", [P([(5, -0.04), (5, 10)])])])
    case("taşan kısalır, toleransı geçeni değil", [obj("line", [P([(0, 0), (10, 0)])]), obj("line", [P([(5, -0.4), (5, 10)])])])
    # Only Uzat and Buda: with Uçlar the parallel line's ends would go onto the first.
    case("ikisi birden: küçüğü", [obj("line", [P([(0, 0), (10, 0)])]), obj("line", [P([(0, 0.06), (10, 0.06)])]), obj("line", [P([(5, 0.02), (5, 10)])])], tol=0.1, works={"ends": False, "vertices": False, "extend": True, "trim": True})
    case("kenara taşınır", [obj("line", [P([(0, 0), (10, 0)])]), obj("line", [P([(5, 0.03), (7, 10)])])], works={"ends": True, "vertices": False, "extend": False, "trim": False})
    # Turning clockwise, the arc leaves its start steeply: it meets the line 4 cm along.
    case("yay ucu kısalır", [obj("line", [P([(0, 0), (10, 0)])]), obj("arc", [P([(5, -0.04), (8, 3)], bulges=[-0.3, 0.0])])])
    case("yay ucu uzamaz, kenara gider", [obj("line", [P([(0, 0), (10, 0)])]), obj("arc", [P([(5, 0.03), (8, 3)], bulges=[0.3, 0.0])])])
    case("dayanak oynamaz", [obj("line", [P([(0, 0), (10, 0)])], fixed=True), obj("line", [P([(10.02, 0.02), (20, 0)])])])
    case("köşeler birleşir", [obj("area", [P([(0, 0), (10, 0), (10, 10), (0, 10)], closed=True)]), obj("area", [P([(10.02, 0.01), (20, 0), (20, 10), (10.01, 9.98)], closed=True)])], works={"ends": True, "vertices": True, "extend": True, "trim": True})
    case("art arda köşeler tek köşe olur", [obj("polyline", [P([(0, 0), (5, 0), (5.02, 0.01), (10, 0)])])], works={"ends": True, "vertices": True, "extend": True, "trim": True})
    case("halka üç köşenin altına inmez", [obj("area", [P([(0, 0), (0.03, 0), (0.0, 0.04)], closed=True)])], works={"ends": True, "vertices": True, "extend": True, "trim": True})
    case("kot taşınır", [obj("line", [P([(0, 0), (10, 0)], zs=[100.0, 101.0])]), obj("line", [P([(10.03, 0), (20, 0)], zs=[105.0, 102.0])])])
    case("uzayan uç eğimi sürer", [obj("line", [P([(0, 0), (10, 0)])]), obj("line", [P([(5, 10), (5, 0.03)], zs=[110.0, 100.0])])])
    case("kesilen uç kenarın kotunu alır", [obj("line", [P([(0, 0), (10, 0)])]), obj("line", [P([(5, 10), (5, -0.04)], zs=[110.0, 100.0])])])
    # A 4 cm line across another: each end would be cut at the same crossing.
    case("iki ucu birden kısalmaz", [obj("line", [P([(0, 0), (0, 1)])]), obj("line", [P([(0.02, 0.5), (-0.02, 0.5)])])], works={"ends": False, "vertices": False, "extend": False, "trim": True})
    case("iki ucu birden kısalır", [obj("line", [P([(0, 0), (0, 1)])]), obj("line", [P([(0.02, 0), (0.02, 1)])]), obj("line", [P([(0.03, 0.5), (-0.01, 0.5)])])], works={"ends": False, "vertices": False, "extend": False, "trim": True})
    case("kenarlar: elipsin kirişleri sınır", [obj("edges", [P([(0, 0), (3, -1), (6, -1.5), (9, -1), (12, 0)])], fixed=True), obj("line", [P([(6, 5), (6, -1.47)])])])
    return c


def drawn(rnd, at, n):
    out = []
    for k in range(n):
        objects = []
        # A small network: lines near a grid, their ends a little off.
        for i in range(6):
            x0, y0 = rnd.uniform(0, 40), rnd.uniform(0, 40)
            x1, y1 = x0 + rnd.uniform(-20, 20), y0 + rnd.uniform(-20, 20)
            objects.append(obj("line", [path([(x0, y0), (x1, y1)], at=at)]))
        for i in range(3):
            # Ends meant to meet another line's end, off by a fraction of the tolerance.
            src = objects[rnd.randrange(len(objects))]["paths"][0]["pts"][1]
            off = rnd.uniform(0.2, 0.7) * 0.05
            th = rnd.uniform(0, 2 * math.pi)
            a = (src["x"] - at[0] + off * math.cos(th), src["y"] - at[1] + off * math.sin(th))
            objects.append(obj("line", [path([a, (a[0] + rnd.uniform(-15, 15), a[1] + rnd.uniform(-15, 15))], at=at)]))
        if rnd.random() < 0.5:
            objects.append(obj("point", [path([(rnd.uniform(0, 40), rnd.uniform(0, 40))], zs=[rnd.uniform(90, 110)], at=at)]))
        # T junctions: a line stopping short of another or running past it, at a fair angle.
        for i in range(2):
            x0, y0 = rnd.uniform(50, 90), rnd.uniform(0, 40)
            th = rnd.uniform(0, math.pi)
            base = ((x0 - 10 * math.cos(th), y0 - 10 * math.sin(th)), (x0 + 10 * math.cos(th), y0 + 10 * math.sin(th)))
            objects.append(obj("line", [path(list(base), at=at, zs=[rnd.uniform(90, 110), rnd.uniform(90, 110)])]))
            phi = th + rnd.uniform(math.pi / 6, 5 * math.pi / 6)
            dx, dy = math.cos(phi), math.sin(phi)
            # The end lies `along` past the base line along the line's own way (negative: short of it).
            along = rnd.choice([1, -1]) * rnd.uniform(0.2, 0.7) * 0.05
            u = rnd.uniform(-3, 3)
            end = (x0 + u * math.cos(th) - dx * along, y0 + u * math.sin(th) - dy * along)
            far = (end[0] + dx * 12, end[1] + dy * 12)
            objects.append(obj("line", [path([far, end], at=at, zs=[rnd.uniform(90, 110), rnd.uniform(90, 110)])]))
        # Two parcels sharing a jittered edge (Köşeler).
        if rnd.random() < 0.6:
            x0, y0 = rnd.uniform(0, 40), rnd.uniform(50, 80)
            j = lambda: (rnd.uniform(0.2, 0.7) * 0.05 * rnd.choice([1, -1]))  # noqa: E731
            a = [(x0, y0), (x0 + 15, y0), (x0 + 15, y0 + 12), (x0, y0 + 12)]
            b = [(x0 + 15 + j(), y0 + j()), (x0 + 30, y0), (x0 + 30, y0 + 12), (x0 + 15 + j(), y0 + 12 + j())]
            objects.append(obj("area", [path(a, closed=True, at=at)]))
            objects.append(obj("area", [path(b, closed=True, at=at)]))
        # A road edge with an arc, ending near a line.
        if rnd.random() < 0.5:
            x0, y0 = rnd.uniform(50, 90), rnd.uniform(50, 80)
            objects.append(obj("line", [path([(x0 - 10, y0), (x0 + 10, y0)], at=at)]))
            stop = rnd.choice([1, -1]) * rnd.uniform(0.2, 0.7) * 0.05
            objects.append(obj("polyline", [path([(x0 + 3, y0 + 9), (x0 + 1, y0 + 4), (x0, y0 + stop)], bulges=[rnd.uniform(-0.3, 0.3), 0.0, 0.0], at=at)]))
        out.append({"name": f"rastgele {k}", "objects": objects, "tolerance": 0.05, "works": dict(ALL, vertices=rnd.random() < 0.4)})
    return out


def document():
    rnd = random.Random(20261001)
    cases = hand((0.0, 0.0)) + hand((E0, N0)) + drawn(rnd, (E0, N0), 24)
    for c in cases:
        c["expected"] = clean(c["objects"], c["tolerance"], c["works"])
    return {
        "format": "kentos.topology-fixtures",
        "version": 1,
        "rules": "docs/adr/0148 §4-§6; scripts/fixtures/topology_cases.py",
        "touch": TOUCH,
        "cases": cases,
    }


def main() -> int:
    doc = document()
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    count = len(doc["cases"])
    if "--check" in sys.argv[1:]:
        if OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} kurallardan yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)")
            return 1
        print(f"{OUT.relative_to(ROOT)}: {count} durum kurallarla tutarlı")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({count} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
