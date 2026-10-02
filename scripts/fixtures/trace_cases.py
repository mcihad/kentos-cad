#!/usr/bin/env python3
"""Independent reference of İzle and Zincir (docs/adr/0161 §1, §2, §5).

Writes fixtures/trace/v1/trace.json from the ADR's rules alone, with
50-digit mpmath and no KentOS code. Both platforms trace every case with the
shared core (`ops::trace`, WASM `tracePath`) and walk every chain with it
(`ops::join::chain`, WASM `joinChain`); they must give the same ways and
chains.

İzle's rules:

1. The line work: lines, polylines and areas' rings (holes and parts too)
   with their arcs (a bulge b is a sweep of 4·atan b), arcs (counter-
   clockwise from a0 to a1) and circles (two half arcs from the angle 0).
   The ends of every edge are input corners.
2. Every edge is cut wherever it meets another: crossings, touches, and the
   ends of an edge lying on another. Points within 1 µm are one vertex; an
   input corner keeps its own coordinates. Pieces lying on top of each
   other are one.
3. A point lies on the line work within 1 µm: on a vertex, or inside a piece.
4. The way from `a` to `b` is the shortest along the pieces (a chord's
   length, an arc's r·|sweep|). When two ways are as short, either is right.
5. The way's corners: `a`, then every vertex it passes that is an input
   corner or where it turns, then `b`. It goes straight on through a cut
   point when the pieces either side are collinear and the same way round,
   or arcs of one circle turning the same way. Each edge's bulge is
   tan(sweep / 4) (0 straight).

Zincir's rules: from the seed (a line, an arc or a polyline, not on a locked
layer), from its last end and then its first, the walk goes on to the one
other end within the tolerance; it stops at none (a free end), at two or
more (a junction), at a locked object (said), and when it reaches the seed
again (closed). The members go from the first end's far side to the last's.

What is compared: the corners within 1e-9 m (input corners bit for bit),
the bulges within 1e-12, the length within 1e-9 m; the chains exactly.
Every case keeps its decisions away from their thresholds (asserted here):
other ways at least 1 mm longer unless as short to 1e-30, points off or on
the line work by more than 10 µm or less than 1e-12 m.
"""

import argparse
import json
import sys
from pathlib import Path

import mpmath as mp

mp.mp.dps = 50

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "trace" / "v1" / "trace.json"
TOL = mp.mpf("1e-6")
PI = mp.pi


def P(x, y):
    return {"x": x, "y": y}


def M(p):
    return (mp.mpf(p["x"]), mp.mpf(p["y"]))


def hyp(a, b):
    return mp.sqrt((a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2)


# ── Edges ──


def seg(a, b):
    return ("seg", a, b)


def arc(c, r, a0, sweep):
    return ("arc", c, r, a0, sweep)


def at(e, t):
    if e[0] == "seg":
        a, b = e[1], e[2]
        return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)
    _, c, r, a0, sw = e
    th = a0 + sw * t
    return (c[0] + r * mp.cos(th), c[1] + r * mp.sin(th))


def length(e):
    return hyp(e[1], e[2]) if e[0] == "seg" else e[2] * abs(e[4])


def bulge_edge(p, q, b):
    """The edge from p to q with bulge b (the ADR's own construction)."""
    if b == 0:
        return seg(p, q)
    th = 4 * mp.atan(b)
    d = hyp(p, q)
    m = ((p[0] + q[0]) / 2, (p[1] + q[1]) / 2)
    n = (-(q[1] - p[1]) / d, (q[0] - p[0]) / d)
    k = (d / 2) * mp.cot(th / 2)
    c = (m[0] + n[0] * k, m[1] + n[1] * k)
    r = hyp(c, p)
    a0 = mp.atan2(p[1] - c[1], p[0] - c[0])
    return arc(c, r, a0, th)


def path_edges(pts, bulges, closed):
    n = len(pts)
    count = n if closed else n - 1
    bs = list(bulges or []) + [0] * n
    return [bulge_edge(M(pts[i]), M(pts[(i + 1) % n]), mp.mpf(bs[i])) for i in range(count)]


