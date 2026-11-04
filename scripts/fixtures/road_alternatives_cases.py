#!/usr/bin/env python3
"""Where two alternatives of a road part, shared cases (docs/adr/0943, plan/road.md Y37.1), from an independent
reference: the alternative sampled every 0.25 m, each sample's distance to the base's polyline at 5 cm (numpy), the
side changes refined by bisection on the exact distance (the alignment reference's places at 40 digits, the foot by
golden-section search), the largest departure by golden section around the best sample; elevations by the vertical
reference. No KentOS code, and not KentOS's method (that one marches by the distance's Lipschitz bound).

The cases with tangents and grades are checked first against closed forms: a tangent leaving at angle θ departs at
t / sin θ, one running past the base's end at the end plus t, a PVI raised by h departs where the grade difference
reaches the height tolerance, two tangents crossing at slope 0.1 lie within t for only 2t / sin θ (20 cm at 1 cm, under
the sampling step: found by halving where the samples could hide a dip).

    uv run --no-project --python 3.11 --with mpmath --with numpy python3 scripts/fixtures/road_alternatives_cases.py
    ... --check   # fails when fixtures/road/v1/alternatives.json is not what this writes
"""

import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

import mpmath as mp
import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_alignment_cases as aref  # noqa: E402
import road_vertical_cases as zref  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "alternatives.json"
STEP = 0.25
pi, pvi, par = aref.pi, zref.pvi, zref.par


class Road:
    def __init__(self, pis, pvis=None):
        self.pis, self.pvis = pis, pvis
        self.els, self.length, _, _ = aref.build(pis)
        self.z = zref.solve(pvis)[1] if pvis else None
        pts = aref.fast_points(self.els, 0.05)
        self.st = np.array([p[0] for p in pts])
        self.xy = np.array([[p[1], p[2]] for p in pts])

    def place(self, s):
        _, q = aref.place_at(self.els, mp.mpf(s))
        return q

    def rough(self, p):
        """The nearest station on the polyline (float)."""
        a, b = self.xy[:-1], self.xy[1:]
        ab = b - a
        t = np.clip(((p[0] - a[:, 0]) * ab[:, 0] + (p[1] - a[:, 1]) * ab[:, 1]) / np.maximum((ab ** 2).sum(1), 1e-30), 0, 1)
        q = a + ab * t[:, None]
        d = np.hypot(q[:, 0] - p[0], q[:, 1] - p[1])
        i = int(np.argmin(d))
        return float(self.st[i] + t[i] * (self.st[i + 1] - self.st[i])), float(d[i])

    def gap(self, p):
        """The exact distance from p to the alignment and the station of its foot (none beyond an end)."""
        s0, _ = self.rough([float(p[0]), float(p[1])])
        lo, hi = max(0.0, s0 - 0.2), min(float(self.length), s0 + 0.2)
        s = aref.golden(lambda s: aref.dist2(self.els, p, s), lo, hi)
        d = mp.sqrt(aref.dist2(self.els, p, s))
        if s - lo < 1e-9 and lo == 0.0 or hi - s < 1e-9 and hi == float(self.length):
            end = mp.mpf(0) if lo == 0.0 and s - lo < 1e-9 else self.length
            return d, (None if beyond(self, p, end) else end)
        return d, s

    def rough_gaps(self, pts):
        return np.array([self.rough(p)[1] for p in pts])


def beyond(road, p, end):
    """Whether p lies past the end `end` (its projection on the end's direction falls outside the alignment)."""
    e = road.els[0] if end == 0 else road.els[-1]
    s = mp.mpf(0) if end == 0 else e.length
    th = e.theta(s)
    q = e.point(s)
    along = (p[0] - q[0]) * mp.cos(th) + (p[1] - q[1]) * mp.sin(th)
    return along < 0 if end == 0 else along > 0


def bisect(f, a, b, inside):
    a, b = mp.mpf(a), mp.mpf(b)
    while b - a > mp.mpf("1e-11"):
        m = (a + b) / 2
        if f(m) == inside:
            a = m
        else:
            b = m
    return a if inside else b


# A sample this near the tolerance by the rough polyline (its chords stray ~1e-6 m from curves of 300 m) is decided
# by the exact distance.
NEAR = 1e-4


