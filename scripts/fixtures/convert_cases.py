#!/usr/bin/env python3
"""Independent reference of the Dönüştürme tools (docs/adr/0218), written from the ADR alone, no KentOS code.

Writes fixtures/processing/v1/convert.kcad (the drawing), fixtures/processing/v1/convert.json (the tools run on it, in
the processing cases' format, fixtures/processing/README.md) and the tables the table tools read
(fixtures/processing/v1/convert-semt.csv, convert-xy.csv, convert-elips.csv).

How it computes, independently of the core's formulas:

1. Areas' centroids by Green's theorem, each edge integrated along its own parametrisation with mpmath's quadrature at
   50 digits (∬x dA = ½∮x² dy, ∬y dA = −½∮y² dx); paths' centroids as ∫(x, y) ds / L the same way.
2. Interior points of straight-edged areas from GEOS itself (shapely's `representative_point`); of areas with arcs by
   the ADR's scan line written here (heights of vertices and arc tops, crossings of the circles at 50 digits).
3. Points along paths (halfway, a substring's ends, a densified edge's) walked at 50 digits; an arc's pieces by angle.
4. Bearings and ellipses by mpmath's sine and cosine; numbers in cells read as the ADR says (a decimal comma a point).
5. Orders by layer_ops_cases.py's `order` (ADR 0217 §6), written without KentOS code.
"""

import json
import sys
from pathlib import Path

import mpmath as mp
import shapely

sys.path.insert(0, str(Path(__file__).resolve().parent))
from layer_ops_cases import new_id, order  # noqa: E402
from numeric_display import shown  # noqa: E402
from table_file_cases import read_text  # noqa: E402

mp.mp.dps = 50

ROOT = Path(__file__).resolve().parents[2]
PROC_DIR = ROOT / "fixtures" / "processing" / "v1"
E, N = 487000, 4420000
TOL = 1e-8
PI = mp.pi


def P(x, y):
    return {"x": E + x, "y": N + y}


def rect(x0, y0, x1, y1):
    return [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]


def mpv(p):
    return (mp.mpf(p["x"]), mp.mpf(p["y"]))


def out(v):
    """A 50-digit point as the drawing writes it."""
    return {"x": float(v[0]), "y": float(v[1])}


# ── Edges ─────────────────────────────────────────────────────────────────
# An edge from a to b with a bulge: straight for 0; else an arc whose sweep is 4·atan(bulge), counter-clockwise for a
# plus sign, its middle at the chord's middle plus (Δy, −Δx)·bulge/2 (the arc's sagitta side).


def arc_of(a, b, bulge):
    """The arc of a bulged edge: centre, radius, start angle, signed sweep, at 50 digits (from its three points)."""
    (ax, ay), (bx, by) = a, b
    b_ = mp.mpf(bulge)
    mx = (ax + bx) / 2 + (by - ay) * b_ / 2
    my = (ay + by) / 2 - (bx - ax) * b_ / 2
    # The circle through a, m and b.
    d = 2 * (ax * (my - by) + mx * (by - ay) + bx * (ay - my))
    ux = ((ax**2 + ay**2) * (my - by) + (mx**2 + my**2) * (by - ay) + (bx**2 + by**2) * (ay - my)) / d
    uy = ((ax**2 + ay**2) * (bx - mx) + (mx**2 + my**2) * (ax - bx) + (bx**2 + by**2) * (mx - ax)) / d
    r = mp.hypot(ax - ux, ay - uy)
    a0 = mp.atan2(ay - uy, ax - ux)
    sweep = 4 * mp.atan(b_)
    return (ux, uy), r, a0, sweep


class Edge:
    def __init__(self, a, b, bulge=0):
        self.a, self.b, self.bulge = a, b, bulge
        if bulge:
            self.c, self.r, self.a0, self.sweep = arc_of(a, b, bulge)
            self.length = self.r * abs(self.sweep)
        else:
            self.length = mp.hypot(b[0] - a[0], b[1] - a[1])

    def at(self, t):
        """The point a fraction t along the edge (an arc's by angle)."""
        if self.bulge:
            ang = self.a0 + self.sweep * t
            return (self.c[0] + self.r * mp.cos(ang), self.c[1] + self.r * mp.sin(ang))
        return (self.a[0] + (self.b[0] - self.a[0]) * t, self.a[1] + (self.b[1] - self.a[1]) * t)

    def deriv(self, t):
        if self.bulge:
            ang = self.a0 + self.sweep * t
            return (-self.r * mp.sin(ang) * self.sweep, self.r * mp.cos(ang) * self.sweep)
        return (self.b[0] - self.a[0], self.b[1] - self.a[1])


def ring_edges(pts, bulges, closed):
    vs = [mpv(p) for p in pts]
    n = len(vs)
    count = n if closed else n - 1
    out_ = []
    for i in range(count):
        b = (bulges[i] if bulges and i < len(bulges) else 0) or 0
        out_.append(Edge(vs[i], vs[(i + 1) % n], b))
    return out_


def integral(f, edges):
    """∑ ∫₀¹ f(edge, t) dt over the edges, at 50 digits."""
    return mp.fsum(mp.quad(lambda t, e=e: f(e, t), [0, 1]) for e in edges)


def area_moments(edges, o):
    """Signed area and first moments about o by Green's theorem along the edges."""
    def a(e, t):
        x, y = e.at(t)
        dx, dy = e.deriv(t)
        return ((x - o[0]) * dy - (y - o[1]) * dx) / 2

    def mx(e, t):
        x, _ = e.at(t)
        _, dy = e.deriv(t)
        return (x - o[0]) ** 2 * dy / 2

    def my(e, t):
        _, y = e.at(t)
        dx, _ = e.deriv(t)
        return -((y - o[1]) ** 2) * dx / 2

    return integral(a, edges), integral(mx, edges), integral(my, edges)


def path_moments(edges, o):
    """Length and first moments of the edges as curves about o."""
    def ds(e, t):
        dx, dy = e.deriv(t)
        return mp.hypot(dx, dy)

    length = mp.fsum(e.length for e in edges)
    mx = integral(lambda e, t: (e.at(t)[0] - o[0]) * ds(e, t), edges)
    my = integral(lambda e, t: (e.at(t)[1] - o[1]) * ds(e, t), edges)
    return length, mx, my


def walk(edges, s):
    """The point s along the edges (from the first edge's start), its edge and fraction."""
    acc = mp.mpf(0)
    for i, e in enumerate(edges):
        if s <= acc + e.length or i == len(edges) - 1:
            t = (s - acc) / e.length if e.length else mp.mpf(0)
            t = min(max(t, mp.mpf(0)), mp.mpf(1))
            return e.at(t), i, t
        acc += e.length
    raise ValueError("boş yol")


# ── An object's rings and paths ────────────────────────────────────────────


def parts_of(e):
    """A polygon's parts as (outer pts, outer bulges, holes [(pts, bulges)]), itself first."""
    first = (e["pts"], e.get("bulges"), [(h["pts"], h.get("bulges")) for h in e.get("holes", [])])
    rest = [(p["pts"], p.get("bulges"), [(h["pts"], h.get("bulges")) for h in p.get("holes", [])]) for p in e.get("parts", [])]
    return [first, *rest]


