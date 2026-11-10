#!/usr/bin/env python3
"""A building section's geometry (docs/adr/8042 §2), from the ADR's rules, in exact fractions, without KentOS code.

Writes fixtures/arch/v1/sections.json; `--check` compares and writes nothing. The shared core's test
(`kentos-geometry-core`, tests/all/sections.rs) reads it and must give the same cut rings and seen pieces (within 1e-7).

The rules, as ADR 8042 §2 gives them:

- Frame: the line from `a` to `b`; the viewer looks to its left (its right when flipped). `u` runs along the line from
  the viewer's left end (`a`, or `b` when flipped) to the right, `d` into the view; `depth` (absent: none) bounds `d`.
- Solid: rings read by the even-odd rule, between two heights (a prism) or a flight: along `dir` from `start`
  (s = (p − start)·dir), its top the height of the last step whose distance it has reached (before the first, the
  first's), its underside max(floor, soffit + slope·s), never over its top.
- Cut: the stretches of the line from 0 to its length where the footprint lies on both sides of the line (a millionth
  of a metre off); a prism there between its heights, less each void's stretch between the void's heights; a flight
  there with its top stepping and its underside sloping, one ring per run of pieces. Each ring counter-clockwise from
  its lowest-leftmost point, without repeated points and points it runs straight through.
- Seen: the faces past the line whose outward normal points back at the viewer (its dot with the look below 0), cut to
  what is in view (d ≥ 0, d ≤ depth, 0 ≤ u ≤ length): a prism's vertical faces along its footprint's edges, with their
  bottom and top edges, and a vertical edge at each end that is the footprint's corner (not where the view cut the
  face); a flight's sides likewise, in pieces between its steps and its underside's bend (each piece's top, its
  underside, the riser's line where the next steps, the ends' verticals at corners), its risers (across it where its
  top steps, looking down it), its underside where it lies over its floor (a plane looking back along the flight; its
  outline's edges); and the marks, cut to what is in view.
- An edge is hidden where it lies inside, or on the outline of, something nearer: what is cut (at depth 0; an edge in
  the plane too, a riser on the line being the cut's own face), or a seen face (each piece of it) nearer than it by
  more than a millionth of a metre. What is left, in pieces longer than a
  millionth of a metre, is seen; an edge two faces share is drawn once.
"""

import argparse
import json
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/arch/v1/sections.json"

SIDE = F(1, 10**6)
BEHIND = F(1, 10**6)
SHORTEST = F(1, 10**6)


def isqrt_fraction(x):
    """The square root of a fraction that is a square of one."""
    from math import isqrt
    n, d = x.numerator, x.denominator
    rn, rd = isqrt(n), isqrt(d)
    assert rn * rn == n and rd * rd == d, f"{x} is not a square"
    return F(rn, rd)


def P(x, y):
    return (F(x), F(y))


class Frame:
    def __init__(self, a, b, flip, depth):
        dx, dy = b[0] - a[0], b[1] - a[1]
        self.length = isqrt_fraction(dx * dx + dy * dy)
        t = (dx / self.length, dy / self.length)
        left = (-t[1], t[0])
        if flip:
            self.origin, self.right, self.look = b, (-t[0], -t[1]), (-left[0], -left[1])
        else:
            self.origin, self.right, self.look = a, t, left
        self.depth = depth

    def u(self, p):
        return (p[0] - self.origin[0]) * self.right[0] + (p[1] - self.origin[1]) * self.right[1]

    def d(self, p):
        return (p[0] - self.origin[0]) * self.look[0] + (p[1] - self.origin[1]) * self.look[1]

    def at(self, u, d):
        return (self.origin[0] + self.right[0] * u + self.look[0] * d,
                self.origin[1] + self.right[1] * u + self.look[1] * d)

    def in_view(self, p, q):
        """The part [t0, t1] of the segment p→q in view, or None."""
        lo, hi = F(0), F(1)
        bounds = [(self.d(p), self.d(q) - self.d(p)),
                  (self.u(p), self.u(q) - self.u(p)),
                  (self.length - self.u(p), self.u(p) - self.u(q))]
        if self.depth is not None:
            bounds.append((self.depth - self.d(p), self.d(p) - self.d(q)))
        for a, b in bounds:  # a + b t ≥ 0
            if b == 0:
                if a < 0:
                    return None
            elif b > 0:
                lo = max(lo, -a / b)
            else:
                hi = min(hi, -a / b)
        return (lo, hi) if hi > lo else None


