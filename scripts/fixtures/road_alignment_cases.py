#!/usr/bin/env python3
"""The horizontal alignment's shared cases (docs/adr/0901, plan/road.md Y3.1), from an independent reference: the
textbook PI formulas (the unequal-clothoid tangent lengths T1 = k1 + ((R+p2) − (R+p1)cos Δ)/sin Δ), mpmath's quadrature
at 40 digits for the clothoids, a point's feet as the local minima of its distance along the whole alignment (sampled,
then golden-section search at 40 digits); no KentOS code.

    uv run --no-project --python 3.11 --with mpmath python3 scripts/fixtures/road_alignment_cases.py
    ... --check   # fails when fixtures/road/v1/alignments.json is not what this writes
"""

import json
import math
import sys
from pathlib import Path

import mpmath as mp

mp.mp.dps = 40
OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "alignments.json"
MEET = mp.mpf("0.001")


def fl(v):
    return float(v)


# ---- elements -----------------------------------------------------------------------------------------------------

class El:
    """θ(s) = θ0 + k0 s + (k1 − k0) s² / (2L)."""

    def __init__(self, id, kind, start, theta0, length, k0, k1, station):
        self.id, self.kind, self.start, self.theta0 = id, kind, start, mp.mpf(theta0)
        self.length, self.k0, self.k1, self.station = mp.mpf(length), mp.mpf(k0), mp.mpf(k1), mp.mpf(station)

    def rate(self):
        return (self.k1 - self.k0) / self.length if self.length else mp.mpf(0)

    def theta(self, s):
        return self.theta0 + self.k0 * s + self.rate() * s * s / 2

    def curvature(self, s):
        return self.k0 + self.rate() * s

    def point(self, s):
        s = mp.mpf(s)
        if s == 0:
            return list(self.start)
        if self.kind == "tangent":
            return [self.start[0] + s * mp.cos(self.theta0), self.start[1] + s * mp.sin(self.theta0)]
        if self.kind == "arc":
            k = self.k0
            return [self.start[0] + (mp.sin(self.theta0 + k * s) - mp.sin(self.theta0)) / k,
                    self.start[1] - (mp.cos(self.theta0 + k * s) - mp.cos(self.theta0)) / k]
        x = mp.quad(lambda t: mp.cos(self.theta(t)), [0, s])
        y = mp.quad(lambda t: mp.sin(self.theta(t)), [0, s])
        return [self.start[0] + x, self.start[1] + y]

    def end(self):
        return self.point(self.length)

    def json(self):
        return {"id": self.id, "kind": self.kind, "station": fl(self.station), "length": fl(self.length),
                "k0": fl(self.k0), "k1": fl(self.k1), "start": [fl(self.start[0]), fl(self.start[1])],
                "theta0": fl(self.theta0)}


def wrap(a):
    a = mp.fmod(a, 2 * mp.pi)
    if a <= -mp.pi:
        a += 2 * mp.pi
    elif a > mp.pi:
        a -= 2 * mp.pi
    return a


def spiral_pk(r, l):
    if l == 0:
        return mp.mpf(0), mp.mpf(0)
    tau = l / (2 * r)
    x = mp.quad(lambda t: mp.cos(t * t / (2 * r * l)), [0, l])
    y = mp.quad(lambda t: mp.sin(t * t / (2 * r * l)), [0, l])
    return y - r * (1 - mp.cos(tau)), x - r * mp.sin(tau)


OVE_SAMPLES = 512


