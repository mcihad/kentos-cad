#!/usr/bin/env python3
"""Openings in walls (docs/adr/0341 §2), from the ADR's rules, without KentOS code.

Writes fixtures/walls/v1/openings.json; `--check` compares and writes nothing. The shared core's test
(`kentos-geometry-core`, tests/all/openings.rs) reads it and must give the same jambs, symbols, lines, caps and bands.

The walls are straight, so the jambs, the wall's lines and the gaps are exact fractions (the walls' joins are the
joins reference's, `wall_join_cases.py`). The rules, as ADR 0341 §2 gives them:

- An opening of width `w` lies on `[s0, s1]` of its wall's base line: centred at `at` (`anchor` center), from `at`
  (start) or to `at` (end). It must lie on one edge, within the path's ends; else it cuts nothing and has no jambs.
- Its jambs: the base line's point at `s0` and `s1` moved along the edge's left normal to the left face
  (`offset + {0, w/2, w}[justify]`) and to the right face (`left − width`): start-left, start-right, end-left, end-right.
- Each wall line (its boundaries' offsets) is interrupted between where it passes those two points' offsets (its own
  points at `s0`, `s1` beside the base line); each component's band and the body between its jambs; the two jamb
  lines are drawn as the wall's ends.
- Its symbol, in the frame of its jambs (origin the start jamb's middle, `u` toward the end jamb's middle, `n` to its
  left, length L between the middles, thickness t across): a frame of width f at each jamb (a ring: face, face + u·f,
  other face + u·f, other face); a swing door's leaf from its hinge (on the face it opens to, at the frame, at its
  `hand` jamb) of length (L − 2f) / leaves, turned from shut (along u, or −u from the end) by `swing` degrees toward the
  side it opens to, and its arc about the hinge from shut to open (none when `showSwing` is off); a sliding door's
  leaves along the wall t/8 either side of the middle, overlapping 5 % of the clear width in the middle; a fixed door a
  line along the middle; a window a line on each face between the frames and a glass line along the middle (a
  sliding window two at t/12 either side, overlapping); a bare opening nothing.

ADR 0719 adds the niche and the flue, and the plan's cut height (1.10 m):

- A bare opening, a niche or a flue is cut in plan only when its sill (none: 0) is below the cut height and its head
  (none: full height) above it; a door and a window always. One not cut takes nothing out of its wall.
- A niche of `depth` is cut from its face (`side`), a flue `inset` from its face (none: centred, (t − depth) / 2) and
  `depth` across, both clamped into the wall's thickness t: across the wall, as distances to the path's left, a left
  niche or flue lies on [left − far, left − near], a right one on [right + near, right + far] (`near` 0 for a niche).
  A recess reaching both faces goes through.
- A wall line is interrupted when it lies strictly inside that interval, or is a face the interval reaches; a line it
  only touches inside the wall stays (a niche's back on a component's boundary). A band (and the body between the
  faces) the interval overlaps is cut between the jambs, and what the interval leaves of its thickness put back
  between them (the upper rest, then the lower, after the band's own pieces). Its jambs are drawn as far across as it
  goes (from its `hi` to its `lo`, at its start then its end); its inner sides where no wall line runs are drawn too.
- Its pieces: a cut flue a cross between its cavity's corners (near start to far end, far start to near end); one not
  cut, dashed: a bare opening its jambs and a cross between them, a niche its sides and back, a flue its cavity and
  its cross. A cut bare opening or niche draws nothing (its wall shows it).
"""

import argparse
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from wall_join_cases import (  # noqa: E402
    Solver, Wall, add, exact_len, kept_strokes, left, mul, offset_path, pt, s_near, sub, band_areas, end_point,
)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/walls/v1/openings.json"


CUT = F(11, 10)


