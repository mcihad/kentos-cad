#!/usr/bin/env python3
"""Revolving, pocket and folding doors and the sliding door's track (docs/adr/0777), from the ADR's rules, without
KentOS code.

Writes fixtures/walls/v1/special-doors.json; `--check` compares and writes nothing. The shared core's test
(`kentos-geometry-core`, tests/all/openings.rs) reads it with openings.json and must give the same jambs, symbols,
sweep zones, lines, caps, bands, body and the pocket's inner sides.

The walls are straight, so the jambs, the pockets and the wall's lines are exact fractions (the joins are the joins
reference's, `wall_join_cases.py`; the openings' gaps the openings reference's, `opening_cases.py`). The rules, as
ADR 0777 gives them, in the frame of the jambs (origin the start jamb's middle, `u` toward the end jamb's, `n` its
left, L between the middles, t the wall's thickness across, a fitting gap `fit` and a frame `f` each side, the light
c = L − 2·fit − 2·f from `fit + f`):

- Döner (revolving), 3 or 4 wings: its drum's inside diameter D (none: the light; at most the light) about the light's
  middle on the wall's middle; the drum a circle of radius r = D/2; the drum's two walls arcs of radius r + 0.03 about
  the wall's line both ways, each 360°/n wide (from θu − 180°/n to θu + 180°/n, and that turned 180°); its wings lines
  from the middle, r long, the first at θu + 90° − 180°/n, each next 360°/n on. Its sweep: the drum's circle, 32
  points from 0°.
- Cebe giren (pocket), 1 or 2 leaves: its leaves shut, bands on the wall's middle the leaf's thickness tl wide (none:
  0.04) across the light (two split as given, none: half); its pockets, each a cavity tl + 2·0.01 wide centred in the
  wall, from the jamb at its `hand` (two: both) into the wall w − lip − fit − f + 0.01 long (w its leaf, lip none:
  0.05), open at that jamb: the jamb draws only the wall each side of the cavity, the cavity's far end its own cap, its
  sides its inner lines. Its sweep: none.
- Katlanır (folding), 2–6 panels: c/n each, from the jamb at its `hand` (both sides: half each, the start the fewer) on
  the face it opens to, zigzagging out and back at `swing` degrees (none: 30°): panel i goes along by cos and out by
  ±sin. Its sweep: a rectangle a panel deep along each group.
- Sürme (sliding): two leaves overlap by the given `overlap` (none: 5 % of the light, as before), each lap/2 past the
  middle; with its track a line along the middle across the light; one leaf on its track hangs outside the face it
  opens to, t/4 out, tl thick, over the light, its track t/8 out from the light's far side to a light past the jamb
  at its `hand`. A sliding window the same lap and track.

ADR 0792's three- and four-leaf sliding doors and windows are written to fixtures/walls/v1/multi-slide.json (the same
core test): their leaves bands on their tracks, the arrows toward the jamb they stack at and the rails (`slide_leaves`,
`multi_slide`).
"""

import argparse
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from wall_join_cases import Solver, Wall, band_areas, pt, s_near  # noqa: E402
from opening_cases import Opening, beside, jambs, symbol as swing_symbol  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/walls/v1/special-doors.json"
# Three- and four-leaf sliding doors and windows (docs/adr/0792 §1), the same rules' cases in their own file.
MULTI = ROOT / "fixtures/walls/v1/multi-slide.json"

PLAY = F(1, 100)
LIP = F(5, 100)
LEAF = F(4, 100)
DRUM_WALL = 0.03
FOLD = 30


