#!/usr/bin/env python3
"""Walls where they meet (docs/adr/0340 §2), from the ADR's rules, in exact fractions, without KentOS code.

Writes fixtures/walls/v1/joins.json; `--check` compares and writes nothing. The shared core's test
(`kentos-geometry-core`, tests/all/wall_joins.rs) reads it and must give the same joints, issues, lines, caps and bands.

The walls here are straight (their paths of straight edges), so every line is the path's offset polyline, every cut a
crossing of two lines, every length exact. The rules, as ADR 0340 §2 gives them:

- A wall's lines are its boundaries' offsets (ADR 0339 §1): the left face `offset + {0, w/2, w}` left of the path, the
  components' boundaries towards the right face `w` further right; an offset polyline's vertex where the offsets of
  its two edges meet.
- Ends whose path ends coincide: two at an angle a corner; anti-parallel (1°) an inline join; within 3° of one
  direction an overlap (said, the second end's fix `none`); sharper than 15° with both ends automatic an acute corner
  (said, the first end's fix `butt`). A corner's mitre line goes through where the first wall's most-left line (left
  of the direction into it) meets the second's most-right, and the first's most-right meets the second's most-left;
  every line of both is cut there (moved along its end edge). An inline join cuts both across, square to the first's
  direction, at the point; their caps go when their faces lie as far to either side. `butt` (or the other's
  `through`): the through wall's lines cut at the butting wall's face farther along the through wall's direction,
  then the butting wall tees into it. Three ends: the anti-parallel pair joins inline, the third tees into both;
  without such a pair it is said (crowded, the shortest wall's end `none`).
- An end inside another wall (square to its path, between its faces, not at an open host's end): a tee; within 3° of
  the host's direction an overlap (said once for the two walls, the end's fix `none`). From the near face (the one on the side
  the joining wall comes from) the host's components in order; each joining component passes the host's while theirs
  is a larger number, `cleanup` on both; each joining line's depth the larger of its two components' (a face its
  one's); it is cut at the host's line at that depth. The host's near face is interrupted between the joining wall's
  faces; its line at depth m ≥ 1 between the outer lines of each run of joining components that pass it (depth > m)
  or stop at it at the same priority; its band between depths q and q + 1 under each run that passes it (cut on both
  of its lines). A host line's interval runs between where the two joining lines' carriers cross it.
- Paths crossing away from both walls' ends: a cross; the host the wall whose strongest component is the stronger,
  then the wider, then the first. The crosser is two tees from both sides: each of its lines interrupted between its
  crossings with the host lines at its depths from either side (none when it passes the whole host from both), each
  of its bands likewise; the host's lines and bands as for two tees.
- A cut that cannot be made (a line shorter than the trim) leaves both walls as they were: said, the end's fix `none`.
"""

import argparse
import json
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/walls/v1/joins.json"
DEFAULT = 3


def sub(a, b):
    return (a[0] - b[0], a[1] - b[1])


def add(a, b):
    return (a[0] + b[0], a[1] + b[1])


def mul(a, k):
    return (a[0] * k, a[1] * k)


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1]


def cross(a, b):
    return a[0] * b[1] - a[1] * b[0]


def left(t):
    return (-t[1], t[0])


def length2(v):
    return dot(v, v)


class Wall:
    def __init__(self, pts, width, justify="center", offset=0, comps=None, closed=False, joins=("auto", "auto"),
                 cleanup=True, caps="closed", wraps=None, passes=(None, None), holes=None):
        self.pts = [(F(x), F(y)) for x, y in pts]
        self.width = F(width)
        self.justify = justify
        self.offset = F(offset)
        self.comps = comps or []          # [(thickness or None, priority or None)]
        self.closed = closed
        self.joins = list(joins)
        self.cleanup = cleanup
        self.caps = caps
        # ADR 0771: the components that turn at the free ends and the holes' jambs, the ends' node decisions, and the
        # through holes along the base line (s0, s1).
        self.wraps = list(wraps) if wraps else [False] * len(self.comps)
        self.passes = list(passes)
        self.holes = [(F(h[0]), F(h[1])) for h in (holes or [])]
        # ADR 0785 §5: a hole's frame across the wall (depth, offset from the face it opens to or None, opens right).
        self.frames = [h[2] if len(h) > 2 else None for h in (holes or [])]

    def frame_band(self, h):
        """The hole's frame as distances to the path's left (top, bottom), or None (the door's frame rule)."""
        f = self.frames[h]
        if f is None:
            return None
        depth, offset, right = F(f[0]), (None if f[1] is None else F(f[1])), f[2]
        t = self.width
        if not (0 < depth < t):
            return None
        near = (t - depth) / 2 if offset is None else offset
        near = min(max(near, F(0)), t - depth)
        dl, dr = (t - near - depth, t - near) if right else (near, near + depth)
        lf = self.boundaries()[0]
        return lf - dl, lf - dr

    def json(self):
        out = {"pts": [[float(x), float(y)] for x, y in self.pts], "width": float(self.width), "justify": self.justify}
        if self.offset:
            out["offset"] = float(self.offset)
        if self.closed:
            out["closed"] = True
        if self.comps:
            out["components"] = [{k: v for k, v in (("thickness", None if t is None else float(t)), ("priority", p),
                                                    ("detail", "standard")) if v is not None} for t, p in self.comps]
            for c, w in zip(out["components"], self.wraps):
                if w:
                    c["wrap"] = True
        if self.joins != ["auto", "auto"] or any(p is not None for p in self.passes):
            out["joins"] = {k: m for k, m in zip(("start", "end"), self.joins) if m != "auto"}
            for k, p in zip(("startPass", "endPass"), self.passes):
                if p is not None:
                    out["joins"][k] = list(p)
        if not self.cleanup:
            out["cleanup"] = False
        if self.caps == "open":
            out["caps"] = "open"
        return out

    def boundaries(self):
        lf = self.offset + {"left": 0, "center": self.width / 2, "right": self.width}[self.justify]
        out = [lf]
        if self.comps:
            fixed = sum(t for t, _ in self.comps if t is not None)
            rest = self.width - fixed
            at = lf
            for t, _ in self.comps[:-1]:
                at -= t if t is not None else rest
                out.append(at)
        out.append(lf - self.width)
        return out

    def ranks(self):
        return [p if p is not None else DEFAULT for _, p in self.comps] if self.comps else [DEFAULT]


def unit_dir(a, b):
    """A direction a→b, scaled so its larger coordinate is 1 (exact; only its sign and ratio matter)."""
    d = sub(b, a)
    m = max(abs(d[0]), abs(d[1]))
    return (d[0] / m, d[1] / m)


def meet(p, u, q, v):
    """Where the line through p along u meets the line through q along v; None when parallel."""
    den = cross(u, v)
    if den == 0:
        return None
    t = cross(sub(q, p), v) / den
    return add(p, mul(u, t))