class Opening:
    def __init__(self, wall, at, width, anchor="center", form="door", leaves=1, operation="swing", hand="start",
                 side="left", swing=90, show_swing=True, frame=0, depth=None, inset=None, sill=None, head=None):
        self.wall = wall
        self.at = F(at)
        self.width = F(width)
        self.anchor = anchor
        self.form = form
        self.leaves = leaves
        self.operation = operation
        self.hand = hand
        self.side = side
        self.swing = swing
        self.show_swing = show_swing
        self.frame = F(frame)
        self.depth = None if depth is None else F(depth)
        self.inset = None if inset is None else F(inset)
        self.sill = None if sill is None else F(sill)
        self.head = None if head is None else F(head)

    def cut(self):
        """Whether plan cuts it (ADR 0719 §2)."""
        if self.form in ("door", "window"):
            return True
        return (self.sill or 0) < CUT and (self.head is None or self.head > CUT)

    def across(self, t):
        """A niche's or flue's near and far distance from its face in a wall t thick, clamped into it."""
        near = F(0) if self.form == "niche" else (self.inset if self.inset is not None else (t - self.depth) / 2)
        near = max(F(0), min(near, t))
        return near, min(near + max(self.depth, F(0)), t)

    def span(self):
        w = self.width
        return {"start": (self.at, self.at + w), "center": (self.at - w / 2, self.at + w / 2),
                "end": (self.at - w, self.at)}[self.anchor]

    def json(self):
        out = {"wall": self.wall, "at": float(self.at), "width": float(self.width), "form": self.form}
        if self.anchor != "center":
            out["anchor"] = self.anchor
        if self.leaves != 1:
            out["leaves"] = self.leaves
        if self.operation != "swing":
            out["operation"] = self.operation
        if self.hand != "start":
            out["hand"] = self.hand
        if self.side != "left":
            out["side"] = self.side
        if self.swing != 90:
            out["swing"] = self.swing
        if not self.show_swing:
            out["showSwing"] = False
        if self.frame:
            out["frame"] = float(self.frame)
        for k in ("sill", "head", "depth", "inset"):
            if getattr(self, k) is not None:
                out[k] = float(getattr(self, k))
        return out


def edges_of(w):
    n = len(w.pts)
    m = n if w.closed else n - 1
    out, s = [], F(0)
    for i in range(m):
        a, b = w.pts[i], w.pts[(i + 1) % n]
        ln = exact_len(sub(b, a))
        out.append((a, b, s, ln))
        s += ln
    return out, s


def beside(w, s, d):
    """The base line's point at s, d to its left (on the edge s is on)."""
    for a, b, s0, ln in edges_of(w)[0]:
        if s <= s0 + ln:
            t = sub(b, a)
            u = (t[0] / ln, t[1] / ln)
            p = add(a, mul(t, (s - s0) / ln))
            return add(p, mul(left(u), d))
    raise ValueError("off the path")


def jambs(w, o):
    s0, s1 = o.span()
    edges, total = edges_of(w)
    if s1 <= s0 or s0 < 0 or s1 > total:
        return None
    if not any(s0 >= a0 and s1 <= a0 + ln for _, _, a0, ln in edges):
        return None
    b = w.boundaries()
    lf, rf = b[0], b[-1]
    return [beside(w, s0, lf), beside(w, s0, rf), beside(w, s1, lf), beside(w, s1, rf)]


def recess_marks(place, o, width):
    """A bare opening's, niche's or flue's pieces (ADR 0719 §2), exact on straight walls, dashed when not cut."""
    line = lambda x, y: {"kind": "line", "a": pt(x), "b": pt(y)}  # noqa: E731
    cut = o.cut()
    l0, r0, l1, r1 = place
    out = []
    if o.form == "opening":
        if not cut:
            out = [line(l0, r0), line(l1, r1), line(l0, r1), line(r0, l1)]
    else:
        f0, o0, f1, o1 = (r0, l0, r1, l1) if o.side == "right" else (l0, r0, l1, r1)
        at = lambda f, q, d: add(f, mul(sub(q, f), d / width))  # noqa: E731
        if o.form == "niche":
            if not cut:
                d = min(o.depth, width)
                a0, b0, a1, b1 = at(f0, o0, 0), at(f0, o0, d), at(f1, o1, 0), at(f1, o1, d)
                out = [line(a0, b0), line(b0, b1), line(b1, a1)]
        else:
            near, far = o.across(width)
            a0, b0, a1, b1 = at(f0, o0, near), at(f0, o0, far), at(f1, o1, near), at(f1, o1, far)
            if not cut:
                out = [line(a0, b0), line(b0, b1), line(b1, a1), line(a1, a0)]
            out += [line(a0, b1), line(b0, a1)]
    if not cut:
        for m in out:
            m["dashed"] = True
    return out