class Door(Opening):
    """An opening with ADR 0777's fields: its construction's fit, leaf thickness, split, drum, overlap, track, both
    sides and lip."""

    def __init__(self, wall, at, width, fit=None, leaf=None, split=None, drum=None, overlap=None, track=False,
                 both=False, lip=None, tracks=None, **kw):
        super().__init__(wall, at, width, **kw)
        self.tracks = tracks
        self.fit = None if fit is None else F(fit)
        self.leaf = None if leaf is None else F(leaf)
        self.split = None if split is None else F(split)
        self.drum = None if drum is None else F(drum)
        self.overlap = None if overlap is None else F(overlap)
        self.track = track
        self.both = both
        self.lip = None if lip is None else F(lip)

    def drawn_swing(self):
        return FOLD if self.operation == "folding" and self.swing == 90 else self.swing

    def json(self):
        out = super().json()
        b = {}
        for k, v in (("fit", self.fit), ("leafThickness", self.leaf), ("split", self.split), ("drum", self.drum),
                     ("overlap", self.overlap), ("lip", self.lip)):
            if v is not None:
                b[k] = float(v)
        if self.track:
            b["track"] = True
        if self.both:
            b["bothSides"] = True
        if self.tracks is not None:
            b["tracks"] = self.tracks
        if b:
            out["build"] = b
        return out

    def light(self):
        return self.width - 2 * (self.fit or 0) - 2 * self.frame

    def pockets(self):
        """Its pockets: (at its end jamb, length, width across), exact."""
        if self.form != "door" or self.operation != "pocket":
            return []
        c = self.light()
        t = self.leaf if self.leaf is not None else LEAF
        lip = self.lip if self.lip is not None else LIP
        fit = self.fit or 0

        def pocket(end, w):
            return end, max(w - lip - fit - self.frame + PLAY, F(0)), t + 2 * PLAY
        if self.leaves == 2:
            share = self.split if self.split is not None else F(1, 2)
            return [pocket(False, c * share), pocket(True, c * (1 - share))]
        return [pocket(self.hand == "end", c)]


def frame(place):
    p = [(float(x), float(y)) for x, y in place]
    m0 = ((p[0][0] + p[1][0]) / 2, (p[0][1] + p[1][1]) / 2)
    m1 = ((p[2][0] + p[3][0]) / 2, (p[2][1] + p[3][1]) / 2)
    d = (m1[0] - m0[0], m1[1] - m0[1])
    length = math.hypot(*d)
    u = (d[0] / length, d[1] / length)
    n = (-u[1], u[0])
    t = math.hypot(p[0][0] - p[1][0], p[0][1] - p[1][1])
    return p, u, n, length, t


def ad(q, v, k):
    return (q[0] + v[0] * k, q[1] + v[1] * k)


def line(x, y):
    return {"kind": "line", "a": list(x), "b": list(y)}


def ring(pts):
    return {"kind": "ring", "pts": [list(q) for q in pts]}