def inside(rings, p):
    odd = False
    for r in rings:
        n = len(r)
        for i in range(n):
            a, b = r[i], r[(i + 1) % n]
            if (a[1] > p[1]) != (b[1] > p[1]) and p[0] < a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]):
                odd = not odd
    return odd


def through(rings, at, along, normal, lo, hi):
    """Stretches w of the line at + w·along (lo..hi) with the rings on both sides, SIDE off along `normal`."""
    def w(p):
        return (p[0] - at[0]) * along[0] + (p[1] - at[1]) * along[1]

    def n(p):
        return (p[0] - at[0]) * normal[0] + (p[1] - at[1]) * normal[1]

    breaks = {lo, hi}
    for r in rings:
        k = len(r)
        for i in range(k):
            p, q = r[i], r[(i + 1) % k]
            if n(p) == 0:
                breaks.add(w(p))
            if (n(p) > 0 and n(q) < 0) or (n(p) < 0 and n(q) > 0):
                t = n(p) / (n(p) - n(q))
                breaks.add(w(p) + (w(q) - w(p)) * t)
    breaks = sorted(b for b in breaks if lo <= b <= hi)
    out = []
    for a, b in zip(breaks, breaks[1:]):
        m = (a + b) / 2
        p = (at[0] + along[0] * m, at[1] + along[1] * m)
        if all(inside(rings, (p[0] + normal[0] * s, p[1] + normal[1] * s)) for s in (SIDE, -SIDE)):
            if out and out[-1][1] == a:
                out[-1] = (out[-1][0], b)
            else:
                out.append((a, b))
    return out


def clean(ring):
    out = []
    for p in ring:
        if not out or out[-1] != p:
            out.append(p)
    while len(out) > 1 and out[0] == out[-1]:
        out.pop()
    changed = True
    while changed and len(out) >= 3:
        changed = False
        n = len(out)
        for i in range(n):
            a, b, c = out[i - 1], out[i], out[(i + 1) % n]
            u, v = (b[0] - a[0], b[1] - a[1]), (c[0] - b[0], c[1] - b[1])
            if u[0] * v[1] - u[1] * v[0] == 0 and u[0] * v[0] + u[1] * v[1] > 0:
                out.pop(i)
                changed = True
                break
    return out


def area2(ring):
    n = len(ring)
    return sum(ring[i][0] * ring[(i + 1) % n][1] - ring[(i + 1) % n][0] * ring[i][1] for i in range(n))


def canonical(ring):
    """Counter-clockwise from its lowest-leftmost point (smallest x, then y)."""
    ring = clean(ring)
    if area2(ring) < 0:
        ring = ring[::-1]
    k = min(range(len(ring)), key=lambda i: (ring[i][0], ring[i][1]))
    return ring[k:] + ring[:k]


class Flight:
    def __init__(self, start, dir, steps, soffit, slope, floor):
        self.start, self.dir, self.steps = start, dir, steps
        self.soffit, self.slope, self.floor = soffit, slope, floor

    def s(self, p):
        return (p[0] - self.start[0]) * self.dir[0] + (p[1] - self.start[1]) * self.dir[1]

    def top(self, s):
        z = self.steps[0][1]
        for frm, h in self.steps:
            if s >= frm:
                z = h
            else:
                break
        return z

    def bottom(self, s):
        return min(max(self.floor, self.soffit + self.slope * s), self.top(s))

    def kink(self):
        return None if self.slope == 0 else (self.floor - self.soffit) / self.slope

    def breaks(self):
        out = [frm for frm, _ in self.steps[1:]]
        if self.kink() is not None:
            out.append(self.kink())
        return sorted(out)


