#!/usr/bin/env python3
"""A vertical curve's least length, shared cases (docs/adr/0935 §1, plan/road.md Y31.1), from an independent reference
at 30 digits. No KentOS code, and not KentOS's closed forms: each criterion is written forward, as the sight a curve of
length L gives (or the acceleration it asks), and L is found by solving that for the sight wanted (mpmath's findroot
and bisection):

- crest: a symmetric parabola falls A·x²/(200L) from its tangent; the sight line from an eye h₁ to an object h₂ touches
  it, each end where the fall equals its height; past the curve the line runs on the grades: the sight is the shortest
  line clearing the curve from h₁ to h₂, found by minimising over the touch;
- sag headlights: the beam from h rising β meets the road S ahead; the road under the beam is the parabola then the
  grade;
- comfort: v²/R with R = 100L/A; bisector: the curve's middle below the PVI, A·L/800; stopping: v·t + v²/(2·g·(a/g + G)).

    uv run --no-project --python 3.11 --with mpmath python3 scripts/fixtures/road_vertical_length_cases.py   (--check)
"""

import json
import sys
from pathlib import Path

import mpmath as mp

mp.mp.dps = 30
OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "vertical-lengths.json"
G = mp.mpf("9.80665")  # standard gravity, as KentOS


def stopping(V, t, a, grade):
    v = mp.mpf(V) / mp.mpf("3.6")
    brake = mp.mpf(a) / G + mp.mpf(grade)
    if brake <= 0:
        return None
    return v * t + v * v / (2 * G * brake)


def fall(A, L, x):
    """The road's drop below the first tangent's line at x from the curve's start (x may be past its end)."""
    A, L = mp.mpf(A) / 100, mp.mpf(L)
    if x <= 0:
        return mp.mpf(0)
    if x <= L:
        return A * x * x / (2 * L)
    return A * L / 2 + A * (x - L)


def crest_sight(A, L, h1, h2):
    """The sight a crest of length L gives: over every eye place, the shortest distance to an object seen; the worst
    place is symmetric about the curve, and the sight line touches the road (or its end)."""
    # Symmetric worst case: eye at -u, object at +w about the curve's middle; the clear distance from a point h1 above
    # the road at its place to a point h2 above at its place over the crest, solved by the touch on the road profile.
    A, L, h1, h2 = map(mp.mpf, (A, L, h1, h2))
    k = A / 100 / L  # curvature on the parabola
    # Road as a function of signed distance from the middle: z(x) = -k x²/2 inside, linear beyond (both sides).

    def z(x):
        if abs(x) <= L / 2:
            return -k * x * x / 2
        return -k * (L / 2) ** 2 / 2 - A / 200 * (abs(x) - L / 2)

    def slope(x):
        if abs(x) <= L / 2:
            return -k * x
        return -A / 200 * mp.sign(x)

    def reach(x0):
        """From an eye above x0 (ahead to +x), how far the object h2 is first hidden: the tangent from the eye to
        the road, then where h2 above the road meets it."""
        ze = z(x0) + h1
        # The touch t > x0 where the line from the eye is tangent to the road: z(t) + slope(t)(x0 - t) = ze.
        f = lambda t: z(t) + slope(t) * (x0 - t) - ze  # noqa: E731
        lo, hi = x0, x0 + 4 * L + 4000
        if f(hi) < 0:
            return mp.inf
        for _ in range(100):
            mid = (lo + hi) / 2
            if f(mid) < 0:
                lo = mid
            else:
                hi = mid
        t = (lo + hi) / 2
        m = slope(t)
        # Beyond the touch the line falls at m; the object is seen while z(x) + h2 ≤ line(x): find where they meet.
        line = lambda x: ze + m * (x - x0)  # noqa: E731
        g = lambda x: z(x) + h2 - line(x)  # noqa: E731
        lo, hi = t, t + 4 * L + 4000
        # The object's top above the sight line is seen (g > 0); the sight ends where it sinks under it.
        for _ in range(100):
            mid = (lo + hi) / 2
            if g(mid) > 0:
                lo = mid
            else:
                hi = mid
        return (lo + hi) / 2 - x0

    # The shortest reach over the eye's place: golden section on x0 in [-L/2 - 2000, 0] (by symmetry the worst eye
    # is at or before the middle).
    return min_over(reach, -L / 2 - 1500, mp.mpf(0))