def offset_path(pts, closed, d):
    """The path offset d to its left: each edge moved along its left normal (scaled by its length), vertices where
    neighbouring offsets meet."""
    n = len(pts)
    edges = [(pts[i], pts[(i + 1) % n]) for i in range(n if closed else n - 1)]

    out = []
    norms = []
    for a, b in edges:
        t = sub(b, a)
        ln = exact_len(t)
        nrm = mul(left(t), d / ln)
        norms.append((add(a, nrm), add(b, nrm), t))
    m = len(norms)
    if closed:
        for i in range(m):
            p0, _, t0 = norms[i - 1]
            p1, _, t1 = norms[i]
            out.append(meet(p0, t0, p1, t1))
        # Vertex i is the start of edge i: the meeting of edge i-1 and i.
        return out
    out.append(norms[0][0])
    for i in range(1, m):
        p0, _, t0 = norms[i - 1]
        p1, _, t1 = norms[i]
        out.append(meet(p0, t0, p1, t1))
    out.append(norms[-1][1])
    return out


def exact_len(t):
    """|t| exactly: the cases' directions are Pythagorean (x² + y² a square of a fraction)."""
    import math
    q = t[0] * t[0] + t[1] * t[1]
    n, d = math.isqrt(q.numerator), math.isqrt(q.denominator)
    if n * n != q.numerator or d * d != q.denominator:
        raise ValueError(f"{t}: not a Pythagorean direction")
    return F(n, d)


def path_len(pts, closed):
    n = len(pts)
    return sum(exact_len(sub(pts[(i + 1) % n], pts[i])) for i in range(n if closed else n - 1))


def s_of(pts, closed, p):
    """The arc length along the polyline of a point on it."""
    n = len(pts)
    s = 0
    for i in range(n if closed else n - 1):
        a, b = pts[i], pts[(i + 1) % n]
        t = sub(b, a)
        if cross(sub(p, a), t) == 0:
            u = dot(sub(p, a), t) / length2(t)
            if 0 <= u <= 1:
                return s + u * exact_len(t)
        s += exact_len(t)
    raise ValueError("point off the line")


def point_at(pts, closed, s):
    n = len(pts)
    total = path_len(pts, closed)
    if closed:
        s = s % total
    for i in range(n if closed else n - 1):
        a, b = pts[i], pts[(i + 1) % n]
        ln = exact_len(sub(b, a))
        if s <= ln:
            return add(a, mul(sub(b, a), s / ln))
        s -= ln
    return pts[-1] if not closed else pts[0]


def sub_line(pts, closed, s0, s1):
    """The stretch from s0 to s1 (s1 past the length on a closed line runs on round)."""
    n = len(pts)
    out = [point_at(pts, closed, s0)]
    cum = 0
    rounds = 2 if closed else 1
    for r in range(rounds):
        for i in range(n if closed else n - 1):
            a, b = pts[i], pts[(i + 1) % n]
            ln = exact_len(sub(b, a))
            cum += ln
            if s0 < cum < s1:
                out.append(b)
    out.append(point_at(pts, closed, s1))
    return dedupe(out)


def dedupe(pts):
    out = []
    for p in pts:
        if not out or out[-1] != p:
            out.append(p)
    return out


def line_crossing(pts, closed, p, u):
    """Where the line through p along u crosses the polyline: (s, point), all of them."""
    n = len(pts)
    out = []
    s = 0
    for i in range(n if closed else n - 1):
        a, b = pts[i], pts[(i + 1) % n]
        t = sub(b, a)
        den = cross(t, u)
        ln = exact_len(t)
        if den != 0:
            k = cross(sub(p, a), u) / den
            if 0 <= k <= 1:
                out.append((s + k * ln, add(a, mul(t, k))))
        s += ln
    return out


def end_point(line, end):
    return line[0] if end == 0 else line[-1]


def end_dir(line, end):
    """The direction of a line's end edge, travel at its end."""
    return sub(line[1], line[0]) if end == 0 else sub(line[-1], line[-2])


def cut(line, end, tp, tu):
    """The line cut at `end` where its end edge's carrier meets the line through tp along tu."""
    if end == 0:
        p, q = line[1], line[0]
    else:
        p, q = line[-2], line[-1]
    x = meet(q, sub(q, p), tp, tu)
    if x is None:
        return None
    # The new end must stay on the end edge's side away from the rest: past p along p→q.
    if dot(sub(x, p), sub(q, p)) <= 0:
        return None
    out = list(line)
    if end == 0:
        out[0] = x
    else:
        out[-1] = x
    return out