def path_edges(e):
    """A path object's edges, all its parts' (a line's one, an arc's one)."""
    k = e["kind"]
    if k == "line":
        return [Edge(mpv(e["a"]), mpv(e["b"]))]
    if k == "arc":
        return [arc_edge(e)]
    if k == "polyline":
        out_ = ring_edges(e["pts"], e.get("bulges"), False)
        for p in e.get("parts", []):
            out_ += ring_edges(p["pts"], p.get("bulges"), False)
        return out_
    raise ValueError(k)


def arc_edge(e):
    """An arc object (counter-clockwise from a0 to a1) as a bulged edge."""
    c, r = mpv(e["c"]), mp.mpf(e["r"])
    a0, a1 = mp.mpf(e["a0"]), mp.mpf(e["a1"])
    sweep = (a1 - a0) % (2 * PI)
    if sweep <= 0:
        sweep += 2 * PI
    a = (c[0] + r * mp.cos(a0), c[1] + r * mp.sin(a0))
    b = (c[0] + r * mp.cos(a1), c[1] + r * mp.sin(a1))
    edge = Edge.__new__(Edge)
    edge.a, edge.b, edge.bulge = a, b, mp.tan(sweep / 4)
    edge.c, edge.r, edge.a0, edge.sweep = c, r, a0, sweep
    edge.length = r * sweep
    return edge


# ── Nesneden nokta (§2) ───────────────────────────────────────────────────


def area_centroid(e):
    """An area's centroid (holes out, parts by their areas); None for no area."""
    o = mpv(e["pts"][0])
    total, sx, sy = mp.mpf(0), mp.mpf(0), mp.mpf(0)
    for outer, ob, holes in parts_of(e):
        a, mx, my = area_moments(ring_edges(outer, ob, True), o)
        sign = 1 if a >= 0 else -1
        a, mx, my = a * sign, mx * sign, my * sign
        for hp, hb in holes:
            ha, hmx, hmy = area_moments(ring_edges(hp, hb, True), o)
            hs = 1 if ha >= 0 else -1
            a, mx, my = a - ha * hs, mx - hmx * hs, my - hmy * hs
        total, sx, sy = total + a, sx + mx, sy + my
    if abs(total) < mp.mpf(10) ** -30:
        return None
    return (o[0] + sx / total, o[1] + sy / total)


def boundary_edges(e):
    out_ = []
    for outer, ob, holes in parts_of(e):
        out_ += ring_edges(outer, ob, True)
        for hp, hb in holes:
            out_ += ring_edges(hp, hb, True)
    return out_


def edges_centroid(edges):
    o = edges[0].a
    length, mx, my = path_moments(edges, o)
    return (o[0] + mx / length, o[1] + my / length) if length > 0 else None


def centroid(e):
    k = e["kind"]
    if k == "point":
        pts = [mpv(e["p"])] + [mpv(q["p"]) for q in e.get("parts", [])]
        return (mp.fsum(p[0] for p in pts) / len(pts), mp.fsum(p[1] for p in pts) / len(pts))
    if k == "circle":
        return mpv(e["c"])
    if k == "polygon":
        return area_centroid(e) or edges_centroid(boundary_edges(e))
    return edges_centroid(path_edges(e))


def scan_interior(rings):
    """The ADR's scan line on one part (rings: lists of edges): its widest interval inside, (width, point) or None."""
    ys = []
    for ring in rings:
        for ed in ring:
            ys.append(ed.a[1])
            if ed.bulge:
                for t in (PI / 2, -PI / 2):
                    d = ((t - ed.a0) if ed.sweep >= 0 else (ed.a0 - t)) % (2 * PI)
                    if d <= abs(ed.sweep):
                        ys.append(ed.c[1] + ed.r * mp.sin(t))
    lo, hi = min(ys), max(ys)
    if not lo < hi:
        return None
    centre = (lo + hi) / 2
    below, above = lo, hi
    for y in ys:
        if y <= centre:
            below = max(below, y)
        elif y < above:
            above = y
    scan = (below + above) / 2
    xs = []
    for ring in rings:
        for ed in ring:
            if ed.bulge:
                dy = scan - ed.c[1]
                if abs(dy) < ed.r:
                    dx = mp.sqrt(ed.r**2 - dy**2)
                    for x in (ed.c[0] - dx, ed.c[0] + dx):
                        t = mp.atan2(dy, x - ed.c[0])
                        d = ((t - ed.a0) if ed.sweep >= 0 else (ed.a0 - t)) % (2 * PI)
                        if d <= abs(ed.sweep):
                            xs.append(x)
            else:
                (x0, y0), (x1, y1) = ed.a, ed.b
                if (y0 > scan and y1 > scan) or (y0 < scan and y1 < scan) or y0 == y1:
                    continue
                xs.append(x0 + (scan - y0) * (x1 - x0) / (y1 - y0))
    xs.sort()
    best = None
    for i in range(0, len(xs) - 1, 2):
        w = xs[i + 1] - xs[i]
        if w > 0 and (best is None or w > best[0]):
            best = (w, ((xs[i] + xs[i + 1]) / 2, scan))
    return best


def straight(e):
    return not any(b for _, ob, holes in parts_of(e) for b in [*(ob or [])] + [x for _, hb in holes for x in (hb or [])])


def interior_point(e):
    k = e["kind"]
    if k == "point":
        mean = centroid(e)
        pts = [mpv(e["p"])] + [mpv(q["p"]) for q in e.get("parts", [])]
        return min(pts, key=lambda p: (mp.hypot(p[0] - mean[0], p[1] - mean[1]), pts.index(p)))
    if k == "circle":
        return mpv(e["c"])
    if k == "polygon":
        if straight(e) and area_centroid(e) is not None:
            # GEOS itself: shapely's representative point of the (multi)polygon.
            polys = [shapely.Polygon([(p["x"], p["y"]) for p in outer], [[(p["x"], p["y"]) for p in hp] for hp, _ in holes])
                     for outer, _, holes in parts_of(e)]
            geom = polys[0] if len(polys) == 1 else shapely.MultiPolygon(polys)
            q = geom.representative_point()
            return (mp.mpf(q.x), mp.mpf(q.y))
        best = None
        for outer, ob, holes in parts_of(e):
            got = scan_interior([ring_edges(outer, ob, True)] + [ring_edges(hp, hb, True) for hp, hb in holes])
            if got and (best is None or got[0] > best[0]):
                best = got
        if best:
            return best[1]
        edges = ring_edges(e["pts"], e.get("bulges"), True)
    else:
        edges = path_edges(first_part(e))
    total = mp.fsum(ed.length for ed in edges)
    return walk(edges, total / 2)[0]


def first_part(e):
    """A path object without its other parts."""
    return {k: v for k, v in e.items() if k != "parts"}


# ── Köşeleri noktaya çevir (§3) ────────────────────────────────────────────


def paths(e):
    """An object's paths in its elevations' order: (pts, bulges, closed, zs, part, ring)."""
    k = e["kind"]
    if k == "line":
        return [([e["a"], e["b"]], None, False, [e.get("za"), e.get("zb")], 1, 0)]
    if k == "arc":
        ed = arc_edge(e)
        return [([out(ed.a), out(ed.b)], [float(ed.bulge), 0], False, [None, None], 1, 0)]
    if k == "polyline":
        res = [(e["pts"], e.get("bulges"), False, e.get("zs"), 1, 0)]
        res += [(p["pts"], p.get("bulges"), False, p.get("zs"), i + 2, 0) for i, p in enumerate(e.get("parts", []))]
        return res
    if k == "polygon":
        res = [(e["pts"], e.get("bulges"), True, e.get("zs"), 1, 0)]
        res += [(h["pts"], h.get("bulges"), True, h.get("zs"), 1, j + 1) for j, h in enumerate(e.get("holes", []))]
        for i, p in enumerate(e.get("parts", [])):
            res.append((p["pts"], p.get("bulges"), True, p.get("zs"), i + 2, 0))
            res += [(h["pts"], h.get("bulges"), True, h.get("zs"), i + 2, j + 1) for j, h in enumerate(p.get("holes", []))]
        return res
    return []