def traps(frame, f, p, q, t0, t1):
    """The flight over p→q, t from t0 to t1, in pieces: (u0, u1, top, b0, b1, d0, d1)."""
    sp, sq = f.s(p), f.s(q)
    ts = {t0, t1}
    if sq != sp:
        for b in f.breaks():
            ts.add((b - sp) / (sq - sp))
    ts = sorted(t for t in ts if t0 <= t <= t1)

    def at(t):
        return (p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t)

    out = []
    for a, b in zip(ts, ts[1:]):
        pa, pb = at(a), at(b)
        tr = (frame.u(pa), frame.u(pb), f.top(sp + (sq - sp) * (a + b) / 2), f.bottom(sp + (sq - sp) * a),
              f.bottom(sp + (sq - sp) * b), frame.d(pa), frame.d(pb))
        if tr[2] > min(tr[3], tr[4]):
            out.append(tr)
    return out


def trap_ring(tr):
    u0, u1, top, b0, b1 = tr[:5]
    left, right = ((u0, b0), (u1, b1)) if u0 <= u1 else ((u1, b1), (u0, b0))
    ring = [left, right]
    if top > right[1]:
        ring.append((right[0], top))
    if top > left[1]:
        ring.append((left[0], top))
    return ring


def outward(rings):
    out = []
    for r in rings:
        n = len(r)
        for k in range(n):
            p, q = r[k], r[(k + 1) % n]
            if p == q:
                continue
            # The right of p→q, checked a millionth of a metre off its middle; its length only scales the test.
            m = (q[1] - p[1], -(q[0] - p[0]))
            mid = ((p[0] + q[0]) / 2, (p[1] + q[1]) / 2)
            ln2 = m[0] * m[0] + m[1] * m[1]
            # A point SIDE off along the unit normal: m / |m| · SIDE; |m| is irrational in general, so step by
            # m · (SIDE / |m|) with |m| bounded below by its largest component (enough to leave the edge, too
            # little to reach another edge in these cases).
            big = max(abs(m[0]), abs(m[1]))
            off = (mid[0] + m[0] * SIDE / big, mid[1] + m[1] * SIDE / big)
            if inside(rings, off):
                m = (-m[0], -m[1])
            out.append((p, q, m, ln2))
    return out


def facing(frame, m):
    return m[0] * frame.look[0] + m[1] * frame.look[1] < 0


def cover(ring, depth, cut=False):
    """A convex ring (counter-clockwise) and its depth a + b u + c z; a cut hides what lies in the plane too."""
    ring = [r for r in ring]
    if len(ring) < 3 or area2(ring) == 0:
        return None
    if area2(ring) < 0:
        ring = ring[::-1]
    a, b, c = depth
    near = min(a + b * x + c * y for x, y in ring)
    return {"ring": ring, "a": a, "b": b, "c": c, "near": near, "behind": -BEHIND if cut else BEHIND}


def vertical_depth(u0, d0, u1, d1):
    if u1 == u0:
        return None
    b = (d1 - d0) / (u1 - u0)
    return (d0 - b * u0, b, F(0))


def hides(cv, e):
    """The part (t0, t1) of edge e = (solid, p, q, dp, dq) inside cv or on its outline, and behind it."""
    _, p, q, dp, dq = e
    behind = cv["behind"]
    if cv["near"] > max(dp, dq) - behind:
        return None
    lo, hi = F(0), F(1)
    ring = cv["ring"]
    n = len(ring)
    for i in range(n):
        r, s = ring[i], ring[(i + 1) % n]
        side = (s[0] - r[0], s[1] - r[1])
        a = side[0] * (p[1] - r[1]) - side[1] * (p[0] - r[0])
        b = side[0] * (q[1] - p[1]) - side[1] * (q[0] - p[0])
        # a + b t ≥ 0
        if b == 0:
            if a < 0:
                return None
        elif b > 0:
            lo = max(lo, -a / b)
        else:
            hi = min(hi, -a / b)
    cp = cv["a"] + cv["b"] * p[0] + cv["c"] * p[1]
    cq = cv["a"] + cv["b"] * q[0] + cv["c"] * q[1]
    a, b = dp - cp - behind, (dq - dp) - (cq - cp)
    # a + b t > 0
    if b == 0:
        if a <= 0:
            return None
    elif b > 0:
        lo = max(lo, -a / b)
    else:
        hi = min(hi, -a / b)
    return (lo, hi) if hi > lo else None


