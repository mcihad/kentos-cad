#!/usr/bin/env python3
"""Topoloji kuralları (docs/adr/0202): the shared cases, written from the ADR without KentOS code.

fixtures/topology-rules/v1/cases.json: small layers, the project's rules over them, what each rule finds (the problem,
the objects, the place, the measure, whether it is an exception, which fixes it offers) and what some fixes make.

How the reference works things out, independently of the core's overlay:

- Areas of straight edges along the axes are cut into the cells of their coordinates (every x and every y a corner has
  makes the grid; a cell is inside an area when its middle is, crossing number over the area's rings). Overlaps, gaps,
  unions and differences are sets of cells; a piece is a group of cells joined by their sides (two groups meeting only
  at a corner are refused as a case); its area, perimeter (the sides it does not share), centroid and holes are exact
  fractions. Two circles' overlap is the lens in closed form.
- Lines, ends, edges, angles: fractions where they are exact (distances compared as squares, a point's nearest place
  on a segment), mpmath at 50 digits where a root comes in. Whether an edge lies within the tolerance of a path: the
  distance from a point moving along the edge to a segment is convex, so the stretch within the tolerance is found by
  ternary search and bisection, and the stretches of the path's edges must cover the whole edge.

    python3 scripts/fixtures/topology_rules_cases.py          # write
    python3 scripts/fixtures/topology_rules_cases.py --check  # compare
"""

import json
import sys
from fractions import Fraction as F
from pathlib import Path

import mpmath as mp

mp.mp.dps = 50

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "topology-rules" / "v1" / "cases.json"
SOURCE = "scripts/fixtures/topology_rules_cases.py (docs/adr/0202)"

E, N = 487000, 4420000
T = F(1, 1000)


def P(x, y):
    return (F(E) + F(x), F(N) + F(y))


def num(v):
    """A number as JSON: an integer when it is one, else the nearest double."""
    if isinstance(v, F):
        return int(v) if v.denominator == 1 else float(v)
    if isinstance(v, int):
        return v
    x = float(v)
    return int(x) if x == int(x) and abs(x) < 2**53 else x


def jp(p):
    return {"x": num(p[0]), "y": num(p[1])}


def mpf(v):
    return mp.mpf(v.numerator) / v.denominator if isinstance(v, F) else mp.mpf(v)


def check(cond, what):
    if not cond:
        raise SystemExit(f"başvuru tutarsız: {what}")


# ── Objects ────────────────────────────────────────────────────────────


class Obj:
    """An object of a case: its layer, its uid, the shape as the contract writes it and its geometry here:
    `areas` [(outer, [holes])], `paths` [[vertices]] (straight), `points`; `kind` the shape's kind."""

    def __init__(self, layer, shape, kind, areas=(), paths=(), points=()):
        self.layer = layer
        self.shape = shape
        self.kind = kind
        self.areas = list(areas)
        self.paths = [list(p) for p in paths]
        self.points = list(points)
        self.uid = None
        self.special = None


def area_obj(layer, pts, holes=(), parts=()):
    shape = {"kind": "polygon", "pts": [jp(p) for p in pts]}
    if holes:
        shape["holes"] = [{"pts": [jp(p) for p in h]} for h in holes]
    if parts:
        shape["parts"] = []
        for q in parts:
            part = {"pts": [jp(p) for p in q[0]]}
            if q[1]:
                part["holes"] = [{"pts": [jp(p) for p in h]} for h in q[1]]
            shape["parts"].append(part)
    areas = [(list(pts), [list(h) for h in holes])] + [(list(q[0]), [list(h) for h in q[1]]) for q in parts]
    return Obj(layer, shape, "polygon", areas=areas)


def rect(x0, y0, x1, y1):
    return [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]


def line_obj(layer, a, b):
    return Obj(layer, {"kind": "line", "a": jp(a), "b": jp(b)}, "line", paths=[[a, b]])


def polyline_obj(layer, pts, parts=()):
    shape = {"kind": "polyline", "pts": [jp(p) for p in pts]}
    if parts:
        shape["parts"] = [{"pts": [jp(p) for p in q]} for q in parts]
    return Obj(layer, shape, "polyline", paths=[pts] + list(parts))


def point_obj(layer, p, more=()):
    shape = {"kind": "point", "p": jp(p)}
    if more:
        shape["parts"] = [{"p": jp(q)} for q in more]
    return Obj(layer, shape, "point", points=[p] + list(more))


def circle_obj(layer, c, r):
    o = Obj(layer, {"kind": "circle", "c": jp(c), "r": num(F(r))}, "circle")
    o.special = ("circle", c, F(r))
    return o


def arc_obj(layer, c, r, a0, a1):
    """An arc counter-clockwise from a0 to a1 (radians, as the doubles written)."""
    o = Obj(layer, {"kind": "arc", "c": jp(c), "r": num(F(r)), "a0": float(a0), "a1": float(a1)}, "arc")
    o.special = ("arc", c, F(r), mp.mpf(float(a0)), mp.mpf(float(a1)))
    return o


def ring_edges(ring):
    return [(ring[i], ring[(i + 1) % len(ring)]) for i in range(len(ring))]


def path_edges(path):
    return [(path[i], path[i + 1]) for i in range(len(path) - 1)]


def boundary_edges(o):
    return [e for outer, holes in o.areas for r in [outer] + holes for e in ring_edges(r)]


def all_path_edges(o):
    return [e for p in o.paths for e in path_edges(p)]


# ── Distances (fractions where exact) ──────────────────────────────────


def d2(p, q):
    return (p[0] - q[0]) ** 2 + (p[1] - q[1]) ** 2


def nearest_on_seg(p, a, b):
    """The nearest place on segment ab to p, exactly."""
    dx, dy = b[0] - a[0], b[1] - a[1]
    l2 = dx * dx + dy * dy
    if l2 == 0:
        return a
    t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2
    t = max(F(0), min(F(1), t))
    return (a[0] + t * dx, a[1] + t * dy)


def dist(p, q):
    return mp.sqrt(mpf(d2(p, q)))


def seg_len(a, b):
    return dist(a, b)


def mid(a, b):
    return ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)


def at_length(edges, s):
    """The place `s` along a run of straight edges."""
    for a, b in edges:
        L = seg_len(a, b)
        if s <= L:
            k = s / L if L else mp.mpf(0)
            return (mpf(a[0]) + k * (mpf(b[0]) - mpf(a[0])), mpf(a[1]) + k * (mpf(b[1]) - mpf(a[1])))
        s -= L
    a, b = edges[-1]
    return (mpf(b[0]), mpf(b[1]))


# ── The cells of axis-parallel areas ───────────────────────────────────


def axis_parallel(ring):
    return all(a[0] == b[0] or a[1] == b[1] for a, b in ring_edges(ring))


def inside_rings(rings, p):
    """Crossing number over all the rings (a cell's middle is never on an edge)."""
    inside = False
    for ring in rings:
        for a, b in ring_edges(ring):
            if (a[1] > p[1]) != (b[1] > p[1]):
                x = a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1])
                if x > p[0]:
                    inside = not inside
    return inside


