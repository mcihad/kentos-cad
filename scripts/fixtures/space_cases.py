#!/usr/bin/env python3
"""Spaces from walls (docs/adr/0342 §2), from the ADR's rules, without KentOS code.

Writes fixtures/walls/v1/spaces.json; `--check` compares and writes nothing. The shared core's test (`kentos-geometry-
core`, tests/all/spaces.rs) reads it and must give the same net face, the same faces moved by each method, and their
areas and perimeters.

The walls are straight and their directions Pythagorean, so every point is an exact fraction. The walls' faces are the
joins reference's (`wall_join_cases.py`) without openings: each wall's first and last line, kept where the joins keep
them, and its ends. The face round the seed is the smallest bounded face of their line work that holds it (half-edges
split at every crossing, each face traced keeping its inside on the left); its holes are the line work's other pieces
inside it, each as its outer boundary. By a method, each edge whose middle lies on a wall's face moves into that wall
(half its width: centre line; its width: outer face), every edge to its right with the outer ring counter-clockwise
and the holes clockwise; consecutive moved edges meet where their lines cross, parallel ones moving alike share a
point, parallel ones moving apart take a step between them.
"""

import argparse
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from wall_join_cases import (  # noqa: E402
    Solver, Wall, cross, dot, end_point, exact_len, kept, norm_spans, sub, sub_line, add, mul,
)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/walls/v1/spaces.json"


def faces(walls):
    """The walls' faces and ends as segments, each with the width of the wall whose face it is (0: an end)."""
    s = Solver(walls)
    s.solve()
    out = []
    for i, w in enumerate(walls):
        ls = s.lines[i]
        for k in sorted({0, len(ls) - 1}):
            line = ls[k]
            gaps = norm_spans(line, w.closed, s.gaps[i][k]) if s.gaps[i][k] else []
            spans = kept(line, w.closed, gaps) if gaps else None
            pieces = [sub_line(line, w.closed, a, b) for a, b in spans] if spans else \
                [line + ([line[0]] if w.closed else [])]
            for p in pieces:
                for a, b in zip(p, p[1:]):
                    if a != b:
                        out.append((a, b, w.width))
        if not w.closed and w.caps != "open":
            for e in (0, 1):
                if not s.joined[i][e]:
                    out.append((end_point(ls[0], e), end_point(ls[-1], e), F(0)))
    return out


def on_segment(p, a, b):
    t = sub(b, a)
    if cross(sub(p, a), t) != 0:
        return False
    u = dot(sub(p, a), t)
    return 0 <= u <= dot(t, t)


def split(segs):
    """The segments split at every point where another touches or crosses them."""
    out = []
    for i, (a, b, w) in enumerate(segs):
        ts = {F(0), F(1)}
        d = sub(b, a)
        for j, (c, e, _) in enumerate(segs):
            if i == j:
                continue
            f = sub(e, c)
            den = cross(d, f)
            if den != 0:
                t = cross(sub(c, a), f) / den
                u = cross(sub(c, a), d) / den
                if 0 <= t <= 1 and 0 <= u <= 1:
                    ts.add(t)
            for q in (c, e):
                if on_segment(q, a, b):
                    ts.add(dot(sub(q, a), d) / dot(d, d))
        ts = sorted(ts)
        for t0, t1 in zip(ts, ts[1:]):
            p, q = add(a, mul(d, t0)), add(a, mul(d, t1))
            if p != q:
                out.append((p, q))
    # Each piece once.
    seen, uniq = set(), []
    for p, q in out:
        key = (min(p, q), max(p, q))
        if key not in seen:
            seen.add(key)
            uniq.append((p, q))
    return uniq


def angle(v):
    return math.atan2(float(v[1]), float(v[0]))


