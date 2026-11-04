#!/usr/bin/env python3
"""Writes fixtures/commands/v1/road.profiles.propose.json: the shared cases of KentOS Yol's grade line proposal
(plan/road.md Y25, docs/adr/0926; ROAD-PROFILE-14), their expectations built here without KentOS code. The drawing,
its alignments, surface and terrain profile are road_profile_command_cases.py's; a profile's elevation is
road_vertical_cases.py's (exact fractions). The design is linear in the PVI elevations: on each stretch between
breakpoints (the terrain profile's points, the PVIs, the curves' ends) both it and the ground are polynomials, so
∫(design − ground)² is a quadratic in the elevations integrated exactly in fractions. The programme (held PVIs, grade
limits, the balance) is started by scipy's SLSQP; its active set is then solved again exactly in fractions and
certified by the KKT conditions (feasible, every active limit's multiplier positive). The areas cut and filled are
read in mpmath at the roots of each stretch's difference. The profiles are on the straight “Yan aks”, where the
terrain profile is exactly linear between its points.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj --with scipy \\
        python3 scripts/fixtures/road_profile_propose_cases.py [--check]
"""

import json
import sys
from fractions import Fraction as F
from pathlib import Path

import mpmath as mp
import numpy as np
from scipy.optimize import minimize

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_profile_command_cases as cmd  # noqa: E402
import road_vertical_cases as vref  # noqa: E402

mp.mp.dps = 40
OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
W = 1e-6
pv = cmd.pv

S2 = {"id": "b-4", "name": "Uzun kot", "alignment": "g-2", "alignmentRevision": 3, "revision": 1, "surface": "t-1",
      "pvis": [pv("a", 0, 111), pv("v1", 150, 106, length=80), pv("v2", 330, 103, lengthIn=60, lengthOut=40),
               pv("v3", 500, 108, length=100), pv("b", 650, 113)]}
S3 = {"id": "b-5", "name": "Düz kot", "alignment": "g-2", "alignmentRevision": 3, "revision": 1, "surface": "t-1",
      "pvis": [pv("a", 0, 110), pv("v1", 200, 105), pv("v2", 420, 107), pv("b", 650, 112)]}
PROFILES = [cmd.MAIN, cmd.SIDE, cmd.PART, S2, S3]


class Refused(Exception):
    def __init__(self, message, path):
        super().__init__(message)
        self.result = cmd.failed("invalid_profile_proposal", message, path)


def design_coeffs(d):
    """A function of a station giving, per PVI, the design's elevation for that PVI's unit elevation (fractions)."""
    base = d["pvis"]
    n = len(base)

    def at(zs, q):
        refs = cmd.to_vref(base)
        for r, z in zip(refs, zs):
            r["elevation"] = z
        curves, _, S, G = vref.solve(refs)
        Z = [F(z) for z in zs]
        for i, (b, e, ev, _, _) in curves.items():
            if b <= q <= e:
                return ev(q)[0]
        for i in range(len(S) - 1):
            if S[i] <= q <= S[i + 1]:
                return Z[i] + G[i] * (q - S[i])
        raise ValueError(q)

    zs0 = [F(p["elevation"]) for p in base]

    def coeffs(q):
        z0 = at(zs0, q)
        out = []
        for j in range(n):
            zs = list(zs0)
            zs[j] += 1
            out.append(at(zs, q) - z0)
        return out

    return coeffs, at


def reach(p):
    if p.get("length"):
        return F(p["length"]) / 2, F(p["length"]) / 2
    return F(p.get("lengthIn", 0)), F(p.get("lengthOut", 0))


def stretches(d):
    S = [F(p["station"]) for p in d["pvis"]]
    breaks = list(S)
    for p, s in zip(d["pvis"], S):
        l1, l2 = reach(p)
        if l1 + l2 > 0:
            breaks += [s - l1, s + l2]
    rows = cmd.ground(d["alignment"])
    out = []
    for p, q in zip(rows, rows[1:]):
        if "z" not in p or "z" not in q or q["gap"] or q["station"] <= p["station"]:
            continue
        ps, qs = F(p["station"]), F(q["station"])
        a, b = max(ps, S[0]), min(qs, S[-1])
        if b <= a:
            continue
        za, zb = F(p["z"]), F(q["z"])
        line = lambda s, ps=ps, qs=qs, za=za, zb=zb: za + (zb - za) * (s - ps) / (qs - ps)  # noqa: E731
        cuts = sorted({x for x in breaks if a < x < b})
        frm = a
        for x in cuts + [b]:
            if x > frm:
                out.append((frm, x, line(frm), line(x)))
            frm = x
    return out