def clip_ring(ring, keep):
    n = len(ring)
    out = []
    for i in range(n):
        p, q = ring[i], ring[(i + 1) % n]
        kp, kq = keep(p), keep(q)
        if kp >= 0:
            out.append(p)
        if (kp >= 0) != (kq >= 0):
            t = kp / (kp - kq)
            out.append((p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t))
    return clean(out)


def section(frame, solids):
    cuts, covers, edges = [], [], []
    for i, solid in enumerate(solids):
        rings = [clean(r) for r in solid["rings"]]
        rings = [r for r in rings if len(r) >= 3]
        # A solid without a footprint (an opening's lines on its wall's faces) is its marks alone.
        if rings:
            stretches = through(rings, frame.origin, frame.right, frame.look, F(0), frame.length)
            h = solid["height"]
            if "flight" not in h:
                bottom, top = h["bottom"], h["top"]
                rects = [(a, b, bottom, top) for a, b in stretches]
                for v in solid["voids"]:
                    for a, b in through([clean(v["ring"])], frame.origin, frame.right, frame.look, F(0), frame.length):
                        hole = (a, b, v["bottom"], v["top"])
                        rects = [piece for r in rects for piece in subtract(r, hole)]
                for u0, u1, z0, z1 in rects:
                    ring = [(u0, z0), (u1, z0), (u1, z1), (u0, z1)]
                    covers.append(cover(ring, (F(0), F(0), F(0)), cut=True))
                    cuts.append({"solid": i, "ring": canonical(ring)})
                for p, q, m, _ in outward(rings):
                    if not facing(frame, m):
                        continue
                    view = frame.in_view(p, q)
                    if view is None:
                        continue
                    t0, t1 = view
                    a = (p[0] + (q[0] - p[0]) * t0, p[1] + (q[1] - p[1]) * t0)
                    b = (p[0] + (q[0] - p[0]) * t1, p[1] + (q[1] - p[1]) * t1)
                    u0, u1, d0, d1 = frame.u(a), frame.u(b), frame.d(a), frame.d(b)
                    dep = vertical_depth(u0, d0, u1, d1)
                    if dep is not None:
                        covers.append(cover([(u0, bottom), (u1, bottom), (u1, top), (u0, top)], dep))
                    for z in (bottom, top):
                        edges.append((i, (u0, z), (u1, z), d0, d1))
                    if t0 == 0:
                        edges.append((i, (u0, bottom), (u0, top), d0, d0))
                    if t1 == 1:
                        edges.append((i, (u1, bottom), (u1, top), d1, d1))
            else:
                f = Flight(**h["flight"])
                for a, b in stretches:
                    p, q = frame.at(a, F(0)), frame.at(b, F(0))
                    trs = traps(frame, f, p, q, F(0), F(1))
                    for tr in trs:
                        covers.append(cover(trap_ring(tr), (F(0), F(0), F(0)), cut=True))
                    for ring in flight_rings(trs):
                        cuts.append({"solid": i, "ring": canonical(ring)})
                flight_faces(frame, i, rings, f, covers, edges)
        for (p, zp), (q, zq) in solid["marks"]:
            view = frame.in_view(p, q)
            if view is None:
                continue
            t0, t1 = view
            a = (p[0] + (q[0] - p[0]) * t0, p[1] + (q[1] - p[1]) * t0)
            b = (p[0] + (q[0] - p[0]) * t1, p[1] + (q[1] - p[1]) * t1)
            edges.append((i, (frame.u(a), zp + (zq - zp) * t0), (frame.u(b), zp + (zq - zp) * t1), frame.d(a),
                          frame.d(b)))
    covers = [c for c in covers if c is not None]
    seen, drawn = [], set()
    for e in edges:
        solid, p, q, dp, dq = e
        len2 = (q[0] - p[0]) ** 2 + (q[1] - p[1]) ** 2
        if len2 <= SHORTEST ** 2:
            continue
        hidden = sorted(h for h in (hides(c, e) for c in covers) if h is not None)
        t, pieces = F(0), []
        for a, b in hidden:
            if a > t:
                pieces.append((t, a))
            t = max(t, b)
        if t < 1:
            pieces.append((t, F(1)))
        for a, b in pieces:
            if (b - a) ** 2 * len2 <= SHORTEST ** 2:
                continue
            pa = (p[0] + (q[0] - p[0]) * a, p[1] + (q[1] - p[1]) * a)
            pb = (p[0] + (q[0] - p[0]) * b, p[1] + (q[1] - p[1]) * b)
            da, db = dp + (dq - dp) * a, dp + (dq - dp) * b
            lo_end, hi_end = sorted([pa, pb])
            k = (solid, lo_end, hi_end)
            if k in drawn:
                continue
            drawn.add(k)
            seen.append({"solid": solid, "a": lo_end, "b": hi_end, "depth": min(da, db)})
    return cuts, seen