class Grid:
    """The cells of every corner's x and y, with a margin of 2 m all round."""

    def __init__(self, objs):
        xs, ys = set(), set()
        for o in objs:
            for outer, holes in o.areas:
                for r in [outer] + holes:
                    check(axis_parallel(r), "hücreler yalnız eksenlere paralel kenarlarla")
                    for p in r:
                        xs.add(p[0])
                        ys.add(p[1])
        xs, ys = sorted(xs), sorted(ys)
        self.xs = [xs[0] - 2] + xs + [xs[-1] + 2]
        self.ys = [ys[0] - 2] + ys + [ys[-1] + 2]
        self.nx, self.ny = len(self.xs) - 1, len(self.ys) - 1

    def centre(self, c):
        i, j = c
        return ((self.xs[i] + self.xs[i + 1]) / 2, (self.ys[j] + self.ys[j + 1]) / 2)

    def cells(self, areas):
        out = set()
        for i in range(self.nx):
            for j in range(self.ny):
                m = self.centre((i, j))
                if any(inside_rings([outer] + holes, m) for outer, holes in areas):
                    out.add((i, j))
        return out

    def size(self, c):
        i, j = c
        return (self.xs[i + 1] - self.xs[i], self.ys[j + 1] - self.ys[j])

    def side(self, c, n):
        """The length of the side two neighbouring cells share."""
        w, h = self.size(c)
        return h if n[1] == c[1] else w

    def neighbours(self, c):
        i, j = c
        for n in ((i + 1, j), (i - 1, j), (i, j + 1), (i, j - 1)):
            if 0 <= n[0] < self.nx and 0 <= n[1] < self.ny:
                yield n

    def all(self):
        return {(i, j) for i in range(self.nx) for j in range(self.ny)}

    def pieces(self, cells):
        """Groups of cells joined by their sides, in the order of their least cell."""
        left = set(cells)
        out = []
        for start in sorted(cells):
            if start not in left:
                continue
            group, stack = set(), [start]
            left.discard(start)
            while stack:
                c = stack.pop()
                group.add(c)
                for n in self.neighbours(c):
                    if n in left:
                        left.discard(n)
                        stack.append(n)
            out.append(group)
        # Two groups meeting only at a corner would be one ring touching itself: not a case.
        for g in out:
            for (i, j) in g:
                for di, dj in ((1, 1), (1, -1), (-1, 1), (-1, -1)):
                    d = (i + di, j + dj)
                    if d in cells and d not in g and (i + di, j) not in cells and (i, j + dj) not in cells:
                        raise SystemExit("başvuru: iki parça yalnız köşede buluşuyor")
        return out

    def area(self, cells):
        return sum((w * h for w, h in map(self.size, cells)), F(0))

    def perimeter(self, cells):
        total = F(0)
        for c in cells:
            i, j = c
            for n in ((i + 1, j), (i - 1, j), (i, j + 1), (i, j - 1)):
                if n not in cells:
                    total += self.side(c, n)
        return total

    def centroid(self, cells):
        a = self.area(cells)
        sx = sy = F(0)
        for c in cells:
            w, h = self.size(c)
            m = self.centre(c)
            sx += w * h * m[0]
            sy += w * h * m[1]
        return (sx / a, sy / a)

    def holes(self, cells):
        """Groups of cells outside the set that do not reach the grid's edge."""
        rest = self.all() - set(cells)
        n = 0
        for g in self.pieces(rest):
            if not any(i in (0, self.nx - 1) or j in (0, self.ny - 1) for i, j in g):
                n += 1
        return n

    def measure(self, cells):
        return {"parts": len(self.pieces(cells)), "holes": self.holes(cells), "area": self.area(cells)}


def width(grid, cells):
    return 2 * grid.area(cells) / grid.perimeter(cells)


def counted(grid, cells, t):
    """The pieces wider (2A/P) than the tolerance, in the order of their least cell."""
    return [g for g in grid.pieces(cells) if width(grid, g) > t]


def largest(grid, pieces):
    best = max(grid.area(g) for g in pieces)
    big = [g for g in pieces if grid.area(g) == best]
    check(len(big) == 1, "en büyük parça tek olmalı")
    return big[0]


# ── Polygons (exact area and centroid, perimeter in mpmath) ────────────


def shoelace(ring):
    return sum((a[0] * b[1] - b[0] * a[1] for a, b in ring_edges(ring)), F(0)) / 2


def ring_centroid_moments(ring):
    a = shoelace(ring)
    cx = sum(((a_[0] + b[0]) * (a_[0] * b[1] - b[0] * a_[1]) for a_, b in ring_edges(ring)), F(0)) / 6
    cy = sum(((a_[1] + b[1]) * (a_[0] * b[1] - b[0] * a_[1]) for a_, b in ring_edges(ring)), F(0)) / 6
    return a, cx, cy


def part_measures(outer, holes):
    """Net area, perimeter and centroid of an area's part (outer counter-clockwise or not)."""
    a0, cx0, cy0 = ring_centroid_moments(outer)
    sign = 1 if a0 > 0 else -1
    A, CX, CY = a0 * sign, cx0 * sign, cy0 * sign
    for h in holes:
        a, cx, cy = ring_centroid_moments(h)
        s = 1 if a > 0 else -1
        A -= a * s
        CX -= cx * s
        CY -= cy * s
    perim = sum(seg_len(a, b) for r in [outer] + holes for a, b in ring_edges(r))
    return A, perim, (CX / A, CY / A)


# ── Within the tolerance along an edge ─────────────────────────────────


def seg_point_dist(p, a, b):
    """mpmath distance from p (mp pair) to segment ab (fractions)."""
    ax, ay, bx, by = mpf(a[0]), mpf(a[1]), mpf(b[0]), mpf(b[1])
    dx, dy = bx - ax, by - ay
    l2 = dx * dx + dy * dy
    t = ((p[0] - ax) * dx + (p[1] - ay) * dy) / l2 if l2 else mp.mpf(0)
    t = max(mp.mpf(0), min(mp.mpf(1), t))
    return mp.sqrt((p[0] - ax - t * dx) ** 2 + (p[1] - ay - t * dy) ** 2)


def near_interval(e, f, t):
    """The stretch [s0, s1] of edge e within t of segment f (convex), or None."""
    a, b = e
    ax, ay, bx, by = mpf(a[0]), mpf(a[1]), mpf(b[0]), mpf(b[1])

    def g(s):
        return seg_point_dist((ax + s * (bx - ax), ay + s * (by - ay)), f[0], f[1])

    lo, hi = mp.mpf(0), mp.mpf(1)
    for _ in range(300):
        m1, m2 = lo + (hi - lo) / 3, hi - (hi - lo) / 3
        if g(m1) <= g(m2):
            hi = m2
        else:
            lo = m1
    s = (lo + hi) / 2
    tt = mpf(t)
    if g(s) > tt:
        return None

    def edge_of(x0, x1, rising):
        # The place between x0 and x1 where g crosses t (g within t at the inner end).
        if (g(x0) <= tt) if rising else (g(x1) <= tt):
            return x0 if rising else x1
        for _ in range(300):
            m = (x0 + x1) / 2
            if (g(m) <= tt) == rising:
                x1 = m
            else:
                x0 = m
        return (x0 + x1) / 2

    return (edge_of(mp.mpf(0), s, True), edge_of(s, mp.mpf(1), False))


def covered(intervals):
    """The union of stretches, as sorted disjoint ones."""
    out = []
    for s0, s1 in sorted(intervals):
        if out and s0 <= out[-1][1] + mp.mpf(10) ** -40:
            out[-1] = (out[-1][0], max(out[-1][1], s1))
        else:
            out.append((s0, s1))
    return out


def near_stretches(e, edges, t):
    return covered([iv for f in edges if (iv := near_interval(e, f, t)) is not None])


def edge_within(e, edges, t):
    c = near_stretches(e, edges, t)
    eps = mp.mpf(10) ** -30
    return len(c) == 1 and c[0][0] <= eps and c[0][1] >= 1 - eps


# ── Findings ───────────────────────────────────────────────────────────


def finding(rule, problem, objects, at, measure, kind, fixes):
    return {
        "rule": rule,
        "problem": problem,
        "objects": objects,
        "at": {"x": float(mpf(at[0])), "y": float(mpf(at[1]))},
        "measure": None if measure is None else float(mpf(measure)),
        "measureKind": kind,
        "fixes": fixes,
    }


