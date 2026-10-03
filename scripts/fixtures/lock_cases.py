#!/usr/bin/env python3
"""Independent reference of the digitizing locks (docs/adr/0166).

Writes fixtures/locks/v1/cases.json from the ADR's rules alone, with Python's
standard library (exact fractions) and mpmath (50 digits) for the angles, no
KentOS code. The geometry core (crates/shared/geometry-core/tests/locks.rs)
and the web through its WASM (apps/web/src/tools/locks.test.ts) must give the
same: points within 1e-8 m (a TM northing's double steps 0.93 nm), unit
directions within 1e-14, none where the reference has none, lock text
exactly.

The rules, as the ADR gives them (§2):

- A direction u and a length L: o + s·L·u, s = 1 one way (Açı, Sapma); both
  ways (Paralel, Dik) the side of the cursor c, s = sign((c − o)·u), 1 when
  the cursor is square to the line.
- A direction alone: o + t·u, t = (c − o)·u; one way t is never below 0.
- A length alone: o + L·(c − o)/|c − o|; none when |c − o| < 1e-9.
- Nothing locked: c.
- With the cursor rules (ortho, polar tracking; tools/point_input): a locked
  direction leaves ortho and polar out; a length alone comes after them. A
  snapped point (exact) is neither ortho'd nor polar'd. No reference point:
  the cursor, nothing locked.

Polar tracking, as the core does it: the cursor's angle from the reference
in degrees, rounded to the nearest step (halves up), its ray captures the
cursor when it lies ahead (along > 0) within `tol` of it.

The angles (ADR 0165 §4): a CAD project's from east counter-clockwise, a CBS
project's semt from north clockwise; degrees or grads. Sapma turns the
previous edge's direction that way: counter-clockwise (left) in CAD,
clockwise (right) in CBS.

Dik kapat (§4): where the line through the last corner square to the last
edge meets the line through the first corner square to the first edge; none
when an edge has no length or the lines are parallel.

Lock text (§6): `<` and a number of the point grammar ([-+]?\\d+(\\.\\d+)?), white
space round them (ECMAScript's); nothing else.
"""

import argparse
import json
import re
import sys
from fractions import Fraction as F
from pathlib import Path

import mpmath as mp

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "locks" / "v1" / "cases.json"
mp.mp.dps = 50

E0, N0 = 487000, 4420000
SAME = F(1, 10**9)


def fl(v):
    """A JSON number: an exact rational or an mpf, to the nearest double; an mpf that is zero but for the 50 digits'
    own residue (cos 90° of an inexact π) is 0."""
    if isinstance(v, F):
        return float(v)
    return float(mp.chop(mp.mpf(v), tol=mp.mpf(10) ** -40))


def pt(p):
    return None if p is None else [fl(p[0]), fl(p[1])]


def exact(v):
    """A double as its exact rational (what the core reads)."""
    return F(float(v))


def ex(p):
    return (exact(p[0]), exact(p[1]))


def sqrt_q(q):
    return mp.sqrt(mp.mpf(q.numerator) / q.denominator)


# ── The lock point (§2) ────────────────────────────────────────────────────────


def lock_point(o, c, length, u, both):
    """o, c, u as doubles; length a double or None. Exact where the rule is rational, mpmath for the square root."""
    o, c = ex(o), ex(c)
    dx, dy = c[0] - o[0], c[1] - o[1]
    if u is not None:
        u = ex(u)
        dot = dx * u[0] + dy * u[1]
        if length is not None:
            s = -1 if both and dot < 0 else 1
            L = exact(length)
            return (o[0] + s * L * u[0], o[1] + s * L * u[1])
        t = dot
        if not both and t < 0:
            t = F(0)
        return (o[0] + t * u[0], o[1] + t * u[1])
    if length is not None:
        n2 = dx * dx + dy * dy
        if n2 < SAME * SAME:
            return None
        n = sqrt_q(n2)
        L = mp.mpf(exact(length).numerator) / exact(length).denominator
        return (
            mp.mpf(o[0].numerator) / o[0].denominator + L * (mp.mpf(dx.numerator) / dx.denominator) / n,
            mp.mpf(o[1].numerator) / o[1].denominator + L * (mp.mpf(dy.numerator) / dy.denominator) / n,
        )
    return c


