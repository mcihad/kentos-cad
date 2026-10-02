#!/usr/bin/env python3
"""Independent reference of Vektör oturtma's solution (docs/adr/0156 §2–§3).

Writes fixtures/fit/v1/solve.json from the rules alone, with Python's
standard library and no KentOS code: every number below is worked out with
exact fractions of the float64 inputs (centroids, sums, normal equations),
and only the answers are rounded to float64. Both platforms solve every case
with the shared core (`ops::fit`, WASM `fitTransform`) and must find the same
parameters, residuals and m0, or the same reason there is no solution.

The rules:

1. A pair is a source (x east, y north) and a target, and `used`. Only the
   used pairs take part; every pair's residual is given.
2. The used pairs' centroids (exact means) are `from` (sources) and `to`
   (targets); x̄ = x − from.x, ȳ = y − from.y, X̄ = X − to.x, Ȳ = Y − to.y.
3. Helmert: X̄ = a·x̄ − b·ȳ, Ȳ = b·x̄ + a·ȳ; a = Σ(x̄·X̄ + ȳ·Ȳ) / Σ(x̄² + ȳ²),
   b = Σ(x̄·Ȳ − ȳ·X̄) / Σ(x̄² + ȳ²). At least 2 used pairs; sources all in one
   place: `coincident`.
4. Affine: X̄ = a·x̄ + c·ȳ, Ȳ = b·x̄ + d·ȳ (the core's `Affine` order), each
   axis from its 2×2 normal equations. At least 3; sources on one line (the
   normal determinant zero; the core also takes one below 1e-12 of
   Σx̄²·Σȳ² for zero): `collinear`.
5. Projective: X̄·(c1·x̄ + c2·ȳ + 1) = a1·x̄ + a2·ȳ + a3 and the same for Ȳ with
   b1, b2, b3: the 8×8 normal equations of these (linear) equations, solved
   exactly. At least 4; a singular system: `singular` (the core: a pivot
   below 1e-12 of the largest diagonal, in frames scaled by their largest
   coordinate). The cases keep clear of the numerical edge: their systems
   are singular exactly or well conditioned.
6. Fewer used pairs than the kind needs: `too_few` with the number needed.
7. Residual v = transformed source − target (in the centred frames, exactly):
   vx, vy and its length. m0 = √([vv] / (2n − u)), u = 4, 6, 8; none when
   2n = u.
8. Helmert also gives its scale √(a² + b²) and rotation atan2(b, a)
   (radians); the affine its X scale √(a² + b²), Y scale √(c² + d²),
   rotation atan2(b, a) and shear: 90° less the angle between the images of
   the axes, atan2(a·c + b·d, a·d − b·c), radians.

What is compared: `from`, `to`, the parameters, every pair's residual and
its length, m0 and the derived values, within the file's tolerances; or the
error and its number.
"""

import argparse
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "fit" / "v1" / "solve.json"
NEED = {"helmert": 2, "affine": 3, "projective": 4}
UNKNOWNS = {"helmert": 4, "affine": 6, "projective": 8}


def solve_exact(m, rhs):
    """Gauss–Jordan elimination over fractions; None when singular."""
    n = len(m)
    # Every entry a fraction: an int over an int would divide into a float.
    a = [[F(v) for v in row] + [F(rhs[i])] for i, row in enumerate(m)]
    for col in range(n):
        pivot = next((r for r in range(col, n) if a[r][col] != 0), None)
        if pivot is None:
            return None
        a[col], a[pivot] = a[pivot], a[col]
        p = a[col][col]
        a[col] = [v / p for v in a[col]]
        for r in range(n):
            if r != col and a[r][col] != 0:
                f = a[r][col]
                a[r] = [v - f * w for v, w in zip(a[r], a[col])]
    return [a[i][n] for i in range(n)]