def subtract(r, v):
    u0, u1, z0, z1 = r
    v0, v1, w0, w1 = v
    if v1 <= u0 or v0 >= u1 or w1 <= z0 or w0 >= z1:
        return [r]
    out = []
    m0, m1 = max(u0, v0), min(u1, v1)
    if m0 > u0:
        out.append((u0, m0, z0, z1))
    if m1 < u1:
        out.append((m1, u1, z0, z1))
    if w0 > z0:
        out.append((m0, m1, z0, w0))
    if w1 < z1:
        out.append((m0, m1, w1, z1))
    return out


def flight_rings(trs):
    norm = []
    for u0, u1, top, b0, b1, d0, d1 in trs:
        norm.append((u0, u1, top, b0, b1) if u0 <= u1 else (u1, u0, top, b1, b0))
    norm.sort()
    runs = []
    for t in norm:
        if runs and runs[-1][-1][1] == t[0]:
            runs[-1].append(t)
        else:
            runs.append([t])
    out = []
    for run in runs:
        ring = [(run[0][0], run[0][3])]
        for u0, u1, top, _, _ in run:
            ring += [(u0, top), (u1, top)]
        ring.append((run[-1][1], run[-1][4]))
        for u0, _, _, b0, _ in reversed(run):
            ring.append((u0, b0))
        ring = clean(ring)
        if len(ring) >= 3:
            out.append(ring)
    return out


def flight_faces(frame, i, rings, f, covers, edges):
    for p, q, m, _ in outward(rings):
        if not facing(frame, m):
            continue
        view = frame.in_view(p, q)
        if view is None:
            continue
        t0, t1 = view
        trs = traps(frame, f, p, q, t0, t1)
        for k, tr in enumerate(trs):
            u0, u1, top, b0, b1, d0, d1 = tr
            dep = vertical_depth(u0, d0, u1, d1)
            if dep is not None:
                covers.append(cover(trap_ring(tr), dep))
            edges.append((i, (u0, top), (u1, top), d0, d1))
            edges.append((i, (u0, b0), (u1, b1), d0, d1))
            if k + 1 < len(trs) and trs[k + 1][2] != top:
                nxt = trs[k + 1][2]
                edges.append((i, (u1, min(top, nxt)), (u1, max(top, nxt)), d1, d1))
        if trs and t0 == 0:
            u0, _, top, b0, _, d0, _ = trs[0]
            edges.append((i, (u0, b0), (u0, top), d0, d0))
        if trs and t1 == 1:
            _, u1, top, _, b1, _, d1 = trs[-1]
            edges.append((i, (u1, b1), (u1, top), d1, d1))
    across = (-f.dir[1], f.dir[0])
    for (_, low), (frm, high) in zip(f.steps, f.steps[1:]):
        m = (-f.dir[0], -f.dir[1]) if high > low else f.dir
        if high == low or not facing(frame, m):
            continue
        at = (f.start[0] + f.dir[0] * frm, f.start[1] + f.dir[1] * frm)
        for w0, w1 in through(rings, at, across, f.dir, F(-10**9), F(10**9)):
            p = (at[0] + across[0] * w0, at[1] + across[1] * w0)
            q = (at[0] + across[0] * w1, at[1] + across[1] * w1)
            view = frame.in_view(p, q)
            if view is None:
                continue
            t0, t1 = view
            a = (p[0] + (q[0] - p[0]) * t0, p[1] + (q[1] - p[1]) * t0)
            b = (p[0] + (q[0] - p[0]) * t1, p[1] + (q[1] - p[1]) * t1)
            u0, u1, d0, d1 = frame.u(a), frame.u(b), frame.d(a), frame.d(b)
            z0, z1 = min(low, high), max(low, high)
            dep = vertical_depth(u0, d0, u1, d1)
            if dep is not None:
                covers.append(cover([(u0, z0), (u1, z0), (u1, z1), (u0, z1)], dep))
            for z in (z0, z1):
                edges.append((i, (u0, z), (u1, z), d0, d1))
            # Its ends where it meets the flight's sides, never where the view cut it.
            if t0 == 0:
                edges.append((i, (u0, z0), (u0, z1), d0, d0))
            if t1 == 1:
                edges.append((i, (u1, z0), (u1, z1), d1, d1))
    kl = f.dir[0] * frame.look[0] + f.dir[1] * frame.look[1]
    if f.slope == 0 or f.slope * kl >= 0 or f.kink() is None:
        return
    up = 1 if f.slope > 0 else -1
    kink = f.kink()
    plan = [clip_ring(r, lambda p: up * (f.s(p) - kink)) for r in rings]
    bounds = [lambda p: frame.d(p), lambda p: frame.u(p), lambda p: frame.length - frame.u(p)]
    if frame.depth is not None:
        bounds.append(lambda p: frame.depth - frame.d(p))
    for bound in bounds:
        plan = [clip_ring(r, bound) for r in plan if len(r) >= 3]
    s0 = f.s(frame.origin)
    k = frame.right[0] * f.dir[0] + frame.right[1] * f.dir[1]
    base = f.soffit + f.slope * s0
    dep = (-base / (f.slope * kl), -k / kl, 1 / (f.slope * kl))
    for ring in plan:
        if len(ring) < 3:
            continue
        lifted = [((frame.u(p), f.soffit + f.slope * f.s(p)), frame.d(p)) for p in ring]
        covers.append(cover([l[0] for l in lifted], dep))
        n = len(lifted)
        for j in range(n):
            a, b = lifted[j], lifted[(j + 1) % n]
            edges.append((i, a[0], b[0], a[1], b[1]))