def edges_of(e):
    k = e["kind"]
    if k == "line":
        return [seg(M(e["a"]), M(e["b"]))]
    if k == "polyline":
        return path_edges(e["pts"], e.get("bulges"), False)
    if k == "polygon":
        out = []
        for ring in [e] + list(e.get("parts") or []):
            out += path_edges(ring["pts"], ring.get("bulges"), True)
            for h in ring.get("holes") or []:
                out += path_edges(h["pts"], h.get("bulges"), True)
        return out
    if k == "arc":
        sw = mp.mpf(e["a1"]) - mp.mpf(e["a0"])
        while sw <= 0:
            sw += 2 * PI
        while sw > 2 * PI:
            sw -= 2 * PI
        return [arc(M(e["c"]), mp.mpf(e["r"]), mp.mpf(e["a0"]), sw)]
    if k == "circle":
        c, r = M(e["c"]), mp.mpf(e["r"])
        return [arc(c, r, mp.mpf(0), PI), arc(c, r, PI, PI)]
    return []


# ── Meeting points ──


def closest(e, p):
    """(distance, t) of the point of e nearest to p."""
    if e[0] == "seg":
        a, b = e[1], e[2]
        vx, vy = b[0] - a[0], b[1] - a[1]
        t = ((p[0] - a[0]) * vx + (p[1] - a[1]) * vy) / (vx * vx + vy * vy)
        t = min(max(t, mp.mpf(0)), mp.mpf(1))
        return hyp(at(e, t), p), t
    _, c, r, a0, sw = e
    th = mp.atan2(p[1] - c[1], p[0] - c[0])
    t = param_of(e, th)
    if t is None:
        d0, d1 = hyp(at(e, 0), p), hyp(at(e, 1), p)
        return (d0, mp.mpf(0)) if d0 <= d1 else (d1, mp.mpf(1))
    return abs(hyp(c, p) - r), t


def param_of(e, th):
    """The parameter of angle th on arc e, or None when off it."""
    _, c, r, a0, sw = e
    d = th - a0
    if sw > 0:
        while d < 0:
            d += 2 * PI
        while d >= 2 * PI:
            d -= 2 * PI
        t = d / sw
    else:
        while d > 0:
            d -= 2 * PI
        while d <= -2 * PI:
            d += 2 * PI
        t = d / sw
    if t <= 1 + mp.mpf("1e-30"):
        return min(t, mp.mpf(1))
    # Just before the start, within the arc's own tolerance.
    if (2 * PI / abs(sw)) - t <= mp.mpf("1e-30"):
        return mp.mpf(0)
    return None


def line_hits(e, f):
    """Parameters (t on e, u on f) where e and f cross or touch."""
    out = []
    if e[0] == "seg" and f[0] == "seg":
        a, b, c, d = e[1], e[2], f[1], f[2]
        rx, ry = b[0] - a[0], b[1] - a[1]
        sx, sy = d[0] - c[0], d[1] - c[1]
        den = rx * sy - ry * sx
        if abs(den) > mp.mpf("1e-40"):
            t = ((c[0] - a[0]) * sy - (c[1] - a[1]) * sx) / den
            u = ((c[0] - a[0]) * ry - (c[1] - a[1]) * rx) / den
            if -mp.mpf("1e-30") <= t <= 1 + mp.mpf("1e-30") and -mp.mpf("1e-30") <= u <= 1 + mp.mpf("1e-30"):
                out.append((t, u))
        return out
    if e[0] == "arc" and f[0] == "seg":
        return [(t, u) for (u, t) in line_hits(f, e)]
    if e[0] == "seg" and f[0] == "arc":
        a, b = e[1], e[2]
        _, c, r, _, _ = f
        dx, dy = b[0] - a[0], b[1] - a[1]
        fx, fy = a[0] - c[0], a[1] - c[1]
        A = dx * dx + dy * dy
        B = 2 * (fx * dx + fy * dy)
        C = fx * fx + fy * fy - r * r
        disc = B * B - 4 * A * C
        if disc < 0:
            return out
        for s in ([1] if disc == 0 else [-1, 1]):
            t = (-B + s * mp.sqrt(disc)) / (2 * A)
            if -mp.mpf("1e-30") <= t <= 1 + mp.mpf("1e-30"):
                q = at(e, t)
                u = param_of(f, mp.atan2(q[1] - c[1], q[0] - c[0]))
                if u is not None:
                    out.append((t, u))
        return out
    # Two arcs.
    _, c1, r1, _, _ = e
    _, c2, r2, _, _ = f
    d = hyp(c1, c2)
    if d == 0 or d > r1 + r2 or d < abs(r1 - r2):
        return out
    a = (r1 * r1 - r2 * r2 + d * d) / (2 * d)
    h2 = r1 * r1 - a * a
    h = mp.sqrt(max(h2, mp.mpf(0)))
    mx = c1[0] + a * (c2[0] - c1[0]) / d
    my = c1[1] + a * (c2[1] - c1[1]) / d
    for s in ([1] if h == 0 else [-1, 1]):
        q = (mx + s * h * (c2[1] - c1[1]) / d, my - s * h * (c2[0] - c1[0]) / d)
        t = param_of(e, mp.atan2(q[1] - c1[1], q[0] - c1[0]))
        u = param_of(f, mp.atan2(q[1] - c2[1], q[0] - c2[0]))
        if t is not None and u is not None:
            out.append((t, u))
    return out