def ove(pis, P, head, i, c1, c2):
    """The ove between curves i and i + 1 (docs/adr/0979 §2), solved on the PIs with the textbook tangent lengths: the
    middle tangent is the clothoid's origin tangent; each circle is where a curve with a clothoid of A²/R from that
    origin O would be, its foot on the middle tangent ((R + p_entry) − (R + p_exit) cos Δ) / sin Δ from its PI; O is
    x_M(R, A²/R) from the foot, before it when the radius falls, after it when it grows; both O's are one."""
    r1, r2 = c1["r"], c2["r"]
    d1, d2 = abs(c1["delta"]), abs(c2["delta"])
    bc = mp.hypot(P[i + 1][0] - P[i][0], P[i + 1][1] - P[i][1])
    closing = r1 > r2
    s = -1 if closing else 1

    def parts(a):
        a = mp.mpf(a)
        p1, x1 = spiral_pk(r1, a * a / r1)
        p2, x2 = spiral_pk(r2, a * a / r2)
        u1 = ((r1 + c1["p1"]) - (r1 + p1) * mp.cos(d1)) / mp.sin(d1)  # foot of circle 1 past PI i
        f2 = ((r2 + c2["p2"]) - (r2 + p2) * mp.cos(d2)) / mp.sin(d2)  # foot of circle 2 before PI i + 1
        return p1, x1, p2, x2, u1, f2

    def g(a):
        p1, x1, p2, x2, u1, f2 = parts(a)
        return bc - f2 - u1 + s * (x2 - x1)

    if closing:
        most = r2 * mp.sqrt(max(0, 2 * (d2 - c2["l2"] / (2 * r2))))
    else:
        most = r1 * mp.sqrt(max(0, 2 * (d1 - c1["l1"] / (2 * r1))))
    g0 = g(0)
    xs = [most * k / OVE_SAMPLES for k in range(OVE_SAMPLES + 1)]
    gs = [g0] + [g(x) for x in xs[1:]]
    roots = []
    # Where the plain circles' feet meet (|g(0)| within MEET) the root at A = 0 is no ove: the scan starts past it.
    for k in range(1 if abs(g0) <= MEET else 0, OVE_SAMPLES):
        if (gs[k] > 0) != (gs[k + 1] > 0):
            roots.append(mp.findroot(g, (xs[k], xs[k + 1]), solver="anderson"))
    if not roots:
        if min(gs) > 0:
            raise Refused(kind="OveTooLong", a=pis[i]["id"], b=pis[i + 1]["id"], excess=fl(min(gs)))
        raise Refused(kind="CurvesOverlap", a=pis[i]["id"], b=pis[i + 1]["id"], excess=fl(-max(gs)))
    if len(roots) > 1:
        raise Refused(kind="OveAmbiguous", a=pis[i]["id"], b=pis[i + 1]["id"], parameters=[fl(r) for r in roots])
    a = roots[0]
    p1, x1, p2, x2, u1, f2 = parts(a)
    o = {"a": a, "closing": closing, "length": a * a * abs(1 / r2 - 1 / r1)}
    # Curve i: its exit side is the virtual clothoid's; its tangent lengths to TS and to O. Its reported shifts stay
    # its own clothoids' (none on the ove's side).
    c1["oveOut"] = o
    c1["t1"] = c1["k1"] + ((r1 + p1) - (r1 + c1["p1"]) * mp.cos(d1)) / mp.sin(d1)
    c1["t2"] = u1 + s * x1
    # Curve i + 1: its entry side is the virtual clothoid's; its centre stands R + p2 off the middle tangent at the
    # foot f2 before its PI.
    c2["oveIn"] = o
    c2["t1"] = bc - c1["t2"]
    c2["foot"], c2["offset"] = f2, r2 + p2
    c2["t2"] = c2["k2"] + ((r2 + p2) - (r2 + c2["p2"]) * mp.cos(d2)) / mp.sin(d2)


class Refused(Exception):
    def __init__(self, **kw):
        self.kw = kw