class Solver:
    def __init__(self, walls):
        self.w = walls
        self.lines = [[offset_path(w.pts, w.closed, d) for d in w.boundaries()] for w in walls]
        self.joined = [[False, False] for _ in walls]
        self.gaps = [[[] for _ in ls] for ls in self.lines]
        self.cuts = [[[] for _ in range(len(w.comps))] for w in walls]
        self.joints = []
        # The joints with their walls (index, end or None), as the core lists them.
        self.member_joints = []
        self.issues = []
        self.tees = []
        self.nodes = []

    def joint(self, kind, at, members):
        self.joints.append({"kind": kind, "at": [float(at[0]), float(at[1])]})
        self.member_joints.append({"kind": kind, "at": [float(at[0]), float(at[1])],
                                   "members": [(w, None if e is None else ["start", "end"][e]) for w, e in members]})

    def ends(self):
        out = []
        for i, w in enumerate(self.w):
            if w.closed:
                continue
            for e in (0, 1):
                if w.joins[e] == "none":
                    continue
                at = w.pts[0] if e == 0 else w.pts[-1]
                d = sub(w.pts[1], w.pts[0]) if e == 0 else sub(w.pts[-2], w.pts[-1])
                out.append((i, e, at, d))
        return out

    def faces(self, i, e):
        n = len(self.lines[i]) - 1
        return (0, n) if e == 0 else (n, 0)

    def rel(self, i, e):
        d = self.w[i].boundaries()
        return (d[0], d[-1]) if e == 0 else (-d[-1], -d[0])

    def issue(self, code, at, walls, fix):
        self.issues.append({"code": code, "at": [float(at[0]), float(at[1])], "walls": walls, "fix": fix})

    def cut_all(self, i, e, tp, tu):
        out = [cut(l, e, tp, tu) for l in self.lines[i]]
        return None if any(x is None for x in out) else out

    def corner(self, a, b, tees):
        ia, ea, at, da = a
        ib, eb, _, db = b
        cosn = dot(da, db)
        la2, lb2 = length2(da), length2(db)
        # cos² against the thresholds, keeping the sign: cos > cos 3°, cos < -cos 1°, angle < 15°.
        c2 = cosn * cosn / (la2 * lb2)
        import math
        if cosn > 0 and c2 > F(math.cos(math.radians(3.0)) ** 2):
            self.issue("wall_join_overlap", at, [ia, ib], {"wall": ib, "end": ["start", "end"][eb], "mode": "none"})
            return
        if cosn < 0 and c2 > F(math.cos(math.radians(1.0)) ** 2):
            self.inline(a, b)
            return
        ma, mb = self.w[ia].joins[ea], self.w[ib].joins[eb]
        butt_a = (ma == "butt" and mb != "butt") or (mb == "through" and ma != "through")
        butt_b = (mb == "butt" and ma != "butt") or (ma == "through" and mb != "through")
        if butt_a != butt_b:
            self.butt(a, b, tees) if butt_a else self.butt(b, a, tees)
            return
        if cosn > 0 and c2 > F(math.cos(math.radians(15.0)) ** 2) and ma == "auto" and mb == "auto":
            self.issue("wall_join_acute", at, [ia, ib], {"wall": ia, "end": ["start", "end"][ea], "mode": "butt"})
            return
        (amax, amin), (bmax, bmin) = self.faces(ia, ea), self.faces(ib, eb)
        la, lb = self.lines[ia], self.lines[ib]
        x1 = meet(end_point(la[amax], ea), end_dir(la[amax], ea), end_point(lb[bmin], eb), end_dir(lb[bmin], eb))
        x2 = meet(end_point(la[amin], ea), end_dir(la[amin], ea), end_point(lb[bmax], eb), end_dir(lb[bmax], eb))
        tu = sub(x2, x1)
        na = self.cut_all(ia, ea, x1, tu)
        if na is None:
            self.issue("wall_join_unresolved", at, [ia, ib], {"wall": ia, "end": ["start", "end"][ea], "mode": "none"})
            return
        old = self.lines[ia]
        self.lines[ia] = na
        nb = self.cut_all(ib, eb, x1, tu)
        if nb is None:
            self.lines[ia] = old
            self.issue("wall_join_unresolved", at, [ib, ia], {"wall": ib, "end": ["start", "end"][eb], "mode": "none"})
            return
        self.lines[ib] = nb
        self.joined[ia][ea] = self.joined[ib][eb] = True
        self.joint("corner", at, [(ia, ea), (ib, eb)])

    def inline(self, a, b):
        ia, ea, at, da = a
        ib, eb, _, _ = b
        n = left(da)
        old = self.lines[ia]
        na = self.cut_all(ia, ea, at, n)
        if na is None:
            self.issue("wall_join_unresolved", at, [ia, ib], {"wall": ia, "end": ["start", "end"][ea], "mode": "none"})
            return
        self.lines[ia] = na
        nb = self.cut_all(ib, eb, at, n)
        if nb is None:
            self.lines[ia] = old
            self.issue("wall_join_unresolved", at, [ib, ia], {"wall": ib, "end": ["start", "end"][eb], "mode": "none"})
            return
        self.lines[ib] = nb
        (al, ar), (bl, br) = self.rel(ia, ea), self.rel(ib, eb)
        if al + br == 0 and ar + bl == 0:
            self.joined[ia][ea] = self.joined[ib][eb] = True
        self.joint("inline", at, [(ia, ea), (ib, eb)])

    def butt(self, butting, through, tees):
        ia, ea, at, da = butting
        ib, eb, _, db = through
        la = self.lines[ia]
        last = len(la) - 1
        far = 0 if dot(sub(end_point(la[0], ea), at), db) <= dot(sub(end_point(la[last], ea), at), db) else last
        nb = self.cut_all(ib, eb, end_point(la[far], ea), end_dir(la[far], ea))
        # The through wall's end line stays: it closes the corner's outside.
        self.lines[ib] = nb
        self.joint("corner", at, [(ia, ea), (ib, eb)])
        tees.append((butting, [ib], False))

    def host_of(self, end):
        i, e, at, d = end
        best = None
        parallel = []
        for h, w in enumerate(self.w):
            if h == i:
                continue
            n = len(w.pts)
            for k in range(n if w.closed else n - 1):
                a, b = w.pts[k], w.pts[(k + 1) % n]
                t = sub(b, a)
                u = dot(sub(at, a), t) / length2(t)
                if not 0 <= u <= 1:
                    continue
                # Not at an open host's end.
                if not w.closed and ((k == 0 and u == 0) or (k == n - 2 and u == 1)):
                    continue
                foot = add(a, mul(t, u))
                across = cross(t, sub(at, foot)) / exact_len(t)
                bd = w.boundaries()
                if not bd[-1] <= across <= bd[0]:
                    continue
                # Within 3° of the host's direction: the two overlap, said once for the pair; no tee. At a vertex of
                # the host (ADR 0771 §1) an edge the end runs along does not count when another edge there does not.
                import math
                c2 = dot(d, t) ** 2 / (length2(d) * length2(t))
                if c2 > F(math.cos(math.radians(3.0)) ** 2):
                    parallel.append(h)
                    continue
                central = abs(across - (bd[0] + bd[-1]) / 2)
                if best is None or central < best[0]:
                    best = (central, h, t)
        if best is None:
            for h in parallel[:1]:
                if not any(x["code"] == "wall_join_overlap" and i in x["walls"] and h in x["walls"] for x in self.issues):
                    self.issue("wall_join_overlap", at, [i, h], {"wall": i, "end": ["start", "end"][e], "mode": "none"})
            return None
        return best[1]

    def host_side(self, h, at, d):
        w = self.w[h]
        n = len(w.pts)
        t = None
        for k in range(n if w.closed else n - 1):
            a, b = w.pts[k], w.pts[(k + 1) % n]
            tt = sub(b, a)
            u = dot(sub(at, a), tt) / length2(tt)
            if 0 <= u <= 1:
                foot = add(a, mul(tt, u))
                # At a vertex (two edges as near), the edge least along the end's direction (ADR 0771 §1).
                key = (length2(sub(at, foot)), dot(d, tt) ** 2 / length2(tt))
                if t is None or key < t[0]:
                    t = (key, tt)
        is_left = dot(d, left(t[1])) > 0
        nh = len(self.lines[h]) - 1
        ranks = self.w[h].ranks()
        if is_left:
            return list(range(nh + 1)), ranks
        return list(range(nh, -1, -1)), list(reversed(ranks))

    def depths(self, j, ranks, cleanup):
        return [sum(1 for _ in iter_while(ranks, p)) if cleanup else 0 for p in self.w[j].ranks()]

    def tee(self, end, hosts, listed):
        j, e, at, d = end
        h = hosts[0]
        pos, ranks = self.host_side(h, at, d)
        # ADR 0771 §1: the user's node decision on the joining end wins, each pass at most the host's components; the
        # host's lines are then interrupted as with cleanup on.
        user = self.w[j].passes[e]
        if user is not None and len(user) != len(self.w[j].ranks()):
            user = None
        cleanup = user is not None or (self.w[j].cleanup and self.w[h].cleanup)
        depth = [min(x, len(ranks)) for x in user] if user is not None else self.depths(j, ranks, cleanup)
        if listed:
            self.nodes.append({"at": [float(at[0]), float(at[1])], "depth": depth, "user": user is not None})
        nj = len(self.lines[j]) - 1
        out = []
        sides = [(h, pos)] + [(o, self.host_side(o, at, d)[0]) for o in hosts[1:]]
        for k, l in enumerate(self.lines[j]):
            dk = depth[0] if k == 0 else depth[nj - 1] if k == nj else max(depth[k - 1], depth[k])
            # The host lines' edges (each host's at the depth, from its own side) the joining line's carrier meets,
            # nearest its end.
            p, q = (l[1], l[0]) if e == 0 else (l[-2], l[-1])
            hits = [x for w, pw in sides for x in line_crossing(self.lines[w][pw[dk]], self.w[w].closed, q, sub(q, p))]
            if not hits:
                self.issue("wall_join_unresolved", at, [j] + hosts, {"wall": j, "end": ["start", "end"][e], "mode": "none"})
                return
            x = min(hits, key=lambda hp: length2(sub(hp[1], q)))[1]
            if dot(sub(x, p), sub(q, p)) <= 0:
                self.issue("wall_join_unresolved", at, [j] + hosts, {"wall": j, "end": ["start", "end"][e], "mode": "none"})
                return
            nl = list(l)
            if e == 0:
                nl[0] = x
            else:
                nl[-1] = x
            out.append(nl)
        self.lines[j] = out
        self.joined[j][e] = True
        if listed:
            self.joint("tee", at, [(j, e)] + [(h, None) for h in hosts])
        self.tees.append((j, at, d, hosts, depth, cleanup))

    def crossing_s(self, host_line, closed, other, at):
        """Where `other`'s carrier (through its points, both ways) crosses the host line, the crossing nearest `at`."""
        best = None
        for k in range(len(other) - 1):
            p, q = other[k], other[k + 1]
            for s, x in line_crossing(host_line, closed, p, sub(q, p)):
                dd = length2(sub(x, at))
                if best is None or dd < best[0]:
                    best = (dd, s)
        return None if best is None else best[1]

    def host_cuts(self, h, j, at, d, depth, cleanup):
        pos, ranks = self.host_side(h, at, d)
        jr = self.w[j].ranks()
        lh, lj = self.lines[h], self.lines[j]
        nh, nc = len(lh) - 1, len(depth)
        closed = self.w[h].closed

        def runs(open_):
            out, c = [], 0
            while c < nc:
                if open_(c):
                    f = c
                    while c < nc and open_(c):
                        c += 1
                    out.append((f, c))
                else:
                    c += 1
            return out

        def span(line, lo, hi):
            """Between the two joining lines' crossings; one missing (a host of two joined inline, the other's part),
            to the line's end nearest the joint."""
            a = self.crossing_s(line, closed, lj[lo], at)
            b = self.crossing_s(line, closed, lj[hi], at)
            if a is not None and b is not None:
                return (a, b, (a + b) / 2)
            if closed or (a is None and b is None):
                return None
            end = F(0) if length2(sub(line[0], at)) <= length2(sub(line[-1], at)) else path_len(line, False)
            return (a if a is not None else b, end, s_near(line, closed, at))

        for m in range(nh):
            for lo, hi in runs(lambda c: m == 0 or (cleanup and (depth[c] > m or (depth[c] == m and ranks[m] == jr[c])))):
                sp = span(lh[pos[m]], lo, hi)
                if sp:
                    self.gaps[h][pos[m]].append(sp)
        if len(self.cuts[h]) != nh:
            return
        for q in range(nh):
            for lo, hi in runs(lambda c: cleanup and depth[c] > q):
                near, far = pos[q], pos[q + 1]
                sn, sf = span(lh[near], lo, hi), span(lh[far], lo, hi)
                if not sn or not sf:
                    continue
                comp = min(near, far)
                self.cuts[h][comp].append((sn, sf) if near < far else (sf, sn))

    def cross(self, host, crosser, at):
        w = self.w[crosser]
        t = unit_dir(w.pts[0], w.pts[-1])
        cleanup = self.w[host].cleanup and w.cleanup
        sides = []
        for dd in (mul(t, -1), t):
            pos, ranks = self.host_side(host, at, dd)
            sides.append((dd, pos, self.depths(crosser, ranks, cleanup)))
        lh, lc = self.lines[host], self.lines[crosser]
        nh, nc = len(lh) - 1, len(sides[0][2])

        def ld(dp, k):
            return dp[0] if k == 0 else dp[nc - 1] if k == nc else max(dp[k - 1], dp[k])

        def gap(k, da, db):
            if da >= nh and db >= nh:
                return None
            a = self.crossing_s(lc[k], w.closed, lh[sides[0][1][da]], at)
            b = self.crossing_s(lc[k], w.closed, lh[sides[1][1][db]], at)
            return (a, b, (a + b) / 2)

        for k in range(nc + 1):
            g = gap(k, ld(sides[0][2], k), ld(sides[1][2], k))
            if g:
                self.gaps[crosser][k].append(g)
        if len(self.cuts[crosser]) == nc:
            for c in range(nc):
                da, db = sides[0][2][c], sides[1][2][c]
                ga, gb = gap(c, da, db), gap(c + 1, da, db)
                if ga and gb:
                    self.cuts[crosser][c].append((ga, gb))
        for dd, _, depth in sides:
            self.host_cuts(host, crosser, at, dd, depth, cleanup)

    def solve(self):
        ends = self.ends()
        groups = {}
        for x in ends:
            groups.setdefault(x[2], []).append(x)
        tees, singles = [], []
        for g in groups.values():
            if len(g) == 1:
                singles.append(g[0])
            elif len(g) == 2:
                self.corner(g[0], g[1], tees)
            else:
                pair = None
                for a in range(len(g)):
                    for b in range(a + 1, len(g)):
                        if dot(g[a][3], g[b][3]) < 0 and cross(g[a][3], g[b][3]) == 0:
                            pair = (a, b)
                if pair is None:
                    shortest = min(g, key=lambda x: path_len(self.w[x[0]].pts, False))
                    self.issue("wall_join_crowded", g[0][2], [x[0] for x in g],
                               {"wall": shortest[0], "end": ["start", "end"][shortest[1]], "mode": "none"})
                    continue
                self.inline(g[pair[0]], g[pair[1]])
                for k, x in enumerate(g):
                    if k not in pair:
                        tees.append((x, [g[pair[0]][0], g[pair[1]][0]], True))
        for x in singles:
            h = self.host_of(x)
            if h is not None:
                tees.append((x, [h], True))
        for end, hosts, listed in tees:
            self.tee(end, hosts, listed)
        crosses = []
        for i in range(len(self.w)):
            for j in range(i + 1, len(self.w)):
                for p in path_crossings(self.w[i], self.w[j]):
                    clear = self.w[i].width + self.w[j].width + abs(self.w[i].offset) + abs(self.w[j].offset)
                    ends_ = [w.pts[k] for w in (self.w[i], self.w[j]) if not w.closed for k in (0, -1)]
                    if any(length2(sub(p, e)) <= clear * clear for e in ends_):
                        continue
                    si = (min(self.w[i].ranks()), -self.w[i].width, i)
                    sj = (min(self.w[j].ranks()), -self.w[j].width, j)
                    host, crosser = (i, j) if si < sj else (j, i)
                    crosses.append((host, crosser, p))
                    self.joint("cross", p, [(host, None), (crosser, None)])
        for j, at, d, hosts, depth, cleanup in self.tees:
            for h in hosts:
                self.host_cuts(h, j, at, d, depth, cleanup)
        for host, crosser, p in crosses:
            self.cross(host, crosser, p)
        return self.drawn()

    def drawn(self):
        out = []
        for i, w in enumerate(self.w):
            ls = self.lines[i]
            n = len(ls) - 1
            plan = wrap_plan(w)
            base = w.pts[0], w.pts[-1]
            length = exact_len(sub(base[1], base[0])) if len(w.pts) == 2 else None
            o = w.boundaries()

            def beside(sv, dv):
                # The point `dv` to the left of the (single-edge) base line at arc length `sv`.
                t = sub(base[1], base[0])
                u = mul(t, F(1) / exact_len(t))
                return add(add(base[0], mul(u, sv)), mul(left(u), dv))

            def span(k, s0, s1):
                return (s_near(ls[k], w.closed, beside(s0, o[k])), s_near(ls[k], w.closed, beside(s1, o[k])),
                        s_near(ls[k], w.closed, beside((s0 + s1) / 2, o[k])))

            gaps = [list(g) for g in self.gaps[i]]
            cuts = [list(c) for c in self.cuts[i]]
            extra = [[] for _ in range(len(self.cuts[i]))]
            closures = []
            caps = []
            if not w.closed and w.caps != "open":
                for e in (0, 1):
                    if not self.joined[i][e]:
                        caps.append([pt(end_point(ls[0], e)), pt(end_point(ls[-1], e))])
            # (end along the base, direction into the wall) of each end the layers turn at.
            sides = []
            if plan and length is not None and not w.closed and w.caps != "open" and length > 2 * plan["reach"]:
                for e, (at, into) in enumerate(((F(0), 1), (length, -1))):
                    if not self.joined[i][e]:
                        sides.append((at, into))
                        for k in range(n + 1):
                            if plan["pull"][k] > 0:
                                a, b = sorted((at, at + into * plan["pull"][k]))
                                gaps[k].append(span(k, a, b))
                        for c in range(n):
                            if plan["band"][c] > 0:
                                a, b = sorted((at, at + into * plan["band"][c]))
                                cuts[c].append((span(c, a, b), span(c + 1, a, b)))
            sides = [(at, into, plan) for at, into in sides]
            for h, (s0, s1) in enumerate(w.holes):
                wrapped = plan is not None and s0 - plan["reach"] > 0 and s1 + plan["reach"] < length and all(
                    t1 + plan["reach"] < s0 - plan["reach"] or t0 - plan["reach"] > s1 + plan["reach"]
                    for g, (t0, t1) in enumerate(w.holes) if g != h)
                # ADR 0785 §5: a framed jamb turns its layers up to the frame's faces.
                band = w.frame_band(h)
                own = (wrap_plan(w, band) if band is not None and wrapped else None) or plan
                for k in range(n + 1):
                    p = own["pull"][k] if wrapped else 0
                    gaps[k].append(span(k, s0 - p, s1 + p))
                for c in range(n):
                    p = own["band"][c] if wrapped else 0
                    cuts[c].append((span(c, s0 - p, s1 + p), span(c + 1, s0 - p, s1 + p)))
                if wrapped:
                    sides.append((s0, -1, own))
                    sides.append((s1, 1, own))
            for at, into, plan in sides:
                for comp, p0, p1, upper, lower in plan["patches"]:
                    a, b = sorted((at + into * p0, at + into * p1))
                    ring = [beside(a, upper), beside(b, upper), beside(b, lower), beside(a, lower)]
                    extra[comp].append(float(area(ring)))
                for comp, pull, upper, lower in plan["closures"]:
                    sv = at + into * pull
                    closures.append([comp, [pt(beside(sv, upper)), pt(beside(sv, lower))]])
                for comp, p0, p1, d in plan["middle"]:
                    a, b = sorted((at + into * p0, at + into * p1))
                    closures.append([comp, [pt(beside(a, d)), pt(beside(b, d))]])
            for s0, s1 in w.holes:
                caps.append([pt(beside(s0, o[0])), pt(beside(s0, o[-1]))])
                caps.append([pt(beside(s1, o[0])), pt(beside(s1, o[-1]))])
            strokes = [kept_strokes(l, w.closed, gaps[k]) for k, l in enumerate(ls)]
            bands = []
            for c in range(len(cuts)):
                bands.append(band_areas(ls[c], ls[c + 1], w.closed, cuts[c]) + extra[c])
            d = {"lines": strokes, "caps": caps, "bands": bands}
            if plan or w.holes:
                d["closures"] = closures
            out.append(d)
        return out