def sub(e, t0, t1):
    if e[0] == "seg":
        return seg(at(e, t0), at(e, t1))
    _, c, r, a0, sw = e
    return arc(c, r, a0 + sw * t0, sw * (t1 - t0))


def reverse(e):
    return sub(e, mp.mpf(1), mp.mpf(0))


# ── The graph ──


def given(lines):
    """The coordinates given as numbers (lines' ends, paths' and areas' vertices): bit for bit."""
    out = set()

    def add(p):
        out.add((p["x"], p["y"]))

    for e in lines:
        k = e["kind"]
        if k == "line":
            add(e["a"])
            add(e["b"])
        elif k in ("polyline", "polygon"):
            for ring in [e] + list(e.get("parts") or []):
                for p in ring["pts"]:
                    add(p)
                for h in ring.get("holes") or []:
                    for p in h["pts"]:
                        add(p)
    return out


class Graph:
    def __init__(self, lines):
        self.pos = []
        self.input = []
        self.given = given(lines)
        edges = [e for line in lines for e in edges_of(line)]
        for e in edges:
            for t in (0, 1):
                self.vertex(at(e, t), True)
        cuts = [[] for _ in edges]
        for i, e in enumerate(edges):
            for j in range(i + 1, len(edges)):
                f = edges[j]
                for t, u in line_hits(e, f):
                    cuts[i].append(t)
                    cuts[j].append(u)
                for x, other in ((i, f), (j, e)):
                    for q in (at(other, 0), at(other, 1)):
                        d, t = closest(edges[x], q)
                        if d <= TOL:
                            cuts[x].append(t)
        self.pieces = []
        for i, e in enumerate(edges):
            ln = length(e)
            ts = [mp.mpf(0)] + sorted(t for t in cuts[i] if t * ln > TOL and (1 - t) * ln > TOL) + [mp.mpf(1)]
            last = mp.mpf(0)
            for k in range(1, len(ts)):
                t = ts[k]
                if k < len(ts) - 1 and (t - last) * ln <= TOL:
                    continue
                piece = sub(e, last, t)
                last = t
                a, b = self.vertex(at(piece, 0), False), self.vertex(at(piece, 1), False)
                if a == b:
                    continue
                mid = at(piece, mp.mpf("0.5"))
                if any({p[1], p[2]} == {a, b} and hyp(at(p[0], mp.mpf("0.5")), mid) <= 10 * TOL for p in self.pieces):
                    continue
                self.pieces.append((piece, a, b))

    def vertex(self, p, inp):
        for i, q in enumerate(self.pos):
            if abs(q[0] - p[0]) <= TOL and abs(q[1] - p[1]) <= TOL:
                if inp and not self.input[i]:
                    self.pos[i], self.input[i] = p, True
                return i
        self.pos.append(p)
        self.input.append(inp)
        return len(self.pos) - 1

    def spot(self, p):
        """('v', vertex) or ('in', piece, t) or None; asserts the margin."""
        best = None
        for i, q in enumerate(self.pos):
            d = hyp(p, q)
            assert d < mp.mpf("1e-12") or d > 10 * TOL, f"{p} near vertex {q}"
            if d <= TOL:
                return ("v", i)
        for k, (piece, _, _) in enumerate(self.pieces):
            d, t = closest(piece, p)
            assert d < mp.mpf("1e-12") or d > 10 * TOL, f"{p} near piece {k}"
            if d <= TOL and (best is None or d < best[0]):
                best = (d, k, t)
        return None if best is None else ("in", best[1], best[2])

    def ways(self, a, b):
        """Every shortest way from a to b as [(edge, node)], and its length."""
        sa, sb = self.spot(a), self.spot(b)
        if sa is None or sb is None or hyp(a, b) <= TOL:
            return [], None
        n = len(self.pos)
        start = sa[1] if sa[0] == "v" else n
        goal = sb[1] if sb[0] == "v" else n + 1
        if start == goal:
            return [], None
        steps = {}

        def add(u, v, e):
            steps.setdefault(u, []).append((v, e))

        for piece, x, y in self.pieces:
            add(x, y, piece)
            add(y, x, reverse(piece))
        if sa[0] == "in":
            piece, x, y = self.pieces[sa[1]]
            add(start, x, sub(piece, sa[2], mp.mpf(0)))
            add(start, y, sub(piece, sa[2], mp.mpf(1)))
        if sb[0] == "in":
            piece, x, y = self.pieces[sb[1]]
            add(x, goal, sub(piece, mp.mpf(0), sb[2]))
            add(y, goal, sub(piece, mp.mpf(1), sb[2]))
            if sa[0] == "in" and sa[1] == sb[1]:
                add(start, goal, sub(piece, sa[2], sb[2]))
        dist = {start: mp.mpf(0)}
        todo = {start}
        done = set()
        while todo:
            u = min(todo, key=lambda v: dist[v])
            todo.discard(u)
            done.add(u)
            for v, e in steps.get(u, []):
                nd = dist[u] + length(e)
                if v not in dist or nd < dist[v]:
                    dist[v] = nd
                    if v not in done:
                        todo.add(v)
        if goal not in dist:
            return [], None
        best = dist[goal]
        # Every way within 1e-30 of the shortest; anything shorter than 1 mm more is too close to call.
        out = []

        def walk(v, tail, total):
            if v == start:
                out.append(list(reversed(tail)))
                return
            for u, steps_u in steps.items():
                for w, e in steps_u:
                    if w != v or u not in dist:
                        continue
                    gap = dist[u] + length(e) - dist[v]
                    if abs(gap) <= mp.mpf("1e-30"):
                        walk(u, tail + [(e, v)], total)
                    else:
                        assert gap > mp.mpf("1e-3") or u in [x[1] for x in tail], f"a way {gap} longer is too close"

        walk(goal, [], best)
        return out, best

    def corners(self, a, b, way):
        n = len(self.pos)
        edges, pts, exact = [], [a], [True]
        for k, (e, node) in enumerate(way):
            through = way[k - 1][1] if k else None
            if edges and through is not None and through < n and not self.input[through] and straight_on(edges[-1], e):
                edges[-1] = join(edges[-1], e)
                pts.pop()
                exact.pop()
            else:
                edges.append(e)
            if node < n:
                pts.append(self.pos[node])
                exact.append(self.input[node] and (float(self.pos[node][0]), float(self.pos[node][1])) in self.given)
            else:
                pts.append(b)
                exact.append(True)
        pts[-1] = b
        exact[-1] = True
        bulges = [mp.mpf(0) if e[0] == "seg" else mp.tan(e[4] / 4) for e in edges]
        return pts, bulges, [i for i, x in enumerate(exact) if x]