def z_at(zs, i):
    return zs[i] if zs and i < len(zs) else None


def vertex_points(e, which):
    """(point, z, index or None, part, ring, distance) of the vertices `which` takes."""
    res = []
    index = 0
    for k, (pts, bulges, closed, zs, part, ring) in enumerate(paths(e)):
        edges = ring_edges(pts, bulges, closed) if len(pts) > 1 else []
        at = [mp.mpf(0)]
        for ed in edges[: len(pts) - 1]:
            at.append(at[-1] + ed.length)
        n = len(pts)
        if which == "all":
            for i in range(n):
                index += 1
                res.append((mpv(pts[i]), z_at(zs, i), index, part, ring, at[i]))
        elif k == 0:
            if which in ("start", "ends"):
                res.append((mpv(pts[0]), z_at(zs, 0), 1, part, ring, at[0]))
            if which == "end" or (which == "ends" and not closed):
                i = 0 if closed else n - 1
                res.append((mpv(pts[i]), z_at(zs, i), i + 1, part, ring, at[i]))
            if which == "middle":
                total = mp.fsum(ed.length for ed in edges)
                p, i, t = walk(edges, total / 2)
                za, zb = z_at(zs, i), z_at(zs, (i + 1) % n)
                z = za + (zb - za) * t if za is not None and zb is not None else None
                res.append((p, z, None, part, ring, total / 2))
    return res


# ── Noktalardan çizgi (§4) ────────────────────────────────────────────────


def point_list(e):
    """A point object's points with their elevations, its own first."""
    return [(e["p"], e.get("z"))] + [(q["p"], q.get("z")) for q in e.get("parts", [])]


def trim(s):
    return s.strip(" \t\n\r\f\v   ﻿") if isinstance(s, str) else s


def points_to_paths(inputs, order_field, group_field, close):
    """(group value or None, [(p, z, order value)]) of each line, and the points dropped as repeats."""
    # The order expression is a field here: its value is the attribute as written, none without it.
    values = [e["attrs"].get(order_field) if order_field else None for e in inputs]
    at = order(values, False, False) if order_field else list(range(len(inputs)))
    groups, seen = [], {}
    for e in inputs:
        g = (trim(e["attrs"].get(group_field, "")) or "(boş)") if group_field else None
        if g not in seen:
            seen[g] = len(groups)
            groups.append((g, []))
    for i in at:
        e = inputs[i]
        g = (trim(e["attrs"].get(group_field, "")) or "(boş)") if group_field else None
        for p, z in point_list(e):
            groups[seen[g]][1].append((p, z, values[i]))
    repeats = 0
    lines = []
    for g, pts in groups:
        kept = []
        for q in pts:
            if kept and kept[-1][0] == q[0]:
                repeats += 1
                continue
            kept.append(q)
        lines.append((g, kept))
    return lines, repeats


# ── Tablolar (§5–§7) ─────────────────────────────────────────────────────


def read_number(cell):
    """A cell as a number (ADR 0180's rule): trimmed, an optional sign, digits and at most one decimal point or comma;
    None otherwise."""
    t = trim(cell or "")
    body = t[1:] if t[:1] in ("+", "-") else t
    if not any(c.isdigit() for c in body) or any(c not in "0123456789.," for c in body) or sum(c in ".," for c in body) > 1:
        return None
    return mp.mpf((t[1:] if t.startswith("+") else t).replace(",", "."))


def radians(value, unit):
    return value * PI / (180 if unit == "deg" else 200)


def row_numbers(row, columns, optional=()):
    """The row's cells in `columns` as numbers, or the fault: ('empty', k), ('number', k)."""
    vals = []
    for k, col in enumerate(columns):
        cell = row.get(col, "")
        if not trim(cell):
            if k in optional:
                vals.append(None)
                continue
            return None, ("empty", k)
        v = read_number(cell)
        if v is None:
            return None, ("number", k)
        vals.append(v)
    return vals, None


def bearing_row(row, cols, unit):
    vals, fault = row_numbers(row, cols)
    if fault:
        return None, fault
    e, n, b, l = vals
    if not l > 0:
        return None, ("positive", 3)
    t = radians(b, unit)
    return {"kind": "line", "a": out((e, n)), "b": out((e + l * mp.sin(t), n + l * mp.cos(t)))}, None


def xy_row(row, cols):
    vals, fault = row_numbers(row, cols)
    if fault:
        return None, fault
    x0, y0, x1, y1 = vals
    if (x0, y0) == (x1, y1):
        return None, ("same", None)
    return {"kind": "line", "a": out((x0, y0)), "b": out((x1, y1))}, None


def ellipse_row(row, cols, unit):
    vals, fault = row_numbers(row, cols, optional=(4,))
    if fault:
        return None, fault
    e, n, a, b, t = vals
    if not a > 0:
        return None, ("positive", 2)
    if not b > 0:
        return None, ("positive", 3)
    t = radians(t or 0, unit)
    if b > a:
        t += PI / 2
        a, b = b, a
    return {"kind": "ellipse", "c": out((e, n)), "major": out((a * mp.sin(t), a * mp.cos(t))), "ratio": float(b / a), "t0": 0, "t1": 0}, None


def fault_text(fault, cols, row):
    """The report's reason (ADR §5): an empty cell, a cell that is no number, one not more than zero, ends in one place."""
    why, k = fault
    if why == "same":
        return "Başlangıç ve bitiş aynı yerde"
    col = cols[k]
    if why == "empty":
        return f"“{col}” boş"
    if why == "number":
        return f"“{col}” sayı değil: {trim(row.get(col, ''))}"
    return f"“{col}” sıfırdan büyük değil"


# ── Çizginin parçası (§8) ─────────────────────────────────────────────────