def lock_case(name, o, c, length=None, u=None, both=False):
    return {"name": name, "o": pt(o), "c": pt(c), "length": length, "u": pt(u), "both": both,
            "expect": pt(lock_point(o, c, length, u, both))}


# ── The cursor rules with the locks ───────────────────────────────────────────


def js_round(x):
    """Math.round: the nearest integer, halves up."""
    return int(mp.floor(x + mp.mpf(1) / 2))


def constrain_cursor(frm, world, exact_point, ortho, polar, tol):
    """tools/point_input's rule: (point, tracking or None), point as mpf/fractions."""
    if frm is None or exact_point:
        return ex(world), None
    o, w = ex(frm), ex(world)
    dx, dy = w[0] - o[0], w[1] - o[1]
    if ortho:
        return ((w[0], o[1]) if abs(dx) > abs(dy) else (o[0], w[1])), None
    if polar is None:
        return w, None
    ang = mp.degrees(mp.atan2(mp.mpf(dy.numerator) / dy.denominator, mp.mpf(dx.numerator) / dx.denominator))
    snapped = js_round(ang / polar) * polar
    rad = mp.radians(snapped)
    ux, uy = mp.cos(rad), mp.sin(rad)
    mdx = mp.mpf(dx.numerator) / dx.denominator
    mdy = mp.mpf(dy.numerator) / dy.denominator
    along = mdx * ux + mdy * uy
    off = abs(-mdx * uy + mdy * ux)
    if along <= 0 or off > tol:
        return w, None
    ox = mp.mpf(o[0].numerator) / o[0].denominator
    oy = mp.mpf(o[1].numerator) / o[1].denominator
    return (ox + ux * along, oy + uy * along), {"origin": pt(frm), "angle": ((snapped % 360) + 360) % 360}


def constrain_locked(frm, world, exact_point, ortho, polar, tol, length, u, both):
    if frm is None:
        return {"point": pt(world), "tracking": None}
    if u is not None:
        p = lock_point(frm, world, length, u, both)
        return None if p is None else {"point": pt(p), "tracking": None}
    p, tracking = constrain_cursor(frm, world, exact_point, ortho, polar, tol)
    # The free point as the core holds it: a double.
    p = (fl(p[0]), fl(p[1]))
    q = lock_point(frm, p, length, None, False)
    return None if q is None else {"point": pt(q), "tracking": tracking}


def constrained_case(name, frm, world, exact_point=False, ortho=False, polar=None, tol=1.0, length=None, u=None,
                     both=False):
    return {"name": name, "from": pt(frm), "world": pt(world), "exact": exact_point, "ortho": ortho,
            "polarStep": polar, "tol": tol, "length": length, "u": pt(u), "both": both,
            "expect": constrain_locked(frm, world, exact_point, ortho, polar, tol, length, u, both)}


# ── Angles in a project's way ─────────────────────────────────────────────────


def radians(angle, grads):
    return mp.mpf(angle) * mp.pi / (200 if grads else 180)


def direction(angle, from_north, grads):
    a = radians(angle, grads)
    return (mp.sin(a), mp.cos(a)) if from_north else (mp.cos(a), mp.sin(a))


def direction_case(name, angle, from_north, grads):
    return {"name": name, "angle": angle, "fromNorth": from_north, "grads": grads,
            "expect": pt(direction(angle, from_north, grads))}


def deflected(prev, frm, angle, from_north, grads):
    p, f = ex(prev), ex(frm)
    dx, dy = f[0] - p[0], f[1] - p[1]
    n2 = dx * dx + dy * dy
    if n2 < SAME * SAME:
        return None
    n = sqrt_q(n2)
    ux = (mp.mpf(dx.numerator) / dx.denominator) / n
    uy = (mp.mpf(dy.numerator) / dy.denominator) / n
    a = radians(angle, grads)
    c, s = mp.cos(a), mp.sin(a)
    if from_north:
        return (ux * c + uy * s, uy * c - ux * s)
    return (ux * c - uy * s, ux * s + uy * c)