def build(pis):
    if len(pis) < 2:
        raise Refused(kind="TooFewPoints")
    ids = set()
    for i, p in enumerate(pis):
        if p["id"] in ids or not p["id"].strip():
            raise Refused(kind="DuplicateId", pi=p["id"])
        ids.add(p["id"])
        end = i in (0, len(pis) - 1)
        curved = p["radius"] or p["spiralIn"] or p["spiralOut"]
        if p["radius"] < 0 or p["spiralIn"] < 0 or p["spiralOut"] < 0 or (end and curved) or (p["radius"] == 0 and curved):
            raise Refused(kind="BadCurve", pi=p["id"])
        if i and p["x"] == pis[i - 1]["x"] and p["y"] == pis[i - 1]["y"]:
            raise Refused(kind="SamePlace", pi=p["id"])
    P = [[mp.mpf(p["x"]), mp.mpf(p["y"])] for p in pis]
    head = [mp.atan2(P[i + 1][1] - P[i][1], P[i + 1][0] - P[i][0]) for i in range(len(P) - 1)]
    curves = {}
    for i in range(1, len(P) - 1):
        p = pis[i]
        delta = wrap(head[i] - head[i - 1])
        if abs(abs(delta) - mp.pi) < mp.mpf("1e-12"):
            raise Refused(kind="TurnTooSharp", pi=p["id"])
        if p["radius"] == 0 or delta == 0:
            continue
        r, l1, l2 = mp.mpf(p["radius"]), mp.mpf(p["spiralIn"]), mp.mpf(p["spiralOut"])
        turn = (l1 + l2) / (2 * r)
        if turn > abs(delta) + mp.mpf("1e-12"):
            raise Refused(kind="SpiralsLong", pi=p["id"], turn=fl(turn), delta=fl(abs(delta)))
        p1, k1 = spiral_pk(r, l1)
        p2, k2 = spiral_pk(r, l2)
        d = abs(delta)
        t1 = k1 + ((r + p2) - (r + p1) * mp.cos(d)) / mp.sin(d)
        t2 = k2 + ((r + p1) - (r + p2) * mp.cos(d)) / mp.sin(d)
        curves[i] = dict(delta=delta, sign=mp.sign(delta), r=r, l1=l1, l2=l2, t1=t1, t2=t2, p1=p1, p2=p2, k1=k1, k2=k2)
    # The oves (docs/adr/0979): each solved on the PIs.
    for i, p in enumerate(pis):
        if not p.get("ove"):
            continue
        if i == 0 or i + 2 >= len(pis) or i not in curves or i + 1 not in curves:
            raise Refused(kind="BadOve", pi=p["id"], why="NoNextCurve")
        c1, c2 = curves[i], curves[i + 1]
        if c1["sign"] != c2["sign"]:
            raise Refused(kind="BadOve", pi=p["id"], why="Reverse")
        if c1["r"] == c2["r"]:
            raise Refused(kind="BadOve", pi=p["id"], why="EqualRadii")
        if c1["l2"] != 0 or c2["l1"] != 0:
            raise Refused(kind="BadOve", pi=p["id"], why="SpiralOnSharedSide")
        if pis[i + 1].get("ove") or "oveIn" in c1:
            raise Refused(kind="BadOve", pi=p["id"], why="Chained")
        ove(pis, P, head, i, c1, c2)
    tin = lambda i: curves[i]["t1"] if i in curves else 0
    tout = lambda i: curves[i]["t2"] if i in curves else 0
    tangents = []
    for i in range(len(P) - 1):
        if i in curves and "oveOut" in curves[i]:
            tangents.append(0)
            continue
        ln = mp.hypot(P[i + 1][0] - P[i][0], P[i + 1][1] - P[i][1]) - tout(i) - tin(i + 1)
        if ln < -MEET:
            raise Refused(kind="CurvesOverlap", a=pis[i]["id"], b=pis[i + 1]["id"], excess=fl(-ln))
        tangents.append(0 if ln <= MEET else ln)
    els, reports, station = [], [], mp.mpf(0)
    for i in range(len(P) - 1):
        u = [mp.cos(head[i]), mp.sin(head[i])]
        if tangents[i] > 0:
            st = [P[i][0] + tout(i) * u[0], P[i][1] + tout(i) * u[1]]
            els.append(El(pis[i]["id"] + "/aliyman", "tangent", st, head[i], tangents[i], 0, 0, station))
            station += tangents[i]
        c = curves.get(i + 1)
        if not c:
            continue
        pid = pis[i + 1]["id"]
        k = c["sign"] / c["r"]
        place = [P[i + 1][0] - c["t1"] * u[0], P[i + 1][1] - c["t1"] * u[1]]
        theta = head[i]
        stations = [station]
        if "oveIn" in c:
            # Entered by the ove: from its end; TS at its start.
            last = els[-1]
            place, theta, stations = last.end(), last.theta(last.length), [last.station]
        if c["l1"] > 0:
            e = El(pid + "/giris", "spiral", place, theta, c["l1"], 0, k, station)
            els.append(e)
            place, theta, station = e.end(), e.theta(e.length), station + e.length
        stations.append(station)
        more = 0
        if "oveOut" in c:
            tau = c["oveOut"]["a"] ** 2 / (2 * c["r"] ** 2)
            more += tau if c["oveOut"]["closing"] else -tau
        if "oveIn" in c:
            tau = c["oveIn"]["a"] ** 2 / (2 * c["r"] ** 2)
            more += -tau if c["oveIn"]["closing"] else tau
        arc = c["r"] * (abs(c["delta"]) - (c["l1"] + c["l2"]) / (2 * c["r"]) + more)
        if arc > 0:
            e = El(pid + "/kurp", "arc", place, theta, arc, k, k, station)
            els.append(e)
            place, theta, station = e.end(), e.theta(e.length), station + e.length
        stations.append(station)
        if c["l2"] > 0:
            e = El(pid + "/cikis", "spiral", place, theta, c["l2"], k, 0, station)
            els.append(e)
            place, theta, station = e.end(), e.theta(e.length), station + e.length
        if "oveOut" in c:
            o, nxt = c["oveOut"], curves[i + 2]
            e = El(pid + "/ove", "spiral", place, theta, o["length"], k, nxt["sign"] / nxt["r"], station)
            els.append(e)
            o["station"] = station
            place, theta, station = e.end(), e.theta(e.length), station + e.length
        stations.append(station)
        # The centre: the foot of the entry tangent's offset line, R + p1 inwards (the entry side's offset: an ove's
        # second circle's is R + p of its virtual clothoid).
        back = c.get("foot", c["t1"] - c["k1"])
        foot = [P[i + 1][0] - back * u[0], P[i + 1][1] - back * u[1]]
        n = [-u[1] * c["sign"], u[0] * c["sign"]]
        off = c.get("offset", c["r"] + c["p1"])
        centre = [foot[0] + off * n[0], foot[1] + off * n[1]]
        c["centre"] = centre
        reports.append({"pi": pid, "delta": fl(c["delta"]), "tIn": fl(c["t1"]), "tOut": fl(c["t2"]), "arc": fl(arc),
                        "pIn": fl(c["p1"]), "pOut": fl(c["p2"]), "stations": [fl(s) for s in stations],
                        "radius": fl(c["r"]), "spiralIn": fl(c["l1"]), "spiralOut": fl(c["l2"]),
                        "centre": [fl(centre[0]), fl(centre[1])]})
    joins = []
    for i in range(1, len(P) - 2):
        if i in curves and i + 1 in curves and tangents[i] == 0:
            j = {"between": [pis[i]["id"], pis[i + 1]["id"]], "reverse": bool(curves[i]["sign"] != curves[i + 1]["sign"])}
            if "oveOut" in curves[i]:
                o, (a1, a2) = curves[i]["oveOut"], (curves[i]["centre"], curves[i + 1]["centre"])
                gap = abs(curves[i]["r"] - curves[i + 1]["r"]) - mp.hypot(a1[0] - a2[0], a1[1] - a2[1])
                j["ove"] = {"a": fl(o["a"]), "length": fl(o["length"]), "station": fl(o["station"]), "gap": fl(gap)}
            joins.append(j)
    return els, station, reports, joins