def of_layer(objs, layer, kinds):
    return [i for i, o in enumerate(objs) if o.layer == layer and o.kind in kinds]


AREA = {"polygon", "circle"}
PATH = {"line", "polyline", "arc"}
POINT = {"point"}


def overlaps(objs, rule, pairs, t):
    grid = Grid([objs[k] for p in pairs for k in p])
    out = []
    for i, j in pairs:
        ci, cj = grid.cells(objs[i].areas), grid.cells(objs[j].areas)
        both = ci & cj
        pieces = counted(grid, both, t)
        if not pieces:
            continue
        big = largest(grid, pieces)
        fixes = []
        if ci - cj:
            fixes.append("subtractFirst")
        if cj - ci:
            fixes.append("subtractSecond")
        out.append(finding(rule, "overlap", [i, j], grid.centroid(big), sum(grid.area(g) for g in pieces), "area", fixes))
    return out


def overlap_fix(objs, i, j, fix):
    grid = Grid([objs[i], objs[j]])
    ci, cj = grid.cells(objs[i].areas), grid.cells(objs[j].areas)
    who, left = (i, ci - cj) if fix == "subtractFirst" else (j, cj - ci)
    return [{"object": who, "kind": "polygon", **{k: num(v) for k, v in grid.measure(left).items()}}]


def intra_pairs(objs, idx):
    return [(a, b) for n, a in enumerate(idx) for b in idx[n + 1:]]


def must_not_overlap(objs, rule, t):
    return overlaps(objs, rule["id"], intra_pairs(objs, of_layer(objs, rule["layer"], AREA)), t)


def must_not_overlap_with(objs, rule, t):
    a = of_layer(objs, rule["layer"], AREA)
    b = of_layer(objs, rule["other"], AREA)
    return overlaps(objs, rule["id"], [(i, j) for i in a for j in b], t)


def gaps(objs, rule, t):
    idx = of_layer(objs, rule["layer"], AREA)
    grid = Grid([objs[i] for i in idx])
    cells = {i: grid.cells(objs[i].areas) for i in idx}
    union = set().union(*cells.values())
    out = []
    found = []
    for g in grid.pieces(grid.all() - union):
        if any(i in (0, grid.nx - 1) or j in (0, grid.ny - 1) for i, j in g):
            continue
        if width(grid, g) <= t:
            continue
        shared = {}
        for c in g:
            for n in grid.neighbours(c):
                if n in g:
                    continue
                for k in idx:
                    if n in cells[k]:
                        shared[k] = shared.get(k, F(0)) + grid.side(c, n)
        order = sorted(shared, key=lambda k: (-shared[k], idx.index(k)))
        c = grid.centroid(g)
        found.append((c, g, order))
    found.sort(key=lambda f: (-f[0][1], f[0][0]))
    for c, g, order in found:
        out.append(finding(rule["id"], "gap", order, c, grid.area(g), "area", ["mergeNeighbour"] if order else []))
    return out


def gap_fix(objs, layer, n, t):
    """The n-th gap (in the findings' order) merged into the neighbour it shares the most with."""
    idx = of_layer(objs, layer, AREA)
    grid = Grid([objs[i] for i in idx])
    cells = {i: grid.cells(objs[i].areas) for i in idx}
    union = set().union(*cells.values())
    found = gaps(objs, {"id": "", "layer": layer}, t)
    want = (found[n]["at"]["x"], found[n]["at"]["y"])
    for g in grid.pieces(grid.all() - union):
        c = grid.centroid(g)
        if (float(c[0]), float(c[1])) == want:
            into = found[n]["objects"][0]
            return [{"object": into, "kind": "polygon", **{k: num(v) for k, v in grid.measure(cells[into] | g).items()}}]
    raise SystemExit("başvuru: boşluk bulunamadı")


def slivers(objs, rule, t):
    g_min = F(rule.get("value", F(1, 10)))
    out = []
    idx = of_layer(objs, rule["layer"], AREA)
    for i in idx:
        o = objs[i]
        for k, (outer, holes) in enumerate(o.areas):
            A, perim, c = part_measures(outer, holes)
            w = 2 * mpf(A) / perim
            if w < mpf(g_min):
                shared = sliver_shared(objs, idx, i, k)
                fixes = (["mergeNeighbour"] if shared else []) + ["deletePart"]
                out.append(finding(rule["id"], "sliver", [i], c, w, "length", fixes))
    return out


def sliver_shared(objs, idx, i, k):
    """The neighbours a part shares its boundary with and how long (cells; the cases' edges meet exactly)."""
    part = Obj("", None, "polygon", areas=[objs[i].areas[k]])
    others = [j for j in idx if j != i]
    grid = Grid([part] + [objs[j] for j in others])
    pc = grid.cells(part.areas)
    shared = {}
    for j in others:
        cj = grid.cells(objs[j].areas)
        check(not (pc & cj), "ince alan komşusuyla örtüşmemeli")
        s = sum((grid.side(c, n) for c in pc for n in grid.neighbours(c) if n in cj), F(0))
        if s > 0:
            shared[j] = s
    return sorted(shared, key=lambda j: (-shared[j], idx.index(j)))


def sliver_fix(objs, layer, i, k, fix):
    idx = of_layer(objs, layer, AREA)
    o = objs[i]
    changes = []
    if fix == "mergeNeighbour":
        into = sliver_shared(objs, idx, i, k)[0]
        part = Obj("", None, "polygon", areas=[o.areas[k]])
        grid = Grid([part, objs[into]])
        merged = grid.cells(part.areas) | grid.cells(objs[into].areas)
        changes.append({"object": into, "kind": "polygon", **{a: num(v) for a, v in grid.measure(merged).items()}})
    rest = [p for n, p in enumerate(o.areas) if n != k]
    if not rest:
        changes.append({"object": i, "remove": True})
    else:
        A = sum(part_measures(outer, holes)[0] for outer, holes in rest)
        changes.append({"object": i, "kind": "polygon", "parts": len(rest), "holes": sum(len(h) for _, h in rest), "area": num(A)})
    return sorted(changes, key=lambda c: c["object"])


def duplicates(objs, rule, t):
    out = []
    idx = [i for i, o in enumerate(objs) if o.layer == rule["layer"] and o.kind in PATH | POINT]
    for n, i in enumerate(idx):
        for j in idx[n + 1:]:
            a, b = objs[i], objs[j]
            if a.kind in PATH and b.kind in PATH:
                da = [e for e in all_path_edges(a) if seg_len(*e) > 2 * mpf(t) and edge_within(e, all_path_edges(b), t)]
                db = [e for e in all_path_edges(b) if seg_len(*e) > 2 * mpf(t) and edge_within(e, all_path_edges(a), t)]
                if not da and not db:
                    continue
                la, lb = sum(seg_len(*e) for e in da), sum(seg_len(*e) for e in db)
                longest = max(da + db, key=lambda e: seg_len(*e))
                whole = lambda o, d: d and len(d) == len([e for e in all_path_edges(o) if seg_len(*e) > 2 * mpf(t)])
                fixes = ["deleteDuplicate"] if whole(b, db) or whole(a, da) else []
                out.append(finding(rule["id"], "duplicateEdge", [i, j], mid(*longest), max(la, lb), "length", fixes))
            elif a.kind in POINT and b.kind in POINT:
                best = None
                for p in a.points:
                    for q in b.points:
                        d = d2(p, q)
                        if best is None or d < best[0]:
                            best = (d, q)
                if best and best[0] <= t * t:
                    fixes = ["deleteDuplicate"] if len(b.points) == 1 else []
                    out.append(finding(rule["id"], "duplicatePoint", [i, j], best[1], mp.sqrt(mpf(best[0])), "distance", fixes))
    return out