def traced(pieces):
    """Every face cycle of the line work: half-edges, each next the one turning most clockwise at its end."""
    out_edges = {}
    for p, q in pieces:
        out_edges.setdefault(p, []).append(q)
        out_edges.setdefault(q, []).append(p)
    for p in out_edges:
        out_edges[p].sort(key=lambda q: angle(sub(q, p)))
    used, cycles = set(), []
    for p in out_edges:
        for q in out_edges[p]:
            if (p, q) in used:
                continue
            cyc, u, v = [], p, q
            while (u, v) not in used:
                used.add((u, v))
                cyc.append(u)
                around = out_edges[v]
                k = around.index(u)
                u, v = v, around[(k - 1) % len(around)]
            cycles.append(cyc)
    return cycles


def area2(ring):
    n = len(ring)
    return sum(cross(ring[i], ring[(i + 1) % n]) for i in range(n))


def inside(ring, p):
    """Strictly inside (exact crossing count)."""
    c = False
    n = len(ring)
    for i in range(n):
        a, b = ring[i], ring[(i + 1) % n]
        if (a[1] > p[1]) != (b[1] > p[1]):
            x = a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1])
            if x > p[0]:
                c = not c
    return c


def components(pieces):
    parent = {}

    def find(x):
        while parent.setdefault(x, x) != x:
            x = parent[x]
        return x
    for p, q in pieces:
        parent[find(p)] = find(q)
    groups = {}
    for p, q in pieces:
        groups.setdefault(find(p), []).append((p, q))
    return list(groups.values())


def net_face(segs, seed):
    pieces = split([(a, b, w) for a, b, w in segs])
    cycles = traced(pieces)
    bounded = [c for c in cycles if area2(c) > 0 and inside(c, seed)]
    if not bounded:
        return None
    outer = min(bounded, key=area2)
    # The holes: the line work's other pieces inside it, not inside another such piece, each its outer boundary.
    own = set(outer)
    holes = []
    for comp in components(pieces):
        pts = {p for e in comp for p in e}
        if pts & own or not all(inside(outer, p) for p in pts):
            continue
        ccw = [c for c in traced(comp) if area2(c) < 0]
        boundary = min(ccw, key=area2)  # the component's outer boundary (the most negative)
        if inside(boundary[::-1], seed):
            continue
        holes.append(boundary)
    holes = [h for h in holes if not any(h is not g and all(inside(g[::-1], p) for p in h) for g in holes)]
    return outer, holes


def rightn(t):
    ln = exact_len(t)
    return (t[1] / ln, -t[0] / ln)


def reach_of(a, b, segs, method):
    m = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
    for c, e, w in segs:
        if w > 0 and on_segment(m, c, e):
            return {"net": F(0), "centerline": w / 2, "gross": w}[method]
    return F(0)


def meet(p, u, q, v):
    den = cross(u, v)
    if den == 0:
        return None
    t = cross(sub(q, p), v) / den
    return add(p, mul(u, t))


def moved(ring, segs, method):
    n = len(ring)
    reach = [reach_of(ring[i], ring[(i + 1) % n], segs, method) for i in range(n)]
    out = []
    for i in range(n):
        prev = (i - 1) % n
        p = ring[i]
        tp, ti = sub(p, ring[prev]), sub(ring[(i + 1) % n], p)
        dp, di = reach[prev], reach[i]
        if dp == 0 and di == 0:
            out.append(p)
            continue
        ap, ai = add(p, mul(rightn(tp), dp)), add(p, mul(rightn(ti), di))
        q = meet(ap, tp, ai, ti)
        if q is not None:
            out.append(q)
        elif dp == di:
            out.append(ai)
        else:
            out.extend([ap, ai])
    return out


def ring_len(r):
    return sum(exact_len(sub(r[(i + 1) % len(r)], r[i])) for i in range(len(r)))


def plain(r):
    """The ring without collinear vertices (what the two sides compare)."""
    out = list(r)
    changed = True
    while changed and len(out) > 3:
        changed = False
        for i in range(len(out)):
            a, b, c = out[i - 1], out[i], out[(i + 1) % len(out)]
            if cross(sub(b, a), sub(c, b)) == 0:
                out.pop(i)
                changed = True
                break
    return out