def at(els, length, s):
    s = mp.mpf(s)
    idx = 0
    for i, e in enumerate(els):
        if e.station <= s:
            idx = i
    e = els[idx]
    x, y = e.point(s - e.station)
    return {"station": fl(s), "x": fl(x), "y": fl(y), "direction": fl(e.theta(s - e.station)),
            "curvature": fl(e.curvature(s - e.station))}


# ---- a point's feet -----------------------------------------------------------------------------------------------

def place_at(els, s):
    e = els[0]
    for x in els:
        if x.station <= s:
            e = x
    return e, e.point(s - e.station)


def dist2(els, p, s):
    _, q = place_at(els, s)
    return (q[0] - p[0]) ** 2 + (q[1] - p[1]) ** 2


def fast_points(els, step):
    """Places along the alignment every `step` metres (and at element ends), by float Simpson steps."""
    out = []
    for e in els:
        n = max(2, math.ceil(float(e.length) / step))
        th0, k0, c = float(e.theta0), float(e.k0), float(e.rate())
        x, y = float(e.start[0]), float(e.start[1])
        h = float(e.length) / n
        th = lambda t: th0 + k0 * t + c * t * t / 2
        for i in range(n):
            out.append((float(e.station) + h * i, x, y))
            a = h * i
            m, b = a + h / 2, a + h
            x += h / 6 * (math.cos(th(a)) + 4 * math.cos(th(m)) + math.cos(th(b)))
            y += h / 6 * (math.sin(th(a)) + 4 * math.sin(th(m)) + math.sin(th(b)))
    last = els[-1]
    ex, ey = last.end()
    out.append((float(last.station + last.length), float(ex), float(ey)))
    return out


def golden(f, a, b):
    a, b = mp.mpf(a), mp.mpf(b)
    g = (mp.sqrt(5) - 1) / 2
    c, d = b - g * (b - a), a + g * (b - a)
    fc, fd = f(c), f(d)
    while b - a > mp.mpf("1e-14"):
        if fc < fd:
            b, d, fd = d, c, fc
            c = b - g * (b - a)
            fc = f(c)
        else:
            a, c, fc = c, d, fd
            d = a + g * (b - a)
            fd = f(d)
    return (a + b) / 2


def direction_at(els, s):
    """The direction at a station; at a kink the mean of both sides."""
    before = after = None
    for e in els:
        if e.station <= s <= e.station + e.length:
            th = e.theta(s - e.station)
            if abs(s - e.station - e.length) < mp.mpf("1e-9"):
                before = th
            elif abs(s - e.station) < mp.mpf("1e-9"):
                after = th
            else:
                return th
    if before is not None and after is not None:
        return before + wrap(after - before) / 2
    return before if before is not None else after


def locate(els, length, p):
    p = [mp.mpf(p[0]), mp.mpf(p[1])]
    pts = fast_points(els, min(0.5, float(length) / 4000))
    d = [(px - float(p[0])) ** 2 + (py - float(p[1])) ** 2 for _, px, py in pts]
    feet = []
    for i in range(1, len(pts) - 1):
        if d[i] <= d[i - 1] and d[i] <= d[i + 1]:
            s = golden(lambda s: dist2(els, p, s), pts[i - 1][0], pts[i + 1][0])
            if 0 < s < length:
                feet.append(s)
    out = []
    for s in feet:
        if any(abs(s - t) < mp.mpf("1e-7") for t in out):
            continue
        out.append(s)
    res = []
    for s in out:
        _, q = place_at(els, s)
        th = direction_at(els, s)
        cross = mp.cos(th) * (p[1] - q[1]) - mp.sin(th) * (p[0] - q[0])
        dd = mp.hypot(p[0] - q[0], p[1] - q[1])
        res.append({"station": s, "offset": -dd if cross > 0 else dd})
    res.sort(key=lambda f: (round(float(abs(f["offset"])), 8), float(f["station"])))
    beyond = None
    if not res:
        ds, de = dist2(els, p, 0), dist2(els, p, length)
        beyond = {"end": "start", "station": 0.0} if ds <= de else {"end": "end", "station": fl(length)}
    return {"point": [fl(p[0]), fl(p[1])], "feet": [{"station": fl(f["station"]), "offset": fl(f["offset"])} for f in res],
            "beyond": beyond}