def min_over(f, a, b):
    g = (mp.sqrt(5) - 1) / 2
    c, d = b - g * (b - a), a + g * (b - a)
    fc, fd = f(c), f(d)
    for _ in range(80):
        if fc < fd:
            b, d, fd = d, c, fc
            c = b - g * (b - a)
            fc = f(c)
        else:
            a, c, fc = c, d, fd
            d = a + g * (b - a)
            fd = f(d)
    return min(fc, fd)


def solve_length(sight_of, S, lo, hi):
    """The least L whose sight is S: bisection on the increasing sight_of(L)."""
    S = mp.mpf(S)
    if sight_of(lo) >= S:
        # The grades alone (a curve as short as nothing) give the sight: no curve is asked.
        return mp.mpf(0)
    for _ in range(64):
        mid = (lo + hi) / 2
        if sight_of(mid) >= S:
            hi = mid
        else:
            lo = mid
    return (lo + hi) / 2


def headlight_sight(A, L, h, beta):
    """How far ahead the beam from h, rising β, meets a sag of length L with the car at its start."""
    A, L, h = mp.mpf(A), mp.mpf(L), mp.mpf(h)
    tb = mp.tan(mp.radians(beta))
    rise = lambda x: A / 100 * x * x / (2 * L) if x <= L else A / 100 * (x - L / 2)  # noqa: E731
    f = lambda x: h + x * tb - rise(x)  # noqa: E731
    lo, hi = mp.mpf("1e-9"), mp.mpf(100000)
    for _ in range(200):
        mid = (lo + hi) / 2
        if f(mid) > 0:
            lo = mid
        else:
            hi = mid
    return (lo + hi) / 2


def main():
    stop = []
    for V, t, a, G_ in [(50, 2.5, 3.4, 0), (80, 2.5, 3.4, 0), (100, 2.5, 3.4, 0), (120, 2.5, 3.4, 0), (60, 2.0, 3.4, 0),
                        (80, 2.5, 3.4, 0.04), (80, 2.5, 3.4, -0.06), (100, 1.5, 4.5, 0), (40, 2.5, 3.4, -0.3)]:
        s = stopping(V, t, a, G_)
        stop.append({"speed": V, "time": t, "decel": a, "grade": G_, "distance": None if s is None else float(s)})
    crest = []
    for A, S, h1, h2 in [(4, 130, 1.08, 0.60), (8, 130, 1.08, 0.60), (2, 130, 1.08, 0.60), (1, 185, 1.08, 0.60),
                         (6, 185, 1.08, 0.60), (5, 500, 1.08, 1.08), (10, 65, 1.08, 0.60), (3, 250, 1.08, 0.15),
                         (0.5, 130, 1.08, 0.60), (12, 85, 1.05, 0.60)]:
        L = solve_length(lambda L: crest_sight(A, L, h1, h2), S, mp.mpf("0.001"), mp.mpf(20000))
        crest.append({"a": A, "sight": S, "eye": h1, "object": h2, "length": float(L)})
    sag = []
    for A, S, h, b in [(4, 130, 0.6, 1.0), (8, 85, 0.6, 1.0), (2, 185, 0.6, 1.0), (1, 65, 0.6, 1.0), (6, 250, 0.75, 1.0),
                       (3, 130, 0.6, 0.5), (10, 50, 0.6, 1.0)]:
        L = solve_length(lambda L: headlight_sight(A, L, h, b), S, mp.mpf("0.001"), mp.mpf(20000))
        sag.append({"a": A, "sight": S, "height": h, "beam": b, "length": float(L)})
    comfort = []
    for A, V, acc in [(4, 80, 0.3), (8, 100, 0.3), (2, 120, 0.25), (6, 50, 0.5)]:
        # v²/R = acc with R = 100L/A.
        v = mp.mpf(V) / mp.mpf("3.6")
        comfort.append({"a": A, "speed": V, "accel": acc, "length": float(v * v * A / (100 * mp.mpf(acc)))})
    bis = []
    for A, E in [(4, 0.5), (8, 1.2), (2.5, 0.25)]:
        # The middle of a symmetric parabola of length L lies A·L/800 below the PVI: solve for L.
        bis.append({"a": A, "external": E, "length": float(mp.findroot(lambda L: mp.mpf(A) * L / 800 - E, 10))})
    text = json.dumps({"stopping": stop, "crest": crest, "headlight": sag, "comfort": comfort, "bisector": bis},
                      ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            sys.exit(f"{OUT} is not what the reference writes")
        return
    OUT.write_text(text)
    print(f"{OUT}: {len(stop)} + {len(crest)} + {len(sag)} + {len(comfort)} + {len(bis)} durum")


if __name__ == "__main__":
    main()