def fit(pairs, kind):
    used = [p for p in pairs if p["used"]]
    if len(used) < NEED[kind]:
        return {"error": "too_few", "need": NEED[kind]}
    n = len(used)
    src = [(F(p["source"]["x"]), F(p["source"]["y"])) for p in used]
    dst = [(F(p["target"]["x"]), F(p["target"]["y"])) for p in used]
    x0 = sum(s[0] for s in src) / n
    y0 = sum(s[1] for s in src) / n
    X0 = sum(d[0] for d in dst) / n
    Y0 = sum(d[1] for d in dst) / n
    c_src = [(x - x0, y - y0) for x, y in src]
    c_dst = [(X - X0, Y - Y0) for X, Y in dst]

    if kind == "helmert":
        s = sum(x * x + y * y for x, y in c_src)
        if s == 0:
            return {"error": "coincident"}
        a = sum(x * X + y * Y for (x, y), (X, Y) in zip(c_src, c_dst)) / s
        b = sum(x * Y - y * X for (x, y), (X, Y) in zip(c_src, c_dst)) / s
        params = [a, b]

        def apply(x, y):
            return (a * x - b * y, b * x + a * y)
    elif kind == "affine":
        sxx = sum(x * x for x, _ in c_src)
        syy = sum(y * y for _, y in c_src)
        sxy = sum(x * y for x, y in c_src)
        det = sxx * syy - sxy * sxy
        if det == 0:
            return {"error": "collinear"}
        sxX = sum(x * X for (x, _), (X, _) in zip(c_src, c_dst))
        syX = sum(y * X for (_, y), (X, _) in zip(c_src, c_dst))
        sxY = sum(x * Y for (x, _), (_, Y) in zip(c_src, c_dst))
        syY = sum(y * Y for (_, y), (_, Y) in zip(c_src, c_dst))
        # [sxx sxy; sxy syy]·[a; c] = [sxX; syX], and b, d for Y.
        a = (sxX * syy - syX * sxy) / det
        c = (syX * sxx - sxX * sxy) / det
        b = (sxY * syy - syY * sxy) / det
        d = (syY * sxx - sxY * sxy) / det
        params = [a, b, c, d]

        def apply(x, y):
            return (a * x + c * y, b * x + d * y)
    else:
        rows, rhs = [], []
        for (x, y), (X, Y) in zip(c_src, c_dst):
            rows.append([x, y, 1, 0, 0, 0, -x * X, -y * X])
            rhs.append(X)
            rows.append([0, 0, 0, x, y, 1, -x * Y, -y * Y])
            rhs.append(Y)
        ata = [[sum(r[i] * r[j] for r in rows) for j in range(8)] for i in range(8)]
        atb = [sum(r[i] * v for r, v in zip(rows, rhs)) for i in range(8)]
        h = solve_exact(ata, atb)
        if h is None:
            return {"error": "singular"}
        params = h
        a1, a2, a3, b1, b2, b3, c1, c2 = h

        def apply(x, y):
            w = c1 * x + c2 * y + 1
            return ((a1 * x + a2 * y + a3) / w, (b1 * x + b2 * y + b3) / w)

    out = {
        "from": {"x": float(x0), "y": float(y0)},
        "to": {"x": float(X0), "y": float(Y0)},
        "params": [float(p) for p in params],
    }
    residuals, vv = [], F(0)
    for p in pairs:
        x, y = F(p["source"]["x"]) - x0, F(p["source"]["y"]) - y0
        X, Y = F(p["target"]["x"]) - X0, F(p["target"]["y"]) - Y0
        tx, ty = apply(x, y)
        vx, vy = tx - X, ty - Y
        if p["used"]:
            vv += vx * vx + vy * vy
        residuals.append([float(vx), float(vy), math.sqrt(float(vx * vx + vy * vy))])
    out["residuals"] = residuals
    dof = 2 * n - UNKNOWNS[kind]
    out["m0"] = math.sqrt(float(vv / dof)) if dof > 0 else None
    if kind == "helmert":
        a, b = params
        out["scale"] = math.sqrt(float(a * a + b * b))
        out["rotation"] = math.atan2(float(b), float(a))
    elif kind == "affine":
        a, b, c, d = params
        out["scaleX"] = math.sqrt(float(a * a + b * b))
        out["scaleY"] = math.sqrt(float(c * c + d * d))
        out["rotation"] = math.atan2(float(b), float(a))
        out["shear"] = math.atan2(float(a * c + b * d), float(a * d - b * c))
    return out


def pair(sx, sy, tx, ty, used=True):
    return {"source": {"x": sx, "y": sy}, "target": {"x": tx, "y": ty}, "used": used}


def helmert_of(scale, angle, tx, ty):
    """A similarity as the cases build their targets: rotate by `angle`, scale, shift."""
    ca, sa = math.cos(angle) * scale, math.sin(angle) * scale
    return lambda x, y: (ca * x - sa * y + tx, sa * x + ca * y + ty)