def dangles(objs, rule, t):
    out = []
    lay = rule["layer"]
    paths = [i for i, o in enumerate(objs) if o.layer == lay and o.kind in PATH]
    areas = [i for i, o in enumerate(objs) if o.layer == lay and o.kind in AREA]
    for i in paths:
        o = objs[i]
        for k, path in enumerate(o.paths):
            if d2(path[0], path[-1]) <= t * t:
                continue
            last = len(path) - 2
            for end, own in ((path[0], {0, 1}), (path[-1], {last, last - 1})):
                best = None
                for j in sorted(paths + areas):
                    q = objs[j]
                    if q.kind in AREA:
                        edges = [(e, None) for e in boundary_edges(q)]
                    else:
                        edges = [(e, (kk, ee)) for kk, p in enumerate(q.paths) for ee, e in enumerate(path_edges(p))]
                    for e, tag in edges:
                        if j == i and tag is not None and tag[0] == k and tag[1] in own:
                            continue
                        near = nearest_on_seg(end, *e)
                        dd = d2(end, near)
                        if best is None or dd < best[0]:
                            best = (dd, near)
                if best is not None and best[0] <= t * t:
                    continue
                fixes = ["snapEnd"] if best is not None and o.kind in {"line", "polyline"} else []
                out.append(finding(rule["id"], "dangle", [i], end, None if best is None else mp.sqrt(mpf(best[0])), "distance", fixes))
    return out


def snap_end_fix(objs, i, k, which, rule_layer, t):
    """The object with the end moved to the nearest place (the dangle's rule)."""
    o = objs[i]
    path = list(o.paths[k])
    end = path[0] if which == "start" else path[-1]
    own = {0, 1} if which == "start" else {len(path) - 2, len(path) - 3}
    best = None
    lay = rule_layer
    for j, q in enumerate(objs):
        if q.layer != lay or q.kind not in PATH | AREA:
            continue
        if q.kind in AREA:
            edges = [(e, None) for e in boundary_edges(q)]
        else:
            edges = [(e, (kk, ee)) for kk, p in enumerate(q.paths) for ee, e in enumerate(path_edges(p))]
        for e, tag in edges:
            if j == i and tag is not None and tag[0] == k and tag[1] in own:
                continue
            near = nearest_on_seg(end, *e)
            dd = d2(end, near)
            if best is None or dd < best[0]:
                best = (dd, near)
    if which == "start":
        path[0] = best[1]
    else:
        path[-1] = best[1]
    return [{"object": i, "pts": [jp(p) for p in path]}]


def short_edges(objs, rule, t):
    kmin = mpf(F(rule.get("value", F(1, 20))))
    out = []
    for i, o in enumerate(objs):
        if o.layer != rule["layer"]:
            continue
        if o.kind == "arc":
            _, c, r, a0, a1 = o.special
            L = mpf(r) * (a1 - a0)
            if L < kmin:
                m = (a0 + a1) / 2
                out.append(finding(rule["id"], "shortEdge", [i], (mpf(c[0]) + mpf(r) * mp.cos(m), mpf(c[1]) + mpf(r) * mp.sin(m)), L, "length", ["deleteObject"]))
            continue
        if o.kind in {"line", "polyline"}:
            for k, path in enumerate(o.paths):
                n = len(path)
                for e, (a, b) in enumerate(path_edges(path)):
                    L = seg_len(a, b)
                    if L < kmin:
                        if n >= 3:
                            fixes = ["removeVertex"]
                        elif len(o.paths) == 1:
                            fixes = ["deleteObject"]
                        else:
                            fixes = []
                        out.append(finding(rule["id"], "shortEdge", [i], mid(a, b), L, "length", fixes))
        elif o.kind == "polygon":
            for outer, holes in o.areas:
                for ring in [outer] + holes:
                    for a, b in ring_edges(ring):
                        L = seg_len(a, b)
                        if L < kmin:
                            out.append(finding(rule["id"], "shortEdge", [i], mid(a, b), L, "length", ["removeVertex"] if len(ring) >= 4 else []))
    return out


def angle_at(prev, v, nxt):
    ux, uy = mpf(prev[0] - v[0]), mpf(prev[1] - v[1])
    wx, wy = mpf(nxt[0] - v[0]), mpf(nxt[1] - v[1])
    return mp.atan2(abs(ux * wy - uy * wx), ux * wx + uy * wy)


def small_angles(objs, rule, t):
    amin = mpf(rule["value"])
    out = []
    for i, o in enumerate(objs):
        if o.layer != rule["layer"]:
            continue
        if o.kind == "polygon":
            for outer, holes in o.areas:
                for ring in [outer] + holes:
                    n = len(ring)
                    for k in range(n):
                        prev, v, nxt = ring[k - 1], ring[k], ring[(k + 1) % n]
                        if prev == v or nxt == v:
                            continue
                        a = angle_at(prev, v, nxt)
                        if a < amin:
                            out.append(finding(rule["id"], "smallAngle", [i], v, a, "angle", ["removeVertex"] if n >= 4 else []))
        elif o.kind in {"line", "polyline"}:
            for path in o.paths:
                n = len(path)
                closed = d2(path[0], path[-1]) <= T * T
                for k in range(0, n - 1):
                    if k == 0:
                        if not closed or n < 4:
                            continue
                        prev, v, nxt = path[-2], path[0], path[1]
                    else:
                        prev, v, nxt = path[k - 1], path[k], path[k + 1]
                    if prev == v or nxt == v:
                        continue
                    a = angle_at(prev, v, nxt)
                    if a < amin:
                        fixes = ["removeVertex"] if k > 0 and n >= 3 else []
                        out.append(finding(rule["id"], "smallAngle", [i], v, a, "angle", fixes))
    return out


def seg_cross(a, b, c, d):
    """Where segments ab and cd cross at one place inside both (fractions), else None."""
    r = (b[0] - a[0], b[1] - a[1])
    s = (d[0] - c[0], d[1] - c[1])
    den = r[0] * s[1] - r[1] * s[0]
    if den == 0:
        return None
    u = ((c[0] - a[0]) * s[1] - (c[1] - a[1]) * s[0]) / den
    v = ((c[0] - a[0]) * r[1] - (c[1] - a[1]) * r[0]) / den
    if 0 < u < 1 and 0 < v < 1:
        return (a[0] + u * r[0], a[1] + u * r[1]), u
    return None


def validity(objs, rule, t):
    """The cases' problems: a ring or a path crossing itself (the first pair's first crossing), a repeated vertex."""
    out = []
    for i, o in enumerate(objs):
        if o.layer != rule["layer"] or o.kind not in AREA | PATH:
            continue
        rings = [(r, True) for outer, holes in o.areas for r in [outer] + holes] + [(p, False) for p in o.paths]
        for ring, closed in rings:
            n = len(ring)
            reps = [k for k in range(n if closed else n - 1) if ring[k] == ring[(k + 1) % n]]
            if reps:
                out.append(finding(rule["id"], "repeated", [i], ring[reps[0]], None, None, ["repair"]))
                continue
            edges = ring_edges(ring) if closed else path_edges(ring)
            hit = None
            for x in range(len(edges)):
                for y in range(x + 2, len(edges)):
                    if closed and x == 0 and y == len(edges) - 1:
                        continue
                    c = seg_cross(*edges[x], *edges[y])
                    if c and (hit is None):
                        hit = c[0]
                if hit:
                    break
            if hit:
                kind = "ringCrossing" if closed else "pathCrossing"
                out.append(finding(rule["id"], kind, [i], hit, None, None, ["repair"] if closed else []))
    return out