def stretches(other, margin_rough, out_exact):
    """The stretches where out_exact(s) holds: samples every STEP by the rough margin (exactly near 0), side changes
    bisected exactly. Two neighbouring samples on one side whose margins add up to less than their spacing could hide
    a dip to the other side between them (the distance moves at most a metre per metre): the gap is halved until it
    cannot, or is a millimetre (a crossing of two alignments lies within the tolerance for only centimetres)."""
    n = int(math.ceil(float(other.length) / STEP))

    def flag(s):
        m = margin_rough(s)
        return (out_exact(s) if m is not None and abs(m) < NEAR else (m is not None and m > 0)), m

    samples = []

    def fill(a, fa, b, fb):
        (ka, ma), (kb, mb) = fa, fb
        if ka == kb and ma is not None and mb is not None and abs(ma) + abs(mb) < b - a and b - a > 1e-3:
            m = (a + b) / 2
            fm = flag(m)
            fill(a, fa, m, fm)
            samples.append((m, fm[0]))
            fill(m, fm, b, fb)

    ss = [float(other.length) * i / n for i in range(n + 1)]
    flags = [flag(s) for s in ss]
    samples.append((ss[0], flags[0][0]))
    for i in range(1, len(ss)):
        fill(ss[i - 1], flags[i - 1], ss[i], flags[i])
        samples.append((ss[i], flags[i][0]))
    out, start = [], (mp.mpf(0) if samples[0][1] else None)
    for (sa, a), (sb, b) in zip(samples, samples[1:]):
        if a != b:
            edge = bisect(out_exact, sa, sb, a)
            if a:
                out.append((start if start is not None else mp.mpf(0), edge))
                start = None
            else:
                start = edge
    if start is not None:
        out.append((start, other.length))
    return out


def largest(value, a, b):
    a, b = float(a), float(b)
    n = max(16, min(4000, int(math.ceil((b - a) / 0.5))))
    h = (b - a) / n
    vals = [(value(a + h * i), a + h * i) for i in range(n + 1)]
    best, at = max(vals, key=lambda v: v[0])
    lo, hi = max(a, at - h), min(b, at + h)
    m = aref.golden(lambda s: -value(s), lo, hi)
    v = value(m)
    return (v, m) if v > best else (best, at)


def plan(base, other, tol):
    tol = mp.mpf(tol)
    pts = lambda s: other.place(s)  # noqa: E731
    rough = lambda s: base.rough([float(c) for c in pts(s)])[1] - float(tol)  # noqa: E731
    exact = lambda s: base.gap(pts(s))[0] > tol  # noqa: E731
    out = []
    for a, b in stretches(other, rough, exact):
        big, at = largest(lambda s: base.gap(pts(s))[0], a, b)
        out.append(dep(a, b, base.gap(pts(a))[1], base.gap(pts(b))[1], big, at))
    return out


def height(base, other, tol, h):
    tol, h = mp.mpf(tol), mp.mpf(h)

    def dz(s):
        p = other.place(s)
        d, st = base.gap(p)
        if d > tol or st is None:
            return None
        zo, zb = other.z(F(float(s))), base.z(F(float(st)))
        if zo is None or zb is None:
            return None
        return mp.mpf(zo[0]) - mp.mpf(zb[0])

    def rough(s):
        p = [float(c) for c in other.place(s)]
        st, d = base.rough(p)
        if abs(d - float(tol)) < NEAR:
            return 0.0  # decided exactly
        if d > float(tol):
            # Apart: not departed in height, by how far from coming together (so a short stretch together between two
            # samples is looked for).
            return -(d - float(tol))
        zo, zb = other.z(F(s)), base.z(F(min(max(st, 0.0), float(base.length))))
        return None if zo is None or zb is None else abs(zo[0] - zb[0]) - float(h)

    exact = lambda s: (lambda v: v is not None and abs(v) > h)(dz(s))  # noqa: E731
    out = []
    for a, b in stretches(other, rough, exact):
        big, at = largest(lambda s: abs(dz(s) or 0), a, b)
        out.append(dep(a, b, base.gap(other.place(a))[1], base.gap(other.place(b))[1], big, at))
    return out


def dep(a, b, sa, sb, big, at):
    return {"from": float(a), "to": float(b), "baseFrom": None if sa is None else float(sa),
            "baseTo": None if sb is None else float(sb), "largest": float(big), "at": float(at)}


def case(name, base, other, tol, h=None, closed=None):
    b, o = Road(*base), Road(*other)
    got = {"plan": plan(b, o, tol)}
    if h is not None:
        got["height"] = height(b, o, tol, h)
    if closed is not None:
        for key, want in closed.items():
            assert len(got[key]) == len(want), (name, key, got[key], want)
            for g, w in zip(got[key], want):
                assert abs(g["from"] - w[0]) < 1e-7 and abs(g["to"] - w[1]) < 1e-7, (name, key, g, w)
    c = {"name": name, "base": {"pis": base[0]}, "other": {"pis": other[0]}, "tolerance": tol, "expected": got}
    if h is not None:
        c["height"] = h
        c["base"]["pvis"], c["other"]["pvis"] = base[1], other[1]
    return c