def nearest(lines, p, reach):
    """The point of the line work nearest to p within reach, and whether it is a given corner."""
    g = Graph(lines)
    best = None
    for k, (piece, _, _) in enumerate(g.pieces):
        d, t = closest(piece, p)
        assert abs(d - reach) > mp.mpf("1e-6"), f"{p} on the reach"
        if d <= reach:
            assert best is None or abs(d - best[0]) > mp.mpf("1e-6") or hyp(at(piece, t), at(g.pieces[best[1]][0], best[2])) < mp.mpf("1e-12"), "two pieces as near"
            if best is None or d < best[0]:
                best = (d, k, t)
    if best is None:
        return None, False
    _, k, t = best
    piece, a, b = g.pieces[k]
    if t <= 0 or t >= 1:
        v = a if t <= 0 else b
        q = g.pos[v]
        return q, g.input[v] and (float(q[0]), float(q[1])) in g.given
    return at(piece, t), False


def straight_on(e, f):
    if e[0] == "seg" and f[0] == "seg":
        u = (e[2][0] - e[1][0], e[2][1] - e[1][1])
        v = (f[2][0] - f[1][0], f[2][1] - f[1][1])
        cross = u[0] * v[1] - u[1] * v[0]
        return abs(cross) <= mp.mpf("1e-30") * hyp(e[1], e[2]) * hyp(f[1], f[2]) and u[0] * v[0] + u[1] * v[1] > 0
    if e[0] == "arc" and f[0] == "arc":
        return hyp(e[1], f[1]) <= TOL and abs(e[2] - f[2]) <= TOL and (e[4] > 0) == (f[4] > 0)
    return False