def missing_vertices(objs, rule, t):
    idx = of_layer(objs, rule["layer"], {"polygon"})
    out = []
    for i in idx:
        for j in idx:
            if i == j:
                continue
            bj = boundary_edges(objs[j])
            vj = [p for outer, holes in objs[j].areas for r in [outer] + holes for p in r]
            for outer, holes in objs[i].areas:
                for r in [outer] + holes:
                    for v in r:
                        dd = min(d2(v, nearest_on_seg(v, *e)) for e in bj)
                        if dd <= t * t and all(d2(v, q) > t * t for q in vj):
                            out.append(finding(rule["id"], "missingVertex", [j, i], v, mp.sqrt(mpf(dd)), "distance", ["addVertex"]))
    return out


def add_vertex_fix(objs, j, v):
    """Object j with v inserted in its nearest edge (rings in order; the first of equal ones)."""
    best = None
    for pn, (outer, holes) in enumerate(objs[j].areas):
        for rn, r in enumerate([outer] + holes):
            for k, e in enumerate(ring_edges(r)):
                dd = d2(v, nearest_on_seg(v, *e))
                if best is None or dd < best[0]:
                    best = (dd, pn, rn, k)
    _, pn, rn, k = best
    outer, holes = objs[j].areas[pn]
    ring = list(([outer] + holes)[rn])
    ring.insert(k + 1, v)
    check(pn == 0 and rn == 0, "durumlar yalnız dış halkaya ekler")
    return [{"object": j, "pts": [jp(p) for p in ring]}]


def covered_by(objs, rule, t):
    lay, other = rule["layer"], rule["other"]
    us = [o for o in objs if o.layer == other and o.kind in AREA]
    out = []
    mine = [i for i, o in enumerate(objs) if o.layer == lay and o.kind in AREA | PATH | POINT]
    grid = Grid(us + [objs[i] for i in mine if objs[i].kind in AREA])
    union = set().union(*[grid.cells(o.areas) for o in us]) if us else set()
    for i in mine:
        o = objs[i]
        if o.kind in AREA:
            outside = grid.cells(o.areas) - union
            pieces = counted(grid, outside, t)
            if pieces:
                big = largest(grid, pieces)
                inside = grid.cells(o.areas) & union
                out.append(finding(rule["id"], "outside", [i], grid.centroid(big), sum(grid.area(g) for g in pieces), "area", ["clipOutside"] if inside else []))
        elif o.kind in PATH:
            stretches = outside_stretches(o, us)
            stretches = [s for s in stretches if sum(seg_len(*e) for e in s) > mpf(t)]
            if stretches:
                lengths = [sum(seg_len(*e) for e in s) for s in stretches]
                long = stretches[lengths.index(max(lengths))]
                total = sum(lengths)
                whole = sum(seg_len(*e) for e in all_path_edges(o))
                out.append(finding(rule["id"], "outside", [i], at_length(long, max(lengths) / 2), total, "length", ["clipOutside"] if total < whole else []))
        else:
            for p in o.points:
                if any(inside_or_on(u, p) for u in us):
                    continue
                d = min((seg_point_dist((mpf(p[0]), mpf(p[1])), *e) for u in us for e in boundary_edges(u)), default=None)
                if d is None or d > mpf(t):
                    out.append(finding(rule["id"], "outside", [i], p, d, "distance", []))
    return out


def inside_or_on(o, p):
    for outer, holes in o.areas:
        for r in [outer] + holes:
            for a, b in ring_edges(r):
                if d2(p, nearest_on_seg(p, a, b)) == 0:
                    return True
        if inside_rings([outer] + holes, p):
            return True
    return False


def outside_stretches(o, us):
    """A path's runs outside the areas (cut where it crosses their boundaries; a piece by its middle)."""
    bounds = [e for u in us for e in boundary_edges(u)]
    runs, run = [], []
    for path in o.paths:
        for a, b in path_edges(path):
            ts = [F(0), F(1)]
            for c, d in bounds:
                x = seg_cross(a, b, c, d)
                if x:
                    ts.append(x[1])
                # A boundary's corner on the edge cuts it too.
                for q in (c, d):
                    dx, dy = b[0] - a[0], b[1] - a[1]
                    if (q[0] - a[0]) * dy - (q[1] - a[1]) * dx == 0:
                        l2 = dx * dx + dy * dy
                        s = ((q[0] - a[0]) * dx + (q[1] - a[1]) * dy) / l2
                        if 0 < s < 1:
                            ts.append(s)
            ts = sorted(set(ts))
            for s0, s1 in zip(ts, ts[1:]):
                p0 = (a[0] + s0 * (b[0] - a[0]), a[1] + s0 * (b[1] - a[1]))
                p1 = (a[0] + s1 * (b[0] - a[0]), a[1] + s1 * (b[1] - a[1]))
                m = mid(p0, p1)
                if any(inside_or_on(u, m) for u in us):
                    if run:
                        runs.append(run)
                        run = []
                else:
                    if run and run[-1][1] != p0:
                        runs.append(run)
                        run = []
                    run.append((p0, p1))
        if run:
            runs.append(run)
            run = []
    return runs


def clip_path_fix(objs, i, us_layer):
    o = objs[i]
    us = [q for q in objs if q.layer == us_layer and q.kind in AREA]
    outside = outside_stretches(o, us)
    total = sum(seg_len(*e) for e in all_path_edges(o)) - sum(seg_len(*e) for s in outside for e in s)
    return total


def boundary_covered_by(objs, rule, t):
    lay, other = rule["layer"], rule["other"]
    cover = [e for o in objs if o.layer == other for e in (boundary_edges(o) if o.kind in AREA else all_path_edges(o))]
    out = []
    for i, o in enumerate(objs):
        if o.layer != lay or o.kind not in AREA:
            continue
        stretches = []
        for outer, holes in o.areas:
            for ring in [outer] + holes:
                pieces = []  # (edge index, s0, s1) uncovered, in ring order
                edges = ring_edges(ring)
                for k, e in enumerate(edges):
                    c = near_stretches(e, cover, t)
                    s = mp.mpf(0)
                    for c0, c1 in c:
                        if c0 > s:
                            pieces.append((k, s, c0))
                        s = max(s, c1)
                    if s < 1:
                        pieces.append((k, s, mp.mpf(1)))
                eps = mp.mpf(10) ** -30
                runs = []
                for k, s0, s1 in pieces:
                    if runs and runs[-1][-1][0] == k - 1 and runs[-1][-1][2] >= 1 - eps and s0 <= eps:
                        runs[-1].append((k, s0, s1))
                    else:
                        runs.append([(k, s0, s1)])
                n = len(edges)
                if len(runs) > 1 and runs[-1][-1][0] == n - 1 and runs[-1][-1][2] >= 1 - eps and runs[0][0][0] == 0 and runs[0][0][1] <= eps:
                    runs[0] = runs.pop() + runs[0]
                for run in runs:
                    segs = []
                    for k, s0, s1 in run:
                        a, b = edges[k]
                        pa = (mpf(a[0]) + s0 * (mpf(b[0]) - mpf(a[0])), mpf(a[1]) + s0 * (mpf(b[1]) - mpf(a[1])))
                        pb = (mpf(a[0]) + s1 * (mpf(b[0]) - mpf(a[0])), mpf(a[1]) + s1 * (mpf(b[1]) - mpf(a[1])))
                        segs.append((pa, pb))
                    L = sum(mp.sqrt((q[0] - p[0]) ** 2 + (q[1] - p[1]) ** 2) for p, q in segs)
                    if L > mpf(t):
                        stretches.append((L, segs))
        if stretches:
            longest = max(stretches, key=lambda s: s[0])
            half = longest[0] / 2
            at = None
            for p, q in longest[1]:
                L = mp.sqrt((q[0] - p[0]) ** 2 + (q[1] - p[1]) ** 2)
                if half <= L:
                    k = half / L
                    at = (p[0] + k * (q[0] - p[0]), p[1] + k * (q[1] - p[1]))
                    break
                half -= L
            out.append(finding(rule["id"], "uncoveredBoundary", [i], at, sum(s[0] for s in stretches), "length", []))
    return out