# ---- km --------------------------------------------------------------------------------------------------------------

def regions(st, length):
    out = [[mp.mpf(0), mp.mpf(st["start"]), 1, length]]
    for i, q in enumerate(st["equations"]):
        last = out[-1]
        a = last[0] + last[2] * (mp.mpf(q["back"]) - last[1])
        if not (a > last[0] and a < length):
            return i
        last[3] = a
        out.append([a, mp.mpf(q["ahead"]), 1 if q["increasing"] else -1, length])
    return out


def km_of(rs, s):
    r = [x for x in rs if x[0] <= s][-1]
    return r[1] + r[2] * (s - r[0])


def stations_of(rs, km):
    out = []
    for i, r in enumerate(rs):
        s = r[0] + r[2] * (km - r[1])
        last = i == len(rs) - 1
        if s >= r[0] - mp.mpf("1e-9") and (s < r[3] - mp.mpf("1e-9") or (last and s <= r[3] + mp.mpf("1e-9"))):
            out.append([i, fl(min(max(s, r[0]), r[3]))])
    return out


# ---- the parallel axis ----------------------------------------------------------------------------------------------

def offset(els, length, d):
    d = mp.mpf(d)
    for e in els:
        if e.kind == "tangent":
            continue
        fa, fb = 1 + e.k0 * d <= 0, 1 + e.k1 * d <= 0
        if fa or fb:
            rate = e.rate()
            sf = (-1 / d - e.k0) / rate if rate else 0
            sf = min(max(sf, 0), e.length)
            frm, to = (0, e.length) if fa and fb else ((0, sf) if fa else (sf, e.length))
            return {"d": fl(d), "cusp": [fl(e.station + frm), fl(e.station + to)]}
    total = sum(mp.quad(lambda s: abs(1 + e.curvature(s) * d), [0, e.length]) for e in els)
    return {"d": fl(d), "length": fl(total)}


# ---- cases -----------------------------------------------------------------------------------------------------------

def pi(id, x, y, r=0, l1=0, l2=0):
    return {"id": id, "x": x, "y": y, "radius": r, "spiralIn": l1, "spiralOut": l2}


def by_legs(start, legs):
    """PIs from a start and (heading in degrees, length, id, radius, l1, l2) legs: each leg ends at a PI."""
    out = [pi("a", *start)]
    x, y = mp.mpf(start[0]), mp.mpf(start[1])
    for h, ln, id, r, l1, l2 in legs:
        x += mp.mpf(ln) * mp.cos(mp.radians(h))
        y += mp.mpf(ln) * mp.sin(mp.radians(h))
        out.append(pi(id, fl(x), fl(y), r, l1, l2))
    return out


def tangents_of(heads, radii):
    """The tangent lengths of curves on headings, to place PIs where two curves just meet."""
    out = []
    for i in range(1, len(heads)):
        d = abs(wrap(mp.radians(heads[i]) - mp.radians(heads[i - 1])))
        r, l1, l2 = radii[i - 1]
        p1, k1 = spiral_pk(mp.mpf(r), mp.mpf(l1))
        p2, k2 = spiral_pk(mp.mpf(r), mp.mpf(l2))
        out.append((k1 + ((r + p2) - (r + p1) * mp.cos(d)) / mp.sin(d), k2 + ((r + p1) - (r + p2) * mp.cos(d)) / mp.sin(d)))
    return out


def meeting(start, heads, radii, first, last):
    """PIs on `heads` with curves `radii`, the legs between curves exactly their tangent lengths' sum."""
    t = tangents_of(heads, radii)
    legs = [(heads[0], first, "p1", *radii[0])]
    for i in range(1, len(heads) - 1):
        legs.append((heads[i], fl(t[i - 1][1] + t[i][0]), f"p{i + 1}", *radii[i]))
    legs.append((heads[-1], last, "b", 0, 0, 0))
    return by_legs(start, legs)


TM = (485_000.0, 4_420_000.0)


