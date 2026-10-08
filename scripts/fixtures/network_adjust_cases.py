#!/usr/bin/env python3
"""Ağ dengelemesi ve kot ağı (docs/adr/0203): the shared cases, written from the ADR without KentOS code.

fixtures/network-adjust/v1/cases.json: horizontal networks (a connected traverse, a braced quadrilateral, weighted
control, a free station, forward intersections for the approximations, a trilateration from the drawing's
approximate places, a blunder, a network without checks, degrees, lengths reduced to the grid, refusals) and levelling
networks (a geometric loop, trigonometric height differences, a weighted benchmark, a blunder, a refusal), each with
what the adjustment gives: the new points' coordinates or heights with their standard deviations and error ellipses,
the stations' orientations, every observation's residual, standard deviation, redundancy number and test value, the
degrees of freedom, vᵀPv, m0 and the model test; and the χ² quantiles the model test takes.

How the reference works, independently of the core:
- the observations are made from true places with fixed pseudo-random noise, rounded as an instrument writes them;
- the approximations follow the ADR's §3 step by step;
- Gauss–Newton at 50 digits (mpmath), the normal equations solved by LU (the core uses Cholesky: another route to the
  same solution), iterated to 1e-30; a singular network is found with the ADR's pivot rule on its own Cholesky;
- the statistics from mpmath's inverse; the χ² quantiles by bisection of mpmath's regularized incomplete gamma;
- the grid case's line factors from ground_survey_cases.py (PROJ and GeographicLib through pyproj), as the survey
  windows' reference takes them.

    python3 scripts/fixtures/network_adjust_cases.py          # write
    python3 scripts/fixtures/network_adjust_cases.py --check  # compare
"""

import argparse
import json
import math
import random
import sys
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "network-adjust" / "v1" / "cases.json"

mp.mp.dps = 50
PI = mp.pi
TAU = 2 * mp.pi
FULL = {"grad": 400, "deg": 360}
W_LIMIT = mp.mpf("3.29")
UNCONTROLLED = mp.mpf("1e-4")
PIVOT = mp.mpf("1e-10")
MAX_ITERATIONS = 30
ONE_GON = PI / 200

# The defaults of ADR §1, as the window resolves them when the project names none; 10 cc as every platform computes
# it in binary64 (π / 200 000, one rounding after π's).
DEFAULTS = {
    "direction": math.pi / 200000,
    "distance": 0.002,
    "ppm": 2.0,
    "centering": 0.001,
    "zenith": math.pi / 200000,
    "levelling": 0.002,
}


# ── Names (ADR §2) ──────────────────────────────────────────────────────

def fold_char(c):
    if c == "I":
        return "ı"
    if c == "İ":
        return "i"
    low = c.lower()
    return low if len(low) == 1 else c


def key(name):
    return "".join(fold_char(c) for c in name.strip())


# ── Angles ──────────────────────────────────────────────────────────────

def wrap(a):
    """Into (−π, π]."""
    a = mp.fmod(a, TAU)
    if a <= -PI:
        a += TAU
    elif a > PI:
        a -= TAU
    return a


def positive(a):
    a = mp.fmod(a, TAU)
    return a + TAU if a < 0 else a


def bearing(a, b):
    return positive(mp.atan2(b[0] - a[0], b[1] - a[1]))