def substring(e, s0, s1):
    """The piece of a single path from s0 to s1 (taken into [0, length]) as the drawing writes it, or None."""
    if e["kind"] not in ("line", "polyline", "arc") or e.get("parts"):
        return None
    if e["kind"] == "arc":
        ed = arc_edge(e)
        length = ed.length
        s0, s1 = min(max(s0, 0), length), min(max(s1, 0), length)
        if not s0 < s1:
            return None
        a0 = (ed.a0 + s0 / ed.r) % (2 * PI)
        a1 = (ed.a0 + s1 / ed.r) % (2 * PI)
        return {"kind": "arc", "c": e["c"], "r": e["r"], "a0": float(a0), "a1": float(a1)}
    pts = [e["a"], e["b"]] if e["kind"] == "line" else e["pts"]
    zs = [e.get("za"), e.get("zb")] if e["kind"] == "line" else e.get("zs")
    edges = ring_edges(pts, e.get("bulges"), False)
    length = mp.fsum(ed.length for ed in edges)
    s0, s1 = min(max(s0, 0), length), min(max(s1, 0), length)
    if not s0 < s1:
        return None
    vs, bulges, z = [], [], []
    acc = mp.mpf(0)
    for i, ed in enumerate(edges):
        lo, hi = max(s0, acc), min(s1, acc + ed.length)
        if hi - lo > mp.mpf(10) ** -9 * max(1, length):
            t0, t1 = (lo - acc) / ed.length, (hi - acc) / ed.length
            if not vs:
                vs.append(ed.at(t0))
                z.append(between(z_at(zs, i), z_at(zs, i + 1), t0))
            vs.append(ed.at(t1))
            z.append(between(z_at(zs, i), z_at(zs, i + 1), t1))
            bulges.append(mp.tan(ed.sweep * (t1 - t0) / 4) if ed.bulge else 0)
        acc += ed.length
    bulges.append(0)
    if e["kind"] == "line" or (len(vs) == 2 and bulges[0] == 0):
        res = {"kind": "line", "a": out(vs[0]), "b": out(vs[-1])}
        if z[0] is not None:
            res["za"] = float(z[0])
        if z[-1] is not None:
            res["zb"] = float(z[-1])
        return res
    res = {"kind": "polyline", "pts": [out(v) for v in vs]}
    if any(b != 0 for b in bulges):
        res["bulges"] = [float(b) for b in bulges]
    if any(x is not None for x in z):
        res["zs"] = [None if x is None else float(x) for x in z]
    return res


def between(za, zb, t):
    """An elevation a fraction t along an edge (ADR 0142): on a vertex its own, inside the edge linearly."""
    if t == 0:
        return za
    if t == 1:
        return zb
    return None if za is None or zb is None else mp.mpf(za) + (mp.mpf(zb) - mp.mpf(za)) * t


# ── Alanları çizgiye çevir, çizgileri alana çevir (§9, §10) ─────────────────


def rings_as_lines(e):
    """(polyline, part, ring) for each ring: its last vertex its first, with its arcs and elevations."""
    res = []
    for pts, bulges, _closed, zs, part, ring in paths(e):
        line = {"kind": "polyline", "pts": [*pts, pts[0]]}
        if bulges and any(bulges):
            line["bulges"] = [*(bulges[i] if i < len(bulges) else 0 for i in range(len(pts))), 0]
        if zs and any(x is not None for x in zs):
            line["zs"] = [*(z_at(zs, i) for i in range(len(pts))), z_at(zs, 0)]
        res.append((line, part, ring))
    return res


def ring_area(pts, bulges):
    a, _, _ = area_moments(ring_edges(pts, bulges, True), mpv(pts[0]))
    return abs(a)


def line_area(e, close):
    """The area a polyline gives (ADR §10), or the reason it gives none: 'open', 'empty', 'parts'."""
    if e.get("parts"):
        return "parts"
    pts, bulges, zs = e["pts"], e.get("bulges"), e.get("zs")
    n = len(pts)
    if n < 2:
        return "empty"
    meets = mp.hypot(mp.mpf(pts[0]["x"]) - pts[-1]["x"], mp.mpf(pts[0]["y"]) - pts[-1]["y"]) <= mp.mpf("1e-6")
    if not meets and not close:
        return "open"
    m = n - 1 if meets else n
    ring_b = [(bulges[i] if bulges and i < len(bulges) else 0) if (i + 1 < n or meets) else 0 for i in range(m)]
    if ring_area(pts[:m], ring_b) <= 0:
        return "empty"
    area = {"kind": "polygon", "pts": pts[:m]}
    if any(ring_b):
        area["bulges"] = ring_b
    if zs and any(z_at(zs, i) is not None for i in range(m)):
        area["zs"] = [z_at(zs, i) for i in range(m)]
    return area


# ── Köşe ekle (§11) ──────────────────────────────────────────────────────


def divided(pts, bulges, closed, zs, by):
    """A path with each edge divided (`by`: ('interval', d) or ('count', k)): points, bulges, elevations, added."""
    edges = ring_edges(pts, bulges, closed)
    n = len(pts)
    vs, bs, z, added = [], [], [], 0
    for i, ed in enumerate(edges):
        if by[0] == "interval":
            parts = int(mp.ceil(ed.length / by[1])) if ed.length > by[1] else 1
        else:
            parts = by[1] + 1
        added += parts - 1
        sub = mp.tan(ed.sweep / parts / 4) if ed.bulge else 0
        for k in range(parts):
            t = mp.mpf(k) / parts
            vs.append(mpv(pts[i]) if k == 0 else ed.at(t))
            bs.append(sub)
            z.append(z_at(zs, i) if k == 0 else between(z_at(zs, i), z_at(zs, (i + 1) % n), t))
    if not closed:
        vs.append(mpv(pts[-1]))
        bs.append(0)
        z.append(z_at(zs, n - 1))
    return vs, bs, z, added


# ── The drawing ──────────────────────────────────────────────────────────


def layer(i, name, color, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25}, "children": []}


LAYERS = [
    layer("cizim", "Çizim", "fg"),
    layer("parsel", "Parsel", "#E5484D"),
    layer("yol", "Yol", "#30A46C"),
    layer("nokta", "Nokta", "#0090FF"),
    layer("olcu", "Ölçüler", "#AB4ABA"),
    layer("kilitli", "Arşiv", "#8E8C99", locked=True),
    layer("tablo", "Tablo noktaları", "#FFB224"),
]


def ent(i, kind, lay, attrs, **geom):
    e = {"kind": kind, "id": i, "layerId": lay, "attrs": attrs}
    e.update(geom)
    return e


QUARTER = float(mp.tan(PI / 8))