def symbol(place, o, width=None):
    """The opening's pieces from its jambs, in floats (the ADR's frame)."""
    if o.form in ("opening", "niche", "flue"):
        return recess_marks(place, o, width)
    p = [(float(x), float(y)) for x, y in place]
    m0 = ((p[0][0] + p[1][0]) / 2, (p[0][1] + p[1][1]) / 2)
    m1 = ((p[2][0] + p[3][0]) / 2, (p[2][1] + p[3][1]) / 2)
    d = (m1[0] - m0[0], m1[1] - m0[1])
    length = math.hypot(*d)
    u = (d[0] / length, d[1] / length)
    n = (-u[1], u[0])
    t = math.hypot(p[0][0] - p[1][0], p[0][1] - p[1][1])
    out = []
    f = min(float(o.frame), length / 2 * 0.999)
    a = lambda q, v, k: (q[0] + v[0] * k, q[1] + v[1] * k)  # noqa: E731
    line = lambda x, y: {"kind": "line", "a": list(x), "b": list(y)}  # noqa: E731
    l0, r0, l1, r1 = p
    if f > 0:
        out.append({"kind": "ring", "pts": [list(l0), list(a(l0, u, f)), list(a(r0, u, f)), list(r0)]})
        out.append({"kind": "ring", "pts": [list(l1), list(a(l1, u, -f)), list(a(r1, u, -f)), list(r1)]})
    clear = length - 2 * f
    start = a(m0, u, f)
    if o.form == "window":
        out.append(line(a(l0, u, f), a(l1, u, -f)))
        out.append(line(a(r0, u, f), a(r1, u, -f)))
        if o.operation == "slide":
            k, lap, half = t / 12, clear * 0.05, clear / 2
            out.append(line(a(start, n, k), a(a(start, u, half + lap), n, k)))
            out.append(line(a(a(start, u, half - lap), n, -k), a(a(start, u, clear), n, -k)))
        else:
            out.append(line(start, a(start, u, clear)))
        return out
    if o.operation == "slide":
        k = t / 8
        if o.leaves == 1:
            out.append(line(a(start, n, k), a(a(start, u, clear), n, k)))
        else:
            lap, half = clear * 0.05, clear / 2
            out.append(line(a(start, n, k), a(a(start, u, half + lap), n, k)))
            out.append(line(a(a(start, u, half - lap), n, -k), a(a(start, u, clear), n, -k)))
        return out
    if o.operation == "fixed":
        out.append(line(start, a(start, u, clear)))
        return out
    face0, face1 = (r0, r1) if o.side == "right" else (l0, l1)
    outward = (-n[0], -n[1]) if o.side == "right" else n

    def leaf(hinge, shut, ln):
        turn = o.swing if shut[0] * outward[1] - shut[1] * outward[0] >= 0 else -o.swing
        r = math.radians(turn)
        opened = (shut[0] * math.cos(r) - shut[1] * math.sin(r), shut[0] * math.sin(r) + shut[1] * math.cos(r))
        out.append(line(hinge, a(hinge, opened, ln)))
        if o.show_swing:
            ac, ao = math.atan2(shut[1], shut[0]), math.atan2(opened[1], opened[0])
            a0, a1 = (ac, ao) if turn > 0 else (ao, ac)
            out.append({"kind": "arc", "c": list(hinge), "r": ln, "a0": a0, "a1": a1})

    if o.leaves == 2:
        leaf(a(face0, u, f), u, clear / 2)
        leaf(a(face1, u, -f), (-u[0], -u[1]), clear / 2)
    elif o.hand == "end":
        leaf(a(face1, u, -f), (-u[0], -u[1]), clear)
    else:
        leaf(a(face0, u, f), u, clear)
    return out


