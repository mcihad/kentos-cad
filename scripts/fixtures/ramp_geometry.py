"""A ramp's plan geometry as ADR 0343, 0705 and 0723 give it, without KentOS code: its walking line, its landings
(one at each inner point, `landing` long and centred on it, within the line's ends; touching ones made one), its sloped
runs between them, its band (half its width each side, mitred at the inner points) and its railings' lines (the band's
sides 5 cm inside). Shared by `ramp_quantity_cases.py` (Metraj's ramp rows, ADR 0723 §3) and `ramp_access_cases.py`
(Mimari denetim's accessibility profile, ADR 0723 §4). Floats are doubles in the order the rules say; the shared
cases compare within 1e-9.
"""

import math

EPS = 1e-9
RAILING_INSET = 0.05


def hypot(x, y):
    return math.hypot(x, y)


def unit(x, y):
    length = hypot(x, y)
    return (x, y) if length == 0.0 else (x / length, y / length)


def xy(p):
    return (p["x"], p["y"]) if isinstance(p, dict) else p


def stations(pts):
    """The arc length at each point of the line."""
    at = [0.0]
    for (ax, ay), (bx, by) in zip(pts, pts[1:]):
        at.append(at[-1] + hypot(bx - ax, by - ay))
    return at


def point(pts, at, s):
    """The point `s` along the line and its segment's direction (at a corner, the segment it ends)."""
    n = len(pts)
    k = 0
    while k + 2 < n and s > at[k + 1]:
        k += 1
    (ax, ay), (bx, by) = pts[k], pts[k + 1]
    length = at[k + 1] - at[k]
    t = 0.0 if length == 0.0 else (s - at[k]) / length
    return (ax + (bx - ax) * t, ay + (by - ay) * t), unit(bx - ax, by - ay)


def landings(pts, landing):
    """The landings: one at each inner point, merged where one starts before the last ends (ADR 0705 §1)."""
    at = stations(pts)
    total = at[-1]
    out = []
    for k in range(1, len(pts) - 1):
        a, b = max(at[k] - landing / 2.0, 0.0), min(at[k] + landing / 2.0, total)
        if out and a < out[-1][1] + EPS:
            out[-1] = (out[-1][0], max(out[-1][1], b))
        else:
            out.append((a, b))
    return out


def runs(pts, landing):
    """The sloped runs between the start, the landings and the end, those of no length left out."""
    at = stations(pts)
    total = at[-1]
    out, start = [], 0.0
    for a, b in landings(pts, landing) + [(total, total)]:
        if a - start > EPS:
            out.append((start, a))
        start = b
    return out


def sloped_length(pts, landing):
    return stations(pts)[-1] - sum(b - a for a, b in landings(pts, landing))


def band(pts, half):
    """The band's ring: its left side from the start, then its right side back; mitred at the inner points."""
    n = len(pts)
    dirs = [unit(b[0] - a[0], b[1] - a[1]) for a, b in zip(pts, pts[1:])]

    def left(d):
        return (-d[1], d[0])

    def side(sign):
        out = []
        for i in range(n):
            a, b = left(dirs[min(max(i - 1, 0), n - 2)]), left(dirs[min(i, n - 2)])
            if i == 0:
                m = b
            elif i == n - 1:
                m = a
            else:
                dot = a[0] * b[0] + a[1] * b[1]
                m = b if 1.0 + dot < 1e-12 else ((a[0] + b[0]) / (1.0 + dot), (a[1] + b[1]) / (1.0 + dot))
            out.append((pts[i][0] + m[0] * half * sign, pts[i][1] + m[1] * half * sign))
        return out

    return side(1.0) + side(-1.0)[::-1]


def band_area(pts, width):
    ring = band(pts, width / 2.0)
    n = len(ring)
    twice = sum(ring[i][0] * ring[(i + 1) % n][1] - ring[(i + 1) % n][0] * ring[i][1] for i in range(n))
    return abs(twice / 2.0)


def railing_length(pts, width, railings):
    """The railings' lines' length: each side asked, the band's side `RAILING_INSET` inside."""
    if railings is None:
        return 0.0
    n = len(pts)
    ring = band(pts, max(width / 2.0 - RAILING_INSET, 0.0))
    sides = {"left": [ring[:n]], "right": [ring[n:][::-1]], "both": [ring[:n], ring[n:][::-1]]}[railings]
    return sum(sum(hypot(b[0] - a[0], b[1] - a[1]) for a, b in zip(line, line[1:])) for line in sides)


def turn_points(pts):
    """The inner points where the line turns; a straight corner does not."""
    out = []
    for k in range(1, len(pts) - 1):
        u = unit(pts[k][0] - pts[k - 1][0], pts[k][1] - pts[k - 1][1])
        v = unit(pts[k + 1][0] - pts[k][0], pts[k + 1][1] - pts[k][1])
        if abs(u[0] * v[1] - u[1] * v[0]) > 1e-9 or u[0] * v[0] + u[1] * v[1] < 0.0:
            out.append(k)
    return out
