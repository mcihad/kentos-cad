#!/usr/bin/env python3
"""A haul fleet on the road, shared cases (docs/adr/0936 §1, plan/road.md Y32.1), from an independent reference. No
KentOS code, and not KentOS's method:

- the run is integrated in TIME, not in distance: the state (u along the slope, v) stepped by fourth order Runge–Kutta
  at 1 ms, the natural acceleration (F(v) − R)/m with F(v) = min(ηP/v, μ·D·m·g·cos θ); after each step the speed is
  projected under the top (or downhill) speed and under the braking curve v² = 2b(L − u); on the braking curve the
  rest is v/b exactly; the wheels' work ∫ max(0, m·dv/dt + R)·du and the engine's idle time from the same steps;
- the loader is the continuous-time Markov chain of n trucks at the loader (0…N), its balance equations solved
  numerically (numpy), not the closed product formula; the deterministic figures by their definition.

    uv run --no-project --python 3.11 --with numpy python3 scripts/fixtures/road_haul_fleet_cases.py   (--check)
"""

import json
import math
import sys
from pathlib import Path

import numpy as np

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "haul-fleet.json"
G = 9.80665
DT = 1e-3


def run(path, mass, t, rolling):
    pieces = []
    for (x0, z0), (x1, z1) in zip(path, path[1:]):
        dx, dz = x1 - x0, z1 - z0
        length = math.hypot(dx, dz)
        pieces.append((x0, length, dx / length, dz / length))
    total = sum(p[1] for p in pieces)
    starts = []
    acc = 0.0
    for p in pieces:
        starts.append(acc)
        acc += p[1]

    def piece(u):
        for i in range(len(pieces) - 1, -1, -1):
            if u >= starts[i] - 1e-12:
                return i
        return 0

    def natural(u, v):
        _, _, cos, sin = pieces[piece(min(u, total - 1e-9))]
        grip = t["traction"] * t["drive"] * mass * G * cos
        pull = grip if v <= 0 else min(t["efficiency"] * t["power"] / v, grip)
        return (pull - mass * G * (rolling * cos + sin)) / mass

    def cap(u):
        _, _, cos, sin = pieces[piece(min(u, total - 1e-9))]
        c = t["top"]
        if t.get("downhill") is not None and rolling * cos + sin < 0:
            c = min(c, t["downhill"])
        return c

    def resist(u):
        _, _, cos, sin = pieces[piece(min(u, total - 1e-9))]
        return mass * G * (rolling * cos + sin)

    u, v, time, work, idle, fastest = 0.0, 0.0, 0.0, 0.0, 0.0, 0.0
    b = t["brake"]
    while True:
        braking_v = math.sqrt(max(2 * b * (total - u), 0.0))
        if total - u < 1e-5:
            # A few micrometres left: stopped on them from the speed left at the deceleration b.
            rest = (math.sqrt(v * v + 2 * b * (total - u)) - v) / b if v < braking_v else v / b
            time += rest
            idle += rest
            break
        if v > 0 and v >= braking_v * (1 - 1e-9) - 1e-9 and natural(u, v) >= -b:
            # On the braking curve to the end: constant deceleration b, the engine idle.
            time += v / b
            idle += v / b
            break
        # RK4 on (u, v) with the natural dynamics, then the projection.
        def f(state):
            uu, vv = state
            return (max(vv, 0.0), natural(uu, max(vv, 0.0)))

        s0 = (u, v)
        k1 = f(s0)
        k2 = f((u + DT / 2 * k1[0], v + DT / 2 * k1[1]))
        k3 = f((u + DT / 2 * k2[0], v + DT / 2 * k2[1]))
        k4 = f((u + DT * k3[0], v + DT * k3[1]))
        u1 = u + DT / 6 * (k1[0] + 2 * k2[0] + 2 * k3[0] + k4[0])
        v1 = v + DT / 6 * (k1[1] + 2 * k2[1] + 2 * k3[1] + k4[1])
        if v1 <= 0 and u1 < total - 1e-6:
            return None
        v1 = min(v1, cap(u), math.sqrt(max(2 * b * (total - u1), 0.0)))
        # The distance with the projected speed: the mean of the two.
        u1 = u + DT * (v + v1) / 2
        if u1 >= total:
            # The last bit: the time to cover what is left at the mean speed.
            dt = (total - u) / ((v + v1) / 2) if v + v1 > 0 else 0.0
            time += dt
            break
        force = mass * (v1 - v) / DT + resist(u)
        if force > 1e-9 * mass:
            work += force * (u1 - u)
        else:
            idle += DT
        time += DT
        fastest = max(fastest, v1)
        u, v = u1, v1
    return {"time": time, "length": total, "work": work, "idle": idle, "fastest": fastest}