def quad(v0, vm, v1, L):
    """p0, p1, p2 of p0 + p1 t + p2 t² through (0, v0), (L/2, vm), (L, v1)."""
    p2 = 2 * (v0 - 2 * vm + v1) / (L * L)
    p1 = (v1 - v0) / L - p2 * L
    return [v0, p1, p2]


def integral(p, q, L):
    return sum(p[i] * q[j] * L ** (i + j + 1) / (i + j + 1) for i in range(3) for j in range(3))


def solve_kkt(H, b, rows, n):
    """H z + Aᵀλ = b, A z = c, exactly."""
    m = n + len(rows)
    A = [[F(0)] * (m + 1) for _ in range(m)]
    for i in range(n):
        for j in range(n):
            A[i][j] = H[i][j]
        A[i][m] = b[i]
    for k, (row, c) in enumerate(rows):
        for i in range(n):
            A[i][n + k] = row[i]
            A[n + k][i] = row[i]
        A[n + k][m] = c
    for c in range(m):
        p = next(i for i in range(c, m) if A[i][c] != 0)
        A[c], A[p] = A[p], A[c]
        piv = A[c][c]
        A[c] = [v / piv for v in A[c]]
        for i in range(m):
            if i != c and A[i][c] != 0:
                f = A[i][c]
                A[i] = [v - f * w for v, w in zip(A[i], A[c])]
    return [A[i][m] for i in range(m)]


def propose(d, hold, most, balanced):
    n = len(d["pvis"])
    coeffs, at = design_coeffs(d)
    pieces = stretches(d)
    H = [[F(0)] * n for _ in range(n)]
    b = [F(0)] * n
    a = [F(0)] * n
    garea = F(0)
    covered = F(0)
    for (s0, s1, g0, g1) in pieces:
        L = s1 - s0
        covered += L
        c0, cm, c1 = coeffs(s0), coeffs((s0 + s1) / 2), coeffs(s1)
        polys = [quad(c0[j], cm[j], c1[j], L) for j in range(n)]
        gp = [g0, (g1 - g0) / L, F(0)]
        for i in range(n):
            b[i] += integral(polys[i], gp, L)
            a[i] += integral(polys[i], [F(1), F(0), F(0)], L)
            for j in range(n):
                H[i][j] += integral(polys[i], polys[j], L)
        garea += integral(gp, [F(1), F(0), F(0)], L)
    S = [F(p["station"]) for p in d["pvis"]]
    eq = []
    for k, p in enumerate(d["pvis"]):
        if p["id"] in hold:
            row = [F(0)] * n
            row[k] = F(1)
            eq.append((row, F(p["elevation"])))
    if balanced:
        eq.append((a, garea))
    ineq = []
    if most is not None:
        for k in range(n - 1):
            for sign in (1, -1):
                row = [F(0)] * n
                row[k + 1], row[k] = F(sign), F(-sign)
                ineq.append((row, F(most) * (S[k + 1] - S[k])))
    # A start from SLSQP; its active limits solved again exactly and certified.
    Hf = np.array([[float(v) for v in r] for r in H])
    bf = np.array([float(v) for v in b])
    cons = [{"type": "eq", "fun": (lambda z, r=r, c=c: np.dot([float(v) for v in r], z) - float(c))} for r, c in eq]
    cons += [{"type": "ineq", "fun": (lambda z, r=r, c=c: float(c) - np.dot([float(v) for v in r], z))} for r, c in ineq]
    z0 = np.array([p["elevation"] for p in d["pvis"]])
    res = minimize(lambda z: z @ Hf @ z - 2 * bf @ z, z0, jac=lambda z: 2 * Hf @ z - 2 * bf, constraints=cons,
                   method="SLSQP", options={"ftol": 1e-15, "maxiter": 2000})
    # A start only: the exact active set below corrects it.
    active = [i for i, (r, c) in enumerate(ineq) if float(c) - np.dot([float(v) for v in r], res.x) < 1e-5 * max(1.0, float(c))]
    for _ in range(50):
        x = solve_kkt(H, b, eq + [ineq[i] for i in active], n)
        z = x[:n]
        lam = x[n + len(eq):]
        bad = [k for k, l in enumerate(lam) if l < 0]
        over = [i for i, (r, c) in enumerate(ineq) if i not in active and sum(p * q for p, q in zip(r, z)) > c]
        if not bad and not over:
            break
        if bad:
            active.pop(bad[0])
        else:
            active.append(over[0])
    else:
        raise AssertionError("no KKT point")
    for k, l in enumerate(lam):
        assert l > F(1, 10**9), f"a weakly active limit: {float(l)}"
    grades = [(z[k + 1] - z[k]) / (S[k + 1] - S[k]) for k in range(n - 1)]
    limited = [("rise" if 2 * k in active else "fall" if 2 * k + 1 in active else None) for k in range(n - 1)]
    # The squares exactly; the areas at the difference's roots in mpmath.
    squares = F(0)
    cut = mp.mpf(0)
    fill = mp.mpf(0)
    for (s0, s1, g0, g1) in pieces:
        L = s1 - s0
        c0, cm, c1 = coeffs(s0), coeffs((s0 + s1) / 2), coeffs(s1)
        dv = [sum(cc * zz for cc, zz in zip(cv, z)) for cv in (c0, cm, c1)]
        gm = (g0 + g1) / 2
        p = quad(dv[0] - g0, dv[1] - gm, dv[2] - g1, L)
        squares += integral(p, p, L)
        P = [mp.mpf(v.numerator) / v.denominator for v in p]
        Lm = mp.mpf(L.numerator) / L.denominator
        roots = []
        if P[2] != 0:
            disc = P[1] ** 2 - 4 * P[2] * P[0]
            if disc > 0:
                roots = [(-P[1] - mp.sqrt(disc)) / (2 * P[2]), (-P[1] + mp.sqrt(disc)) / (2 * P[2])]
        elif P[1] != 0:
            roots = [-P[0] / P[1]]
        cuts = sorted(r for r in roots if 0 < r < Lm)
        frm = mp.mpf(0)
        for t in cuts + [Lm]:
            v = P[0] * (t - frm) + P[1] * (t * t - frm * frm) / 2 + P[2] * (t ** 3 - frm ** 3) / 3
            mid = (frm + t) / 2
            if P[0] + P[1] * mid + P[2] * mid * mid >= 0:
                fill += v
            else:
                cut -= v
            frm = t
    return {"z": z, "grades": grades, "limited": limited, "squares": squares, "covered": covered, "cut": cut,
            "fill": fill}