def wrap_plan(w, frame=None):
    """ADR 0771 §3: how the outermost turning layers of each face turn at an end, or None. Lines and components from
    the left face; `pull` how far before the end a line stops, `band` a component's band; the turns (comp, from, to,
    upper, lower) as pulls; the closing lines (comp, pull, upper, lower) across; the lines where the turns stop (comp,
    from, to, at). ADR 0785 §5: at a jamb with a frame between `top` and `bottom` (distances to the path's left) the left
    layers turn down to its top face, the right ones up to its bottom face, the inner components run to the jamb behind
    it; a frame not between the faces' turning layers gives None (the caller turns to the middle)."""
    n = len(w.comps)
    if n < 2 or not any(w.wraps):
        return None
    o = w.boundaries()
    t = [o[k] - o[k + 1] for k in range(n)]
    L = next((k for k in range(n) if not w.wraps[k]), n)
    R = next((k for k in range(n) if not w.wraps[n - 1 - k]), n)
    if L + R >= n:
        return None
    a = [F(0)] * (n + 1)
    for j in range(L):
        a[j + 1] = a[j] + t[j]
    b = [F(0)] * (n + 1)
    for k in range(n - 1, n - R - 1, -1):
        b[k] = b[k + 1] + t[k]
    TL, TR = a[L], b[n - R]
    if frame is None:
        m = (o[L] + o[n - R]) / 2 if L and R else (o[n] if L else o[0])
        top = bottom = m
    else:
        top, bottom = frame
        if not (top >= bottom and (L == 0 or top <= o[L]) and (R == 0 or bottom >= o[n - R])):
            return None

    def inner(d):
        over, under = d >= top, d <= bottom
        return max(TL, TR) if over and under else TL if over else TR if under else F(0)
    pull = [F(0) if k in (0, n) else a[k] if k <= L else b[k] if k >= n - R else inner(o[k]) for k in range(n + 1)]
    band, patches = [], []
    for c in range(n):
        if c < L:
            band.append(a[c])
            patches.append((c, a[c], a[c + 1], o[c + 1], top))
        elif c >= n - R:
            band.append(b[c + 1])
            patches.append((c, b[c + 1], b[c], bottom, o[c]))
        else:
            pieces = []
            if o[c] > top:
                pieces.append((o[c], max(o[c + 1], top), TL))
            hi, lo = min(o[c], top), max(o[c + 1], bottom)
            if hi > lo:
                pieces.append((hi, lo, F(0)))
            if o[c + 1] < bottom:
                pieces.append((min(o[c], bottom), o[c + 1], TR))
            deepest = max([F(0)] + [p for _, _, p in pieces])
            band.append(deepest)
            for upper, lower, p in pieces:
                if p < deepest:
                    patches.append((c, p, deepest, upper, lower))
    closures = [(j - 1, a[j], o[j], top) for j in range(1, L + 1)] + [(k, b[k], o[k], bottom) for k in range(n - R, n)]
    middle = []
    if frame is None or top == bottom:
        if L and R:
            middle.append((L - 1, F(0), max(TL, TR), top))
        elif frame is not None and L:
            middle.append((L - 1, F(0), TL, top))
        elif frame is not None and R:
            middle.append((n - R, F(0), TR, bottom))
    else:
        if L:
            middle.append((L - 1, F(0), TL, top))
        if R:
            middle.append((n - R, F(0), TR, bottom))
    return {"pull": pull, "band": band, "patches": patches, "closures": closures, "middle": middle,
            "reach": max(TL, TR)}