def join(e, f):
    if e[0] == "seg":
        return seg(e[1], f[2])
    return arc(e[1], e[2], e[3], e[4] + f[4])


def fl(p):
    return P(float(p[0]), float(p[1]))


# ── Zincir ──


def chain(objects, seed, tol):
    def ends(o):
        s = o["shape"]
        k = s["kind"]
        if k == "line":
            return [M(s["a"]), M(s["b"])]
        if k == "polyline":
            return [M(s["pts"][0]), M(s["pts"][-1])]
        if k == "arc":
            c, r = M(s["c"]), mp.mpf(s["r"])
            return [(c[0] + r * mp.cos(mp.mpf(s[a])), c[1] + r * mp.sin(mp.mpf(s[a]))) for a in ("a0", "a1")]
        return None

    E = [ends(o) for o in objects]
    found = {"members": [], "locked": False, "closed": False}
    if E[seed] is None or objects[seed].get("locked"):
        return found
    sides = [[], []]
    for side in (1, 0):
        if found["closed"]:
            break
        current, end, here = seed, side, E[seed][side]
        while True:
            others = []
            for j, e in enumerate(E):
                if e is None:
                    continue
                for k in (0, 1):
                    if (j, k) == (current, end):
                        continue
                    d = hyp(e[k], here)
                    assert d < tol / 2 or d > tol * 2, f"an end {d} from the tolerance {tol}"
                    if d <= tol:
                        others.append((j, k))
            if len(others) != 1:
                break
            j, k = others[0]
            if j == seed:
                found["closed"] = True
                break
            if j in sides[0] or j in sides[1]:
                break
            if objects[j].get("locked"):
                found["locked"] = True
                break
            sides[side].append(j)
            current, end = j, 1 - k
            here = E[j][end]
    found["members"] = list(reversed(sides[0])) + [seed] + sides[1]
    return found


# ── Cases ──


def line(a, b, **kw):
    return {"kind": "line", "a": P(*a), "b": P(*b), **kw}


def poly(pts, closed=False, bulges=None, **kw):
    out = {"kind": "polygon" if closed else "polyline", "pts": [P(*p) for p in pts], **kw}
    if bulges:
        out["bulges"] = bulges
    return out