def ove_pis(r1, r2, a, d1, d2, l_in=0, l_out=0, legs=(300, 300), at=(0, 0), heading=0, right=False, opening=False):
    """Four PIs whose middle pair holds an ove of parameter `a` from radius r1 to r2 (r1 > r2), built from the egg
    clothoid itself (docs/adr/0979): its origin O on the middle tangent, the osculating circles at A²/r1 and A²/r2
    (centre P(s) + (A²/s)(−sin τ, cos τ)), the entry tangent turned d1 before and the exit tangent d2 after, each its
    clothoid's shift off its circle; the PIs where those tangents cross the middle one. Turned and moved to `at` and
    `heading` (degrees), mirrored for a right turn; reversed for an opening ove (the chain read backwards)."""
    A, R1, R2 = mp.mpf(a), mp.mpf(r1), mp.mpf(r2)
    pt = lambda s: [mp.quad(lambda t: mp.cos(t * t / (2 * A * A)), [0, s]), mp.quad(lambda t: mp.sin(t * t / (2 * A * A)), [0, s])]

    def centre(r):
        s = A * A / r
        p, tau = pt(s), s * s / (2 * A * A)
        return [p[0] - r * mp.sin(tau), p[1] + r * mp.cos(tau)]

    def crossing(c, r, l, h):
        # The tangent at heading h, r + p(r, l) to the right of the centre (a left turn's centre is on the left).
        p, _ = spiral_pk(r, mp.mpf(l))
        n = [-mp.sin(h), mp.cos(h)]
        q = [c[0] - (r + p) * n[0], c[1] - (r + p) * n[1]]
        return [q[0] - q[1] * mp.cos(h) / mp.sin(h), mp.mpf(0)]

    din, dout = -mp.mpf(d1), mp.mpf(d2)
    b, c = crossing(centre(R1), R1, l_in, din), crossing(centre(R2), R2, l_out, dout)
    a0 = [b[0] - legs[0] * mp.cos(din), b[1] - legs[0] * mp.sin(din)]
    d = [c[0] + legs[1] * mp.cos(dout), c[1] + legs[1] * mp.sin(dout)]
    pts = [a0, b, c, d]
    if right:
        pts = [[x, -y] for x, y in pts]
    h = mp.radians(heading)
    pts = [[at[0] + x * mp.cos(h) - y * mp.sin(h), at[1] + x * mp.sin(h) + y * mp.cos(h)] for x, y in pts]
    out = [pi("a", fl(pts[0][0]), fl(pts[0][1])), pi("p1", fl(pts[1][0]), fl(pts[1][1]), r1, l_in, 0),
           pi("p2", fl(pts[2][0]), fl(pts[2][1]), r2, 0, l_out), pi("b", fl(pts[3][0]), fl(pts[3][1]))]
    out[1]["ove"] = True
    if opening:
        rev = list(reversed(out))
        out = [pi("a", rev[0]["x"], rev[0]["y"]), pi("p1", rev[1]["x"], rev[1]["y"], r2, l_out, 0),
               pi("p2", rev[2]["x"], rev[2]["y"], r1, 0, l_in), pi("b", rev[3]["x"], rev[3]["y"])]
        out[1]["ove"] = True
    return out


def moved(pis, k, dx, dy):
    """`pis` with PI k moved by (dx, dy)."""
    out = [dict(p) for p in pis]
    out[k]["x"], out[k]["y"] = out[k]["x"] + dx, out[k]["y"] + dy
    return out


def flagged(pis, k, **kw):
    out = [dict(p) for p in pis]
    out[k].update(kw)
    return out


OVE = ove_pis(500, 200, 250, 0.6, 1.0)
OVE_LONG = ove_pis(500, 200, 250, 0.6, 1.0, legs=(300, 800))
OVE_SPIRALS = ove_pis(500, 200, 250, 0.6, 1.1, l_in=80, l_out=60)