# ── Cases ────────────────────────────────────────────────────────────

def rect(x0, y0, x1, y1):
    return [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]


def prism(ring, bottom, top, voids=(), marks=()):
    return {"rings": [ring], "height": {"bottom": F(bottom), "top": F(top)}, "voids": list(voids),
            "marks": list(marks)}


def void(ring, bottom, top):
    return {"ring": ring, "bottom": F(bottom), "top": F(top)}


def outline(x0, y0, x1, y1, bottom, top):
    """An opening's outline on the face from (x0, y0) to (x1, y1), as marks."""
    c = [((F(x0), F(y0)), F(bottom)), ((F(x1), F(y1)), F(bottom)), ((F(x1), F(y1)), F(top)),
         ((F(x0), F(y0)), F(top))]
    return [(c[k], c[(k + 1) % 4]) for k in range(4)]


def flight(ring, start, dir, risers, rise, tread, waist_v, floor, base=F(0)):
    """A straight flight from `start` along `dir`: `risers` risers of `rise` over treads of `tread` (its footprint
    `ring`), its underside `waist_v` under its steps' roots (measured upright), never below `floor`."""
    rise, tread = F(rise), F(tread)
    steps = [(tread * k, base + rise * (k + 1)) for k in range(risers - 1)]
    slope = rise / tread
    return {"rings": [ring], "height": {"flight": {"start": P(*start), "dir": P(*dir), "steps": steps,
                                                   "soffit": base - F(waist_v), "slope": slope,
                                                   "floor": F(floor)}},
            "voids": [], "marks": []}


