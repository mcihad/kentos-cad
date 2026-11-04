#!/usr/bin/env python3
"""A curve's traveled way widening for a design vehicle, shared cases (docs/adr/0932 §1, plan/road.md Y28.1), from an
independent reference at 40 digits. No KentOS code, and not KentOS's formulas:

- the steady-state turn is built axle by axle: the vehicle's axis placed in the plane with the curve's centre at the
  origin, the front axle's centre on the circle of radius R, each fixed axle's line through the centre (its axle
  perpendicular to the axis there), the rear axle found by intersecting, the kingpin as a point on the tractor's axis,
  the trailer's axle as the point at its wheelbase behind the kingpin whose axle line passes through the centre
  (solved by bisection); the off-tracking is R less the rearmost axle centre's distance from the origin;
- the front overhang's width is the distance of the body's front corner line (A ahead of the front axle on the axis)
  from the origin, less R;
- the widening is the Green Book's sum written out term by term.

    python3 scripts/fixtures/road_widening_cases.py           (--check: fails when the file differs)
"""

import json
import sys
from pathlib import Path

import mpmath as mp

mp.mp.dps = 40
OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "widening.json"


def turn(R, L1, kingpin=None, L2=None):
    """The steady-state turn: the front axle's centre at (0, R) heading +x along the circle (the centre at the origin).
    Returns the rear axle's, the kingpin's and the trailer axle's distances from the centre (or None: no steady turn)."""
    R, L1 = mp.mpf(R), mp.mpf(L1)
    # The tractor's axis: from the rear axle P to the front axle F = (0, R), length L1. The rear axle's line (through P,
    # perpendicular to the axis) passes through the origin: |P|² + L1² = |F|² by the right angle at P.
    if R * R - L1 * L1 <= 0:
        return None
    # Find P = (x, y) with |P - F| = L1 and (F - P)·P = 0 (the axis is perpendicular to OP at P). Parametrise the
    # axis's direction by an angle t: P = F - L1·(cos t, sin t); solve (cos t, sin t)·P = 0 by bisection on t.
    F = mp.matrix([0, R])

    def at(t):
        d = mp.matrix([mp.cos(t), mp.sin(t)])
        P = F - L1 * d
        return P, d, d[0] * P[0] + d[1] * P[1]

    lo, hi = mp.mpf(0), mp.pi / 2
    for _ in range(200):
        mid = (lo + hi) / 2
        if at(lo)[2] * at(mid)[2] <= 0:
            hi = mid
        else:
            lo = mid
    P, d, _ = at((lo + hi) / 2)
    rear = mp.sqrt(P[0] ** 2 + P[1] ** 2)
    if kingpin is None:
        return rear, None, None
    K = P - mp.mpf(kingpin) * d  # behind the rear axle along the axis
    kr = mp.sqrt(K[0] ** 2 + K[1] ** 2)
    L2 = mp.mpf(L2)
    if kr * kr - L2 * L2 <= 0:
        return None
    # The trailer's axle T = K - L2·e with e·T = 0, e a unit direction: bisection on its angle.

    def tat(t):
        e = mp.matrix([mp.cos(t), mp.sin(t)])
        T = K - L2 * e
        return T, e[0] * T[0] + e[1] * T[1]

    base = mp.atan2(K[1], K[0]) - mp.pi / 2  # tangent direction at K (heading of travel, clockwise turn)
    lo, hi = base - mp.pi / 2 + mp.mpf("1e-30"), base + mp.pi / 2 - mp.mpf("1e-30")
    # The root with T between K and the centre side: scan for a sign change, then bisect.
    grid = [lo + (hi - lo) * i / 400 for i in range(401)]
    for a, b in zip(grid, grid[1:]):
        if tat(a)[1] * tat(b)[1] <= 0:
            lo, hi = a, b
            break
    for _ in range(200):
        mid = (lo + hi) / 2
        if tat(lo)[1] * tat(mid)[1] <= 0:
            hi = mid
        else:
            lo = mid
    T, _ = tat((lo + hi) / 2)
    return rear, kr, mp.sqrt(T[0] ** 2 + T[1] ** 2)


def reference(c):
    R = mp.mpf(c["radius"])
    v, road = c["vehicle"], c["road"]
    tr = v.get("trailer")
    got = turn(R, v["wheelbase"], *(tr if tr else (None, None)))
    if got is None:
        return {"error": "unturnable"}
    rear, _, axle = got
    last = axle if axle is not None else rear
    off = R - last
    track = mp.mpf(v["track"]) + off
    # The front corner: A ahead of the front axle on the tractor's axis; its distance from the centre by Pythagoras on
    # the rear axle's right angle (the corner is L1 + A ahead of the rear axle).
    if road["method"] == "standard":
        overhang = mp.sqrt(rear * rear + (mp.mpf(v["wheelbase"]) + mp.mpf(v["front"])) ** 2) - R
        extra = mp.mpf("0.1") * mp.mpf(road["speed"]) / mp.sqrt(R) if road.get("speed") is not None else mp.mpf(0)
    else:
        overhang, extra = mp.mpf(0), mp.mpf(0)
    n = road["lanes"]
    width = n * (track + mp.mpf(road["clearance"])) + (n - 1) * overhang + extra
    out = {
        "offtracking": float(off),
        "track": float(track),
        "overhang": float(overhang),
        "allowance": float(extra),
        "width": float(width),
        "widening": float(width - mp.mpf(road["tangent"])),
    }
    if "round" in c:
        w = width - mp.mpf(road["tangent"])
        step, least = c["round"].get("step"), c["round"].get("least")
        if w <= 0 or (least is not None and w < mp.mpf(least)):
            out["rounded"] = 0.0
        elif step:
            k = mp.ceil(w / mp.mpf(step))
            out["rounded"] = float(k * mp.mpf(step))
        else:
            out["rounded"] = float(w)
    return out