def on_end_of(objs, rule, t):
    lay, other = rule["layer"], rule["other"]
    ends = []
    for o in objs:
        if o.layer == other and o.kind in {"line", "polyline"}:
            for p in o.paths:
                ends.append(p[0])
                if d2(p[0], p[-1]) > t * t:
                    ends.append(p[-1])
    out = []
    for i, o in enumerate(objs):
        if o.layer != lay or o.kind not in POINT:
            continue
        for p in o.points:
            ds = [d2(p, q) for q in ends]
            d = min(ds) if ds else None
            if d is None or d > t * t:
                out.append(finding(rule["id"], "notOnEnd", [i], p, None if d is None else mp.sqrt(mpf(d)), "distance", ["snapToEnd"] if ds else []))
    return out


def snap_to_end_fix(objs, i, other, t):
    """The point object moved to the nearest end of the other layer's lines (the first of equal ones)."""
    ends = []
    for o in objs:
        if o.layer == other and o.kind in {"line", "polyline"}:
            for p in o.paths:
                ends.append(p[0])
                if d2(p[0], p[-1]) > t * t:
                    ends.append(p[-1])
    p = objs[i].points[0]
    best = min(range(len(ends)), key=lambda k: (d2(p, ends[k]), k))
    return [{"object": i, "p": jp(ends[best])}]


RULES = {
    "mustNotOverlap": must_not_overlap,
    "mustNotHaveGaps": gaps,
    "mustNotHaveSlivers": slivers,
    "mustNotHaveDuplicates": duplicates,
    "mustNotHaveDangles": dangles,
    "mustNotHaveShortEdges": short_edges,
    "mustNotHaveSmallAngles": small_angles,
    "mustBeValid": validity,
    "mustNotHaveMissingVertices": missing_vertices,
    "mustNotOverlapWith": must_not_overlap_with,
    "mustBeCoveredBy": covered_by,
    "boundaryMustBeCoveredBy": boundary_covered_by,
    "mustBeOnEndOf": on_end_of,
}


def run(objs, rules, t, exceptions=()):
    out = []
    for rule in rules:
        for f in RULES[rule["kind"]](objs, rule, t):
            uids = [objs[k].uid for k in f["objects"]]
            f["exception"] = any(
                x["rule"] == rule["id"] and x["objects"] == uids and (F(x["at"]["x"]) - F(f["at"]["x"])) ** 2 + (F(x["at"]["y"]) - F(f["at"]["y"])) ** 2 <= t * t
                for x in exceptions
            )
            out.append(f)
    return out


# ── The cases ──────────────────────────────────────────────────────────


def uid_of(case, n):
    return f"0190{case:04x}-0000-7000-8000-{n:012x}"


def case(n, cid, title, objs, rules, fixes=(), t=T, exceptions=()):
    for k, o in enumerate(objs):
        o.uid = uid_of(n, k + 1)
    exc = [dict(x, objects=[objs[k].uid for k in x["objects"]]) for x in exceptions]
    findings = run(objs, rules, t, exc)
    out = {
        "id": cid,
        "title": title,
        "tolerance": num(t),
        "objects": [{"layer": o.layer, "uid": o.uid, "shape": o.shape} for o in objs],
        "rules": [dict(r, value=num(r["value"])) if "value" in r else r for r in rules],
        "findings": findings,
    }
    if exc:
        out["exceptions"] = exc
    made = []
    for k, fix, changes in fixes:
        check(fix in findings[k]["fixes"], f"{cid}: {k}. bulguda {fix} yok")
        made.append({"finding": k, "fix": fix, "changes": changes})
    if made:
        out["fixed"] = made
    return out


def rule(rid, kind, layer, other=None, value=None):
    r = {"id": rid, "kind": kind, "layer": layer}
    if other is not None:
        r["other"] = other
    if value is not None:
        r["value"] = value
    return r