def deflected_case(name, prev, frm, angle, from_north, grads):
    return {"name": name, "prev": pt(prev), "from": pt(frm), "angle": angle, "fromNorth": from_north,
            "grads": grads, "expect": pt(deflected(prev, frm, angle, from_north, grads))}


# ── Dik kapat ─────────────────────────────────────────────────────────────────


def square_corner(first, second, prev, last):
    f, s, p, l = ex(first), ex(second), ex(prev), ex(last)
    ax, ay = l[0] - p[0], l[1] - p[1]
    bx, by = s[0] - f[0], s[1] - f[1]
    if ax == 0 and ay == 0 or bx == 0 and by == 0:
        return None
    pa, pb = (-ay, ax), (-by, bx)
    cross = pa[0] * pb[1] - pa[1] * pb[0]
    if cross == 0:
        return None
    wx, wy = f[0] - l[0], f[1] - l[1]
    t = (wx * pb[1] - wy * pb[0]) / cross
    return (l[0] + t * pa[0], l[1] + t * pa[1])


def square_case(name, first, second, prev, last):
    return {"name": name, "first": pt(first), "second": pt(second), "prev": pt(prev), "last": pt(last),
            "expect": pt(square_corner(first, second, prev, last))}


# ── Lock text ─────────────────────────────────────────────────────────────────

# ECMAScript's white space and line terminators (what trim() removes and \s matches).
JS_SPACE = "\u0009\u000a\u000b\u000c\u000d   " + "".join(chr(c) for c in range(0x2000, 0x200b)) + \
    "    　﻿"
LOCK = re.compile(r"^<[" + re.escape(JS_SPACE) + r"]*([-+]?[0-9]+(?:\.[0-9]+)?)$")


def lock_text(text):
    m = LOCK.match(text.strip(JS_SPACE))
    return float(F(m.group(1))) if m else None


def text_case(text):
    return {"text": text, "expect": lock_text(text)}


# ── The cases ─────────────────────────────────────────────────────────────────