# Design vehicles for the cases: sizes in the range of the Green Book's metric vehicles (SU-9, BUS-12, WB-12, WB-19,
# WB-20) and a passenger car; the cases do not claim to be the book's own tables.
SU = {"track": 2.44, "front": 1.2, "wheelbase": 6.1}
BUS = {"track": 2.59, "front": 2.1, "wheelbase": 7.6}
WB12 = {"track": 2.44, "front": 0.9, "wheelbase": 3.8, "trailer": [0.0, 8.2]}
WB19 = {"track": 2.59, "front": 1.2, "wheelbase": 6.6, "trailer": [-0.8, 12.5]}
WB19K = {"track": 2.59, "front": 1.2, "wheelbase": 6.6, "trailer": [0.8, 12.5]}
P = {"track": 1.83, "front": 0.9, "wheelbase": 3.4}


def road(lanes=2, tangent=7.2, clearance=0.9, speed=80, method="standard"):
    r = {"lanes": lanes, "tangent": tangent, "clearance": clearance, "method": method}
    if speed is not None:
        r["speed"] = speed
    return r


RAW = [
    ("WB-19, R 150 m, 7,2 m, 80 km/sa", 150, WB19, road()),
    ("WB-19, R 300 m", 300, WB19, road(speed=90)),
    ("WB-19, R 500 m", 500, WB19, road(speed=100)),
    ("WB-19, R 1000 m: küçük genişleme", 1000, WB19, road(speed=110)),
    ("WB-19, R 70 m, 6,0 m, 50 km/sa", 70, WB19, road(tangent=6.0, clearance=0.6, speed=50)),
    ("Mafsal arka dingilin 0,8 m arkasında: öndekiyle aynı (k² sayılır)", 150, WB19K, road()),
    ("WB-12, mafsal arka dingilde", 120, WB12, road(tangent=6.6, clearance=0.75, speed=60)),
    ("SU-9, R 100 m", 100, SU, road(tangent=6.6, clearance=0.75, speed=60)),
    ("BUS-12, R 60 m", 60, BUS, road(tangent=6.0, clearance=0.6, speed=40)),
    ("Binek, R 50 m", 50, P, road(tangent=6.0, clearance=0.6, speed=40)),
    ("Hızsız: Z yok", 150, WB19, road(speed=None)),
    ("Basit yöntem: N(U + C)", 150, WB19, road(method="simple")),
    ("Basit yöntem, tek taşıt", 100, SU, road(method="simple", tangent=6.6, clearance=0.75)),
    ("Tek şerit (rampa): F_A yok", 80, WB19, road(lanes=1, tangent=4.5, clearance=0.9, speed=50)),
    ("Üç şerit", 250, WB19, road(lanes=3, tangent=10.8, speed=90)),
    ("Dört şerit, 3,5 m'lik", 400, WB19, road(lanes=4, tangent=14.0, speed=100)),
    ("Çok büyük yarıçap: sadeleşmeden kesin", 20000, WB19, road(speed=120)),
    ("Çıkıntısız taşıt", 150, {"track": 2.59, "front": 0.0, "wheelbase": 6.6, "trailer": [-0.8, 12.5]}, road()),
    ("Sınıra yakın: R = 15 m, SU-9", 15, SU, road(lanes=1, tangent=4.0, clearance=0.6, speed=20)),
    ("Kararlı dönemez: R² ≤ ΣL²", 12, WB19, road(speed=20)),
    ("Kararlı dönemez: tek taşıt R = L", 6.1, SU, road(speed=20)),
]

ROUND = [
    ("Yuvarlama: 0,1 m'ye yukarı", 150, WB19, road(), {"step": 0.1}),
    ("Yuvarlama: 0,25 m'ye, en az 0,6 m", 300, WB19, road(speed=90), {"step": 0.25, "least": 0.6}),
    ("En az genişlemenin altında: yazılmaz", 500, WB19, road(speed=100), {"step": 0.1, "least": 0.7}),
    ("Eksi genişleme yazılmaz", 50, P, road(tangent=6.0, clearance=0.6, speed=40), {"step": 0.1}),
    ("Yuvarlamasız, en az 0,3 m", 150, WB19, road(), {"least": 0.3}),
]


def main():
    cases = []
    for name, R, v, r in RAW:
        c = {"name": name, "radius": R, "vehicle": v, "road": r}
        c.update(reference(c))
        cases.append(c)
    for name, R, v, r, rnd in ROUND:
        c = {"name": name, "radius": R, "vehicle": v, "road": r, "round": rnd}
        c.update(reference(c))
        cases.append(c)
    text = json.dumps({"cases": cases}, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            sys.exit(f"{OUT} is not what the reference writes")
        return
    OUT.write_text(text)
    print(f"{OUT}: {len(cases)} durum")


if __name__ == "__main__":
    main()