def loader(n, away, service):
    """The chain of k trucks at the loader: k → k+1 at (n − k)/away, k → k−1 at 1/service."""
    q = np.zeros((n + 1, n + 1))
    for k in range(n + 1):
        if k < n:
            q[k, k + 1] = (n - k) / away
        if k > 0:
            q[k, k - 1] = 1 / service
        q[k, k] = -q[k].sum()
    a = np.vstack([q.T, np.ones(n + 1)])
    rhs = np.zeros(n + 2)
    rhs[-1] = 1
    p = np.linalg.lstsq(a, rhs, rcond=None)[0]
    busy = 1 - p[0]
    queued = busy / service
    return {
        "matchFactor": n * service / (away + service),
        "steady": min(n / (away + service), 1 / service),
        "queued": queued,
        "busy": busy,
        "wait": max(n / queued - away - service, 0.0),
    }


TRUCK = {"empty": 15000.0, "gross": 40000.0, "power": 300000.0, "efficiency": 0.8, "top": 60 / 3.6,
         "downhill": 30 / 3.6, "traction": 0.6, "drive": 0.7, "brake": 2.0}


def main():
    runs = []
    flat = [[0, 100], [1500, 100]]
    climb = [[0, 100], [600, 130]]  # 5 %
    steep = [[0, 100], [400, 132]]  # 8 %
    down = [[0, 100], [800, 52]]  # −6 %
    mixed = [[0, 100], [300, 103], [700, 123], [900, 123], [1300, 99], [1600, 101]]
    for name, path, mass, truck, rolling in [
        ("düz yüklü", flat, 40000.0, TRUCK, 0.03),
        ("düz boş", flat, 15000.0, TRUCK, 0.03),
        ("%5 çıkış yüklü", climb, 40000.0, TRUCK, 0.03),
        ("%5 iniş boş", [[0, 130], [600, 100]], 15000.0, TRUCK, 0.03),
        ("%8 çıkış yüklü", steep, 40000.0, TRUCK, 0.03),
        ("%6 iniş, iniş hızı sınırı", down, 40000.0, TRUCK, 0.03),
        ("%6 iniş, sınırsız", down, 40000.0, {**TRUCK, "downhill": None}, 0.03),
        ("kırıklı güzergâh yüklü", mixed, 40000.0, TRUCK, 0.03),
        ("kırıklı güzergâh boş", mixed, 15000.0, TRUCK, 0.03),
        ("kısa: en yüksek hıza varmaz", [[0, 100], [120, 100]], 40000.0, TRUCK, 0.03),
        ("uzun düz, yumuşak yol", [[0, 100], [4000, 100]], 40000.0, TRUCK, 0.06),
        ("az güçlü, %5", climb, 40000.0, {**TRUCK, "power": 150000.0}, 0.03),
        ("kaygan: çekişle kalkış", flat, 40000.0, {**TRUCK, "traction": 0.15}, 0.03),
        ("kaygan: %8'i çıkamaz", steep, 40000.0, {**TRUCK, "traction": 0.15}, 0.03),
        ("sert fren", flat, 40000.0, {**TRUCK, "brake": 4.0}, 0.03),
        ("yavaş üst hız", flat, 40000.0, {**TRUCK, "top": 25 / 3.6}, 0.02),
    ]:
        r = run(path, mass, truck, rolling)
        runs.append({"name": name, "path": path, "mass": mass, "truck": truck, "rolling": rolling, "run": r})
    loaders = []
    for n, away, service in [(1, 600, 120), (3, 600, 120), (5, 600, 120), (6, 600, 120), (8, 600, 120),
                             (4, 300, 90), (10, 1200, 150), (2, 100, 100)]:
        loaders.append({"trucks": n, "away": away, "service": service, "loader": loader(n, away, service)})
    text = json.dumps({"runs": runs, "loaders": loaders}, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            sys.exit(f"{OUT} is not what the reference writes")
        return
    OUT.write_text(text)
    print(f"{OUT}: {len(runs)} + {len(loaders)} durum")


if __name__ == "__main__":
    main()