# What each issue says, as ADR 0340 §2 words it: what is wrong, and what resolves it.
MESSAGES = {
    "wall_join_overlap": ("İki duvar aynı doğrultuda üst üste geliyor; birleştirilmedi.",
                          "Duvarlardan birinin ucunu Yok yapın ya da duvarları Birleştir ile tek duvar yapın."),
    "wall_join_crowded": ("Bu noktada ikiden çok duvar ucu buluşuyor ve ikisi bir doğru üzerinde değil; birleştirilmedi.",
                          "En kısa duvarın ucunu Yok yapın ya da duvarlardan birini diğerinin ortasına uzatın."),
    "wall_join_acute": ("Köşe çok dar; gönye birleşimi duvarın çok dışına taşar, birleştirilmedi.",
                        "Bir ucu Alın yapın: o duvar diğerinin yüzünde durur."),
    "wall_join_unresolved": ("Birleşimin çizgileri kesilemedi: duvar birleşimden kısa ya da diğer duvar birleşimden önce bitiyor.",
                             "Duvarı uzatın ya da bu ucu Yok yapın."),
}


def from_json(w):
    """A wall of the commands' JSON (floats, exact binary fractions) as the solver takes it."""
    comps = [(None if "thickness" not in c else F(c["thickness"]), c.get("priority")) for c in w.get("components", [])]
    joins = w.get("joins", {})
    return Wall([(F(p["x"]), F(p["y"])) for p in w["pts"]], F(w["width"]), w.get("justify", "center"), F(w.get("offset", 0.0)),
                comps, w.get("closed", False), (joins.get("start", "auto"), joins.get("end", "auto")),
                w.get("cleanup", True), w.get("caps", "closed"),
                wraps=[c.get("wrap", False) for c in w.get("components", [])],
                passes=(joins.get("startPass"), joins.get("endPass")))