def symbol(place, o):
    """The door's or window's pieces from its jambs, in floats (a swinging door's the openings reference's)."""
    if o.form == "door" and o.operation == "swing":
        return swing_symbol(place, o)
    p, u, n, length, t = frame(place)
    l0, r0, l1, r1 = p
    half_len = length / 2 * 0.999
    fit = min(max(float(o.fit or 0), 0.0), half_len)
    f = min(max(float(o.frame), 0.0), max(half_len - fit, 0.0))
    c = length - 2 * f - 2 * fit

    def along_start(s, d):
        return ad(ad(l0, (r0[0] - l0[0], r0[1] - l0[1]), d / t), u, s)

    def along_end(s, d):
        return ad(ad(l1, (r1[0] - l1[0], r1[1] - l1[1]), d / t), u, -s)
    out = []
    if f > 0:
        out.append(ring([along_start(fit, 0), along_start(fit + f, 0), along_start(fit + f, t), along_start(fit, t)]))
        out.append(ring([along_end(fit, 0), along_end(fit + f, 0), along_end(fit + f, t), along_end(fit, t)]))
    start = fit + f
    m0 = along_start(start, t / 2)
    overlap = None if o.overlap is None else float(o.overlap)

    def lapped(k):
        lap = c * 0.05 if overlap is None else overlap / 2
        half = c / 2
        out.append(line(ad(m0, n, k), ad(ad(m0, u, half + lap), n, k)))
        out.append(line(ad(ad(m0, u, half - lap), n, -k), ad(ad(m0, u, c), n, -k)))
        if o.track:
            out.append(line(m0, ad(m0, u, c)))
    if o.form == "window":
        out.append(line(along_start(start, 0), along_end(start, 0)))
        out.append(line(along_start(start, t), along_end(start, t)))
        if o.operation == "slide" and o.leaves > 2:
            out += multi_slide(o, along_start, along_end, u, n, c, start, t, True)
        elif o.operation == "slide":
            lapped(t / 12)
        else:
            out.append(line(m0, ad(m0, u, c)))
        return out
    tl = float(o.leaf) if o.leaf is not None and o.leaf > 0 else float(LEAF)
    if o.operation == "slide":
        if o.leaves > 2:
            out += multi_slide(o, along_start, along_end, u, n, c, start, t, False)
        elif o.leaves == 1 and o.track:
            left_side = o.side != "right"
            d = 0 if left_side else t
            out_dir = n if left_side else (-n[0], -n[1])
            k = t / 4

            def on(s, off):
                return ad(along_start(s, d), out_dir, off)
            out.append(ring([on(start, k), on(start + c, k), on(start + c, k + tl), on(start, k + tl)]))
            a, z = (start, start + 2 * c) if o.hand == "end" else (start + c, start - c)
            out.append(line(on(a, k / 2), on(z, k / 2)))
        elif o.leaves == 1:
            k = t / 8
            out.append(line(ad(m0, n, k), ad(ad(m0, u, c), n, k)))
        else:
            lapped(t / 8)
        return out
    if o.operation == "revolving":
        r = min(float(o.drum) if o.drum is not None else c, c) / 2
        if not r > 0:
            return out
        centre = along_start(start + c / 2, t / 2)
        k = min(max(o.leaves, 3), 4)
        a = math.atan2(u[1], u[0])
        half = math.pi / k
        out.append({"kind": "circle", "c": list(centre), "r": r})
        for base in (a, a + math.pi):
            out.append({"kind": "arc", "c": list(centre), "r": r + DRUM_WALL, "a0": base - half, "a1": base + half})
        for i in range(k):
            ang = a + math.pi / 2 - half + 2 * math.pi * i / k
            out.append(line(centre, (centre[0] + r * math.cos(ang), centre[1] + r * math.sin(ang))))
        return out
    if o.operation == "pocket":
        lo, hi = t / 2 - tl / 2, t / 2 + tl / 2

        def band(s0, s1):
            return ring([along_start(s0, lo), along_start(s1, lo), along_start(s1, hi), along_start(s0, hi)])
        if o.leaves == 2:
            share = float(o.split) if o.split is not None and 0 < o.split < 1 else 0.5
            mid = start + c * share
            out += [band(start, mid), band(mid, start + c)]
        else:
            out.append(band(start, start + c))
        return out
    if o.operation == "folding":
        for g in fold_groups(o, along_start, along_end, u, n, c, start, t):
            hinge, dirv, outv, panels, width, cs, sn = g
            at = hinge
            for i in range(panels):
                rise = sn if i % 2 == 0 else -sn
                nxt = ad(ad(at, dirv, width * cs), outv, width * rise)
                out.append(line(at, nxt))
                at = nxt
        return out
    raise ValueError(o.operation)


def slide_leaves(o, c):
    """ADR 0792 §1: the leaves counted from the jamb they stack at (both sides: two from each jamb), the k-th on track
    k and sliding while k < the tracks (none: 2), the rest staying on track 0; neighbours on different tracks overlap
    by the overlap (none: 5 % of the light over the n − 1 meetings), on one track they butt; each leaf
    (light + laps · overlap) / n wide. Returns the tracks, the width and (start, track, slides, toward the end)."""
    n = min(max(o.leaves, 3), 4)
    tr = 2 if o.tracks is None else min(max(o.tracks, 2), 3)
    both = o.both and n == 4
    lap = c * 0.05 / (n - 1) if o.overlap is None else float(o.overlap)

    def group(count, to_end):
        return [(k, k > 0, to_end) if k < tr else (0, False, to_end) for k in range(count)]
    if both:
        order = group(2, False) + list(reversed(group(2, True)))
    else:
        order = group(n, o.hand == "end")
        if o.hand == "end":
            order.reverse()
    laps = sum(1 for a, b in zip(order, order[1:]) if a[0] != b[0])
    w = (c + laps * lap) / n
    out, s = [], 0.0
    for i, (track, slides, to_end) in enumerate(order):
        if i > 0:
            s += w - lap if order[i - 1][0] != track else w
        out.append((s, track, slides, to_end))
    return tr, w, out