class OpeningSolver(Solver):
    """The joins reference's solver with the openings' gaps, cuts and jamb lines."""

    def __init__(self, walls, openings):
        super().__init__(walls)
        self.openings = openings

    def drawn(self):
        # The spans on the lines as the joins left them (cut back or run on at their ends).
        self.holes = [[] for _ in self.w]
        for o in self.openings:
            w = self.w[o.wall]
            j = jambs(w, o)
            if j is None or not o.cut():
                continue
            s0, s1 = o.span()
            b = w.boundaries()
            left, right = b[0], b[-1]
            if o.form in ("niche", "flue"):
                near, far = o.across(w.width)
                lo, hi = (right + near, right + far) if o.side == "right" else (left - far, left - near)
            else:
                lo, hi = right, left
            spans = []
            for k, d in enumerate(b):
                line = self.lines[o.wall][k]
                on = lambda s: s_near(line, w.closed, beside(w, s, d))  # noqa: E731
                spans.append((on(s0), on(s1), on((s0 + s1) / 2)))
            self.holes[o.wall].append((j, spans, lo, hi, s0, s1))
        backs = [[[] for _ in range(len(self.cuts[i]))] for i in range(len(self.w))]
        for i, holes in enumerate(self.holes):
            b = self.w[i].boundaries()
            left, right = b[0], b[-1]
            for _, spans, lo, hi, s0, s1 in holes:
                for k, sp in enumerate(spans):
                    d = b[k]
                    if lo < d < hi or (d == left and hi >= left) or (d == right and lo <= right):
                        self.gaps[i][k].append(sp)
                for c in range(len(self.cuts[i])):
                    upper, lower = b[c], b[c + 1]
                    top, bottom = min(hi, upper), max(lo, lower)
                    if top <= bottom:
                        continue
                    self.cuts[i][c].append((spans[c], spans[c + 1]))
                    for u, v in ((upper, top), (bottom, lower)):
                        if u > v:
                            backs[i][c].append(float((s1 - s0) * (u - v)))
        out = super().drawn()
        for i, holes in enumerate(self.holes):
            w = self.w[i]
            b = w.boundaries()
            left, right = b[0], b[-1]
            ls = self.lines[i]
            for c, extra in enumerate(backs[i]):
                out[i]["bands"][c] += extra
            fill_cuts, fill_back = [], []
            for _, spans, lo, hi, s0, s1 in holes:
                top, bottom = min(hi, left), max(lo, right)
                if top <= bottom:
                    continue
                fill_cuts.append((spans[0], spans[-1]))
                for u, v in ((left, top), (bottom, right)):
                    if u > v:
                        fill_back.append(float((s1 - s0) * (u - v)))
            out[i]["fill"] = band_areas(ls[0], ls[-1], w.closed, fill_cuts) + fill_back
            recesses = []
            for j, _, lo, hi, s0, s1 in holes:
                if lo <= right and hi >= left:
                    out[i]["caps"].append([pt(j[0]), pt(j[1])])
                    out[i]["caps"].append([pt(j[2]), pt(j[3])])
                    continue
                for s in (s0, s1):
                    out[i]["caps"].append([pt(beside(w, s, hi)), pt(beside(w, s, lo))])
                for side in (hi, lo):
                    if right < side < left and side not in b:
                        recesses.append({"pts": [pt(beside(w, s0, side)), pt(beside(w, s1, side))], "closed": False})
            out[i]["recesses"] = recesses
        return out