def iter_while(ranks, p):
    for q in ranks:
        if q > p:
            yield q
        else:
            return


def s_near(line, closed, at):
    """The arc length of the point of the line nearest `at` (on its edges)."""
    n = len(line)
    best = None
    s = 0
    for i in range(n if closed else n - 1):
        a, b = line[i], line[(i + 1) % n]
        t = sub(b, a)
        u = max(F(0), min(F(1), dot(sub(at, a), t) / length2(t)))
        foot = add(a, mul(t, u))
        dd = length2(sub(at, foot))
        if best is None or dd < best[0]:
            best = (dd, s + u * exact_len(t))
        s += exact_len(t)
    return best[1]


def norm_spans(line, closed, raw):
    total = path_len(line, closed)
    v = []
    for a, b, mid in raw:
        lo, hi = min(a, b), max(a, b)
        if closed and not (lo <= mid <= hi):
            v.append((hi, lo + total))
        elif closed:
            v.append((lo, hi))
        else:
            v.append((max(lo, F(0)), min(hi, total)))
    v = sorted(x for x in v if x[1] > x[0])
    out = []
    for a, b in v:
        if out and a <= out[-1][1]:
            out[-1] = (out[-1][0], max(out[-1][1], b))
        else:
            out.append((a, b))
    if closed and len(out) > 1 and out[-1][1] >= out[0][0] + total:
        out[-1] = (out[-1][0], max(out[-1][1], out[0][1] + total))
        out.pop(0)
    return out


def kept(line, closed, gaps):
    total = path_len(line, closed)
    out = []
    if closed:
        for i in range(len(gaps)):
            f = gaps[i][1]
            t = gaps[i + 1][0] if i + 1 < len(gaps) else gaps[0][0] + total
            if t > f:
                out.append((f, t))
    else:
        at = F(0)
        for a, b in gaps:
            if a > at:
                out.append((at, a))
            at = b
        if total > at:
            out.append((at, total))
    return out


def pt(p):
    return [float(p[0]), float(p[1])]


def kept_strokes(line, closed, raw):
    if not raw:
        return [{"pts": [pt(p) for p in line], "closed": closed}]
    gaps = norm_spans(line, closed, raw)
    return [{"pts": [pt(p) for p in sub_line(line, closed, a, b)], "closed": False} for a, b in kept(line, closed, gaps)]


def area(ring):
    s = 0
    for i in range(len(ring)):
        a, b = ring[i], ring[(i + 1) % len(ring)]
        s += cross(a, b)
    return abs(s) / 2


def band_areas(a, b, closed, cuts):
    if not cuts:
        if closed:
            return [float(abs(area(a) - area(b)))]
        return [float(area(list(a) + list(reversed(b))))]
    pairs = []
    for x, y in cuts:
        sx, sy = norm_spans(a, closed, [x]), norm_spans(b, closed, [y])
        if sx and sy:
            pairs.append((sx[0], sy[0]))
    pairs.sort(key=lambda p: p[0][0])
    merged = []
    for p in pairs:
        if merged and p[0][0] <= merged[-1][0][1]:
            m = merged[-1]
            merged[-1] = ((m[0][0], max(m[0][1], p[0][1])), (min(m[1][0], p[1][0]), max(m[1][1], p[1][1])))
        else:
            merged.append(p)
    ka = kept(a, closed, [p[0] for p in merged])
    kb = kept(b, closed, [p[1] for p in merged])
    out = []
    for (a0, a1), (b0, b1) in zip(ka, kb):
        ring = sub_line(a, closed, a0, a1) + list(reversed(sub_line(b, closed, b0, b1)))
        out.append(float(area(ring)))
    return out