def dist(a, b):
    return mp.sqrt((b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2)


def rad(v, unit):
    return mp.mpf(v) * TAU / FULL[unit]


# ── Linear algebra ──────────────────────────────────────────────────────

def cholesky_singular(n_mat):
    """The first unknown whose Cholesky pivot is not above PIVOT × its diagonal; None when none."""
    m = n_mat.rows
    lo = mp.matrix(m, m)
    for k in range(m):
        s = n_mat[k, k] - mp.fsum(lo[k, j] ** 2 for j in range(k))
        if not s > PIVOT * n_mat[k, k]:
            return k
        lo[k, k] = mp.sqrt(s)
        for i in range(k + 1, m):
            lo[i, k] = (n_mat[i, k] - mp.fsum(lo[i, j] * lo[k, j] for j in range(k))) / lo[k, k]
    return None


def chi2_quantile(f, p=mp.mpf("0.95")):
    """x with P(f/2, x/2) = p, by bisection."""
    lo, hi = mp.mpf(0), mp.mpf(1)
    reg = lambda x: mp.gammainc(mp.mpf(f) / 2, 0, x / 2, regularized=True)
    while reg(hi) < p:
        hi *= 2
    for _ in range(400):
        mid = (lo + hi) / 2
        if reg(mid) < p:
            lo = mid
        else:
            hi = mid
    return (lo + hi) / 2


# ── Horizontal networks ─────────────────────────────────────────────────

class Refused(Exception):
    pass


def horizontal(case):
    unit = case["unit"]
    sig = {**DEFAULTS, **case.get("sigma", {})}
    grid = case.get("grid")
    known = {}
    order_known = []
    for k in case["known"]:
        kk = key(k["name"])
        if kk in known:
            raise Refused(f"{k['name'].strip()} bilinen noktalarda iki kez var.")
        known[kk] = (k["name"].strip(), (mp.mpf(k["y"]), mp.mpf(k["x"])), None if k.get("sigma") is None else mp.mpf(k["sigma"]))
        order_known.append(kk)
    rows = case["rows"]
    if not rows:
        raise Refused("Gözlem yok: gözlemler tablosuna doğrultu ya da kenar girin.")
    names = {}
    for kk in order_known:
        names[kk] = known[kk][0]
    appearance = []
    stations = []
    for i, r in enumerate(rows, 1):
        s, t = key(r["station"]), key(r["target"])
        if s == t:
            raise Refused(f"{i}. satırda durulan ve bakılan aynı nokta.")
        if r.get("distance") is not None and not mp.mpf(r["distance"]) > 0:
            raise Refused(f"{i}. satırda kenar sıfırdan büyük olmalı.")
        for kk, nm in ((s, r["station"]), (t, r["target"])):
            names.setdefault(kk, nm.strip())
            if kk not in appearance:
                appearance.append(kk)
        if r.get("direction") is not None and s not in stations:
            stations.append(s)
    pos = {kk: known[kk][1] for kk in order_known}
    for a in case.get("approx", []):
        kk = key(a["name"])
        if kk not in pos:
            pos[kk] = (mp.mpf(a["y"]), mp.mpf(a["x"]))
    orient = {}
    dirs = lambda s: [(i, r) for i, r in enumerate(rows) if key(r["station"]) == s and r.get("direction") is not None]

    def orient_station(s):
        diffs = [wrap(bearing(pos[s], pos[key(r["target"])]) - rad(r["direction"], unit))
                 for _, r in dirs(s) if key(r["target"]) in pos and dist(pos[s], pos[key(r["target"])]) > 0]
        if not diffs:
            return False
        z0 = diffs[0]
        orient[s] = z0 + mp.fsum(wrap(d - z0) for d in diffs) / len(diffs)
        return True

    while True:
        changed = False
        for s in stations:
            if s in pos and s not in orient and orient_station(s):
                changed = True
        for r in rows:
            s, t = key(r["station"]), key(r["target"])
            if s in pos and s in orient and t not in pos and r.get("direction") is not None and r.get("distance") is not None:
                z = orient[s] + rad(r["direction"], unit)
                d = mp.mpf(r["distance"])
                pos[t] = (pos[s][0] + d * mp.sin(z), pos[s][1] + d * mp.cos(z))
                changed = True
        for s in stations:
            if s in pos:
                continue
            got = [(rad(r["direction"], unit), mp.mpf(r["distance"]), pos[key(r["target"])])
                   for _, r in dirs(s) if r.get("distance") is not None and key(r["target"]) in pos]
            pair = None
            for i in range(len(got)):
                for j in range(i + 1, len(got)):
                    if dist(got[i][2], got[j][2]) > 0:
                        pair = (got[i], got[j])
                        break
                if pair:
                    break
            if pair:
                (r1, d1, k1), (r2, d2, k2) = pair
                l1 = (d1 * mp.sin(r1), d1 * mp.cos(r1))
                l2 = (d2 * mp.sin(r2), d2 * mp.cos(r2))
                theta = bearing(k1, k2) - bearing(l1, l2)
                pos[s] = (k1[0] - d1 * mp.sin(r1 + theta), k1[1] - d1 * mp.cos(r1 + theta))
                orient[s] = theta
                changed = True
        for t in appearance:
            if t in pos:
                continue
            rays = [(pos[key(r["station"])], orient[key(r["station"])] + rad(r["direction"], unit))
                    for r in rows if key(r["target"]) == t and r.get("direction") is not None
                    and key(r["station"]) in pos and key(r["station"]) in orient]
            done = False
            for i in range(len(rays)):
                for j in range(i + 1, len(rays)):
                    (p1, t1), (p2, t2) = rays[i], rays[j]
                    if abs(mp.sin(t1 - t2)) > mp.sin(ONE_GON):
                        # p1 + u·(sin t1, cos t1) = p2 + w·(sin t2, cos t2)
                        det = mp.sin(t1) * (-mp.cos(t2)) - mp.cos(t1) * (-mp.sin(t2))
                        dy, dx = p2[0] - p1[0], p2[1] - p1[1]
                        u = (dy * (-mp.cos(t2)) - dx * (-mp.sin(t2))) / det
                        pos[t] = (p1[0] + u * mp.sin(t1), p1[1] + u * mp.cos(t1))
                        changed = done = True
                        break
                if done:
                    break
        if not changed:
            break
    missing = [names[k] for k in appearance if k not in pos]
    if missing:
        raise Refused(f"Yaklaşık yeri bulunamayan nokta: {', '.join(missing)}. Noktayı çizime yaklaşık yeriyle ekleyin "
                      "ya da onu belirleyen doğrultu ve kenar ölçüleri girin.")
    for s in stations:
        if s not in orient:
            orient_station(s)

    weighted = [k for k in order_known if known[k][2] is not None]
    fixed = {k for k in order_known if known[k][2] is None}
    new = [k for k in appearance if k not in known]
    points = weighted + new
    col = {}
    for i, k in enumerate(points):
        col[k] = 2 * i
    ocol = {s: 2 * len(points) + j for j, s in enumerate(stations)}
    u = 2 * len(points) + len(stations)

    def grid_factor(a, b):
        if not grid:
            return mp.mpf(1)
        import ground_survey_cases as gs
        k, h = gs.factors(grid["system"], grid["height"], (float(a[0]), float(a[1])), (float(b[0]), float(b[1])))
        return mp.mpf(k) * mp.mpf(h)

    def observations():
        """Each observation: (kind, row, design row as {col: coefficient}, l = observed − computed, σ)."""
        out = []
        for i, r in enumerate(rows):
            s, t = key(r["station"]), key(r["target"])
            ps, pt = pos[s], pos[t]
            dy, dx = pt[0] - ps[0], pt[1] - ps[1]
            s2 = dy * dy + dx * dx
            length = mp.sqrt(s2)
            if r.get("direction") is not None:
                a = {}
                if s in col:
                    a[col[s]] = a.get(col[s], 0) - dx / s2
                    a[col[s] + 1] = a.get(col[s] + 1, 0) + dy / s2
                if t in col:
                    a[col[t]] = a.get(col[t], 0) + dx / s2
                    a[col[t] + 1] = a.get(col[t] + 1, 0) - dy / s2
                a[ocol[s]] = mp.mpf(-1)
                l = wrap(rad(r["direction"], unit) - (bearing(ps, pt) - orient[s]))
                sigma = mp.sqrt(mp.mpf(sig["direction"]) ** 2 + 2 * (mp.mpf(sig["centering"]) / length) ** 2)
                out.append(("direction", i, a, l, sigma))
            if r.get("distance") is not None:
                a = {}
                if s in col:
                    a[col[s]] = a.get(col[s], 0) - dy / length
                    a[col[s] + 1] = a.get(col[s] + 1, 0) - dx / length
                if t in col:
                    a[col[t]] = a.get(col[t], 0) + dy / length
                    a[col[t] + 1] = a.get(col[t] + 1, 0) + dx / length
                observed = mp.mpf(r["distance"]) * grid_factor(ps, pt)
                l = observed - length
                plain = mp.mpf(sig["distance"]) + mp.mpf(sig["ppm"]) * mp.mpf("1e-6") * mp.mpf(r["distance"])
                sigma = mp.sqrt(plain ** 2 + 2 * mp.mpf(sig["centering"]) ** 2)
                out.append(("distance", i, a, l, sigma))
        for k in weighted:
            y0, x0 = known[k][1]
            sigma = known[k][2]
            out.append(("y", order_known.index(k), {col[k]: mp.mpf(1)}, y0 - pos[k][0], sigma))
            out.append(("x", order_known.index(k), {col[k] + 1: mp.mpf(1)}, x0 - pos[k][1], sigma))
        return out

    def normal(obs):
        n_mat = mp.matrix(u, u)
        rhs = mp.matrix(u, 1)
        for _, _, a, l, sigma in obs:
            p = 1 / sigma ** 2
            for i, ai in a.items():
                rhs[i] += p * ai * l
                for j, aj in a.items():
                    n_mat[i, j] += p * ai * aj
        return n_mat, rhs

    def unknown_name(c):
        if c < 2 * len(points):
            nm = names[points[c // 2]]
            return f"{nm} noktasının Y'si" if c % 2 == 0 else f"{nm} noktasının X'i"
        return f"{names[stations[c - 2 * len(points)]]} istasyonunun yöneltmesi"

    iterations = 0
    while True:
        obs = observations()
        n_mat, rhs = normal(obs)
        bad = cholesky_singular(n_mat)
        if bad is not None:
            raise Refused(f"Ağın dayanağı ya da gözlemleri yetersiz: {unknown_name(bad)} belirlenemiyor. En az iki sabit "
                          "ya da ağırlıklı nokta ve her noktayı belirleyen gözlemler gerekir.")
        delta = mp.lu_solve(n_mat, rhs)
        iterations += 1
        for k in points:
            pos[k] = (pos[k][0] + delta[col[k]], pos[k][1] + delta[col[k] + 1])
        for s in stations:
            orient[s] += delta[ocol[s]]
        biggest = max([abs(delta[i]) for i in range(u)] or [mp.mpf(0)])
        if biggest < mp.mpf("1e-30"):
            break
        if iterations > 60:
            raise RuntimeError("the reference did not converge")
    obs = observations()
    n_mat, _ = normal(obs)
    q = mp.inverse(n_mat)
    return statistics(obs, q, u, points=[(names[k], pos[k], col[k]) for k in points],
                      orientations=[(names[s], positive(orient[s])) for s in stations])


def statistics(obs, q, u, points, orientations=None, heights=False):
    n = len(obs)
    f = n - u
    rows = []
    omega = mp.mpf(0)
    for kind, row, a, l, sigma in obs:
        v = -l  # computed − observed at the solution
        p = 1 / sigma ** 2
        omega += p * v * v
        aqa = mp.fsum(ai * q[i, j] * aj for i, ai in a.items() for j, aj in a.items())
        qvv = 1 / p - aqa
        r = p * qvv
        if r < UNCONTROLLED:
            w, flag = None, "uncontrolled"
        else:
            w = abs(v) / mp.sqrt(qvv)
            flag = "blunder" if w > W_LIMIT else "ok"
        rows.append({"kind": kind, "row": row, "v": float(v), "sigma": float(sigma), "r": float(r),
                     "w": None if w is None else float(w), "flag": flag})
    flagged = [(o["w"], i) for i, o in enumerate(rows) if o["flag"] == "blunder"]
    worst = None
    for w, i in flagged:
        if worst is None or w > rows[worst]["w"]:
            worst = i
    m0 = mp.sqrt(omega / f) if f > 0 else None
    s0 = m0 if f > 0 else mp.mpf(1)
    out_points = []
    for name, p, c in points:
        if heights:
            out_points.append({"name": name, "h": float(p), "sh": float(s0 * mp.sqrt(q[c, c]))})
            continue
        qyy, qxx, qyx = q[c, c], q[c + 1, c + 1], q[c, c + 1]
        half = (qyy + qxx) / 2
        root = mp.sqrt(((qyy - qxx) / 2) ** 2 + qyx ** 2)
        theta = mp.atan2(2 * qyx, qxx - qyy) / 2
        theta = mp.fmod(theta, PI)
        if theta < 0:
            theta += PI
        sy, sx = s0 * mp.sqrt(qyy), s0 * mp.sqrt(qxx)
        out_points.append({"name": name, "y": float(p[0]), "x": float(p[1]), "sy": float(sy), "sx": float(sx),
                           "sp": float(mp.sqrt(sy * sy + sx * sx)), "a": float(s0 * mp.sqrt(half + root)),
                           "b": float(s0 * mp.sqrt(half - root)), "theta": float(theta)})
    result = {"points": out_points, "observations": rows, "worst": worst, "n": n, "u": u, "f": f, "omega": float(omega),
              "m0": None if m0 is None else float(m0)}
    if orientations is not None:
        result["orientations"] = [{"station": s, "z": float(z)} for s, z in orientations]
    if f > 0:
        chi = chi2_quantile(f)
        result["chi2"] = float(chi)
        result["passed"] = bool(omega <= chi)
    else:
        result["chi2"] = None
        result["passed"] = None
    return result


# ── Levelling networks (ADR §5) ─────────────────────────────────────────

def levelling(case):
    sig = {**DEFAULTS, **case.get("sigma", {})}
    trig = case["levelKind"] == "trigonometric"
    known = {}
    order_known = []
    for k in case["known"]:
        kk = key(k["name"])
        if kk in known:
            raise Refused(f"{k['name'].strip()} bilinen noktalarda iki kez var.")
        known[kk] = (k["name"].strip(), mp.mpf(k["h"]), None if k.get("sigma") is None else mp.mpf(k["sigma"]))
        order_known.append(kk)
    rows = case["rows"]
    if not rows:
        raise Refused("Gözlem yok: gözlemler tablosuna kot farkı girin.")
    names = {k: known[k][0] for k in order_known}
    appearance = []
    for i, r in enumerate(rows, 1):
        a, b = key(r["from"]), key(r["to"])
        if a == b:
            raise Refused(f"{i}. satırda başlangıç ve bitiş aynı nokta.")
        if not mp.mpf(r["length"]) > 0:
            raise Refused(f"{i}. satırda uzunluk sıfırdan büyük olmalı.")
        for kk, nm in ((a, r["from"]), (b, r["to"])):
            names.setdefault(kk, nm.strip())
            if kk not in appearance:
                appearance.append(kk)
    h = {k: known[k][1] for k in order_known}
    while True:
        changed = False
        for r in rows:
            a, b = key(r["from"]), key(r["to"])
            if a in h and b not in h:
                h[b] = h[a] + mp.mpf(r["dh"])
                changed = True
            elif b in h and a not in h:
                h[a] = h[b] - mp.mpf(r["dh"])
                changed = True
        if not changed:
            break
    missing = [names[k] for k in appearance if k not in h]
    if missing:
        raise Refused(f"Bilinen bir kota bağlı olmayan nokta: {', '.join(missing)}. Noktayı bilinen bir kota bağlayan kot "
                      "farkı girin.")
    weighted = [k for k in order_known if known[k][2] is not None]
    new = [k for k in appearance if k not in known]
    points = weighted + new
    col = {k: i for i, k in enumerate(points)}
    u = len(points)

    def observations():
        out = []
        for i, r in enumerate(rows):
            a, b = key(r["from"]), key(r["to"])
            coef = {}
            if a in col:
                coef[col[a]] = mp.mpf(-1)
            if b in col:
                coef[col[b]] = mp.mpf(1)
            l = mp.mpf(r["dh"]) - (h[b] - h[a])
            length = mp.mpf(r["length"])
            if trig:
                plain = mp.mpf(sig["distance"]) + mp.mpf(sig["ppm"]) * mp.mpf("1e-6") * length
                sigma = mp.sqrt((length * mp.mpf(sig["zenith"])) ** 2 + (mp.mpf(r["dh"]) / length * plain) ** 2)
            else:
                sigma = mp.mpf(sig["levelling"]) * mp.sqrt(length / 1000)
            out.append(("dh", i, coef, l, sigma))
        for k in weighted:
            out.append(("h", order_known.index(k), {col[k]: mp.mpf(1)}, known[k][1] - h[k], known[k][2]))
        return out

    iterations = 0
    while True:
        obs = observations()
        n_mat = mp.matrix(u, u)
        rhs = mp.matrix(u, 1)
        for _, _, a, l, sigma in obs:
            p = 1 / sigma ** 2
            for i, ai in a.items():
                rhs[i] += p * ai * l
                for j, aj in a.items():
                    n_mat[i, j] += p * ai * aj
        bad = cholesky_singular(n_mat)
        if bad is not None:
            raise Refused(f"Ağın dayanağı ya da gözlemleri yetersiz: {names[points[bad]]} noktasının kotu belirlenemiyor. "
                          "En az bir sabit ya da ağırlıklı kot ve her noktayı belirleyen gözlemler gerekir.")
        delta = mp.lu_solve(n_mat, rhs)
        iterations += 1
        for k in points:
            h[k] += delta[col[k]]
        if max(abs(delta[i]) for i in range(u)) < mp.mpf("1e-30") or iterations > 5:
            break
    obs = observations()
    n_mat = mp.matrix(u, u)
    for _, _, a, _, sigma in obs:
        p = 1 / sigma ** 2
        for i, ai in a.items():
            for j, aj in a.items():
                n_mat[i, j] += p * ai * aj
    q = mp.inverse(n_mat)
    return statistics(obs, q, u, points=[(names[k], h[k], col[k]) for k in points], heights=True)


# ── The cases ───────────────────────────────────────────────────────────

def observe(true, station, target, unit, rnd, z0, noise_cc, noise_mm, direction=True, distance=True, digits=4):
    """A row made from true places: the reading (bearing − orientation) with noise, the distance with noise,
    rounded as an instrument writes them (1 cc or 0.1″ and 0.1 mm)."""
    ps, pt = true[station], true[target]
    row = {"station": station, "target": target}
    if direction:
        t = bearing(ps, pt) - z0
        reading = positive(t + rnd.uniform(-1, 1) * noise_cc * PI / 2_000_000) * FULL[unit] / TAU
        row["direction"] = float(mp.nint(reading * 10 ** digits) / 10 ** digits)
    else:
        row["direction"] = None
    if distance:
        d = dist(ps, pt) + rnd.uniform(-1, 1) * noise_mm / 1000
        row["distance"] = float(mp.nint(d * 10000) / 10000)
    else:
        row["distance"] = None
    return row


def at(e, n):
    return (mp.mpf(e), mp.mpf(n))


def build():
    cases = []
    E0, N0 = 487000, 4420000

    # 1. A connected traverse between known points, oriented at both ends.
    true = {"A": at(E0, N0), "B": at(E0 - 120, N0 + 260), "P1": at(E0 + 180, N0 + 40), "P2": at(E0 + 360, N0 + 15),
            "P3": at(E0 + 545, N0 + 70), "C": at(E0 + 720, N0 + 30), "D": at(E0 + 900, N0 + 250)}
    rnd = random.Random(1)
    rows = []
    path = ["A", "P1", "P2", "P3", "C"]
    backs = {"A": "B", "C": "D"}
    for i, s in enumerate(path):
        z0 = rnd.uniform(0, 2 * float(PI))
        before = backs.get(s) if i == 0 else path[i - 1]
        after = path[i + 1] if i + 1 < len(path) else backs["C"]
        rows.append(observe(true, s, before, "grad", rnd, z0, 8, 2, distance=(i > 0)))
        rows.append(observe(true, s, after, "grad", rnd, z0, 8, 2, distance=(i + 1 < len(path))))
    cases.append({"name": "Bağlı poligon: A'dan C'ye üç yeni nokta, iki uçta yöneltme; kenarlar iki yönden", "kind": "horizontal",
                  "unit": "grad", "known": [{"name": n, "y": float(true[n][0]), "x": float(true[n][1])} for n in ("A", "B", "C", "D")],
                  "rows": rows})

    # 2. A braced quadrilateral: two fixed points, two new, every direction and distance.
    true = {"K1": at(E0, N0), "K2": at(E0 + 400, N0 + 20), "Y1": at(E0 + 30, N0 + 350), "Y2": at(E0 + 420, N0 + 380)}
    rnd = random.Random(2)
    rows = []
    for s in ("K1", "K2", "Y1", "Y2"):
        z0 = rnd.uniform(0, 2 * float(PI))
        for t in ("K1", "K2", "Y1", "Y2"):
            if t != s:
                rows.append(observe(true, s, t, "grad", rnd, z0, 10, 2, distance=(s < t)))
    quad_rows = rows
    cases.append({"name": "Çaprazlı dörtgen: iki sabit, iki yeni nokta, bütün doğrultular ve kenarlar", "kind": "horizontal",
                  "unit": "grad", "known": [{"name": "K1", "y": float(true["K1"][0]), "x": float(true["K1"][1])},
                                            {"name": "K2", "y": float(true["K2"][0]), "x": float(true["K2"][1])}],
                  "rows": quad_rows})

    # 3. Weighted control: the same quadrilateral, one point fixed, two weighted with 10 mm.
    cases.append({"name": "Ağırlıklı kontrol: bir sabit, iki ağırlıklı (σ 10 mm) nokta; sözde gözlemler", "kind": "horizontal",
                  "unit": "grad", "known": [{"name": "K1", "y": float(true["K1"][0]), "x": float(true["K1"][1])},
                                            {"name": "K2", "y": float(true["K2"][0]) + 0.004, "x": float(true["K2"][1]) - 0.003, "sigma": 0.01},
                                            {"name": "Y2", "y": float(true["Y2"][0]) - 0.006, "x": float(true["Y2"][1]) + 0.002, "sigma": 0.01}],
                  "rows": quad_rows})

    # 4. A blunder: the quadrilateral with one distance 6 cm long.
    blunder = [dict(r) for r in quad_rows]
    for r in blunder:
        if r["station"] == "K2" and r["target"] == "Y1" and r["distance"] is not None:
            r["distance"] = round(r["distance"] + 0.06, 4)
            break
    else:
        for r in blunder:
            if r["distance"] is not None:
                r["distance"] = round(r["distance"] + 0.06, 4)
                break
    cases.append({"name": "Uyuşumsuz ölçü: bir kenar 6 cm uzun; en büyük w onun, model testi kalır", "kind": "horizontal",
                  "unit": "grad", "known": [{"name": "K1", "y": float(true["K1"][0]), "x": float(true["K1"][1])},
                                            {"name": "K2", "y": float(true["K2"][0]), "x": float(true["K2"][1])}],
                  "rows": blunder})

    # 5. A free station: an unknown station on three known points (directions and distances); names typed loosely.
    true = {"S": at(E0 + 50, N0 + 60), "R1": at(E0 - 150, N0 + 300), "R2": at(E0 + 320, N0 + 250), "R3": at(E0 + 200, N0 - 230)}
    rnd = random.Random(5)
    z0 = 1.234
    rows = [observe(true, "S", t, "grad", rnd, z0, 8, 2) for t in ("R1", "R2", "R3")]
    rows[1]["target"] = " r2 "
    cases.append({"name": "Serbest istasyon: yeri bilinmeyen S üç bilinen noktaya; ad küçük harfle ve boşlukla", "kind": "horizontal",
                  "unit": "grad", "known": [{"name": n, "y": float(true[n][0]), "x": float(true[n][1])} for n in ("R1", "R2", "R3")],
                  "rows": rows})

    # 6. Forward intersections: two known stations, each oriented on the other, see two new points by directions only.
    true = {"A": at(E0, N0), "B": at(E0 + 500, N0), "N1": at(E0 + 120, N0 + 280), "N2": at(E0 + 390, N0 + 310)}
    rnd = random.Random(6)
    rows = []
    for s, o in (("A", "B"), ("B", "A")):
        z0 = rnd.uniform(0, 2 * float(PI))
        rows.append(observe(true, s, o, "grad", rnd, z0, 8, 2, distance=False))
        rows.append(observe(true, s, "N1", "grad", rnd, z0, 8, 2, distance=False))
        rows.append(observe(true, s, "N2", "grad", rnd, z0, 8, 2, distance=False))
    rows.append(observe(true, "N1", "N2", "grad", rnd, 0, 8, 2, direction=False))
    cases.append({"name": "Önden kestirme: iki bilinen istasyondan yalnız doğrultular, yeni noktalar arası bir kenar", "kind": "horizontal",
                  "unit": "grad", "known": [{"name": n, "y": float(true[n][0]), "x": float(true[n][1])} for n in ("A", "B")],
                  "rows": rows})

    # 7. A trilateration: distances only; the new points' approximate places from the drawing.
    true = {"T1": at(E0, N0), "T2": at(E0 + 300, N0 + 10), "T3": at(E0 + 150, N0 + 260), "T4": at(E0 + 420, N0 + 280)}
    rnd = random.Random(7)
    rows = []
    for s, t in (("T1", "T3"), ("T2", "T3"), ("T1", "T2"), ("T2", "T4"), ("T3", "T4"), ("T1", "T4")):
        rows.append(observe(true, s, t, "grad", rnd, 0, 8, 3, direction=False))
    cases.append({"name": "Trilaterasyon: yalnız kenarlar, yeni noktaların yaklaşık yerleri çizimden", "kind": "horizontal",
                  "unit": "grad", "known": [{"name": n, "y": float(true[n][0]), "x": float(true[n][1])} for n in ("T1", "T2")],
                  "approx": [{"name": "T3", "y": float(true["T3"][0]) + 0.8, "x": float(true["T3"][1]) - 0.5},
                             {"name": "T4", "y": float(true["T4"][0]) - 1.2, "x": float(true["T4"][1]) + 0.7}],
                  "rows": rows})
    trilat_rows = rows

    # 8. The same trilateration without the drawing: refused, the points named.
    cases.append({"name": "Trilaterasyon çizimsiz: yaklaşık yer bulunamaz, noktalar adıyla", "kind": "horizontal", "unit": "grad",
                  "known": [{"name": n, "y": float(true[n][0]), "x": float(true[n][1])} for n in ("T1", "T2")],
                  "rows": trilat_rows})

    # 9. One known point only: the datum is short, the first unknown it cannot fix named.
    true = {"K1": at(E0, N0), "Y1": at(E0 + 200, N0 + 100), "Y2": at(E0 + 50, N0 + 300)}
    rnd = random.Random(9)
    rows = []
    for s in ("K1", "Y1"):
        z0 = rnd.uniform(0, 2 * float(PI))
        for t in ("K1", "Y1", "Y2"):
            if t != s:
                rows.append(observe(true, s, t, "grad", rnd, z0, 8, 2))
    cases.append({"name": "Tek bilinen nokta: dayanak eksik, belirlenemeyen ilk bilinmeyen adıyla", "kind": "horizontal",
                  "unit": "grad", "approx": [{"name": "Y1", "y": float(true["Y1"][0]), "x": float(true["Y1"][1])},
                                             {"name": "Y2", "y": float(true["Y2"][0]), "x": float(true["Y2"][1])}],
                  "known": [{"name": "K1", "y": float(true["K1"][0]), "x": float(true["K1"][1])}], "rows": rows})

    # 10. Degrees: the braced quadrilateral read in degrees, the sigmas the project's.
    true = {"K1": at(E0, N0), "K2": at(E0 + 400, N0 + 20), "Y1": at(E0 + 30, N0 + 350), "Y2": at(E0 + 420, N0 + 380)}
    rnd = random.Random(10)
    rows = []
    for s in ("K1", "K2", "Y1", "Y2"):
        z0 = rnd.uniform(0, 2 * float(PI))
        for t in ("K1", "K2", "Y1", "Y2"):
            if t != s:
                rows.append(observe(true, s, t, "deg", rnd, z0, 10, 2, distance=(s < t), digits=5))
    cases.append({"name": "Derece: çaprazlı dörtgen derece okumalarla, projenin önsel doğruluklarıyla", "kind": "horizontal",
                  "unit": "deg", "sigma": {"direction": float(PI / 648000 * 3), "distance": 0.003, "ppm": 1.5, "centering": 0.0005},
                  "known": [{"name": "K1", "y": float(true["K1"][0]), "x": float(true["K1"][1])},
                            {"name": "K2", "y": float(true["K2"][0]), "x": float(true["K2"][1])}],
                  "rows": rows})

    # 11. No checks: a station on two known points and one new point, f = 0 after the minimum.
    true = {"A": at(E0, N0), "B": at(E0 + 300, N0 + 50), "N": at(E0 + 120, N0 + 210)}
    rnd = random.Random(11)
    z0 = 0.5
    rows = [observe(true, "A", "B", "grad", rnd, z0, 8, 2, distance=False), observe(true, "A", "N", "grad", rnd, z0, 8, 2)]
    cases.append({"name": "Denetimsiz: kutupsal tek nokta, f = 0; m0 ve model testi yok, ölçüler denetlenemez", "kind": "horizontal",
                  "unit": "grad", "known": [{"name": n, "y": float(true[n][0]), "x": float(true[n][1])} for n in ("A", "B")],
                  "rows": rows})

    # 12. The grid: a traverse whose distances were measured on the ground, the project's TM30 at 850 m.
    system = {"kind": "tm", "datum": "TUREF", "centralMeridian": 30, "scaleFactor": 1, "falseEasting": 500000, "falseNorthing": 0}
    E1, N1 = 623823.0, 4430536.0
    true = {"A": at(E1, N1), "B": at(E1 - 150, N1 + 400), "P1": at(E1 + 600, N1 + 120), "P2": at(E1 + 1250, N1 - 80),
            "C": at(E1 + 1900, N1 + 60), "D": at(E1 + 2300, N1 + 500)}
    import ground_survey_cases as gs
    rnd = random.Random(12)
    rows = []
    path = ["A", "P1", "P2", "C"]
    backs = {"A": "B", "C": "D"}
    for i, s in enumerate(path):
        z0 = rnd.uniform(0, 2 * float(PI))
        before = backs.get(s) if i == 0 else path[i - 1]
        after = path[i + 1] if i + 1 < len(path) else backs["C"]
        for t, with_d in ((before, i > 0), (after, i + 1 < len(path))):
            row = observe(true, s, t, "grad", rnd, z0, 6, 0, distance=False)
            if with_d:
                k, hf = gs.factors(system, 850.0, (float(true[s][0]), float(true[s][1])), (float(true[t][0]), float(true[t][1])))
                ground = dist(true[s], true[t]) / (mp.mpf(k) * mp.mpf(hf)) + rnd.uniform(-1, 1) * 0.002
                row["distance"] = float(mp.nint(ground * 10000) / 10000)
            rows.append(row)
    # PROJ's scales are numerical derivatives (within 1e-10, ground_survey_cases.py): the statistics agree within 1e-4.
    cases.append({"name": "Düzleme indirme: zeminde ölçülmüş kenarlar, projenin TM30'u ve 850 m'lik ortalama yüksekliği", "kind": "horizontal",
                  "unit": "grad", "grid": {"system": system, "height": 850.0}, "tolerances": {"relative": 1e-4, "w": 1e-3},
                  "known": [{"name": n, "y": float(true[n][0]), "x": float(true[n][1])} for n in ("A", "B", "C", "D")],
                  "rows": rows})

    # 13. Station and target the same: refused.
    cases.append({"name": "Durulan ve bakılan aynı: ret, satırıyla", "kind": "horizontal", "unit": "grad",
                  "known": [{"name": "A", "y": E0, "x": N0}, {"name": "B", "y": E0 + 10, "x": N0}],
                  "rows": [{"station": "A", "target": "B", "direction": 0.0, "distance": None},
                           {"station": "A", "target": "a", "direction": 10.0, "distance": None}]})

    # ── Levelling ──
    # 14. A geometric loop: two benchmarks, three new points, two loops.
    hs = {"R1": mp.mpf("102.4567"), "R2": mp.mpf("98.1234"), "N1": mp.mpf("105.0021"), "N2": mp.mpf("101.7788"), "N3": mp.mpf("99.3412")}
    rnd = random.Random(14)
    lines = [("R1", "N1", 820), ("N1", "N2", 640), ("N2", "R2", 910), ("R2", "N3", 450), ("N3", "R1", 1200), ("N1", "N3", 700), ("N2", "N3", 530)]
    rows = [{"from": a, "to": b, "dh": float(mp.nint((hs[b] - hs[a] + mp.mpf(rnd.uniform(-1, 1)) * mp.mpf(0.002) * mp.sqrt(mp.mpf(l) / 1000)) * 100000) / 100000),
             "length": float(l)} for a, b, l in lines]
    cases.append({"name": "Nivelman halkaları: iki reper, üç yeni nokta, geometrik nivelman", "kind": "level", "levelKind": "geometric",
                  "known": [{"name": "R1", "h": float(hs["R1"])}, {"name": "R2", "h": float(hs["R2"])}], "rows": rows})
    level_rows = rows

    # 15. A weighted benchmark with 5 mm and the projects' own levelling sigma.
    cases.append({"name": "Ağırlıklı reper: R2 σ 5 mm, projenin nivelman doğruluğu 1 mm/√km", "kind": "level", "levelKind": "geometric",
                  "sigma": {"levelling": 0.001},
                  "known": [{"name": "R1", "h": float(hs["R1"])}, {"name": "R2", "h": float(hs["R2"]) + 0.003, "sigma": 0.005}],
                  "rows": level_rows})

    # 16. A blunder of 3 cm in one line.
    rows = [dict(r) for r in level_rows]
    rows[2]["dh"] = round(rows[2]["dh"] + 0.03, 5)
    cases.append({"name": "Uyuşumsuz kot farkı: bir hatta 3 cm; en büyük w onun, model testi kalır", "kind": "level", "levelKind": "geometric",
                  "known": [{"name": "R1", "h": float(hs["R1"])}, {"name": "R2", "h": float(hs["R2"])}], "rows": rows})

    # 17. Trigonometric height differences from a field book: both ways, horizontal distances.
    hs = {"S1": mp.mpf("850.112"), "S2": mp.mpf("857.903"), "S3": mp.mpf("846.215"), "S4": mp.mpf("861.004")}
    rnd = random.Random(17)
    rows = []
    for a, b, l in (("S1", "S2", 312.4), ("S2", "S1", 312.4), ("S2", "S3", 288.1), ("S3", "S2", 288.1), ("S3", "S4", 401.7),
                    ("S4", "S1", 350.2), ("S1", "S3", 276.0)):
        rows.append({"from": a, "to": b, "dh": float(mp.nint((hs[b] - hs[a] + mp.mpf(rnd.uniform(-1, 1)) * mp.mpf(l) * mp.mpf(1.5e-5)) * 10000) / 10000),
                     "length": l})
    cases.append({"name": "Trigonometrik kot farkları: iki yönden, yatay uzunluklarla; σ başucu açısından ve kenardan", "kind": "level",
                  "levelKind": "trigonometric", "known": [{"name": "S1", "h": float(hs["S1"])}], "rows": rows})

    # 18. A point tied to no known height: refused.
    cases.append({"name": "Bağlı olmayan nokta: ret, adıyla", "kind": "level", "levelKind": "geometric",
                  "known": [{"name": "R1", "h": 100.0}],
                  "rows": [{"from": "R1", "to": "N1", "dh": 1.234, "length": 300.0}, {"from": "N7", "to": "N8", "dh": 0.5, "length": 200.0}]})

    for c in cases:
        # The sigmas as the window resolves them: the project's, else the defaults.
        c["sigma"] = {**DEFAULTS, **c.get("sigma", {})}
        try:
            c["expect"] = horizontal(c) if c["kind"] == "horizontal" else levelling(c)
        except Refused as e:
            c["error"] = str(e)
    quantiles = [{"f": f, "chi2": float(chi2_quantile(f))} for f in (1, 2, 3, 4, 5, 8, 10, 15, 20, 30, 50, 100)]
    return {
        "format": "kentos.network-adjust-cases",
        "version": 1,
        "source": "docs/adr/0203; scripts/fixtures/network_adjust_cases.py (mpmath 50 digits, no KentOS code)",
        "defaults": DEFAULTS,
        "tolerances": {"coordinate": 1e-6, "height": 1e-7, "relative": 1e-6, "angle": 1e-9, "w": 1e-5},
        "chi2": quantiles,
        "cases": cases,
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    data = build()
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if args.check:
        old = OUT.read_text() if OUT.exists() else ""
        if old != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check'siz çalıştırıp farkı okuyun.")
            sys.exit(1)
        print(f"{OUT.relative_to(ROOT)} güncel ({len(data['cases'])} durum)")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)} yazıldı ({len(data['cases'])} durum)")


if __name__ == "__main__":
    main()