def answer(inp, profiles):
    name = inp["profile"]
    d = next((p for p in profiles if p["id"] == name), None) or next(
        (p for p in profiles if p["name"].lower() == name.lower()), None)
    if d is None:
        raise Refused2("unknown_road_profile", f"“{name}” kimlikli ya da adlı boykesit projede yok.", "profile")
    hold = inp.get("hold", [])
    for j, h in enumerate(hold):
        if not any(p["id"] == h for p in d["pvis"]):
            raise Refused(f"“{d['name']}” boykesitinde “{h}” PVI'si yok.", f"hold[{j}]")
    most = inp.get("mostGrade")
    if most is not None and not most > 0:
        raise Refused("En büyük eğim (mostGrade) 0'dan büyük bir oran olmalı.", "mostGrade")
    if d.get("surface") is None:
        raise Refused(f"“{d['name']}” boykesitinin yüzeyi yok; öneri arazi boykesitine göre kurulur.", "profile")
    warnings = []
    if cmd.REV[d["alignment"]] != d["alignmentRevision"]:
        warnings.append({"code": "profile_stale", "path": "profile",
                         "message": f"“{d['name']}” boykesitinin aksı değişti (boykesit {d['alignmentRevision']}. "
                                    f"revizyona göre kurulu, aks şimdi {cmd.REV[d['alignment']]}.); PVI'ları denetleyip "
                                    f"aksa yeniden bağlayın."})
    for p in d["pvis"]:
        if p.get("radius"):
            raise Refused(f"“{d['name']}” boykesitinin {p['id']} PVI'sinde dairesel kurp var; öneri yalnız parabolik "
                          f"kurplarla kurulur.", "profile")
    r = propose(d, set(hold), most, inp.get("balanced", False))
    out = {"profile": d["id"],
           "pvis": [{"id": p["id"], "station": p["station"], "elevation": float(z), "was": p["elevation"],
                     "held": p["id"] in hold} for p, z in zip(d["pvis"], r["z"])],
           "grades": [{"between": [p["id"], q["id"]], "grade": float(g), **({"limit": lm} if lm else {})}
                      for p, q, g, lm in zip(d["pvis"], d["pvis"][1:], r["grades"], r["limited"])],
           "squares": float(r["squares"]), "covered": float(r["covered"]),
           "rms": float(mp.sqrt(mp.mpf(r["squares"].numerator) / r["squares"].denominator
                                / (mp.mpf(r["covered"].numerator) / r["covered"].denominator))),
           "cut": float(r["cut"]), "fill": float(r["fill"])}
    return out, warnings


class Refused2(Exception):
    def __init__(self, code, message, path):
        super().__init__(message)
        self.result = cmd.failed(code, message, path)