def build():
    u_semt50 = pt(direction(50, True, True))
    u35 = (0.6, 0.8)
    lock = [
        lock_case("nothing locked: the cursor", (1, 2), (5, 7)),
        lock_case("a length alone: that far towards the cursor (3, 4, 5)", (0, 0), (3, 4), length=10),
        lock_case("a length alone in TM coordinates", (E0 + 0.5, N0 + 0.25), (E0 + 30.5, N0 + 40.25), length=12.5),
        lock_case("a length alone, the cursor on the reference: none", (1, 1), (1, 1), length=5),
        lock_case("a length alone, a short way still gives the way", (0, 0), (1e-6, 0), length=3),
        lock_case("a length alone, the cursor nearer than the length", (2, -1), (2.5, -1.5), length=7.25),
        lock_case("one way, no length: the cursor projected", (0, 0), (10, 0), u=u35),
        lock_case("one way, behind the reference: the reference", (0, 0), (-10, 0), u=u35),
        lock_case("both ways, behind the reference: the other side", (0, 0), (-10, 0), u=u35, both=True),
        lock_case("one way with a length: the cursor does not matter", (2, 3), (-50, -50), length=5, u=u35),
        lock_case("both ways with a length: the cursor's side", (2, 3), (-50, -50), length=5, u=u35, both=True),
        lock_case("both ways, the cursor square to the line: this way", (0, 0), (0, 5), length=2, u=(1, 0), both=True),
        lock_case("a snapped corner projected: as far as its level", (0, 0), (7, 3), u=(0, 1)),
        lock_case("TM, one way along a 50 grad semt", (E0, N0), (E0 + 20, N0 - 5), u=u_semt50),
        lock_case("TM, both ways along a 50 grad semt with a length", (E0, N0), (E0 - 20, N0 - 5), length=14.125,
                  u=u_semt50, both=True),
    ]
    constrained = [
        constrained_case("no reference: the cursor, nothing locked", None, (3, 4), ortho=True, length=5),
        constrained_case("ortho first, then the length", (0, 0), (10, 2), ortho=True, length=4),
        constrained_case("ortho, the larger move north", (0, 0), (1, -9), ortho=True, length=4),
        constrained_case("polar tracking captures, then the length along its ray", (0, 0), (7, 7.4), polar=45,
                         length=10),
        constrained_case("polar tracking does not capture: the length towards the cursor", (0, 0), (7, 9),
                         polar=45, length=10),
        constrained_case("a snapped point: no ortho, the length towards it", (0, 0), (3, 4), exact_point=True,
                         ortho=True, length=10),
        constrained_case("a locked direction leaves ortho and polar out", (0, 0), (10, 2), ortho=True, polar=45,
                         u=(0, 1)),
        constrained_case("a direction and a length: exactly there", (1, 1), (100, -100), length=2.5, u=(1, 0)),
        constrained_case("a length alone, the cursor on the reference: none", (1, 1), (1, 1), length=3),
        constrained_case("TM: ortho and a length", (E0, N0), (E0 - 8, N0 + 30), ortho=True, length=12.25),
        constrained_case("TM: polar tracking alone, no lock", (E0, N0), (E0 + 10, N0 + 0.3), polar=15),
    ]
    directions = [
        direction_case("CAD 0°: east", 0, False, False),
        direction_case("CAD 30°", 30, False, False),
        direction_case("CAD 90°: north", 90, False, False),
        direction_case("CAD 225°", 225, False, False),
        direction_case("CAD −90°: south", -90, False, False),
        direction_case("CAD in grads, 100: north", 100, False, True),
        direction_case("CBS 50 grads: north-east", 50, True, True),
        direction_case("CBS 100 grads: east", 100, True, True),
        direction_case("CBS 300 grads: west", 300, True, True),
        direction_case("CBS in degrees, 30°", 30, True, False),
    ]
    deflections = [
        deflected_case("CAD: 90° turns left of an edge east", (0, 0), (10, 0), 90, False, False),
        deflected_case("CBS: 100 grads turns right of an edge east", (0, 0), (10, 0), 100, True, True),
        deflected_case("CAD: −45° turns right", (0, 0), (10, 0), -45, False, False),
        deflected_case("CBS: 0, straight on", (0, 0), (3, 4), 0, True, True),
        deflected_case("CAD: 30° left of an edge (3, 4)", (1, 1), (4, 5), 30, False, False),
        deflected_case("CBS in degrees: 90° right of an edge north", (0, 0), (0, 7), 90, True, False),
        deflected_case("CBS, TM coordinates: 50 grads right of an edge (3, 4)", (E0, N0), (E0 + 30, N0 + 40), 50,
                       True, True),
        deflected_case("CAD: 180° back the way it came", (2, 2), (5, 6), 180, False, False),
        deflected_case("no previous edge: none", (4, 4), (4, 4), 30, False, False),
    ]
    squares = [
        square_case("a rectangle closes square on the first corner", (0, 0), (10, 0), (10, 0), (10, 5)),
        square_case("a skewed rectangle (8, 6)", (0, 0), (8, 6), (8, 6), (5, 10)),
        square_case("an L shape's last corner", (0, 0), (20, 0), (12, 10), (12, 18)),
        square_case("a skewed rectangle in TM coordinates", (E0, N0), (E0 + 8, N0 + 6), (E0 + 8, N0 + 6),
                    (E0 + 5, N0 + 10)),
        square_case("edges not square still meet", (0, 0), (10, 0), (10, 0), (13, 7)),
        square_case("the first and the last edge parallel: none", (0, 0), (10, 0), (20, 5), (30, 5)),
        square_case("no last edge: none", (0, 0), (10, 0), (10, 5), (10, 5)),
        square_case("no first edge: none", (0, 0), (0, 0), (10, 0), (10, 5)),
    ]
    texts = [text_case(t) for t in ["<45", " < 12.5 ", "<-30", "<+30", "<0", "<400", "<\t7", "　<8　",
                                    "<", "45", "@10<45", "10<45", "<4,5", "<<45", "<45x", "<.5", "<45.", "<1e3",
                                    "< 4 5", "<١٢"]]
    return {"format": "kentos.locks", "version": 1, "lockPoint": lock, "constrainLocked": constrained,
            "direction": directions, "deflected": deflections, "squareCorner": squares, "lockText": texts}


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="fail when the fixture is not what the rules give")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} kurallardan çıkan değil; yeniden yazın: python3 {Path(__file__).relative_to(ROOT)}",
                  file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} kurallardan çıkanla aynı.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