def cases():
    curve = [pi("a", 0, 0), pi("p1", 400, 0, 300, 50, 50), pi("b", 800, 200)]
    moved = lambda **kw: [curve[0], {**curve[1], **kw}, curve[2]]  # noqa: E731
    straight = [pi("a", 0, 0), pi("b", 1000, 0)]
    theta = math.atan2(100, 1000)
    flat = [pvi("a", 0, 100), pvi("m", 500, 105), pvi("b", 1000, 102)]
    out = [
        case("Aynı aks", (curve,), (curve,), 0.01, closed={"plan": []}),
        case("Kurp yarıçapı 300 → 400", (curve,), (moved(radius=400),), 0.01),
        case("Klotoidler 50 → 80", (curve,), (moved(spiralIn=80, spiralOut=80),), 0.01),
        case("PI 15 m yana kaymış", (curve,), (moved(y=15),), 0.05),
        case("Paralel 5 m, tolerans 1 m: baştan sona", (straight,), ([pi("a", 0, 5), pi("b", 1000, 5)],), 1.0,
             closed={"plan": [(0, 1000)]}),
        case("Paralel 5 m, tolerans 6 m: aynı", (straight,), ([pi("a", 0, 5), pi("b", 1000, 5)],), 6.0,
             closed={"plan": []}),
        case("Açıyla ayrılan aliyman", (straight,), ([pi("a", 0, 0), pi("b", 1000, 100)],), 0.5,
             closed={"plan": [(0.5 / math.sin(theta), math.hypot(1000, 100))]}),
        case("Uzayan aks: sonu tabanın ötesinde", ([pi("a", 0, 0), pi("b", 600, 0)],), (straight,), 0.2,
             closed={"plan": [(600.2, 1000)]}),
        case("Kesişen iki aks: 20 cm birlikte", (straight,), ([pi("a", 0, -50), pi("b", 1000, 50)],), 0.01,
             closed={"plan": [(0, (50 - 0.01) / math.sin(theta)), ((50 + 0.01) / math.sin(theta), math.hypot(1000, 100))]}),
        case("Ters yönde aynı aks", (curve,), (list(reversed([{**p, "id": p["id"] + "r"} for p in curve])),), 0.01,
             closed={"plan": []}),
        case("Ortada sapıp geri dönen", (straight,),
             ([pi("a", 0, 0), pi("p", 300, 0, 400), pi("q", 450, 25, 400), pi("r", 600, 0, 400), pi("b", 1000, 0)],),
             0.05),
        case("Ötelenmiş koordinatlar", ([dict(p, x=p["x"] + 500000, y=p["y"] + 4400000) for p in curve],),
             ([dict(p, x=p["x"] + 500000, y=p["y"] + 4400000) for p in moved(y=15)],), 0.05),
        case("Aynı kotlar", (straight, flat), (straight, flat), 0.01, 0.05, closed={"plan": [], "height": []}),
        case("Kırmızı kot ortada 1 m yükseltilmiş", (straight, flat),
             (straight, [flat[0], pvi("m", 500, 106), flat[2]]), 0.01, 0.05,
             closed={"plan": [], "height": [(25, 975)]}),
        case("Düşey kurp 200 → 300 m", (straight, [flat[0], pvi("m", 500, 105, par(200)), flat[2]]),
             (straight, [flat[0], pvi("m", 500, 105, par(300)), flat[2]]), 0.01, 0.01),
        case("Plan ayrılınca kot karşılaştırılmaz", (straight, flat),
             ([pi("a", 0, 0), pi("p", 300, 0, 400), pi("q", 450, 25, 400), pi("r", 600, 0, 400), pi("b", 1000, 0)],
              [flat[0], pvi("m", 500, 105.5), flat[2]]), 0.05, 0.05),
    ]
    return out


def main():
    data = {"cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            sys.exit(f"{OUT} is not what scripts/fixtures/road_alternatives_cases.py writes")
        print(f"{OUT}: {len(data['cases'])} cases")
        return
    OUT.write_text(text)
    print(f"wrote {OUT}: {len(data['cases'])} cases")
    for c in data["cases"]:
        e = c["expected"]
        words = lambda ds: " ".join(f"[{d['from']:.3f}–{d['to']:.3f} ↑{d['largest']:.3f}]" for d in ds) or "—"  # noqa: E731
        print(f"  {c['name'][:42]:42} plan {words(e['plan'])}" + (f"  kot {words(e['height'])}" if "height" in e else ""))


if __name__ == "__main__":
    main()