ENTITIES = [
    # Parcels: a square; a thin L whose centroid is outside it; one with a hole and elevations; one with an arc on top;
    # a two-part one; a flat one with no area.
    ent(1, "polygon", "parsel", {"Parsel": "1", "Ada": "101"}, pts=rect(0, 0, 20, 20)),
    ent(2, "polygon", "parsel", {"Parsel": "2", "Ada": "101"},
        pts=[P(20, 0), P(50, 0), P(50, 4), P(24, 4), P(24, 30), P(20, 30)]),
    ent(3, "polygon", "parsel", {"Parsel": "3", "Ada": "102"}, pts=rect(60, 0, 90, 30), zs=[10, 11, 12, 13],
        holes=[{"pts": rect(70, 10, 80, 20)}]),
    ent(4, "polygon", "parsel", {"Parsel": "4", "Ada": "102"}, pts=rect(100, 0, 120, 20), bulges=[0, 0, 0.5, 0]),
    ent(5, "polygon", "parsel", {"Parsel": "5", "Ada": "103"}, pts=rect(130, 0, 140, 10), parts=[{"pts": rect(150, 0, 170, 12)}]),
    ent(6, "polygon", "parsel", {"Parsel": "6", "Ada": "103"}, pts=[P(0, 40), P(10, 40), P(30, 40)]),
    # Roads: a polyline with elevations, one with a quarter arc, a line with elevations, an arc, a two-part polyline,
    # a closed polyline, a two-vertex one.
    ent(11, "polyline", "yol", {"Ad": "Atatürk Cd."}, pts=[P(0, 60), P(30, 60), P(30, 80)], zs=[100, 101.5, 103]),
    ent(12, "polyline", "yol", {"Ad": "Okul Sk."}, pts=[P(40, 60), P(60, 60), P(80, 80)], bulges=[0, QUARTER, 0]),
    ent(13, "line", "yol", {"Ad": "Hat"}, a=P(90, 60), b=P(110, 70), za=5, zb=7),
    ent(14, "arc", "yol", {"Ad": "Kavis"}, c=P(130, 60), r=10, a0=0, a1=float(PI / 2)),
    ent(15, "polyline", "yol", {"Ad": "Çift"}, pts=[P(0, 90), P(20, 90)], parts=[{"pts": [P(0, 95), P(20, 95)]}]),
    ent(16, "polyline", "yol", {"Ad": "Kapalı"}, pts=[P(40, 90), P(60, 90), P(60, 100), P(40, 100), P(40, 90)]),
    ent(17, "polyline", "yol", {"Ad": "Kısa"}, pts=[P(70, 90), P(80, 90)]),
    # Points of two routes A and B ordered by Sıra, a repeat in B, a lone C, a three-point D.
    ent(21, "point", "nokta", {"Hat": "A", "Sıra": "3"}, p=P(0, 120), z=10),
    ent(22, "point", "nokta", {"Hat": "A", "Sıra": "1"}, p=P(10, 121), z=11),
    ent(23, "point", "nokta", {"Hat": "B", "Sıra": "2"}, p=P(20, 119)),
    ent(24, "point", "nokta", {"Hat": "A", "Sıra": "2"}, p=P(5, 125), z=12),
    ent(25, "point", "nokta", {"Hat": "B", "Sıra": "1"}, p=P(30, 130)),
    ent(26, "point", "nokta", {"Hat": "B", "Sıra": "1"}, p=P(30, 130)),
    ent(27, "point", "nokta", {"Hat": "C", "Sıra": "1"}, p=P(40, 120)),
    ent(28, "point", "nokta", {"Hat": "D", "Sıra": "1"}, p=P(60, 120), parts=[{"p": P(62, 121)}, {"p": P(64, 119)}]),
    # Measured bearings on points (their attributes as rows; the angles in degrees).
    ent(31, "point", "olcu", {"Ad": "Ö1", "Doğu": "487200", "Kuzey": "4420100", "Semt": "45", "Uzunluk": "20"}, p=P(200, 100)),
    ent(32, "point", "olcu", {"Ad": "Ö2", "Doğu": "487200", "Kuzey": "4420100", "Semt": "270,5", "Uzunluk": "12,25"}, p=P(200, 100)),
    ent(33, "point", "olcu", {"Ad": "Ö3", "Doğu": "487200", "Kuzey": "4420100", "Semt": "kuzey", "Uzunluk": "5"}, p=P(200, 100)),
    # Rows on objects (docs/adr/0741): two ends and an error ellipse each, the angles in degrees; one row's ends meet.
    ent(51, "point", "tablo", {"Y1": "487300", "X1": "4420100", "Y2": "487320,5", "X2": "4420130", "Y": "487300", "X": "4420100",
                               "a": "3", "b": "1,5", "Semt": "30"}, p=P(300, 100)),
    ent(52, "point", "tablo", {"Y1": "487310", "X1": "4420110", "Y2": "487310", "X2": "4420110", "Y": "487310", "X": "4420110",
                               "a": "1", "b": "2", "Semt": "120,5"}, p=P(310, 110)),
    ent(53, "point", "tablo", {"Y1": "487320", "X1": "x", "Y2": "487330", "X2": "4420140", "Y": "487320", "X": "4420120",
                               "a": "2,5", "b": "2,5"}, p=P(320, 120)),
    # A locked layer's parcel: read, not written.
    ent(41, "polygon", "kilitli", {"Parsel": "41"}, pts=rect(0, 150, 10, 160)),
]

DOCUMENT = {
    "format": "kentos.document",
    "version": 1,
    "name": "İşlem durumları: geometri dönüştürme",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                 "workspace": "gis", "drawingFont": "barlow"},
    "origin": {"x": E, "y": N},
    "layers": LAYERS,
    "activeLayer": "cizim",
    "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}

# The tables the table tools read: a header, then the rows (the drawing's numbers, a decimal comma, bad cells).
SEMT = "Ad;Doğu;Kuzey;Semt;Uzunluk\nS1;487000;4420200;0;100\nS2;487000;4420200;100;50\nS3;487010,5;4420210;250,5;30\nS4;abc;4420200;50;10\nS5;487000;4420200;50;0\nS6;487000;4420200;;10\n"
XY = "Ad,Y1,X1,Y2,X2\nA,487000,4420300,487050,4420300\nB,487000,4420300,487000,4420300\nC,487010.25,4420310.5,487020.75,4420330.125\n"
ELIPS = "No;Y;X;a;b;Semt\nP1;487000;4420400;0,05;0,02;35,2\nP2;487100;4420400;3;5;0\nP3;487200;4420400;4;4;\nP4;487300;4420400;-1;2;0\n"


# ── The cases ────────────────────────────────────────────────────────────

OUTPUT_COLOR = {
    "convert.toPoints": ("Merkezler", "#E54666"),
    "convert.vertices": ("Köşeler", "#0090FF"),
    "convert.pointsToPath": ("Yollar", "#30A46C"),
    "convert.bearingLine": ("Semt çizgileri", "#AB4ABA"),
    "convert.xyLine": ("Çizgiler", "#12A594"),
    "convert.tableEllipse": ("Elipsler", "#F76B15"),
    "convert.substring": ("Parçalar", "#3E63DD"),
    "convert.polygonsToLines": ("Sınırlar", "#AD7F58"),
    "convert.linesToPolygons": ("Alanlar", "#8E4EC6"),
    "convert.densify": ("Sıklaştırılan", "#E5484D"),
}
UNDO = {
    "convert.toPoints": "Nesneden nokta",
    "convert.vertices": "Köşeleri noktaya çevir",
    "convert.pointsToPath": "Noktalardan çizgi",
    "convert.bearingLine": "Semt ve uzunluktan çizgi",
    "convert.xyLine": "XY'den çizgi",
    "convert.tableEllipse": "Tablodan elips",
    "convert.substring": "Çizginin parçası",
    "convert.polygonsToLines": "Alanları çizgiye çevir",
    "convert.linesToPolygons": "Çizgileri alana çevir",
    "convert.densify": "Köşe ekle",
}


def by_id(i):
    return next(e for e in ENTITIES if e["id"] == i)


def objects(lay):
    return [e for e in ENTITIES if e["layerId"] == lay]