CASES = [
    {"name": "Tek aliyman", "pis": [pi("a", 0, 0), pi("b", 300, 400)], "at": [0, 250, 500],
     "locate": [[300, 0], [-100, 50]]},
    {"name": "Kurpsuz kırık nokta", "pis": [pi("a", 0, 0), pi("p1", 200, 0), pi("b", 200, 200)], "at": [100, 200, 300],
     "locate": [[250, -50], [150, 50], [100, 100]]},
    {"name": "Yalnız kurp, sola", "pis": [pi("a", 0, 0), pi("p1", 400, 0, 300), pi("b", 700, 300)],
     "at": [10, 200, 300, 450], "locate": [[350, 60], [380, -40]]},
    {"name": "Simetrik klotoid, sola", "pis": [pi("a", 1000, 2000), pi("p1", 1600, 2000, 400, 80, 80), pi("b", 2100, 2500)],
     "at": [100, 450, 500, 560, 700, 900], "locate": [[1520, 2030], [1650, 2100], [1700, 1900]], "offset": [10, -25, -410]},
    {"name": "Simetrik klotoid, sağa", "pis": [pi("a", 0, 0), pi("p1", 500, 0, 250, 60, 60), pi("b", 700, -400)],
     "at": [300, 420, 500, 600], "locate": [[480, -60], [520, 40]], "offset": [15, 260]},
    {"name": "Eşit olmayan klotoidler", "pis": [pi("a", 0, 0), pi("p1", 600, 100, 350, 60, 120), pi("b", 900, 700)],
     "at": [400, 500, 600, 700, 800], "locate": [[600, 300], [560, 120]]},
    {"name": "Yalnız giriş klotoidi", "pis": [pi("a", 0, 0), pi("p1", 500, 0, 300, 90, 0), pi("b", 500, 600)],
     "at": [300, 400, 500, 650], "locate": [[480, 150]]},
    {"name": "Büyük koordinatlarda beş PI", "pis": [
        pi("a", TM[0], TM[1]), pi("p1", TM[0] + 800, TM[1] + 120, 500, 100, 100),
        pi("p2", TM[0] + 1500, TM[1] - 300, 350, 70, 90), pi("p3", TM[0] + 2400, TM[1] - 250),
        pi("p4", TM[0] + 3100, TM[1] + 400, 600, 0, 0), pi("b", TM[0] + 3900, TM[1] + 500)],
     "at": [0, 700, 1234.5, 2000, 3000], "locate": [[TM[0] + 1500, TM[1] - 150], [TM[0] + 3150, TM[1] + 300]],
     "stationing": {"start": 12000, "equations": []}, "km": [0, 1234.5]},
    {"name": "Ters kurp", "pis": meeting((0, 0), [0, 40, -10], [(300, 60, 60), (250, 50, 50)], 400, 300),
     "at": [300, 500, 700]},
    {"name": "Bileşik kurp", "pis": meeting((0, 0), [0, 25, 70], [(500, 0, 0), (200, 0, 0)], 300, 300),
     "at": [200, 400, 600]},
    {"name": "Kurplar binişiyor", "pis": [pi("a", 0, 0), pi("p1", 300, 0, 300, 50, 50), pi("p2", 450, 150, 300, 50, 50),
                                          pi("b", 450, 500)]},
    {"name": "Klotoidler sapmadan uzun", "pis": [pi("a", 0, 0), pi("p1", 400, 0, 200, 150, 150), pi("b", 800, 100)]},
    {"name": "Geri dönen doğrultu", "pis": [pi("a", 0, 0), pi("p1", 100, 0), pi("b", 50, 0)]},
    {"name": "Aynı yerde iki PI", "pis": [pi("a", 0, 0), pi("p1", 100, 0), pi("p2", 100, 0), pi("b", 200, 50)]},
    {"name": "Başlangıçta kurp", "pis": [pi("a", 0, 0, 100), pi("b", 100, 0)]},
    {"name": "Yarıçapsız klotoid", "pis": [pi("a", 0, 0), pi("p1", 100, 0, 0, 20, 0), pi("b", 200, 50)]},
    {"name": "Aynı kimlik", "pis": [pi("a", 0, 0), pi("a", 100, 0), pi("b", 200, 50)]},
    {"name": "Tek nokta", "pis": [pi("a", 0, 0)]},
    {"name": "Km eşitlikleri", "pis": [pi("a", 0, 0), pi("p1", 500, 0, 400, 60, 60), pi("b", 900, 300)],
     "stationing": {"start": 2000, "equations": [{"back": 2200, "ahead": 2500, "increasing": True},
                                                 {"back": 2650, "ahead": 2700, "increasing": False}]},
     "km": [0, 150, 200, 250, 350, 400, 600], "kmStations": [2100, 2550, 2600, 2650, 2690, 9999]},
    {"name": "Km eşitliği aksın dışında", "pis": [pi("a", 0, 0), pi("b", 300, 0)],
     "stationing": {"start": 0, "equations": [{"back": 400, "ahead": 500, "increasing": True}]}},
    {"name": "Km eşitliği geri gidiyor", "pis": [pi("a", 0, 0), pi("b", 300, 0)],
     "stationing": {"start": 100, "equations": [{"back": 50, "ahead": 500, "increasing": True}]}},
    {"name": "Kırık noktada eşit iki ayak", "pis": [pi("a", 0, 0), pi("p1", 100, 0), pi("b", 100, 100)],
     "locate": [[80, 20], [50, 50]]},
    {"name": "Uçların ötesi", "pis": [pi("a", 0, 0), pi("p1", 300, 0, 200, 40, 40), pi("b", 400, 300)],
     "locate": [[-50, 10], [450, 450]]},
    {"name": "Kurp içinde ve dışında", "pis": [pi("a", 0, 0), pi("p1", 400, 0, 300), pi("b", 400, 400)],
     "locate": [[300, 200], [430, -30], [200, 280]], "offset": [-100, 100, -320]},
    {"name": "Ove: R 500'den R 200'e, A 250", "pis": OVE, "at": [100, 300, 400, 500, 700],
     "locate": [[350, 120]], "offset": [-10, 12]},
    {"name": "Ove: giriş ve çıkış klotoidli", "pis": OVE_SPIRALS, "at": [200, 450, 600]},
    {"name": "Ove: sağa dönen", "pis": ove_pis(500, 200, 250, 0.6, 1.0, right=True), "at": [300, 450]},
    {"name": "Ove: açılan (R 200'den R 500'e)", "pis": ove_pis(500, 200, 250, 0.6, 1.0, opening=True), "at": [300, 450]},
    {"name": "Ove: açılan, sağa, klotoidli", "pis": ove_pis(500, 200, 250, 0.6, 1.1, 80, 60, right=True, opening=True),
     "at": [250, 500]},
    {"name": "Ove: büyük koordinatlarda, 37° dönük", "pis": ove_pis(800, 350, 400, 0.5, 0.9, 60, 60, at=TM, heading=37),
     "at": [400, 700], "stationing": {"start": 5000, "equations": []}, "km": [0, 600]},
    {"name": "Ove: yakın yarıçaplar R 1000 ve 900, kısa", "pis": ove_pis(1000, 900, 120, 0.3, 0.015), "at": [300, 500]},
    {"name": "Ove: iki çözüm, hangisi PI'larla seçilmeli", "pis": ove_pis(1000, 900, 120, 0.3, 0.3)},
    {"name": "Ove: çok farklı yarıçaplar R 2000 ve 150", "pis": ove_pis(2000, 150, 200, 0.2, 1.1), "at": [300, 450]},
    {"name": "Ove: uzun (ikinci kurbun yayı az kalır)", "pis": ove_pis(600, 250, 480, 0.5, 1.95, legs=(600, 900)), "at": [400, 700]},
    {"name": "Ove: geniş açılı kurplar", "pis": ove_pis(300, 120, 150, 1.2, 1.3), "at": [200, 400, 600]},
    {"name": "Ove ve ardından ayrı bir kurp",
     "pis": OVE_LONG[:3] + [pi("p3", OVE_LONG[3]["x"], OVE_LONG[3]["y"], 300, 40, 40),
                            pi("b", OVE_LONG[3]["x"] + 600, OVE_LONG[3]["y"] - 100)], "at": [300, 600, 900]},
    {"name": "Ove: ters kurpta olmaz",
     "pis": flagged(meeting((0, 0), [0, 40, -10], [(300, 0, 0), (250, 0, 0)], 400, 300), 1, ove=True)},
    {"name": "Ove: eşit yarıçapta olmaz",
     "pis": flagged(meeting((0, 0), [0, 25, 70], [(300, 0, 0), (300, 0, 0)], 300, 300), 1, ove=True)},
    {"name": "Ove: ilk kurbun çıkış klotoidi varken olmaz", "pis": flagged(OVE, 1, spiralOut=30)},
    {"name": "Ove: bitişten önceki PI'da olmaz", "pis": flagged(OVE, 2, ove=True, spiralOut=0) if False else
     [OVE[0], dict(OVE[1], ove=False), dict(OVE[2], ove=True), OVE[3]]},
    {"name": "Ove: art arda iki ove olmaz",
     "pis": OVE[:2] + [dict(OVE[2], ove=True), pi("p3", OVE[3]["x"], OVE[3]["y"], 120),
                       pi("b", OVE[3]["x"] + 300, OVE[3]["y"] + 400)]},
    {"name": "Ove: ortadaki aliyman çok uzun",
     "pis": moved(moved(OVE, 2, 400 * math.cos(0), 0), 3, 400, 0)},
    {"name": "Ove: daireler binişiyor", "pis": moved(moved(OVE, 2, -200, 0), 3, -200, 0)},
    {"name": "Ove klotoid", "element": {"kind": "spiral", "start": [100, 200], "theta0": 0.3, "length": 120,
                                        "k0": 1 / 600, "k1": 1 / 250}, "s": [0, 30, 60, 120]},
    {"name": "Sağa ove klotoid", "element": {"kind": "spiral", "start": [0, 0], "theta0": -1.2, "length": 200,
                                             "k0": -1 / 150, "k1": -1 / 900}, "s": [0, 50, 199.5, 200]},
]