def multi_slide(o, along_start, along_end, u, n, c, start, t, window):
    """ADR 0792 §1's plan: each leaf a band min(its thickness (none: 0.04), 4/5 of the spacing) wide about its track across
    its share; tracks evenly across the wall, depth/(2·tracks)
    apart for a door, depth/(3·tracks) for a window, track 0 on the face away from the side it opens to; beside each
    sliding leaf, half a spacing toward the side it opens to, an arrow toward its jamb: a shaft a = min(w/3, 0.3) long
    about the leaf's middle, its head two strokes back h = a/3 along and h/2 across; with its track the rails."""
    tr, w, leaves = slide_leaves(o, c)
    sp = t / ((3.0 if window else 2.0) * tr)
    sign = 1.0 if o.side == "right" else -1.0
    half = (tr - 1) / 2.0

    def d_of(track):
        return t / 2 + sign * (track - half) * sp
    tl = float(o.leaf) if o.leaf is not None and o.leaf > 0 else float(LEAF)
    hb = min(tl, 0.8 * sp) / 2
    out = []
    for s, track, slides, to_end in leaves:
        d = d_of(track)
        a, z = start + s, start + s + w
        out.append(ring([along_start(a, d - hb), along_start(z, d - hb), along_start(z, d + hb), along_start(a, d + hb)]))
        if slides:
            dirv = u if to_end else (-u[0], -u[1])
            cpt = along_start(start + s + w / 2, d + sign * sp / 2)
            a = min(w / 3, 0.3)
            h = a / 3
            tip = ad(cpt, dirv, a / 2)
            tail = ad(cpt, dirv, -(a / 2))
            back = ad(tip, dirv, -h)
            out.append(line(tail, tip))
            out.append(line(tip, ad(back, n, h / 2)))
            out.append(line(tip, ad(back, n, -(h / 2))))
    if o.track:
        for k in range(tr):
            d = d_of(k)
            out.append(line(along_start(start, d), along_end(start, d)))
    return out