def cases():
    # A sheet digitised in a local system (metres from its corner) and the same corners measured in TM30.
    local = [(0.0, 0.0), (250.0, 0.0), (250.0, 200.0), (0.0, 200.0), (125.0, 100.0), (60.25, 180.5)]
    to_tm = helmert_of(1.000180, 0.0723, 487012.341, 4419876.552)
    tm = [to_tm(x, y) for x, y in local]
    # Measurement noise in millimetres, fixed.
    noise = [(0.012, -0.007), (-0.004, 0.009), (0.006, 0.011), (-0.010, -0.003), (0.002, -0.012), (0.008, 0.004)]
    noisy = [(X + dx, Y + dy) for (X, Y), (dx, dy) in zip(tm, noise)]
    # A paper sheet shrunk 0.2 % along X and 0.1 % along Y, a little sheared, placed in TM30.
    def paper(x, y):
        return (0.998 * x + 0.0007 * y + 487100.0, -0.0004 * x + 0.999 * y + 4420050.0)
    sheet = [(12.5, 8.25), (480.0, 10.0), (478.5, 610.75), (10.0, 612.0), (240.0, 300.0), (120.75, 455.5), (390.25, 140.0)]
    shrunk = [paper(x, y) for x, y in sheet]
    shrunk_noisy = [(X + dx, Y + dy) for (X, Y), (dx, dy) in zip(shrunk, noise + [(0.005, -0.006)])]
    # A plan photographed at an angle: a projective map.
    def persp(x, y):
        w = 0.00002 * x - 0.00001 * y + 1
        return ((1.01 * x + 0.02 * y + 30.0) / w + 487000.0, (-0.015 * x + 0.99 * y - 12.0) / w + 4420000.0)
    plan = [(0.0, 0.0), (300.0, 0.0), (300.0, 200.0), (0.0, 200.0), (150.0, 100.0), (75.0, 160.0)]
    seen = [persp(x, y) for x, y in plan]
    seen_noisy = [(X + dx, Y + dy) for (X, Y), (dx, dy) in zip(seen, noise)]

    rows = []

    def add(name, kind, pairs):
        rows.append({"name": name, "kind": kind, "pairs": pairs, "expected": fit(pairs, kind)})

    add("Helmert, iki çift: tam geçer, m0 yok", "helmert", [pair(*local[i], *tm[i]) for i in (0, 2)])
    add("Helmert, altı çift, milimetrelik gürültü, ülke koordinatları", "helmert", [pair(*s, *t) for s, t in zip(local, noisy)])
    add(
        "Helmert, bir çift çıkarılmış: artığı yine hesaplanır",
        "helmert",
        [pair(*s, *t, used=(i != 4)) for i, (s, t) in enumerate(zip(local, [(X + 0.25, Y - 0.4) if i == 4 else (X, Y) for i, (X, Y) in enumerate(noisy)]))],
    )
    add("Helmert, kaba hatalı bir çift: m0 büyür", "helmert", [pair(*s, *t) for s, t in zip(local, [(X + 0.25, Y - 0.4) if i == 4 else (X, Y) for i, (X, Y) in enumerate(noisy)])])
    add("Afin, üç çift: tam geçer", "affine", [pair(*sheet[i], *shrunk[i]) for i in (0, 1, 2)])
    add("Afin, yedi çift: büzülen ve kayan pafta", "affine", [pair(*s, *t) for s, t in zip(sheet, shrunk_noisy)])
    add("Afin, aynı Helmert verisi: benzerliğe yakın", "affine", [pair(*s, *t) for s, t in zip(local, noisy)])
    add("Projektif, dört çift: tam geçer", "projective", [pair(*plan[i], *seen[i]) for i in range(4)])
    add("Projektif, altı çift, gürültülü", "projective", [pair(*s, *t) for s, t in zip(plan, seen_noisy)])
    add("Helmert, bir çift: en az iki gerekir", "helmert", [pair(*local[0], *tm[0]), pair(*local[1], *tm[1], used=False)])
    add("Helmert, kaynaklar aynı yerde", "helmert", [pair(10.0, 20.0, *tm[0]), pair(10.0, 20.0, *tm[1])])
    add("Afin, kaynaklar bir doğru üstünde", "affine", [pair(0.0, 0.0, *tm[0]), pair(10.0, 5.0, *tm[1]), pair(30.0, 15.0, *tm[2]), pair(-20.0, -10.0, *tm[3])])
    add("Afin, iki çift: en az üç gerekir", "affine", [pair(*sheet[0], *shrunk[0]), pair(*sheet[1], *shrunk[1])])
    add("Projektif, üç çift: en az dört gerekir", "projective", [pair(*plan[i], *seen[i]) for i in range(3)])
    add("Projektif, kaynakların hepsi bir doğru üstünde", "projective", [pair(float(i) * 10.0, float(i) * 5.0, *seen[i]) for i in range(5)])
    return rows


def build():
    return {
        "format": "kentos.fit-solve",
        "version": 1,
        # Coordinates and residuals in metres; parameters relative to their size (at least 1); m0 and the derived
        # values relative to theirs.
        "tolerance": {"metres": 1e-9, "relative": 1e-12},
        "cases": cases(),
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--check", action="store_true", help="compare with the written file instead of writing it")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT} güncel değil: python3 scripts/fixtures/fit_cases.py", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT}: güncel")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f"{OUT}: {len(json.loads(text)['cases'])} durum")


if __name__ == "__main__":
    main()