def paths():
    E, N = 487000.0, 4420000.0
    sq = poly([(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)], True)
    return [
        ("Kesişimden dönme: çizginin ortasından kesişime, köşeye, sonra öbür çizgiye",
         [line((0.0, 0.0), (10.0, 0.0)), line((10.0, 0.0), (10.0, 10.0)), line((5.0, -5.0), (5.0, 5.0))],
         (5.0, 5.0), (10.0, 4.0)),
        ("Kesişimden düz geçme: köşe olmaz",
         [line((0.0, 0.0), (10.0, 0.0)), line((5.0, -5.0), (5.0, 5.0))], (1.0, 0.0), (9.0, 0.0)),
        ("Aynı kenarda iki nokta", [sq], (2.0, 0.0), (7.5, 0.0)),
        ("Kenarın ortasından öbür kenarın köşesine", [sq], (4.0, 0.0), (10.0, 10.0)),
        ("İki yoldan kısası: kapalı halkanın kısa yanı (kapanış kenarından)",
         [poly([(0.0, 0.0), (40.0, 0.0), (40.0, 10.0), (0.0, 10.0)], True)], (5.0, 0.0), (5.0, 10.0)),
        ("Eşit iki yol: karenin karşı kenar ortaları", [sq], (5.0, 0.0), (5.0, 10.0)),
        ("Yaylı sınır: kabarıklığıyla yay kalır",
         [poly([(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)], True, [0.0, 0.25, 0.0, 0.0])], (10.0, 0.0), (10.0, 10.0)),
        ("Yayın tepesinden başlama: yayın yarısı kendi çemberinde",
         [poly([(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)], True, [0.0, 0.25, 0.0, 0.0])], (11.25, 5.0), (0.0, 10.0)),
        ("Bağlı olmayan çizgiler", [line((0.0, 0.0), (10.0, 0.0)), line((0.0, 5.0), (10.0, 5.0))], (1.0, 0.0), (9.0, 5.0)),
        ("Çizgi dışında bir nokta", [line((0.0, 0.0), (10.0, 0.0))], (1.0, 0.5), (9.0, 0.0)),
        ("Delik halkası boyunca",
         [{"kind": "polygon", "pts": [P(0.0, 0.0), P(30.0, 0.0), P(30.0, 30.0), P(0.0, 30.0)],
           "holes": [{"pts": [P(10.0, 10.0), P(20.0, 10.0), P(20.0, 20.0), P(10.0, 20.0)]}]}],
         (10.0, 10.0), (20.0, 15.0)),
        ("Parça halkası boyunca",
         [{"kind": "polygon", "pts": [P(0.0, 0.0), P(10.0, 0.0), P(10.0, 10.0), P(0.0, 10.0)],
           "parts": [{"pts": [P(20.0, 0.0), P(30.0, 0.0), P(30.0, 10.0), P(20.0, 10.0)]}]}],
         (20.0, 0.0), (30.0, 5.0)),
        ("Üst üste binen ortak kenar: iki parselin sınırı bir kez",
         [poly([(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)], True),
          poly([(10.0, 0.0), (20.0, 0.0), (20.0, 10.0), (10.0, 10.0)], True)],
         (0.0, 0.0), (10.0, 8.0)),
        ("Ortak kenarın ortasındaki komşu köşesi: girdi köşesi olarak kalır",
         [poly([(0.0, 0.0), (20.0, 0.0), (20.0, 10.0), (0.0, 10.0)], True),
          poly([(10.0, 0.0), (15.0, -8.0), (5.0, -8.0)], True)],
         (0.0, 0.0), (20.0, 5.0)),
        ("Daire boyunca: kısa yandan, dairenin doğu (bölme) noktası köşedir",
         [{"kind": "circle", "c": P(0.0, 0.0), "r": 5.0}, line((0.0, 5.0), (0.0, 12.0)), line((3.0, -4.0), (6.0, -8.0))],
         (0.0, 12.0), (6.0, -8.0)),
        ("Yayın ucundan çizgiye",
         [{"kind": "arc", "c": P(0.0, 0.0), "r": 10.0, "a0": 0.0, "a1": 1.5707963267948966}, line((0.0, 10.0), (-10.0, 10.0))],
         (10.0, 0.0), (-6.0, 10.0)),
        ("Büyük koordinatlar: pafta sınırı boyunca, köşeler bit bit",
         [poly([(E + 0.125, N + 0.375), (E + 20.25, N + 0.5), (E + 20.5, N + 30.75), (E + 0.25, N + 30.5)], True),
          line((E + 20.25, N + 0.5), (E + 40.0, N - 0.25))],
         (E + 0.25, N + 30.5), (E + 40.0, N - 0.25)),
    ]


def nearests():
    E, N = 487000.0, 4420000.0
    return [
        ("Kenarın ortasına", [line((0.0, 0.0), (10.0, 0.0))], (4.0, 0.3), 0.5),
        ("Köşeye: iki kenarın ortak ucu, bit bit", [line((0.0, 0.0), (10.0, 0.0)), line((10.0, 0.0), (10.0, 10.0))], (10.2, -0.2), 0.5),
        ("Uzakta: yok", [line((0.0, 0.0), (10.0, 0.0))], (5.0, 2.0), 0.5),
        ("Yaya: çemberin üstünde", [{"kind": "arc", "c": P(0.0, 0.0), "r": 10.0, "a0": 0.0, "a1": 1.5707963267948966}], (7.2, 7.2), 0.5),
        ("İki çizginin arasında yakın olana", [line((0.0, 0.0), (10.0, 0.0)), line((0.0, 1.0), (10.0, 1.0))], (5.0, 0.4), 0.5),
        ("Büyük koordinatlarda", [line((E, N), (E + 20.0, N + 0.5))], (E + 10.0, N + 0.35), 0.5),
    ]