def path_crossings(wa, wb):
    out = []
    na, nb = len(wa.pts), len(wb.pts)
    for i in range(na if wa.closed else na - 1):
        p, q = wa.pts[i], wa.pts[(i + 1) % na]
        for j in range(nb if wb.closed else nb - 1):
            r, s = wb.pts[j], wb.pts[(j + 1) % nb]
            den = cross(sub(q, p), sub(s, r))
            if den == 0:
                continue
            t = cross(sub(r, p), sub(s, r)) / den
            u = cross(sub(r, p), sub(q, p)) / den
            if 0 <= t <= 1 and 0 <= u <= 1:
                x = add(p, mul(sub(q, p), t))
                if x not in out:
                    out.append(x)
    return out


def layered(p_core=1, finish=F(2, 100), p_finish=5):
    return [(finish, p_finish), (None, p_core), (finish, p_finish)]


def cases():
    c = []

    def case(name, walls):
        s = Solver(walls)
        drawn = s.solve()
        one = {"name": name, "walls": [w.json() for w in walls], "joints": s.joints, "issues": s.issues,
               "drawn": drawn}
        if any(w.holes for w in walls):
            one["openings"] = [[[float(a), float(b)] + ([] if f is None else [
                {k: v for k, v in (("depth", float(f[0])), ("offset", None if f[1] is None else float(f[1])),
                                   ("right", f[2] or None)) if v is not None}])
                                for (a, b), f in zip(w.holes, w.frames)] for w in walls]
        if any(p is not None for w in walls for p in w.passes):
            one["nodes"] = s.nodes
        c.append(one)

    w2 = F(2, 10)
    case("Köşe: eşit iki duvar, gönye", [Wall([(0, 0), (4, 0)], w2), Wall([(4, 0), (4, 3)], w2)])
    case("Köşe: farklı genişlik ve hiza", [Wall([(0, 0), (4, 0)], F(3, 10), "left"), Wall([(4, 0), (4, 3)], w2, "right")])
    case("Köşe: iki başlangıç", [Wall([(0, 0), (4, 0)], w2), Wall([(0, 0), (0, 3)], w2)])
    case("Köşe: eğik (3-4-5)", [Wall([(0, 0), (4, 0)], w2), Wall([(4, 0), (7, 4)], w2)])
    case("Köşe: Alın", [Wall([(0, 0), (4, 0)], w2, joins=("auto", "butt")), Wall([(4, 0), (4, 3)], F(3, 10))])
    case("Köşe: Geçen", [Wall([(0, 0), (4, 0)], w2, joins=("auto", "through")), Wall([(4, 0), (4, 3)], F(3, 10))])
    case("Köşe: bir uç Yok", [Wall([(0, 0), (4, 0)], w2, joins=("auto", "none")), Wall([(4, 0), (4, 3)], w2)])
    case("Düz devam: eşit", [Wall([(0, 0), (4, 0)], w2), Wall([(4, 0), (8, 0)], w2)])
    case("Düz devam: farklı genişlik", [Wall([(0, 0), (4, 0)], w2), Wall([(4, 0), (8, 0)], F(3, 10))])
    case("Düz devam: biri ters", [Wall([(0, 0), (4, 0)], w2), Wall([(8, 0), (4, 0)], w2)])
    case("T: yalın", [Wall([(0, 0), (6, 0)], w2), Wall([(3, 4), (3, 0)], w2)])
    case("T: yüzde biten", [Wall([(0, 0), (6, 0)], w2), Wall([(3, 4), (3, F(1, 10))], w2)])
    case("T: içeri taşan", [Wall([(0, 0), (6, 0)], w2), Wall([(3, 4), (3, F(-5, 100))], w2)])
    case("T: alttan", [Wall([(0, 0), (6, 0)], w2), Wall([(3, -4), (3, 0)], F(15, 100))])
    case("T: çekirdek sıvadan geçer", [Wall([(0, 0), (6, 0)], w2, comps=layered()), Wall([(3, 4), (3, 0)], w2, comps=layered())])
    case("T: çekirdek bütün duvardan geçer", [Wall([(0, 0), (6, 0)], w2, comps=[(F(2, 100), 5), (None, 3), (F(2, 100), 5)]),
                                              Wall([(3, 4), (3, 0)], w2, comps=layered())])
    case("T: temizlik kapalı", [Wall([(0, 0), (6, 0)], w2, comps=layered(), cleanup=False), Wall([(3, 4), (3, 0)], w2, comps=layered())])
    case("T: sola hizalı katmanlı konak, sağdan", [Wall([(0, 0), (6, 0)], F(25, 100), "left", comps=[(F(3, 100), 5), (None, 1), (F(5, 100), 3)]),
                                                     Wall([(3, -4), (3, 0)], w2, comps=layered())])
    case("T: eğik (3-4-5)", [Wall([(0, 0), (8, 0)], w2), Wall([(1, 4), (4, 0)], w2)])
    case("T: odanın iki duvarına", [Wall([(0, 0), (6, 0), (6, 4), (0, 4)], w2, closed=True), Wall([(3, 4), (3, 0)], F(15, 100))])
    case("T: odaya katmanlı", [Wall([(0, 0), (6, 0), (6, 4), (0, 4)], F(25, 100), comps=layered(finish=F(3, 100)), closed=True),
                               Wall([(3, 4), (3, 0)], w2, comps=layered())])
    case("X: eşit iki duvar", [Wall([(0, 0), (6, 0)], w2), Wall([(3, -3), (3, 3)], w2)])
    case("X: güçlü çekirdekli duvar konak", [Wall([(0, 0), (6, 0)], w2), Wall([(3, -3), (3, 3)], w2, comps=layered())])
    case("X: katmanlı iki duvar", [Wall([(0, 0), (6, 0)], w2, comps=layered()), Wall([(3, -3), (3, 3)], w2, comps=layered())])
    case("Üç uç: düz devam ve T", [Wall([(0, 0), (4, 0)], w2), Wall([(4, 0), (8, 0)], w2), Wall([(4, 3), (4, 0)], w2)])
    # 14.25° (63, 16, 65).
    case("Sorun: dar köşe", [Wall([(0, 0), (4, 0)], w2), Wall([(4, 0), (F(1, 16), 1)], w2)])
    case("Dar köşe Alın ile", [Wall([(0, 0), (4, 0)], w2, joins=("auto", "butt")), Wall([(4, 0), (F(1, 16), 1)], w2)])
    # 1.79° (4095, 128, 4097), binary exact.
    case("Sorun: aynı doğrultu", [Wall([(0, 0), (4, 0)], w2), Wall([(4, 0), (F(1, 1024), F(1, 8))], w2)])
    case("Sorun: üç uç bir doğruda değil", [Wall([(0, 0), (4, 0)], w2), Wall([(0, 0), (-3, 4)], w2), Wall([(0, 0), (-3, -4)], w2)])
    case("Sorun: duvar birleşimden kısa", [Wall([(0, 0), (4, 0)], w2), Wall([(4, 0), (4, F(5, 100))], w2)])
    case("Uçları açık duvarlar", [Wall([(0, 0), (4, 0)], w2, caps="open"), Wall([(4, 0), (4, 3)], w2, caps="open")])
    # ADR 0771 §3: layers that turn at the free ends and the jambs; §1: the user's node decisions.
    sv, ins, ren = F(2, 100), F(5, 100), F(2, 100)
    plaster = [(sv, 5), (None, 1), (sv, 5)]
    insulated = [(ren, 5), (ins, 4), (None, 1), (sv, 5)]
    case("Uçta döner: iki yüzü sıvalı duvarın serbest uçları", [Wall([(0, 0), (4, 0)], w2, comps=plaster, wraps=[True, False, True])])
    case("Uçta döner: yalnız dış yüzün yalıtımı", [Wall([(0, 0), (4, 0)], F(25, 100), comps=[(ins, 4), (None, 1), (sv, 5)],
                                                      wraps=[True, False, False])])
    case("Uçta döner: dış yüzde iki katman, iç yüzde sıva", [Wall([(0, 0), (5, 0)], F(3, 10), comps=insulated,
                                                                 wraps=[True, True, False, True])])
    case("Uçta döner: sağa hizalı, iç yüz yalnız", [Wall([(0, 0), (0, 4)], F(25, 100), "right", comps=insulated,
                                                         wraps=[False, False, False, True])])
    case("Pencere sövesi: açıklıkta dönen sıva ve yalıtım", [Wall([(0, 0), (6, 0)], F(3, 10), comps=insulated,
                                                                 wraps=[True, True, False, True], caps="open",
                                                                 holes=[(F(3, 2), F(27, 10))])])
    case("Kapı: iki açıklıkta da dönen sıva", [Wall([(0, 0), (8, 0)], w2, comps=plaster, wraps=[True, False, True],
                                                    caps="open", holes=[(1, 2), (5, F(59, 10))])])
    case("Açıklık: dönen katmansız duvarda düz söve", [Wall([(0, 0), (6, 0)], F(3, 10), comps=insulated, caps="open",
                                                           holes=[(F(3, 2), F(27, 10))])])
    case("Açıklık duvar ucuna çok yakın: katmanlar dönmez", [Wall([(0, 0), (4, 0)], w2, comps=plaster,
                                                                   wraps=[True, False, True], caps="open",
                                                                   holes=[(F(1, 100), 1)])])
    case("Köşede birleşik uç dönmez, serbest uç döner", [Wall([(0, 0), (4, 0)], w2, comps=plaster, wraps=[True, False, True]),
                                                           Wall([(4, 0), (4, 3)], w2, comps=plaster)])
    case("Düğüm kararı: çekirdek konağın yüzünde durur", [Wall([(0, 0), (6, 0)], w2, comps=layered()),
                                                          Wall([(3, 4), (3, 0)], w2, comps=layered(), passes=(None, [0, 0, 0]))])
    case("Düğüm kararı: bütün katmanlar konağın çekirdeğine girer", [Wall([(0, 0), (6, 0)], w2, comps=layered()),
                                                                      Wall([(3, 4), (3, 0)], w2, comps=layered(), passes=(None, [1, 2, 1]))])
    case("Düğüm kararı: konaktan fazlası konağın bileşen sayısında kalır", [Wall([(0, 0), (6, 0)], w2, comps=layered()),
                                                                          Wall([(3, 4), (3, 0)], w2, comps=layered(), passes=(None, [9, 9, 9]))])
    case("Düğüm kararı: temizlik kapalıyken de uygulanır", [Wall([(0, 0), (6, 0)], w2, comps=layered(), cleanup=False),
                                                            Wall([(3, 4), (3, 0)], w2, comps=layered(), passes=(None, [0, 1, 0]))])
    case("Düğüm kararı: ters yönlü konağa alttan, başlangıç ucunda", [Wall([(6, 0), (0, 0)], w2, comps=layered()),
                                                                     Wall([(3, 0), (3, -4)], w2, comps=layered(), passes=([0, 2, 0], None))])
    case("Odanın köşesinden kenarın doğrultusunda çıkan duvar öbür kenara T birleşir",
         [Wall([(0, 0), (4, 0), (4, 3), (0, 3)], w2, closed=True), Wall([(4, 0), (7, 0)], w2)])
    case("U duvarı iki ucuyla odanın iki köşesine birleşir",
         [Wall([(0, 0), (4, 0), (4, 3), (0, 3)], w2, closed=True), Wall([(4, 0), (7, 0), (7, 3), (4, 3)], w2)])
    # ADR 0785 §5: at a framed jamb the turning layers turn up to the frame's faces.
    case("Doğramaya kadar dönüş: ortalanmış kasa, iki yüz sıvalı", [Wall([(0, 0), (6, 0)], F(25, 100), comps=plaster,
                                                                         wraps=[True, False, True], caps="open",
                                                                         holes=[(F(3, 2), F(27, 10), (F(8, 100), None, False))])])
    case("Doğramaya kadar dönüş: kasa dış yüzden 5 cm, yalıtımlı duvar", [Wall([(0, 0), (6, 0)], F(3, 10), comps=insulated,
                                                                              wraps=[True, True, False, True], caps="open",
                                                                              holes=[(F(3, 2), F(27, 10), (F(6, 100), F(5, 100), False))])])
    case("Doğramaya kadar dönüş: sağa açılan kasa, yalnız dış yüz döner", [Wall([(0, 0), (6, 0)], F(25, 100),
                                                                                comps=[(ins, 4), (None, 1), (sv, 5)],
                                                                                wraps=[True, False, False], caps="open",
                                                                                holes=[(F(2), F(3), (F(7, 100), F(3, 100), True))])])
    case("Doğramaya kadar dönüş: kasa sıvanın içine taşarsa ortada buluşur", [Wall([(0, 0), (6, 0)], F(25, 100), comps=plaster,
                                                                                   wraps=[True, False, True], caps="open",
                                                                                   holes=[(F(3, 2), F(27, 10), (F(8, 100), F(0), False))])])
    case("Doğramaya kadar dönüş: kasalı ve kasasız iki pencere", [Wall([(0, 0), (8, 0)], w2, comps=plaster,
                                                                       wraps=[True, False, True], caps="open",
                                                                       holes=[(1, 2, (F(6, 100), None, False)), (5, F(59, 10))])])
    case("Düğüm kararı: yalın duvar konağa tek sayıyla", [Wall([(0, 0), (6, 0)], w2, comps=layered()),
                                                          Wall([(3, 4), (3, 0)], F(15, 100), passes=(None, [1]))])
    return c


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    text = json.dumps({"cases": cases()}, ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil: betiği --check olmadan çalıştırın", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)}: güncel")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(cases())} durum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