def out_layer(tool):
    """The tool's new output layer: its id by the runner's rule (layer_ops_cases.py's `new_id`), its look (the
    geometry tools' output style: the colour, a 0.25 mm line, the colour faint inside)."""
    name, color = OUTPUT_COLOR[tool]
    return {"id": new_id(name, set()), "name": name,
            "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25, "fill": f"{color}26"}}


def new_object(kind_geom, lay, attrs):
    o = {"kind": kind_geom["kind"], "layerId": lay, "attrs": attrs}
    o.update({k: v for k, v in kind_geom.items() if k != "kind"})
    return o


def case(cid, title, tool, values, expect, selection=None, document="convert.kcad"):
    c = {"id": cid, "title": title, "document": document, "run": {"tool": tool}, "values": values, "expect": expect}
    if selection is not None:
        c["selection"] = selection
    return c


def L(i):
    return {"scope": "layer", "layerId": i}


def ok(tool, added, summary, log=None, outputs=None):
    """A run's expectation: a new output layer and one undo step when it writes; nothing changed when not."""
    e = {"status": "ok", "summary": summary, "undo": UNDO[tool] if added else None}
    if added:
        e["layers"] = [out_layer(tool)]
        e["added"] = added
    if log:
        e["log"] = log
    if outputs is not None:
        e["outputs"] = outputs
    return e


def point_obj(p, lay, attrs, z=None):
    o = {"kind": "point", "layerId": lay, "attrs": attrs, "p": out(p)}
    if z is not None:
        o["z"] = z
    return o


def readable(inputs):
    """A tool's input: its objects in the drawing's order (none is written, so a locked layer's are read too)."""
    return inputs


# ── Nesneden nokta ──


def piece_list(e):
    """An object's parts as objects of their own (a polygon's with their holes, a polyline's, a point's)."""
    if e["kind"] == "point":
        return [{"kind": "point", "p": e["p"]}] + [{"kind": "point", "p": q["p"]} for q in e.get("parts", [])]
    if e["kind"] in ("polygon", "polyline") and e.get("parts"):
        first = {k: v for k, v in e.items() if k != "parts"}
        return [first] + [{"kind": e["kind"], **p} for p in e["parts"]]
    return [e]


def has_elevation(e):
    """Whether an object has an elevation: a point's, a line's ends', a path's or an area's vertices'."""
    k = e["kind"]
    if k == "point":
        return e.get("z") is not None or any(q.get("z") is not None for q in e.get("parts", []))
    if k == "line":
        return e.get("za") is not None or e.get("zb") is not None
    rings_ = [e, *e.get("holes", []), *e.get("parts", []), *[h for p in e.get("parts", []) for h in p.get("holes", [])]]
    return any(z is not None for r in rings_ for z in (r.get("zs") or []))


def to_points(tool, inputs, inside, parts):
    lay = out_layer(tool)["id"]
    heights = sum(1 for e in inputs if has_elevation(e))
    log = [{"level": "warn", "text": f"{heights} nesnenin kotları sonuca taşınmadı."}] if heights else None
    added = []
    for e in inputs:
        if parts:
            for k, piece in enumerate(piece_list(e), 1):
                added.append(point_obj(interior_point(piece) if inside else centroid(piece), lay, {**e["attrs"], "Parça": str(k)}))
        else:
            added.append(point_obj(interior_point(e) if inside else centroid(e), lay, dict(e["attrs"])))
    return ok(tool, added, f"{len(inputs)} nesneden {len(added)} nokta yazıldı.", log=log, outputs={"count": len(added)})


# ── Köşeleri noktaya çevir ──


def vertices(tool, inputs, which):
    lay = out_layer(tool)["id"]
    added = []
    for e in inputs:
        for p, z, index, part, ring, dist in vertex_points(e, which):
            attrs = dict(e["attrs"])
            if index is not None:
                attrs["Köşe"] = str(index)
            attrs["Parça"] = str(part)
            attrs["Halka"] = str(ring)
            attrs["Uzaklık"] = shown(float(dist), 3)
            added.append(point_obj(p, lay, attrs, None if z is None else float(z)))
    return ok(tool, added, f"{len(inputs)} nesneden {len(added)} nokta yazıldı.", outputs={"count": len(added)})


# ── Noktalardan çizgi ──


def to_paths(tool, inputs, order_field, group_field, close):
    lay = out_layer(tool)["id"]
    lines, repeats = points_to_paths(inputs, order_field, group_field, close)
    need = 3 if close else 2
    added, short = [], 0
    for g, pts in lines:
        if len(pts) < need:
            short += 1
            continue
        attrs = {}
        if group_field:
            attrs[group_field] = g
        attrs["Nokta sayısı"] = str(len(pts))
        if order_field:
            attrs["Başlangıç"] = pts[0][2] if pts[0][2] is not None else ""
            attrs["Bitiş"] = pts[-1][2] if pts[-1][2] is not None else ""
        o = {"kind": "polygon" if close else "polyline", "layerId": lay, "attrs": attrs, "pts": [q[0] for q in pts]}
        if any(q[1] is not None for q in pts):
            o["zs"] = [q[1] for q in pts]
        added.append(o)
    log = []
    if repeats:
        log.append({"level": "warn", "text": f"{repeats} nokta bir öncekiyle aynı yerde olduğu için atlandı."})
    if short:
        log.append({"level": "warn", "text": f"{short} grupta çizgi için yeterli nokta yok (en az {need})."})
    points = sum(len(point_list(e)) for e in inputs)
    return ok(tool, added, f"{points} noktadan {len(added)} çizgi yazıldı.", log=log, outputs={"count": len(added)})


# ── Tablolar ──


def file_rows(name):
    """A table file's header and rows as Tablo ekle's reader reads it (table_file_cases.py)."""
    rows = read_text((PROC_DIR / name).read_bytes())["sheets"][0]["rows"]
    header = rows[0]
    return header, [dict(zip(header, r)) for r in rows[1:]]


def layer_rows(lay):
    return [e["attrs"] for e in objects(lay)], [f"#{e['id']}" for e in objects(lay)]


def from_rows(tool, rows, labels, make, cols, what):
    lay = out_layer(tool)["id"]
    added, report = [], []
    for row, label in zip(rows, labels):
        shape, fault = make(row)
        if fault:
            report.append([label, fault_text(fault, cols, row)])
            continue
        attrs = {k: v for k, v in row.items() if trim(v)}
        added.append({**shape, "layerId": lay, "attrs": attrs})
    log = [{"level": "warn", "text": f"{len(report)} satır atlandı; nedenleri tabloda."}] if report else None
    outputs = {"count": len(added)}
    if report:
        outputs["table"] = {"columns": ["Satır", "Neden"], "rows": report}
    return ok(tool, [{"kind": a["kind"], "layerId": a["layerId"], "attrs": a["attrs"], **{k: v for k, v in a.items() if k not in ("kind", "layerId", "attrs")}} for a in added],
              f"{len(rows)} satırdan {len(added)} {what} yazıldı.", log=log, outputs=outputs)


def bearing_file(tool, name, cols, unit):
    _, rows = file_rows(name)
    return from_rows(tool, rows, [str(i + 2) for i in range(len(rows))], lambda r: bearing_row(r, cols, unit), cols, "çizgi")


def bearing_layer(tool, lay, cols, unit):
    rows, labels = layer_rows(lay)
    return from_rows(tool, rows, labels, lambda r: bearing_row(r, cols, unit), cols, "çizgi")


def xy_file(tool, name, cols):
    _, rows = file_rows(name)
    return from_rows(tool, rows, [str(i + 2) for i in range(len(rows))], lambda r: xy_row(r, cols), cols, "çizgi")


def ellipse_file(tool, name, cols, unit):
    _, rows = file_rows(name)
    return from_rows(tool, rows, [str(i + 2) for i in range(len(rows))], lambda r: ellipse_row(r, cols, unit), cols, "elips")


# ── Çizginin parçası ──


def pieces(tool, inputs, starts, ends):
    """Each object's piece between its two distances (numbers here; an expression's values in the tool)."""
    lay = out_layer(tool)["id"]
    added, multi, empty = [], 0, 0
    for e, s0, s1 in zip(inputs, starts, ends):
        if e.get("parts"):
            multi += 1
            continue
        piece = substring(e, mp.mpf(s0), mp.mpf(s1))
        if piece is None:
            empty += 1
            continue
        added.append({**piece, "layerId": lay, "attrs": dict(e["attrs"])})
    log = []
    if multi:
        log.append({"level": "warn", "text": f"{multi} çok parçalı çizgi alınmadı."})
    if empty:
        log.append({"level": "warn", "text": f"{empty} nesnede başlangıç bitişten küçük değil; yazılmadı."})
    return ok(tool, added, f"{len(inputs)} nesneden {len(added)} parça yazıldı.", log=log, outputs={"count": len(added)})


def path_length(e):
    if e["kind"] == "arc":
        return arc_edge(e).length
    return mp.fsum(ed.length for ed in path_edges(e))


# ── Alanlar ve çizgiler ──


def rings(tool, inputs):
    lay = out_layer(tool)["id"]
    added = []
    for e in inputs:
        for line, part, ring in rings_as_lines(e):
            added.append({**line, "layerId": lay, "attrs": {**e["attrs"], "Parça": str(part), "Halka": str(ring)}})
    return ok(tool, added, f"{len(inputs)} alandan {len(added)} çizgi yazıldı.", outputs={"count": len(added)})


def areas(tool, inputs, close):
    lay = out_layer(tool)["id"]
    added, why = [], {"open": 0, "empty": 0, "parts": 0}
    for e in inputs:
        got = line_area(e, close)
        if isinstance(got, str):
            why[got] += 1
            continue
        added.append({**got, "layerId": lay, "attrs": dict(e["attrs"])})
    log = []
    if why["open"]:
        log.append({"level": "warn", "text": f"{why['open']} açık çizgi atlandı."})
    if why["empty"]:
        log.append({"level": "warn", "text": f"{why['empty']} çizginin alanı yok; yazılmadı."})
    if why["parts"]:
        log.append({"level": "warn", "text": f"{why['parts']} çok parçalı çizgi alınmadı."})
    return ok(tool, added, f"{len(inputs)} çizgiden {len(added)} alan yazıldı.", log=log, outputs={"count": len(added)})


# ── Köşe ekle ──


def densified(e, by):
    """An object with its edges divided, as the drawing writes it, and the vertices added."""
    k = e["kind"]
    if k == "line":
        e = {"kind": "polyline", "pts": [e["a"], e["b"]], "zs": [e.get("za"), e.get("zb")]}
        k = "polyline"
    total = 0

    def one(pts, bulges, closed, zs):
        nonlocal total
        vs, bs, z, added = divided(pts, bulges, closed, zs, by)
        total += added
        ring = {"pts": [out(v) for v in vs]}
        if any(b != 0 for b in bs):
            ring["bulges"] = [float(b) for b in bs]
        if any(x is not None for x in z):
            ring["zs"] = [None if x is None else float(x) for x in z]
        return ring

    closed = k == "polygon"
    res = {"kind": k, **one(e["pts"], e.get("bulges"), closed, e.get("zs"))}
    if e.get("holes"):
        res["holes"] = [one(h["pts"], h.get("bulges"), True, h.get("zs")) for h in e["holes"]]
    if e.get("parts"):
        res["parts"] = []
        for p in e["parts"]:
            part = one(p["pts"], p.get("bulges"), closed, p.get("zs"))
            if p.get("holes"):
                part["holes"] = [one(h["pts"], h.get("bulges"), True, h.get("zs")) for h in p["holes"]]
            res["parts"].append(part)
    return res, total


def densify(tool, inputs, by):
    lay = out_layer(tool)["id"]
    added, total = [], 0
    for e in inputs:
        shape, n = densified(e, by)
        total += n
        added.append({**shape, "layerId": lay, "attrs": dict(e["attrs"])})
    return ok(tool, added, f"{len(inputs)} nesneye {total} köşe eklendi.", outputs={"count": total})


def build():
    T = {k: k for k in UNDO}
    sel = lambda *ids: [by_id(i) for i in ids]  # noqa: E731
    semt_cols = ["Doğu", "Kuzey", "Semt", "Uzunluk"]
    xy_cols = ["Y1", "X1", "Y2", "X2"]
    elips_cols = ["Y", "X", "a", "b", "Semt"]
    road = objects("yol")
    cases = [
        case("points-centroid", "Parsellerin ağırlık merkezleri: kare, dışında kalan L, delikli, yaylı, iki parçalı, alanı olmayan",
             T["convert.toPoints"], {"input": L("parsel"), "location": "centroid", "parts": False},
             to_points("convert.toPoints", objects("parsel"), False, False)),
        case("points-inside", "Parsellerin iç noktaları: düz kenarlılar GEOS'un, yaylının tarama doğrusu yayın tepesiyle, alanı olmayanın sınırının yarısı",
             T["convert.toPoints"], {"input": L("parsel"), "location": "inside", "parts": False},
             to_points("convert.toPoints", objects("parsel"), True, False)),
        case("points-inside-lines", "Yolların iç noktaları: yolların yarısı (çok parçalının ilk parçasınınki), yayın ortası",
             T["convert.toPoints"], {"input": L("yol"), "location": "inside", "parts": False},
             to_points("convert.toPoints", road, True, False)),
        case("points-centroid-lines", "Yolların ağırlık merkezleri: uzunlukla ağırlıklı, yaylar kapalı biçimde",
             T["convert.toPoints"], {"input": L("yol"), "location": "centroid", "parts": False},
             to_points("convert.toPoints", road, False, False)),
        case("points-parts", "Her parça için: iki parçalı parsel ve üç noktalı nesne parça parça",
             T["convert.toPoints"], {"input": {"scope": "selection"}, "location": "centroid", "parts": True},
             to_points("convert.toPoints", sel(5, 28), False, True), selection=[5, 28]),
        case("points-locked", "Kilitli katmanın nesnesi okunur: merkezi yeni katmana yazılır",
             T["convert.toPoints"], {"input": L("kilitli"), "location": "inside", "parts": False},
             to_points("convert.toPoints", objects("kilitli"), True, False)),
        case("vertices-all", "Bütün köşeler: delikli ve kotlu alan, yaylı alan, kotlu çizgi, yay, iki parçalı çizgi",
             T["convert.vertices"], {"input": {"scope": "selection"}, "which": "all"},
             vertices("convert.vertices", sel(3, 4, 13, 14, 15), "all"), selection=[3, 4, 13, 14, 15]),
        case("vertices-ends", "Yolların iki ucu: kapalı çizginin başı ve sonu aynı yerde iki nokta, kotlar",
             T["convert.vertices"], {"input": L("yol"), "which": "ends"}, vertices("convert.vertices", road, "ends")),
        case("vertices-middle", "Orta: delikli alanın dış halkasının yarısı, yaylı çizginin ve kotlu çizginin ortası",
             T["convert.vertices"], {"input": {"scope": "selection"}, "which": "middle"},
             vertices("convert.vertices", sel(3, 12, 11), "middle"), selection=[3, 12, 11]),
        case("vertices-start", "Başlangıç ve bitiş: parsellerin ilk köşesi",
             T["convert.vertices"], {"input": L("parsel"), "which": "end"}, vertices("convert.vertices", objects("parsel"), "end")),
        case("path-grouped", "Noktaları hatlarına göre sıralı çizgiye çevir: aynı yerdeki nokta atlanır, tek noktalı hat çizgi vermez, çok noktalı nesne",
             T["convert.pointsToPath"], {"input": L("nokta"), "order": "Sıra", "group": "Hat", "close": False},
             to_paths("convert.pointsToPath", objects("nokta"), "Sıra", "Hat", False)),
        case("path-closed", "Bütün noktalar tek kapalı alan: girdinin sırasıyla",
             T["convert.pointsToPath"], {"input": L("nokta"), "order": "", "group": "", "close": True},
             to_paths("convert.pointsToPath", objects("nokta"), "", "", True)),
        case("bearing-file", "Semt ve uzunluktan çizgi: dosyadan, grad; virgüllü ondalık, okunamayan, sıfır uzunluk ve boş semt atlanır",
             T["convert.bearingLine"],
             {"sourceKind": "file", "file": "convert-semt.csv", "east": "Doğu", "north": "Kuzey", "bearing": "Semt", "length": "Uzunluk", "unit": "grad"},
             bearing_file("convert.bearingLine", "convert-semt.csv", semt_cols, "grad")),
        case("bearing-layer", "Semt ve uzunluktan çizgi: katmanın nesnelerinden, derece; okunamayan semt",
             T["convert.bearingLine"],
             {"sourceKind": "layer", "source": L("olcu"), "east": "Doğu", "north": "Kuzey", "bearing": "Semt", "length": "Uzunluk", "unit": "deg"},
             bearing_layer("convert.bearingLine", "olcu", semt_cols, "deg")),
        case("xy-file", "XY'den çizgi: başlangıcı ve bitişi aynı satır atlanır",
             T["convert.xyLine"],
             {"sourceKind": "file", "file": "convert-xy.csv", "startEast": "Y1", "startNorth": "X1", "endEast": "Y2", "endNorth": "X2"},
             xy_file("convert.xyLine", "convert-xy.csv", xy_cols)),
        case("ellipse-file", "Tablodan elips: hata elipsi, yer değiştiren eksenler, boş semt, eksi yarı eksen",
             T["convert.tableEllipse"],
             {"sourceKind": "file", "file": "convert-elips.csv", "east": "Y", "north": "X", "major": "a", "minor": "b", "bearing": "Semt", "unit": "grad"},
             ellipse_file("convert.tableEllipse", "convert-elips.csv", elips_cols, "grad")),
        # The options the first cases leave untried (docs/adr/0741 §2).
        case("vertices-begin", "Başlangıç: parsellerin ilk köşesi",
             T["convert.vertices"], {"input": L("parsel"), "which": "start"}, vertices("convert.vertices", objects("parsel"), "start")),
        case("xy-layer", "XY'den çizgi: katmanın nesnelerinden; uçları aynı yerde ve okunamayan satır atlanır",
             T["convert.xyLine"],
             {"sourceKind": "layer", "source": L("tablo"), "startEast": "Y1", "startNorth": "X1", "endEast": "Y2", "endNorth": "X2"},
             from_rows("convert.xyLine", *layer_rows("tablo"), lambda r: xy_row(r, xy_cols), xy_cols, "çizgi")),
        case("ellipse-layer", "Tablodan elips: katmanın nesnelerinden, derece; yer değiştiren eksenler ve boş semt",
             T["convert.tableEllipse"],
             {"sourceKind": "layer", "source": L("tablo"), "east": "Y", "north": "X", "major": "a", "minor": "b", "bearing": "Semt",
              "unit": "deg"},
             from_rows("convert.tableEllipse", *layer_rows("tablo"), lambda r: ellipse_row(r, elips_cols, "deg"), elips_cols, "elips")),
        case("substring-fixed", "Çizginin parçası 5 ile 25 m arası: kotlar boyunca, yay parçası, çizgi, yay, iki parçalı ve kısa çizgi",
             T["convert.substring"], {"input": L("yol"), "start": "5", "end": "25"},
             pieces("convert.substring", road, [5] * len(road), [25] * len(road))),
        case("substring-expr", "Çizginin parçası ifadelerle: uzunluğun dörtte birinden sonuna 2 m kalana",
             T["convert.substring"], {"input": {"scope": "selection"}, "start": "$uzunluk / 4", "end": "$uzunluk - 2"},
             pieces("convert.substring", sel(11, 12, 14), [path_length(e) / 4 for e in sel(11, 12, 14)], [path_length(e) - 2 for e in sel(11, 12, 14)]),
             selection=[11, 12, 14]),
        case("rings", "Alanları çizgiye çevir: delik, iki parça, yay ve kotlar",
             T["convert.polygonsToLines"], {"input": L("parsel")}, rings("convert.polygonsToLines", objects("parsel"))),
        case("areas-skip", "Çizgileri alana çevir: kapalı çizgi alan olur, açıklar atlanır",
             T["convert.linesToPolygons"], {"input": L("yol"), "open": "skip"},
             areas("convert.linesToPolygons", [e for e in road if e["kind"] == "polyline"], False)),
        case("areas-close", "Açık çizgileri kapat: düz kenarla kapanır, yay ve kot kalır; iki köşeli ve çok parçalı alan vermez",
             T["convert.linesToPolygons"], {"input": L("yol"), "open": "close"},
             areas("convert.linesToPolygons", [e for e in road if e["kind"] == "polyline"], True)),
        case("densify-interval", "5 m aralıkla köşe ekle: kare, yaylı alan, kotlu çizgi, yaylı çizgi, kotlu doğru",
             T["convert.densify"], {"input": {"scope": "selection"}, "method": "interval", "interval": 5, "count": 1},
             densify("convert.densify", sel(1, 4, 11, 12, 13), ("interval", 5)), selection=[1, 4, 11, 12, 13]),
        case("densify-count", "Kenar başına bir köşe: delikli ve kotlu alan, iki parçalı alan",
             T["convert.densify"], {"input": {"scope": "selection"}, "method": "count", "interval": 1, "count": 1},
             densify("convert.densify", sel(3, 5), ("count", 1)), selection=[3, 5]),
    ]
    files = {"convert-semt.csv": "convert-semt.csv", "convert-xy.csv": "convert-xy.csv", "convert-elips.csv": "convert-elips.csv"}
    return {"format": "kentos.processing-cases", "version": 1, "tolerance": TOL,
            "toleranceFields": ["bulges", "zs", "z", "za", "zb", "a0", "a1", "parts"],
            "documents": {}, "files": files, "cases": cases}


def main():
    tables = [(PROC_DIR / "convert-semt.csv", SEMT), (PROC_DIR / "convert-xy.csv", XY), (PROC_DIR / "convert-elips.csv", ELIPS)]
    doc = json.dumps(DOCUMENT, ensure_ascii=False, indent=1) + "\n"
    check = "--check" in sys.argv[1:]
    if not check:
        # The tables first: the cases read them.
        for p, t in tables:
            p.write_text(t)
    proc = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    outs = [*tables, (PROC_DIR / "convert.json", proc), (PROC_DIR / "convert.kcad", doc)]
    if check:
        bad = [p for p, t in outs if not p.exists() or p.read_text() != t]
        for p in bad:
            print(f"{p.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
        if bad:
            return 1
        print("geometri dönüştürmenin durumları tutarlı: fixtures/processing/v1/convert*")
        return 0
    for p, t in outs:
        p.write_text(t)
        print(f"yazıldı: {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