def cases():
    out = []

    def q(name, inp, op="execute", setup_=None, result=None):
        if result is None:
            try:
                o, w = answer(inp, PROFILES)
                result = cmd.completed(None if op == "validate" else o, w)
            except (Refused, Refused2) as r:
                result = r.result
        road = setup_["settings"].get("road") if setup_ is not None else cmd.road_of(PROFILES)
        c = {"name": name, "steps": [{"op": op, "input": inp, "result": result,
                                      "expect": {**cmd.UNCHANGED, "road": road}, "within": W}]}
        if setup_ is not None:
            c["setup"] = setup_
        out.append(c)

    q("Kurpsuz boykesit, kural yok", {"profile": "b-5"})
    q("Kurpsuz, uçlar kilitli", {"profile": "b-5", "hold": ["a", "b"]})
    q("Kurpsuz, en büyük eğim %1", {"profile": "b-5", "mostGrade": 0.01})
    q("Kurpsuz, uçlar kilitli ve en büyük eğim %1,5", {"profile": "b-5", "hold": ["a", "b"], "mostGrade": 0.015})
    q("Kurpsuz, dengeli", {"profile": "b-5", "balanced": True})
    q("Kurpsuz, dengeli ve uçlar kilitli", {"profile": "Düz KOT", "hold": ["a", "b"], "balanced": True})
    q("Kurplu boykesit, kural yok", {"profile": "b-4"})
    q("Kurplu, uçlar kilitli", {"profile": "b-4", "hold": ["a", "b"]})
    q("Kurplu, ortadaki PVI kilitli", {"profile": "b-4", "hold": ["v2"]})
    q("Kurplu, en büyük eğim %1,2", {"profile": "b-4", "mostGrade": 0.012})
    q("Kurplu, dengeli ve en büyük eğim %2", {"profile": "b-4", "balanced": True, "mostGrade": 0.02})
    q("Kurplu, hepsi kilitli: kotlar değişmez", {"profile": "b-4", "hold": ["a", "v1", "v2", "v3", "b"]})
    q("Eskiyen boykesit uyarılır", {"profile": "b-2"})
    q("Eskiyen, en büyük eğim %1", {"profile": "Yan kot", "mostGrade": 0.01, "hold": ["a"]})
    q("Plan cevabı verir", {"profile": "b-5", "mostGrade": 0.01}, op="plan")
    q("Doğrulama uyarıları verir", {"profile": "b-2"}, op="validate")
    # Refusals.
    q("Yol projesi değil", {"profile": "b-5"}, setup_=cmd.setup(work="map"), result=cmd.NOT_ROAD)
    q("Olmayan boykesit", {"profile": "yok"})
    q("Olmayan PVI kilitli", {"profile": "b-5", "hold": ["a", "x"]})
    q("En büyük eğim sıfır", {"profile": "b-5", "mostGrade": 0.0})
    q("En büyük eğim eksi", {"profile": "b-5", "mostGrade": -0.01})
    q("Dairesel kurplu boykesit", {"profile": "b-1"})
    q("Yüzeyi olmayan boykesit", {"profile": "b-3"})
    q("Kilitli PVI'lar en büyük eğimi aşıyor", {"profile": "b-5", "hold": ["a", "v1"], "mostGrade": 0.01},
      result=cmd.failed("invalid_profile_proposal", "“Düz kot” boykesitinin kuralları birlikte tutmuyor: kilitli PVI'lar "
                        "arasındaki eğim en büyük eğimi aşıyor ya da denge kurulamıyor.", "profile"))
    return out


def main():
    found = cases()
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.profiles.propose", "commandVersion": 1,
           "title": "Kırmızı kot önerisi: araziye en yakın düşey güzergâh",
           "note": "scripts/fixtures/road_profile_propose_cases.py yazar; çizim, aks, yüzey ve arazi boykesiti "
                   "road_profile_command_cases.py'nin, düşey güzergâh road_vertical_cases.py'nin kesirleridir; kareler "
                   "toplamı kesirlerle tam, program SLSQP'den başlar ve etkin kümesi kesirlerle KKT koşullarıyla "
                   "doğrulanır; KentOS kodu yok.",
           "setup": cmd.setup(PROFILES), "cases": found}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    path = OUT / "road.profiles.propose.json"
    if "--check" in sys.argv:
        if not path.exists() or path.read_text() != text:
            sys.exit("güncel değil: road.profiles.propose.json")
    else:
        path.write_text(text)
    print(f"road.profiles.propose.json: {len(found)} durum")


if __name__ == "__main__":
    main()