def cases():
    east = (P(0, 0), P(10, 0), False, None)
    out = []
    out.append(("Duvar kesilir, arkasındaki duvar görünür", east, [
        prism(rect(2, -1, F("2.25"), 4), 0, 3),
        prism(rect(0, 4, 10, F("4.25")), 0, 3),
    ]))
    out.append(("Ters bakış: güneydeki duvar aynalanmış görünür, kuzeydeki görünmez", (P(0, 0), P(10, 0), True, None), [
        prism(rect(0, 4, 10, F("4.25")), 0, 3),
        prism(rect(1, F("-3.25"), 4, -3), 0, 3),
    ]))
    out.append(("Çizginin üstündeki yüz görünür, kesilmez", east, [prism(rect(1, 0, 3, F("0.25")), 0, 3)]))
    out.append(("Pencere: kesitte boşluk, sövelerin çizgisi görünür", east, [
        prism(rect(2, -1, F("2.25"), 4), 0, 3, voids=[void(rect(2, F("-0.6"), F("2.25"), F("0.6")), F("0.9"),
                                                              F("2.1"))],
              marks=outline(2, F("-0.6"), 2, F("0.6"), F("0.9"), F("2.1"))
              + outline(F("2.25"), F("-0.6"), F("2.25"), F("0.6"), F("0.9"), F("2.1"))),
    ]))
    out.append(("Görünen duvardaki pencere: ön yüzündeki çerçeve görünür, arka yüzündeki görünmez", east, [
        prism(rect(0, 4, 10, F("4.25")), 0, 3,
              voids=[void(rect(4, 4, F("5.2"), F("4.25")), F("0.9"), F("2.1"))],
              marks=outline(4, 4, F("5.2"), 4, F("0.9"), F("2.1")) + outline(4, F("4.25"), F("5.2"), F("4.25"),
                                                                             F("0.9"), F("2.1"))),
    ]))
    # The same window with its lines as a solid of their own (no footprint, its marks alone; docs/adr/8043).
    out.append(("Doğramanın çizgileri kendi katısında: ön yüzdeki görünür, arka yüzdeki görünmez", east, [
        prism(rect(0, 4, 10, F("4.25")), 0, 3, voids=[void(rect(4, 4, F("5.2"), F("4.25")), F("0.9"), F("2.1"))]),
        {"rings": [], "height": {"bottom": F("0.9"), "top": F("2.1")}, "voids": [],
         "marks": outline(4, 4, F("5.2"), 4, F("0.9"), F("2.1")) + outline(4, F("4.25"), F("5.2"), F("4.25"),
                                                                          F("0.9"), F("2.1"))},
    ]))
    slab = rect(0, -2, 6, 3)
    out.append(("Döşeme katmanları: kaplama, şap, taşıyıcı ve sıva ayrı ayrı kesilir", east, [
        prism(slab, F("2.98"), 3), prism(slab, F("2.93"), F("2.98")), prism(slab, F("2.78"), F("2.93")),
        prism(slab, F("2.76"), F("2.78")),
    ]))
    out.append(("Kolon ve kiriş kesilir; arkadaki kiriş öndekinin ardında kalır", east, [
        prism(rect(1, F("-0.2"), F("1.4"), F("0.2")), 0, F("2.6")),
        prism(rect(1, F("-0.125"), 6, F("0.125")), F("2.5"), 3),
        prism(rect(1, F("1.875"), 6, F("2.125")), F("2.5"), 3),
    ]))
    # A flight from x = 1 east, 9 risers of 0.17 over treads of 0.28; its underside 0.15·√(1+m²) is not
    # rational, so the upright waist is given (0.18).
    run = rect(1, F("-0.6"), F("3.24"), F("0.6"))
    out.append(("Merdiven boyuna kesilir: basamaklar ve eğik plak", east, [
        flight(run, (1, 0), (1, 0), 9, F("0.17"), F("0.28"), F("0.18"), 0),
    ]))
    out.append(("Merdiven yandan görünür: basamakların dişleri", (P(0, -2), P(5, -2), False, None), [
        flight(run, (1, 0), (1, 0), 9, F("0.17"), F("0.28"), F("0.18"), 0),
    ]))
    out.append(("Merdiven önden görünür: rıhtlar", (P(2, -1), P(2, 1), True, None), [
        flight(run, (1, 0), (1, 0), 9, F("0.17"), F("0.28"), F("0.18"), 0),
    ]))
    out.append(("Merdiven arkadan görünür: plağın altı", (P(4, -1), P(4, 1), False, None), [
        flight(run, (1, 0), (1, 0), 9, F("0.17"), F("0.28"), F("0.18"), 0),
    ]))
    # The line on the fifth riser (1.12 along the run): the riser is the cut's face, its nosing not drawn through it.
    out.append(("Merdiven rıhtın üstünden enine kesilir: basamak burnu kesitin içinden geçmez",
                (P(F("2.12"), -1), P(F("2.12"), 1), True, None), [
        flight(run, (1, 0), (1, 0), 9, F("0.17"), F("0.28"), F("0.18"), 0),
    ]))
    out.append(("Derinlik: uzaktaki duvar görünmez", (P(0, 0), P(10, 0), False, F(3)), [
        prism(rect(0, 2, 10, F("2.25")), 0, 3),
        prism(rect(0, 4, 10, F("4.25")), 0, F("3.5")),
    ]))
    out.append(("Çizginin boyu: dışındaki görünmez, kırpılan yüzün ucu çizilmez", (P(2, 0), P(8, 0), False, None), [
        prism(rect(0, 4, 10, F("4.25")), 0, 3),
        prism(rect(9, 1, 12, 2), 0, 3),
    ]))
    out.append(("Eğik çizgi: 3-4-5 doğrultusunda kesilen duvar", (P(0, 0), P(6, 8), False, None), [
        prism(rect(1, 1, F("1.25"), 6), 0, 3),
        prism(rect(-4, 9, 8, F("9.25")), 0, 3),
    ]))
    out.append(("Önde alçak duvar arkadaki yüksek duvarın altını örter", east, [
        prism(rect(2, 2, 6, F("2.1")), 0, F("1.1")),
        prism(rect(0, 4, 10, F("4.25")), 0, 3),
    ]))
    out.append(("İki kat: duvarlar, döşeme ve üst kattaki pencere", east, [
        prism(rect(0, -1, F("0.25"), 5), 0, F("2.85")),
        prism(rect(0, -1, F("0.25"), 5), 3, F("5.85")),
        prism(rect(0, -1, 6, 5), F("2.85"), 3),
        prism(rect(0, 5, 6, F("5.25")), 0, F("5.85"),
              voids=[void(rect(2, 5, F("3.5"), F("5.25")), F("3.9"), F("5.1"))],
              marks=outline(2, 5, F("3.5"), 5, F("3.9"), F("5.1"))),
    ]))
    return out