def chains():
    ln = lambda a, b, locked=False: {"shape": line(a, b), "locked": locked}
    return [
        ("Üç çizgi uç uca, sonunda kavşak",
         [ln((0.0, 0.0), (10.0, 0.0)), ln((10.0, 0.0), (20.0, 5.0)), ln((20.0, 5.0), (30.0, 5.0)),
          ln((30.0, 5.0), (40.0, 0.0)), ln((30.0, 5.0), (40.0, 10.0))], 1, 0.001),
        ("Kapanan halka", [ln((0.0, 0.0), (10.0, 0.0)), ln((10.0, 0.0), (10.0, 10.0)), ln((10.0, 10.0), (0.0, 10.0)), ln((0.0, 10.0), (0.0, 0.0))], 0, 0.001),
        ("Ters yönlü parçalar", [ln((10.0, 0.0), (0.0, 0.0)), ln((20.0, 0.0), (10.0, 0.0)), ln((20.0, 0.0), (30.0, 0.0))], 1, 0.001),
        ("Kilitli nesnede durur", [ln((0.0, 0.0), (10.0, 0.0)), ln((10.0, 0.0), (20.0, 0.0), True), ln((-10.0, 0.0), (0.0, 0.0))], 0, 0.001),
        ("Tolerans içindeki uç birleşir, dışındaki birleşmez",
         [ln((0.0, 0.0), (10.0, 0.0)), ln((10.0004, 0.0), (20.0, 0.0)), ln((20.003, 0.0), (30.0, 0.0))], 0, 0.001),
        ("Çoklu çizgi ve yay",
         [{"shape": poly([(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]), "locked": False},
          {"shape": {"kind": "arc", "c": P(0.0, 10.0), "r": 10.0, "a0": 0.0, "a1": 1.5707963267948966}, "locked": False},
          ln((0.0, 20.0), (-10.0, 20.0))], 0, 0.001),
        ("Kapalı alan zincir değildir", [{"shape": poly([(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)], True), "locked": False}, ln((0.0, 0.0), (-5.0, 0.0))], 0, 0.001),
    ]


def build():
    out_paths = []
    for name, lines, a, b in paths():
        g = Graph(lines)
        ways, best = g.ways(M(P(*a)), M(P(*b)))
        if not ways:
            expected = None
        else:
            seen = []
            for w in ways:
                pts, bulges, exact = g.corners(M(P(*a)), M(P(*b)), w)
                item = {"pts": [fl(p) for p in pts], "bulges": [float(x) for x in bulges], "exact": exact}
                if item not in seen:
                    seen.append(item)
            expected = {"ways": seen, "length": float(best)}
        out_paths.append({"name": name, "lines": lines, "a": P(*a), "b": P(*b), "expected": expected})
    out_nearest = []
    for name, lines, p, reach in nearests():
        q, exact = nearest(lines, M(P(*p)), mp.mpf(reach))
        out_nearest.append({"name": name, "lines": lines, "p": P(*p), "reach": reach, "expected": None if q is None else fl(q), "exact": exact})
    out_chains = [
        {"name": name, "objects": objects, "seed": seed, "tol": tol, "expected": chain(objects, seed, mp.mpf(tol))}
        for name, objects, seed, tol in chains()
    ]
    return {
        "format": "kentos.trace",
        "version": 1,
        "about": "İzle ve Zincir (docs/adr/0161): bağımsız başvuru, scripts/fixtures/trace_cases.py",
        "paths": out_paths,
        "nearest": out_nearest,
        "chains": out_chains,
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT}: güncel değil; betiği --check olmadan çalıştırın", file=sys.stderr)
            return 1
        print(f"{OUT}: güncel")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    data = json.loads(text)
    print(f"{OUT}: {len(data['paths'])} yol, {len(data['nearest'])} en yakın nokta, {len(data['chains'])} zincir")
    return 0


if __name__ == "__main__":
    sys.exit(main())