def fold_groups(o, along_start, along_end, u, n, c, start, t):
    k = min(max(o.leaves, 2), 6)
    width = c / k
    face = t if o.side == "right" else 0
    outv = (-n[0], -n[1]) if o.side == "right" else n
    r = math.radians(o.drawn_swing())
    cs, sn = math.cos(r), math.sin(r)
    at0, at1 = along_start(start, face), along_end(start, face)
    back = (-u[0], -u[1])
    if o.both:
        return [(at0, u, outv, k // 2, width, cs, sn), (at1, back, outv, k - k // 2, width, cs, sn)]
    if o.hand == "end":
        return [(at1, back, outv, k, width, cs, sn)]
    return [(at0, u, outv, k, width, cs, sn)]


def zones(place, o):
    """Its sweep (ADR 0777 §1, §3): a revolving door's drum, a folding door's panels' reach; none for the others."""
    p, u, n, length, t = frame(place)
    l0, r0, l1, r1 = p
    half_len = length / 2 * 0.999
    fit = min(max(float(o.fit or 0), 0.0), half_len)
    f = min(max(float(o.frame), 0.0), max(half_len - fit, 0.0))
    c = length - 2 * f - 2 * fit
    start = fit + f

    def along_start(s, d):
        return ad(ad(l0, (r0[0] - l0[0], r0[1] - l0[1]), d / t), u, s)

    def along_end(s, d):
        return ad(ad(l1, (r1[0] - l1[0], r1[1] - l1[1]), d / t), u, -s)
    if o.operation == "revolving":
        r = min(float(o.drum) if o.drum is not None else c, c) / 2
        centre = along_start(start + c / 2, t / 2)
        return [[[centre[0] + r * math.cos(2 * math.pi * i / 32), centre[1] + r * math.sin(2 * math.pi * i / 32)]
                 for i in range(32)]]
    if o.operation == "folding":
        out = []
        for hinge, dirv, outv, panels, width, _, _ in fold_groups(o, along_start, along_end, u, n, c, start, t):
            along = (dirv[0] * width * panels, dirv[1] * width * panels)
            deep = (outv[0] * width, outv[1] * width)
            a = hinge
            b = (hinge[0] + along[0], hinge[1] + along[1])
            out.append([list(a), list(b), [b[0] + deep[0], b[1] + deep[1]], [a[0] + deep[0], a[1] + deep[1]]])
        return out
    return []


class PocketSolver(Solver):
    """The joins reference's solver with the doors' gaps, their pockets' cavities open at their jambs, caps and inner
    sides (the openings reference's rules with ADR 0777 §2)."""

    def __init__(self, walls, openings):
        super().__init__(walls)
        self.openings = openings

    def drawn(self):
        # Each hole: its jambs (a door's), its spans on the lines, across (lo, hi), along (s0, s1), and the cavity it
        # opens into at s0 and s1.
        self.holes = [[] for _ in self.w]
        for o in self.openings:
            w = self.w[o.wall]
            j = jambs(w, o)
            if j is None:
                continue
            s0, s1 = o.span()
            b = w.boundaries()
            left, right = b[0], b[-1]

            def spans_of(a, z):
                out = []
                for k, d in enumerate(b):
                    ln = self.lines[o.wall][k]
                    on = lambda s, d=d, ln=ln: s_near(ln, w.closed, beside(w, s, d))  # noqa: E731
                    out.append((on(a), on(z), on((a + z) / 2)))
                return out
            main = {"j": j, "spans": spans_of(s0, s1), "lo": right, "hi": left, "s0": s0, "s1": s1, "open": [None, None]}
            cavities = []
            for end, ln, wd in o.pockets():
                if ln <= 0:
                    continue
                near = (w.width - wd) / 2
                across = (left - (near + wd), left - near)
                a, z, k = (s1, s1 + ln, 1) if end else (s0 - ln, s0, 0)
                main["open"][k] = across
                op = [None, None]
                op[1 - k] = across
                cavities.append({"j": None, "spans": spans_of(a, z), "lo": across[0], "hi": across[1], "s0": a,
                                 "s1": z, "open": op})
            self.holes[o.wall] += [main] + cavities
        backs = [[[] for _ in range(len(self.cuts[i]))] for i in range(len(self.w))]
        for i, holes in enumerate(self.holes):
            b = self.w[i].boundaries()
            left, right = b[0], b[-1]
            for h in holes:
                lo, hi = h["lo"], h["hi"]
                for k, sp in enumerate(h["spans"]):
                    d = b[k]
                    if lo < d < hi or (d == left and hi >= left) or (d == right and lo <= right):
                        self.gaps[i][k].append(sp)
                for c in range(len(self.cuts[i])):
                    upper, lower = b[c], b[c + 1]
                    top, bottom = min(hi, upper), max(lo, lower)
                    if top <= bottom:
                        continue
                    self.cuts[i][c].append((h["spans"][c], h["spans"][c + 1]))
                    for u, v in ((upper, top), (bottom, lower)):
                        if u > v:
                            backs[i][c].append(float((h["s1"] - h["s0"]) * (u - v)))
        out = super().drawn()
        for i, holes in enumerate(self.holes):
            w = self.w[i]
            b = w.boundaries()
            left, right = b[0], b[-1]
            ls = self.lines[i]
            for c, extra in enumerate(backs[i]):
                out[i]["bands"][c] += extra
            fill_cuts, fill_back = [], []
            for h in holes:
                top, bottom = min(h["hi"], left), max(h["lo"], right)
                if top <= bottom:
                    continue
                fill_cuts.append((h["spans"][0], h["spans"][-1]))
                for u, v in ((left, top), (bottom, right)):
                    if u > v:
                        fill_back.append(float((h["s1"] - h["s0"]) * (u - v)))
            out[i]["fill"] = band_areas(ls[0], ls[-1], w.closed, fill_cuts) + fill_back
            recesses = []
            for h in holes:
                lo, hi, s0, s1 = h["lo"], h["hi"], h["s0"], h["s1"]
                if lo <= right and hi >= left:
                    for k, s in ((0, s0), (1, s1)):
                        if h["open"][k] is None:
                            out[i]["caps"].append([pt(h["j"][2 * k]), pt(h["j"][2 * k + 1])])
                        else:
                            clo, chi = h["open"][k]
                            out[i]["caps"].append([pt(beside(w, s, left)), pt(beside(w, s, chi))])
                            out[i]["caps"].append([pt(beside(w, s, clo)), pt(beside(w, s, right))])
                    continue
                for k, s in ((0, s0), (1, s1)):
                    if h["open"][k] is None:
                        out[i]["caps"].append([pt(beside(w, s, hi)), pt(beside(w, s, lo))])
                for side in (hi, lo):
                    if right < side < left and side not in b:
                        recesses.append({"pts": [pt(beside(w, s0, side)), pt(beside(w, s1, side))], "closed": False})
            out[i]["recesses"] = recesses
        return out


def cases(multi=False):
    c = []

    def case(name, walls, openings):
        drawn = PocketSolver(walls, openings).solve()
        places = [jambs(walls[o.wall], o) for o in openings]
        c.append({
            "name": name,
            "walls": [w.json() for w in walls],
            "openings": [o.json() for o in openings],
            "jambs": [None if j is None else [pt(q) for q in j] for j in places],
            "symbols": [[] if j is None else symbol(j, o) for j, o in zip(places, openings)],
            # A swinging door's sweep is ADR 0772's (its own cases); none here.
            "zones": [None if o.form == "door" and o.operation == "swing" else [] if j is None else zones(j, o)
                      for j, o in zip(places, openings)],
            "drawn": drawn,
        })

    w2, w3 = F(2, 10), F(3, 10)
    layered = [(F(2, 100), 5), (None, 1), (F(2, 100), 5)]
    straight = lambda t, **kw: [Wall([(0, 0), (8, 0)], t, **kw)]  # noqa: E731
    if multi:
        multi_cases(case, straight, layered)
        return c
    # Döner (§1).
    case("Döner: dört kanat, tambur ışık kadar", straight(w3),
         [Door(0, 4, F(24, 10), leaves=4, operation="revolving", frame=F(1, 10))])
    case("Döner: üç kanat, tambur 1,8 m", straight(w3),
         [Door(0, 4, F(24, 10), leaves=3, operation="revolving", frame=F(1, 10), drum=F(18, 10))])
    case("Döner: tambur ışıktan geniş, ışık kadar çizilir", straight(w3),
         [Door(0, 4, 2, leaves=4, operation="revolving", drum=3)])
    case("Döner: eğik duvarda", [Wall([(0, 0), (6, 8)], w3)],
         [Door(0, 5, F(22, 10), leaves=4, operation="revolving", frame=F(5, 100))])
    # Cebe giren (§2).
    case("Cebe giren: menteşe başta, tek kanat", straight(w3), [Door(0, 4, F(9, 10), operation="pocket")])
    case("Cebe giren: sonda, kasalı, montaj boşluklu", straight(w3),
         [Door(0, 4, F(9, 10), operation="pocket", hand="end", frame=F(3, 100), fit=F(1, 100))])
    case("Cebe giren: iki kanat iki cebe", straight(w3),
         [Door(0, 4, F(16, 10), leaves=2, operation="pocket", frame=F(3, 100))])
    case("Cebe giren: iki kanat, pay 0,6", straight(w3),
         [Door(0, 4, F(16, 10), leaves=2, operation="pocket", split=F(6, 10))])
    case("Cebe giren: kalın kanat, kısa pay", straight(w3),
         [Door(0, 4, F(9, 10), operation="pocket", leaf=F(5, 100), lip=F(2, 100))])
    case("Cebe giren: katmanlı duvarda", [Wall([(0, 0), (8, 0)], F(25, 100), comps=layered)],
         [Door(0, 4, F(9, 10), operation="pocket")])
    case("Cebe giren: ince duvar, sığar", straight(F(11, 100)), [Door(0, 4, F(8, 10), operation="pocket")])
    case("Cebe giren: eğik duvarda", [Wall([(0, 0), (6, 8)], w3)], [Door(0, 5, F(9, 10), operation="pocket")])
    case("Cebe giren: odanın duvarında", [Wall([(0, 0), (6, 0), (6, 4), (0, 4)], w3, closed=True)],
         [Door(0, 3, F(9, 10), operation="pocket", hand="end")])
    case("Cebe giren: yanında kapı", straight(w3),
         [Door(0, 2, F(9, 10), operation="pocket", hand="end"), Door(0, 6, F(9, 10))])
    case("Cebe giren: ucu aşan kapı kesmez", straight(w3), [Door(0, F(2, 10), F(9, 10), operation="pocket")])
    # Katlanır (§3).
    case("Katlanır: dört panel başa", straight(w2), [Door(0, 4, F(16, 10), leaves=4, operation="folding")])
    case("Katlanır: üç panel sona, sağa, 45°", straight(w2),
         [Door(0, 4, F(15, 10), leaves=3, operation="folding", hand="end", side="right", swing=45)])
    case("Katlanır: altı panel iki yana", straight(w2),
         [Door(0, 4, F(24, 10), leaves=6, operation="folding", both=True, frame=F(4, 100))])
    case("Katlanır: iki panel 60°", straight(w2), [Door(0, 4, 1, leaves=2, operation="folding", swing=60)])
    case("Katlanır: beş panel iki yana, baş az", straight(w2),
         [Door(0, 4, 2, leaves=5, operation="folding", both=True)])
    # Sürme (§4).
    case("Sürme: iki kanat, bindirme 10 cm, raylı", straight(w2),
         [Door(0, 4, F(16, 10), leaves=2, operation="slide", frame=F(5, 100), overlap=F(1, 10), track=True)])
    case("Sürme: tek kanat yüzde rayda, sona kayar", straight(w2),
         [Door(0, 4, F(9, 10), operation="slide", hand="end", track=True)])
    case("Sürme: tek kanat yüzde rayda, sağ yüz, başa kayar", straight(w2),
         [Door(0, 4, F(9, 10), operation="slide", side="right", track=True, leaf=F(5, 100))])
    case("Sürme pencere: bindirme ve ray", [Wall([(0, 0), (8, 0)], F(25, 100), comps=layered)],
         [Door(0, 4, F(15, 10), form="window", operation="slide", frame=F(6, 100), overlap=F(8, 100), track=True)])
    return c


def multi_cases(case, straight, layered):
    """ADR 0792 §1: three and four sliding leaves on two or three tracks, stacking at a jamb or parting to both."""
    w2, w3 = F(2, 10), F(3, 10)
    S = lambda **kw: Door(0, 4, F(24, 10), operation="slide", **kw)  # noqa: E731
    case("Üç kanat iki rayda, başa toplanır (OXO)", straight(w2), [S(leaves=3)])
    case("Üç kanat iki rayda, sona toplanır (OXO)", straight(w2), [S(leaves=3, hand="end")])
    case("Üç kanat üç rayda, teleskop (OXX)", straight(w2), [S(leaves=3, tracks=3)])
    case("Üç kanat üç rayda, sona (XXO)", straight(w2), [S(leaves=3, tracks=3, hand="end")])
    case("Dört kanat iki rayda, başa (OXOO)", straight(w2), [Door(0, 4, F(32, 10), operation="slide", leaves=4)])
    case("Dört kanat üç rayda, başa (OXXO)", straight(w2),
         [Door(0, 4, F(32, 10), operation="slide", leaves=4, tracks=3)])
    case("Dört kanat üç rayda, sona", straight(w2),
         [Door(0, 4, F(32, 10), operation="slide", leaves=4, tracks=3, hand="end")])
    case("Dört kanat iki yana (OXXO)", straight(w2),
         [Door(0, 4, F(32, 10), operation="slide", leaves=4, both=True)])
    case("Dört kanat iki yana, kasalı, raylı", straight(w2),
         [Door(0, 4, F(32, 10), operation="slide", leaves=4, both=True, frame=F(5, 100), track=True)])
    case("Üç kanat, 8 cm bindirme, raylı", straight(w2), [S(leaves=3, overlap=F(8, 100), track=True)])
    case("Üç kanat üç rayda, raylı, sağa açılır", straight(w2), [S(leaves=3, tracks=3, track=True, side="right")])
    case("Dört kanat iki yana, sağa açılır", straight(w2),
         [Door(0, 4, F(32, 10), operation="slide", leaves=4, both=True, side="right")])
    case("Üç kanat, montaj boşluklu, kasalı", straight(w3),
         [S(leaves=3, fit=F(1, 100), frame=F(4, 100), overlap=F(5, 100))])
    case("Dört kanat, bindirmesiz", straight(w2),
         [Door(0, 4, F(32, 10), operation="slide", leaves=4, tracks=3, overlap=0)])
    case("Üç kanat, dar açıklık, ok kısa", straight(w2), [Door(0, 4, F(9, 10), operation="slide", leaves=3)])
    case("Üç kanat eğik duvarda", [Wall([(0, 0), (6, 8)], w3)],
         [Door(0, 5, F(24, 10), operation="slide", leaves=3, tracks=3)])
    case("Dört kanat katmanlı duvarda", [Wall([(0, 0), (8, 0)], F(25, 100), comps=layered)],
         [Door(0, 4, F(32, 10), operation="slide", leaves=4, both=True, track=True)])
    case("Üç kanatlı sürme pencere, iki ray", straight(w3),
         [Door(0, 4, F(24, 10), form="window", operation="slide", leaves=3, frame=F(6, 100))])
    case("Dört kanatlı sürme pencere, iki yana, raylı", straight(w3),
         [Door(0, 4, F(32, 10), form="window", operation="slide", leaves=4, both=True, track=True)])
    case("Üç kanatlı pencere üç rayda, sağa, sona", straight(w3),
         [Door(0, 4, F(24, 10), form="window", operation="slide", leaves=3, tracks=3, side="right", hand="end")])
    case("Odanın duvarında dört kanat", [Wall([(0, 0), (6, 0), (6, 4), (0, 4)], w3, closed=True)],
         [Door(0, 3, F(32, 10), operation="slide", leaves=4, tracks=3)])
    case("Yanında kapı, üç kanat", straight(w2),
         [Door(0, F(25, 10), F(24, 10), operation="slide", leaves=3), Door(0, 6, F(9, 10))])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    bad = 0
    for out, multi in ((OUT, False), (MULTI, True)):
        text = json.dumps({"cases": cases(multi)}, ensure_ascii=False, indent=1) + "\n"
        if args.check:
            if not out.exists() or out.read_text() != text:
                print(f"{out.relative_to(ROOT)} güncel değil: betiği --check olmadan çalıştırın", file=sys.stderr)
                bad = 1
            else:
                print(f"{out.relative_to(ROOT)}: güncel")
            continue
        out.write_text(text)
        print(f"{out.relative_to(ROOT)}: {len(json.loads(text)['cases'])} durum")
    return bad


if __name__ == "__main__":
    sys.exit(main())