def build():
    cases = []

    # 1. Two parcels overlap in a 2 × 6 m strip; a third touches nothing.
    objs = [area_obj("parsel", rect(0, 0, 10, 10)), area_obj("parsel", rect(8, 2, 18, 8)), area_obj("parsel", rect(20, 0, 30, 10))]
    rules = [rule("r1", "mustNotOverlap", "parsel")]
    cases.append(case(1, "overlap", "Çakışmamalı: iki parselin 2 × 6 m'lik ortak parçası; iki düzeltme", objs, rules,
                      fixes=[(0, "subtractFirst", overlap_fix(objs, 0, 1, "subtractFirst")), (0, "subtractSecond", overlap_fix(objs, 0, 1, "subtractSecond"))]))

    # 2. A 0.4 mm overlap is noise at a 1 mm tolerance; a 2 mm one is not.
    objs = [area_obj("parsel", rect(0, 0, 10, 10)), area_obj("parsel", rect(F("9.9996"), 0, 20, 10)), area_obj("parsel", rect(F("19.998"), 0, 30, 10))]
    cases.append(case(2, "overlap-noise", "Çakışmamalı: toleranstan dar (0,4 mm) parça sayılmaz, 2 mm'lik sayılır", objs, [rule("r1", "mustNotOverlap", "parsel")]))

    # 3. A U-shaped parcel overlaps a strip in two pieces; the larger one gives the place.
    u = [P(0, 0), P(30, 0), P(30, 10), P(20, 10), P(20, 4), P(10, 4), P(10, 10), P(0, 10)]
    objs = [area_obj("parsel", u), area_obj("parsel", rect(5, 6, 26, 12))]
    cases.append(case(3, "overlap-pieces", "Çakışmamalı: iki ayrı parça; ölçü ikisinin toplamı, yer büyüğünün ağırlık merkezi", objs, [rule("r1", "mustNotOverlap", "parsel")],
                      fixes=[(0, "subtractFirst", overlap_fix(objs, 0, 1, "subtractFirst")), (0, "subtractSecond", overlap_fix(objs, 0, 1, "subtractSecond"))]))

    # 4. Gaps: a block with its middle missing (one neighbour 2 m nearer), a pinwheel's 0.5 mm hole (noise), a parcel's hole with an island.
    s = F(5, 10000)
    objs = [
        area_obj("parsel", rect(0, 0, 10, 10)),
        area_obj("parsel", rect(10, 0, 20, 10)),
        area_obj("parsel", rect(20, 0, 30, 10)),
        area_obj("parsel", rect(0, 10, 10, 20)),
        area_obj("parsel", rect(18, 10, 30, 20)),
        area_obj("parsel", rect(0, 20, 10, 30)),
        area_obj("parsel", rect(10, 20, 20, 30)),
        area_obj("parsel", rect(20, 20, 30, 30)),
        area_obj("parsel", rect(40, 0, 50 + s, 10)),
        area_obj("parsel", rect(50 + s, 0, 60, 10 + s)),
        area_obj("parsel", rect(50, 10 + s, 60, 20)),
        area_obj("parsel", rect(40, 10, 50, 20)),
        area_obj("parsel", rect(70, 0, 90, 20), holes=[rect(75, 5, 85, 15)]),
        area_obj("parsel", rect(78, 8, 82, 12)),
    ]
    cases.append(case(4, "gaps", "Boşluk olmamalı: ortası eksik ada (komşular ortak sınırlarıyla), 0,5 mm'lik gürültü, delikteki ada; kuzeyden güneye", objs,
                      [rule("r1", "mustNotHaveGaps", "parsel")],
                      fixes=[(0, "mergeNeighbour", gap_fix(objs, "parsel", 0, T)), (1, "mergeNeighbour", gap_fix(objs, "parsel", 1, T))]))

    # 5. Slivers: a 5 cm parcel between two neighbours (the longer shared side wins), a multi-part area's thin part.
    objs = [
        area_obj("parsel", rect(0, 0, 10, 10)),
        area_obj("parsel", rect(10, 0, F("10.05"), 10)),
        area_obj("parsel", rect(F("10.05"), 2, 20, 10)),
        area_obj("parsel", rect(30, 0, 40, 10), parts=[(rect(45, 0, F("45.08"), 5), [])]),
    ]
    rules = [rule("r1", "mustNotHaveSlivers", "parsel", value=F(1, 10))]
    cases.append(case(5, "slivers", "İnce alan olmamalı: 5 cm'lik parsel (en uzun ortak sınırlı komşuya katılır), çok parçalı alanın ince parçası", objs, rules,
                      fixes=[(0, "mergeNeighbour", sliver_fix(objs, "parsel", 1, 0, "mergeNeighbour")), (0, "deletePart", sliver_fix(objs, "parsel", 1, 0, "deletePart")),
                             (1, "deletePart", sliver_fix(objs, "parsel", 3, 1, "deletePart"))]))

    # 6. Duplicates: a line 0.4 mm off a polyline's first edge, two lines overlapping only in part (not a duplicate), two points 0.67 mm apart.
    objs = [
        polyline_obj("yol", [P(0, 0), P(10, 0), P(20, 5)]),
        line_obj("yol", P(0, F("0.0004")), P(10, F("0.0004"))),
        line_obj("yol", P(30, 0), P(40, 0)),
        polyline_obj("yol", [P(35, 0), P(45, 0), P(45, 10)]),
        point_obj("nokta", P(50, 0)),
        point_obj("nokta", P(F("50.0006"), F("0.0003"))),
        point_obj("nokta", P(52, 0)),
    ]
    rules = [rule("r1", "mustNotHaveDuplicates", "yol"), rule("r2", "mustNotHaveDuplicates", "nokta")]
    cases.append(case(6, "duplicates", "Yinelenmemeli: kenarı bütünüyle öbürünün 0,4 mm yakınında olan çizgi, yarısı örtüşen çizgiler (yinelenen değil), 0,67 mm'lik iki nokta", objs, rules,
                      fixes=[(0, "deleteDuplicate", [{"object": 1, "remove": True}]), (1, "deleteDuplicate", [{"object": 5, "remove": True}])]))

    # 7. Dangles: lines meeting at a T, an end 30 cm short, a closed path, a line ending on an area's boundary.
    objs = [
        polyline_obj("yol", [P(0, 0), P(10, 0), P(10, 10)]),
        line_obj("yol", P(10, 5), P(F("19.7"), 5)),
        line_obj("yol", P(20, 0), P(20, 10)),
        polyline_obj("yol", [P(40, 0), P(50, 0), P(50, 10), P(40, 0)]),
        line_obj("yol", P(61, 3), P(70, 3)),
        area_obj("yol", rect(70, 0, 80, 10)),
    ]
    rules = [rule("r1", "mustNotHaveDangles", "yol")]
    found = dangles(objs, rules[0], T)
    k_short = next(n for n, f in enumerate(found) if f["objects"] == [1])
    cases.append(case(7, "dangles", "Sarkan uç olmamalı: T birleşimi bağlı, 30 cm kısa uç (en yakın çizgiye taşınır), kapalı yol, alan sınırına değen uç", objs, rules,
                      fixes=[(k_short, "snapEnd", snap_end_fix(objs, 1, 0, "end", "yol", T))]))

    # 8. Short edges: a parcel's 3 cm edge, a polyline's 2 cm last edge, a 4 cm line, a 4 cm arc, a hole's 2 cm edge.
    hole = [P(23, 3), P(27, 3), P(27, 7), P(F("23.02"), 7), P(23, 7)]
    objs = [
        area_obj("parsel", [P(0, 0), P(10, 0), P(10, F("0.03")), P(10, 10), P(0, 10)]),
        polyline_obj("parsel", [P(30, 0), P(40, 0), P(F("40.02"), 0)]),
        line_obj("parsel", P(50, 0), P(F("50.04"), 0)),
        arc_obj("parsel", P(60, 0), F(4, 100), 0.5, 1.5),
        area_obj("parsel", rect(20, 0, 30, 10), holes=[hole]),
    ]
    rules = [rule("r1", "mustNotHaveShortEdges", "parsel")]
    cases.append(case(8, "short-edges", "Kısa kenar olmamalı: alanın 3 cm'lik kenarı, çoklu çizginin son kenarı (baştaki köşesi silinir), 4 cm'lik çizgi ve yay, deliğin kenarı", objs, rules,
                      fixes=[(0, "removeVertex", [{"object": 0, "pts": [jp(p) for p in [P(0, 0), P(10, 0), P(10, 10), P(0, 10)]]}]),
                             (1, "removeVertex", [{"object": 1, "pts": [jp(p) for p in [P(30, 0), P(F("40.02"), 0)]]}]),
                             (2, "deleteObject", [{"object": 2, "remove": True}]),
                             (3, "deleteObject", [{"object": 3, "remove": True}]),
                             (4, "removeVertex", [{"object": 4, "pts": [jp(p) for p in rect(20, 0, 30, 10)], "holes": [[jp(p) for p in hole[:4]]]}])]))

    # 9. Small angles: a parcel's spike, a polyline's sharp turn.
    import math

    objs = [
        area_obj("parsel", [P(0, 0), P(10, 0), P(10, 10), P(5, 10), P(5, 40), P(F("4.8"), 10), P(0, 10)]),
        polyline_obj("parsel", [P(20, 0), P(30, 0), P(F("20.5"), F("0.5"))]),
    ]
    rules = [rule("r1", "mustNotHaveSmallAngles", "parsel", value=5 * math.pi / 180)]
    cases.append(case(9, "small-angles", "Küçük açı olmamalı: parselin 0,38°'lik sivri ucu, çoklu çizginin 3°'lik dönüşü", objs, rules,
                      fixes=[(0, "removeVertex", [{"object": 0, "pts": [jp(p) for p in [P(0, 0), P(10, 0), P(10, 10), P(5, 10), P(F("4.8"), 10), P(0, 10)]]}]),
                             (1, "removeVertex", [{"object": 1, "pts": [jp(p) for p in [P(20, 0), P(F("20.5"), F("0.5"))]]}])]))

    # 10. Validity: a bow tie, a repeated vertex, a path crossing itself (no fix).
    objs = [
        area_obj("parsel", [P(0, 0), P(10, 10), P(10, 0), P(0, 10)]),
        area_obj("parsel", [P(20, 0), P(30, 0), P(30, 0), P(30, 10), P(20, 10)]),
        polyline_obj("parsel", [P(40, 0), P(50, 10), P(50, 0), P(40, 10)]),
    ]
    rules = [rule("r1", "mustBeValid", "parsel")]
    cases.append(case(10, "validity", "Geçerli olmalı: papyon (Onar iki üçgen yapar), yinelenen köşe, kendini kesen yol (düzeltme yok)", objs, rules,
                      fixes=[(0, "repair", [{"object": 0, "kind": "polygon", "parts": 2, "holes": 0, "area": 50}]),
                             (1, "repair", [{"object": 1, "kind": "polygon", "parts": 1, "holes": 0, "area": 100}])]))

    # 11. Missing vertices: a vertex on the neighbour's edge, a T junction, 0.5 mm off an edge (within), 2 mm off (beyond).
    objs = [
        area_obj("parsel", [P(0, 0), P(10, 0), P(10, 4), P(10, 10), P(0, 10)]),
        area_obj("parsel", rect(10, 0, 20, 10)),
        area_obj("parsel", rect(0, 10, 5, 20)),
        area_obj("parsel", rect(30, 0, 40, 10)),
        area_obj("parsel", [P(40, 0), P(50, 0), P(50, 10), P(40, 10), P(F("40.0005"), 5)]),
        area_obj("parsel", [P(30, 10), P(40, 10), P(40, 20), P(F("35"), 20), P(F("35"), F("10.002")), P(30, 20)]),
    ]
    rules = [rule("r1", "mustNotHaveMissingVertices", "parsel")]
    found = missing_vertices(objs, rules[0], T)
    cases.append(case(11, "missing-vertices", "Ortak sınırda köşe eksik olmamalı: komşunun kenarındaki köşe, T birleşimi, kenara 0,5 mm (içinde) ve 2 mm (dışında)", objs, rules,
                      fixes=[(n, "addVertex", vertex_of(objs, f)) for n, f in enumerate(found)]))

    # 12. Between layers: a building overlapping a road.
    objs = [area_obj("bina", rect(0, 0, 10, 10)), area_obj("yol", rect(8, -5, 12, 15)), area_obj("bina", rect(20, 0, 30, 10))]
    rules = [rule("r1", "mustNotOverlapWith", "bina", other="yol")]
    cases.append(case(12, "overlap-with", "… ile çakışmamalı: yola taşan bina; iki düzeltme", objs, rules,
                      fixes=[(0, "subtractFirst", overlap_fix(objs, 0, 1, "subtractFirst")), (0, "subtractSecond", overlap_fix(objs, 0, 1, "subtractSecond"))]))

    # 13. Covered by: a building out of its parcel, a line out of it, points in, on and out.
    objs = [
        area_obj("parsel", rect(0, 0, 20, 20)),
        area_obj("bina", rect(2, 2, 8, 8)),
        area_obj("bina", rect(15, 5, 25, 10)),
        line_obj("bina", P(10, 12), P(30, 12)),
        point_obj("bina", P(5, 5)),
        point_obj("bina", P(F("20.0005"), 1)),
        point_obj("bina", P(25, 1)),
    ]
    rules = [rule("r1", "mustBeCoveredBy", "bina", other="parsel")]
    grid = Grid([objs[0], objs[2]])
    inside = grid.cells(objs[2].areas) & grid.cells(objs[0].areas)
    cases.append(case(13, "covered-by", "… içinde kalmalı: parseline taşan bina ve çizgi (dışarıdaki kesilir), içerideki, sınırdaki (toleransta) ve dışarıdaki noktalar", objs, rules,
                      fixes=[(0, "clipOutside", [{"object": 2, "kind": "polygon", **{k: num(v) for k, v in grid.measure(inside).items()}}]),
                             (1, "clipOutside", [{"object": 3, "kind": "polyline", "parts": 1, "length": num(clip_path_fix(objs, 3, "parsel"))}])]))

    # 14. Boundary covered by: a block's edges along its parcels and a boundary line; a run around the ring's start.
    objs = [
        area_obj("ada", rect(0, 0, 20, 10)),
        area_obj("parsel", rect(0, F("0.0005"), 10, 10)),
        area_obj("parsel", rect(10, 0, 18, 10)),
        line_obj("parsel", P(20, 0), P(20, 4)),
        area_obj("ada", [P(40, 0), P(40, 10), P(30, 10), P(30, 0)]),
        area_obj("parsel", rect(30, 0, 38, 10)),
    ]
    rules = [rule("r1", "boundaryMustBeCoveredBy", "ada", other="parsel")]
    cases.append(case(14, "boundary-covered-by", "Sınırı … sınırlarında olmalı: parsellerle ve sınır çizgisiyle örtülen ada kenarları, halkanın başından geçen kısım", objs, rules))

    # 15. On ends: points at an end, at a vertex, near an end (within), far; a closed path's one end.
    objs = [
        polyline_obj("yol", [P(0, 0), P(10, 0), P(10, 10)]),
        polyline_obj("yol", [P(20, 0), P(30, 0), P(30, 10), P(20, 0)]),
        point_obj("nokta", P(0, 0)),
        point_obj("nokta", P(10, F("0.5"))),
        point_obj("nokta", P(F("20.0004"), F("0.0003"))),
        point_obj("nokta", P(35, 0)),
    ]
    rules = [rule("r1", "mustBeOnEndOf", "nokta", other="yol")]
    cases.append(case(15, "on-end-of", "… çizgilerinin ucunda olmalı: uçtaki, köşedeki, uca toleransta yakın ve uzaktaki nokta; kapalı yolun tek ucu", objs, rules,
                      fixes=[(0, "snapToEnd", snap_to_end_fix(objs, 3, "yol", T)), (1, "snapToEnd", snap_to_end_fix(objs, 5, "yol", T))]))

    # 16. Exceptions: the overlap of case 1 marked at its place; another rule's exception at a moved place does not hold.
    objs = [area_obj("parsel", rect(0, 0, 10, 10)), area_obj("parsel", rect(8, 2, 18, 8)), area_obj("parsel", rect(30, 0, 40, 10)), area_obj("parsel", rect(38, 2, 48, 8))]
    rules = [rule("r1", "mustNotOverlap", "parsel")]
    cases.append(case(16, "exceptions", "İstisna: aynı kural, aynı nesneler ve toleransta aynı yer; yeri değişen bulgu açık kalır", objs, rules,
                      exceptions=[{"rule": "r1", "objects": [0, 1], "at": jp(P(9, 5))}, {"rule": "r1", "objects": [2, 3], "at": jp(P(39, F("5.01")))}]))

    # 17. Two circles overlap: the lens in closed form; Birinci nesneden çıkar leaves the circle less the lens.
    r, d = mp.mpf(5), mp.mpf(6)
    lens = 2 * r * r * mp.acos(d / (2 * r)) - (d / 2) * mp.sqrt(4 * r * r - d * d)
    objs = [circle_obj("parsel", P(0, 0), 5), circle_obj("parsel", P(6, 0), 5)]
    rules = [rule("r1", "mustNotOverlap", "parsel")]
    c17 = {
        "id": "circles",
        "title": "Çakışmamalı: iki dairenin merceği (kapalı biçim), Birinci nesneden çıkar",
        "tolerance": num(T),
        "objects": [],
        "rules": rules,
        "findings": [finding("r1", "overlap", [0, 1], P(3, 0), lens, "area", ["subtractFirst", "subtractSecond"])],
        "fixed": [{"finding": 0, "fix": "subtractFirst", "changes": [{"object": 0, "kind": "polygon", "parts": 1, "holes": 0, "area": float(mp.pi * 25 - lens)}]}],
    }
    for k, o in enumerate(objs):
        o.uid = uid_of(17, k + 1)
        c17["objects"].append({"layer": o.layer, "uid": o.uid, "shape": o.shape})
    c17["findings"][0]["exception"] = False
    cases.append(c17)

    return cases


def vertex_of(objs, f):
    """The vertex a missing-vertex finding names, as the fractions it was written with."""
    owner = objs[f["objects"][1]]
    for outer, holes in owner.areas:
        for r in [outer] + holes:
            for v in r:
                if float(v[0]) == f["at"]["x"] and float(v[1]) == f["at"]["y"]:
                    return add_vertex_fix(objs, f["objects"][0], v)
    raise SystemExit("başvuru: köşe bulunamadı")


def main():
    data = {
        "format": "kentos.topology-rule-cases",
        "version": 1,
        "source": SOURCE,
        "note": "Konumlar ve ölçüler 1e-6 göreli payla karşılaştırılır; fixed'deki pts ve p tam, alan ve uzunluk 1e-9 göreli.",
        "cases": build(),
    }
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        old = OUT.read_text() if OUT.exists() else ""
        if old != text:
            raise SystemExit(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check'siz çalıştırın")
        print(f"{OUT.relative_to(ROOT)} güncel ({len(data['cases'])} durum)")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)} yazıldı ({len(data['cases'])} durum)")


if __name__ == "__main__":
    main()