def cases():
    c = []

    def case(name, walls, openings):
        s = OpeningSolver(walls, openings)
        drawn = s.solve()
        places = [jambs(walls[o.wall], o) for o in openings]
        c.append({
            "name": name,
            "walls": [w.json() for w in walls],
            "openings": [o.json() for o in openings],
            "jambs": [None if j is None else [pt(q) for q in j] for j in places],
            "symbols": [[] if j is None else symbol(j, o, walls[o.wall].width) for j, o in zip(places, openings)],
            "drawn": drawn,
        })

    w2 = F(2, 10)
    layered = [(F(2, 100), 5), (None, 1), (F(2, 100), 5)]
    case("Kapı: düz duvarın ortasında", [Wall([(0, 0), (6, 0)], w2)], [Opening(0, 3, F(9, 10))])
    case("Kapı: kasalı", [Wall([(0, 0), (6, 0)], w2)], [Opening(0, 3, F(9, 10), frame=F(5, 100))])
    case("Kapı: baştan dayanaklı", [Wall([(0, 0), (6, 0)], w2)], [Opening(0, F(1, 2), F(8, 10), anchor="start")])
    case("Kapı: sondan dayanaklı", [Wall([(0, 0), (6, 0)], w2)], [Opening(0, F(11, 2), F(8, 10), anchor="end")])
    case("Kapı: menteşe sonda, sağa açılır", [Wall([(0, 0), (6, 0)], w2)],
         [Opening(0, 3, F(9, 10), hand="end", side="right", frame=F(5, 100))])
    case("Kapı: 60° açılır", [Wall([(0, 0), (6, 0)], w2)], [Opening(0, 3, F(9, 10), swing=60)])
    case("Kapı: açılım yayı gizli", [Wall([(0, 0), (6, 0)], w2)], [Opening(0, 3, F(9, 10), show_swing=False)])
    case("Kapı: iki kanatlı", [Wall([(0, 0), (6, 0)], w2)], [Opening(0, 3, F(14, 10), leaves=2, frame=F(5, 100))])
    case("Kapı: sürme iki kanat", [Wall([(0, 0), (6, 0)], w2)],
         [Opening(0, 3, F(16, 10), leaves=2, operation="slide", frame=F(5, 100))])
    case("Kapı: sürme tek kanat", [Wall([(0, 0), (6, 0)], w2)], [Opening(0, 3, F(9, 10), operation="slide")])
    case("Kapı: sabit", [Wall([(0, 0), (6, 0)], w2)], [Opening(0, 3, F(9, 10), operation="fixed")])
    case("Pencere: kanatlı", [Wall([(0, 0), (6, 0)], F(25, 100), comps=layered)],
         [Opening(0, 3, F(12, 10), form="window", leaves=2, frame=F(6, 100))])
    case("Pencere: sürme", [Wall([(0, 0), (6, 0)], F(25, 100), comps=layered)],
         [Opening(0, 3, F(15, 10), form="window", operation="slide", frame=F(6, 100))])
    case("Boşluk", [Wall([(0, 0), (6, 0)], w2)], [Opening(0, 3, 1, form="opening")])
    case("Sola hizalı duvar", [Wall([(0, 0), (6, 0)], w2, "left")], [Opening(0, 2, F(9, 10))])
    case("Ötelenmiş duvar", [Wall([(0, 0), (6, 0)], w2, offset=F(1, 10))], [Opening(0, 2, F(9, 10), side="right")])
    case("Eğik duvar (3-4-5)", [Wall([(0, 0), (6, 8)], w2)], [Opening(0, 5, 1, form="window")])
    case("İki açıklık bir duvarda", [Wall([(0, 0), (8, 0)], w2)],
         [Opening(0, 2, F(9, 10)), Opening(0, 6, F(12, 10), form="window")])
    case("Çok köşeli duvarın ikinci kenarı", [Wall([(0, 0), (4, 0), (4, 5)], w2)], [Opening(0, 6, F(9, 10))])
    case("Köşede birleşen duvarlar", [Wall([(0, 0), (4, 0)], w2), Wall([(4, 0), (4, 3)], w2)],
         [Opening(0, 2, F(9, 10)), Opening(1, F(3, 2), F(8, 10), form="window")])
    case("T'nin konağında kapı", [Wall([(0, 0), (8, 0)], w2), Wall([(5, 4), (5, 0)], w2)], [Opening(0, 2, F(9, 10))])
    case("Odanın duvarında", [Wall([(0, 0), (6, 0), (6, 4), (0, 4)], w2, closed=True)],
         [Opening(0, 3, F(9, 10)), Opening(0, 8, F(12, 10), form="window")])
    case("Katmanlı duvarda kapı", [Wall([(0, 0), (6, 0)], F(25, 100), comps=layered)], [Opening(0, 3, F(9, 10))])
    case("Köşeyi aşan açıklık kesmez", [Wall([(0, 0), (4, 0), (4, 5)], w2)], [Opening(0, 4, F(9, 10))])
    case("Duvarın ucunu aşan açıklık kesmez", [Wall([(0, 0), (6, 0)], w2)], [Opening(0, F(2, 10), F(9, 10))])
    # ADR 0719: niches, flues and the plan's cut height.
    w3 = F(3, 10)
    niche = dict(form="niche", operation="fixed", depth=F(1, 10), sill=F(9, 10), head=F(15, 10))
    case("Niş: sol yüzden", [Wall([(0, 0), (6, 0)], w3)], [Opening(0, 3, F(6, 10), **niche)])
    case("Niş: sağ yüzden", [Wall([(0, 0), (6, 0)], w3)], [Opening(0, 3, F(6, 10), side="right", **niche)])
    case("Niş: katman sınırına kadar", [Wall([(0, 0), (6, 0)], F(25, 100), comps=layered)],
         [Opening(0, 3, F(6, 10), **{**niche, "depth": F(2, 100)})])
    case("Niş: katmanın ortasında biter", [Wall([(0, 0), (6, 0)], F(25, 100), comps=layered)],
         [Opening(0, 3, F(6, 10), **niche)])
    case("Niş: eğik duvarda", [Wall([(0, 0), (6, 8)], w3)], [Opening(0, 5, 1, side="right", **niche)])
    case("Niş: kapıyla aynı duvarda", [Wall([(0, 0), (8, 0)], w3)],
         [Opening(0, 2, F(9, 10)), Opening(0, 6, F(12, 10), **niche)])
    case("Niş: odanın duvarında", [Wall([(0, 0), (6, 0), (6, 4), (0, 4)], w3, closed=True)],
         [Opening(0, 8, F(8, 10), side="right", **niche)])
    case("Niş: kesmenin üstünde, kesik çizgi", [Wall([(0, 0), (6, 0)], w3)],
         [Opening(0, 3, F(6, 10), **{**niche, "sill": F(12, 10), "head": F(18, 10)})])
    case("Kalorifer nişi: kesmenin altında, kesik çizgi", [Wall([(0, 0), (6, 0)], w3)],
         [Opening(0, 3, 1, side="right", **{**niche, "sill": F(1, 10), "head": F(8, 10)})])
    case("Niş: duvardan derin, duvarı geçer", [Wall([(0, 0), (6, 0)], w3)],
         [Opening(0, 3, F(6, 10), **{**niche, "depth": F(35, 100)})])
    flue = dict(form="flue", operation="fixed", depth=F(2, 10))
    case("Baca: ortada", [Wall([(0, 0), (6, 0)], w3)], [Opening(0, 3, F(2, 10), **flue)])
    case("Baca: sağ yüzden 5 cm", [Wall([(0, 0), (6, 0)], w3)], [Opening(0, 3, F(2, 10), side="right", inset=F(5, 100), **flue)])
    case("Baca: katmanlı duvarda", [Wall([(0, 0), (6, 0)], F(25, 100), comps=layered)],
         [Opening(0, 3, F(2, 10), **{**flue, "depth": F(1, 10)})])
    case("Baca: yüksekte başlar, kesik çizgi", [Wall([(0, 0), (6, 0)], w3)],
         [Opening(0, 3, F(2, 10), sill=2, **flue)])
    case("Tesisat boşluğu: kesmenin üstünde, kesik çizgi", [Wall([(0, 0), (6, 0)], w2)],
         [Opening(0, 3, F(2, 10), form="opening", operation="fixed", sill=F(23, 10), head=F(25, 10))])
    case("Boşluk: kesmeyi kesen alt ve üst kot", [Wall([(0, 0), (6, 0)], w2)],
         [Opening(0, 3, 1, form="opening", sill=F(3, 10), head=F(21, 10))])
    return c


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    text = json.dumps({"cases": cases()}, ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil: betiği --check olmadan çalıştırın", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)}: güncel")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(cases())} durum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