def num(x):
    return float(x)


def pt(p):
    return [num(p[0]), num(p[1])]


def doc_solid(s):
    out = {"rings": [[pt(p) for p in r] for r in s["rings"]]}
    h = s["height"]
    if "flight" in h:
        f = h["flight"]
        out["flight"] = {"start": pt(f["start"]), "dir": pt(f["dir"]), "steps": [[num(a), num(b)] for a, b in f["steps"]],
                         "soffit": num(f["soffit"]), "slope": num(f["slope"]), "floor": num(f["floor"])}
    else:
        out["bottom"], out["top"] = num(h["bottom"]), num(h["top"])
    if s["voids"]:
        out["voids"] = [{"ring": [pt(p) for p in v["ring"]], "bottom": num(v["bottom"]), "top": num(v["top"])}
                        for v in s["voids"]]
    if s["marks"]:
        out["marks"] = [[[*pt(a[0]), num(a[1])], [*pt(b[0]), num(b[1])]] for a, b in s["marks"]]
    return out


def build():
    result = []
    for name, (a, b, flip, depth), solids in cases():
        frame = Frame(a, b, flip, depth)
        cuts, seen = section(frame, solids)
        cuts.sort(key=lambda c: (c["solid"], c["ring"][0][0], c["ring"][0][1]))
        seen.sort(key=lambda s: (s["solid"], s["a"], s["b"]))
        case = {"name": name, "line": {"a": pt(a), "b": pt(b)}, "solids": [doc_solid(s) for s in solids],
                "expect": {"cut": [{"solid": c["solid"], "ring": [pt(p) for p in c["ring"]]} for c in cuts],
                           "seen": [{"solid": s["solid"], "a": pt(s["a"]), "b": pt(s["b"]), "depth": num(s["depth"])}
                                    for s in seen]}}
        if flip:
            case["line"]["flip"] = True
        if depth is not None:
            case["line"]["depth"] = num(depth)
        result.append(case)
    return {"format": "kentos.sections", "version": 1, "cases": result}


def main():
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--check", action="store_true")
    a = p.parse_args()
    doc = build()
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if a.check:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} başvuruyla aynı değil", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum başvuruyla aynı.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, "utf-8")
    print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