def make(case):
    out = {"name": case["name"]}
    if "element" in case:
        e = case["element"]
        el = El("e", e["kind"], [mp.mpf(e["start"][0]), mp.mpf(e["start"][1])], e["theta0"], e["length"], e["k0"],
                e["k1"], 0)
        out["element"] = e
        out["points"] = []
        for s in case["s"]:
            x, y = el.point(s)
            out["points"].append({"s": s, "x": fl(x), "y": fl(y), "direction": fl(el.theta(mp.mpf(s))),
                                  "curvature": fl(el.curvature(mp.mpf(s)))})
        return out
    out["pis"] = case["pis"]
    try:
        els, length, reports, joins = build(case["pis"])
    except Refused as r:
        out["error"] = r.kw
        return out
    out["length"] = fl(length)
    out["elements"] = [e.json() for e in els]
    out["curves"] = reports
    out["joins"] = joins
    out["at"] = [at(els, length, s) for s in case.get("at", [])]
    out["locate"] = [locate(els, length, p) for p in case.get("locate", [])]
    out["offset"] = [offset(els, length, d) for d in case.get("offset", [])]
    if "stationing" in case:
        st = case["stationing"]
        out["stationing"] = st
        rs = regions(st, length)
        if isinstance(rs, int):
            out["stationingError"] = rs
        else:
            out["km"] = [{"station": s, "km": fl(km_of(rs, mp.mpf(s)))} for s in case.get("km", [])]
            out["kmStations"] = [{"km": k, "stations": stations_of(rs, mp.mpf(k))} for k in case.get("kmStations", [])]
    return out


def main():
    text = json.dumps({"cases": [make(c) for c in CASES]}, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            sys.exit(f"{OUT} is not what the reference writes")
        print(f"{OUT.name}: {len(CASES)} cases, the same")
        return
    OUT.write_text(text)
    print(f"wrote {OUT} ({len(CASES)} cases)")


if __name__ == "__main__":
    main()