def pts(r):
    return sorted([float(p[0]), float(p[1])] for p in plain(r))


def cases():
    c = []

    def case(name, walls, seed):
        segs = faces(walls)
        found = net_face(segs, (F(seed[0]), F(seed[1])))
        entry = {"name": name, "walls": [w.json() for w in walls], "seed": [float(seed[0]), float(seed[1])]}
        if found is None:
            entry["net"] = None
            c.append(entry)
            return
        outer, holes = found
        holes = [h if area2(h) < 0 else h[::-1] for h in holes]
        for method in ("net", "centerline", "gross"):
            o = moved(outer, segs, method)
            hs = [moved(h, segs, method) for h in holes]
            area = abs(area2(o)) / 2 - sum(abs(area2(h)) / 2 for h in hs)
            entry[method] = {"outer": pts(o), "holes": sorted(pts(h) for h in hs), "area": float(area),
                             "perimeter": float(ring_len(o) + sum(ring_len(h) for h in hs))}
        c.append(entry)

    w2, w3 = F(2, 10), F(3, 10)
    case("Kapalı tek duvar oda", [Wall([(0, 0), (6, 0), (6, 4), (0, 4)], w2, closed=True)], (3, 2))
    case("Dört ayrı duvar, köşeler gönye", [Wall([(0, 0), (6, 0)], w2), Wall([(6, 0), (6, 4)], w2),
                                            Wall([(6, 4), (0, 4)], w2), Wall([(0, 4), (0, 0)], w2)], (3, 2))
    room = [Wall([(0, 0), (8, 0)], w2), Wall([(8, 0), (8, 5)], w2), Wall([(8, 5), (0, 5)], w2), Wall([(0, 5), (0, 0)], w2)]
    case("Ara duvarla iki oda: sol", room + [Wall([(5, 0), (5, 5)], F(15, 100))], (2, 2))
    case("Ara duvarla iki oda: sağ", room + [Wall([(5, 0), (5, 5)], F(15, 100))], (7, 2))
    case("Kalın alt duvar", [Wall([(0, 0), (6, 0)], w3), Wall([(6, 0), (6, 4)], w2),
                              Wall([(6, 4), (0, 4)], w2), Wall([(0, 4), (0, 0)], w2)], (3, 2))
    case("Ortasında kolon duvarı: delik", [Wall([(0, 0), (8, 0), (8, 6), (0, 6)], w2, closed=True),
                                          Wall([(3, 2), (5, 2), (5, 4), (3, 4)], w2, closed=True)], (1, 1))
    case("L oda", [Wall([(0, 0), (6, 0), (6, 3), (3, 3), (3, 6), (0, 6)], w2, closed=True)], (1, 1))
    case("Sola hizalı duvarlar", [Wall([(0, 0), (6, 0), (6, 4), (0, 4)], w2, "left", closed=True)], (3, 2))
    case("Eğik oda (3-4-5)", [Wall([(0, 0), (8, 6), (5, 10), (-3, 4)], w2, closed=True)], (2, 5))
    case("Kapanmayan duvarlar: mahal yok", [Wall([(0, 0), (6, 0)], w2), Wall([(6, 0), (6, 4)], w2),
                                           Wall([(6, 4), (0, 4)], w2)], (3, 2))
    case("Kalınlığı değişen alt kenar: basamak", [Wall([(0, 0), (3, 0)], w2, joins=("auto", "none")),
                                                  Wall([(3, 0), (6, 0)], w3, joins=("none", "auto")),
                                                  Wall([(6, 0), (6, 4)], w2), Wall([(6, 4), (0, 4)], w2),
                                                  Wall([(0, 4), (0, 0)], w2)], (1.5, 2))
    return c


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    text = json.dumps({"format": "kentos.space-cases", "version": 1, "cases": cases()}, ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
            return 1
        print(f"{OUT.relative_to(ROOT)}: güncel")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(cases())} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
