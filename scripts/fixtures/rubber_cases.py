#!/usr/bin/env python3
"""Independent reference of Kauçuk levha's solution (docs/adr/0158 §2).

Writes fixtures/fit/v1/rubber.json from the rules alone, with mpmath at 50
digits and no KentOS code. Both platforms solve every case with the shared
core (`ops::rubber`, WASM `rubberSheet`) and must find the same images of
the probes and the same derivatives there, or the same reason there is no
sheet.

The rules:

1. A link is a source and a target; the displacement there is target −
   source. A fixed point is a link whose target is its source.
2. The sheet is the thin plate spline of the displacements, for east and
   north: d(p) = a0 + a1·x + a2·y + Σ wi·φ(|p − si|), φ(r) = r²·ln r
   (φ(0) = 0), with d(si) = di and Σ wi = Σ wi·xi = Σ wi·yi = 0. Here the
   equations are solved in the drawing's own coordinates, unscaled, at 50
   digits (the core scales them; the spline is the same).
3. A probe p goes to p + d(p); the derivative is the 2×2 matrix whose
   columns are the images of a metre east and a metre north:
   J = I + ∂d/∂p, with ∂φ/∂x = (x − xi)·(2·ln r + 1) (0 at the source),
   written [a, b, c, d] = [∂x'/∂x, ∂y'/∂x, ∂x'/∂y, ∂y'/∂y].
4. No sheet: fewer than 3 links (`too_few`), two links from one point
   (`duplicate`), every source on one line (`collinear`).

What is compared: every probe's image (metres) and the derivative, within
the file's tolerances, or the error.
"""

import argparse
import json
import math
import random
import sys
from pathlib import Path

import mpmath as mp

mp.mp.dps = 50

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "fit" / "v1" / "rubber.json"


def phi(r2):
    """r²·ln r of a squared distance, 0 at 0."""
    return mp.mpf(0) if r2 == 0 else r2 * mp.log(r2) / 2


def collinear(points):
    """Every point on one line (exactly: float64 inputs are exact rationals)."""
    from fractions import Fraction as F

    pts = [(F(x), F(y)) for x, y in points]
    a = pts[0]
    b = next((p for p in pts if p != a), None)
    if b is None:
        return True
    return all((b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]) == 0 for p in pts)


def solve(links):
    n = len(links)
    if n < 3:
        return {"error": "too_few"}
    sources = [(l["from"]["x"], l["from"]["y"]) for l in links]
    if len(set(sources)) != n:
        return {"error": "duplicate"}
    if collinear(sources):
        return {"error": "collinear"}
    s = [(mp.mpf(x), mp.mpf(y)) for x, y in sources]
    m = n + 3
    A = mp.matrix(m, m)
    bx = mp.matrix(m, 1)
    by = mp.matrix(m, 1)
    for i in range(n):
        for j in range(n):
            dx, dy = s[i][0] - s[j][0], s[i][1] - s[j][1]
            A[i, j] = phi(dx * dx + dy * dy)
        for k, v in enumerate([mp.mpf(1), s[i][0], s[i][1]]):
            A[i, n + k] = v
            A[n + k, i] = v
        bx[i] = mp.mpf(links[i]["to"]["x"]) - s[i][0]
        by[i] = mp.mpf(links[i]["to"]["y"]) - s[i][1]
    wx = mp.lu_solve(A, bx)
    wy = mp.lu_solve(A, by)
    return {"sources": s, "wx": wx, "wy": wy, "n": n}


def evaluate(sheet, p):
    n, s, wx, wy = sheet["n"], sheet["sources"], sheet["wx"], sheet["wy"]
    x, y = mp.mpf(p["x"]), mp.mpf(p["y"])
    d = [wx[n] + wx[n + 1] * x + wx[n + 2] * y, wy[n] + wy[n + 1] * x + wy[n + 2] * y]
    g = [[wx[n + 1], wx[n + 2]], [wy[n + 1], wy[n + 2]]]  # [component][∂x, ∂y]
    for i in range(n):
        dx, dy = x - s[i][0], y - s[i][1]
        r2 = dx * dx + dy * dy
        f = phi(r2)
        d[0] += wx[i] * f
        d[1] += wy[i] * f
        if r2 != 0:
            k = mp.log(r2) + 1
            g[0][0] += wx[i] * dx * k
            g[0][1] += wx[i] * dy * k
            g[1][0] += wy[i] * dx * k
            g[1][1] += wy[i] * dy * k
    image = {"x": float(x + d[0]), "y": float(y + d[1])}
    jac = [float(1 + g[0][0]), float(g[1][0]), float(g[0][1]), float(1 + g[1][1])]
    return image, jac


def link(fx, fy, tx, ty):
    return {"from": {"x": fx, "y": fy}, "to": {"x": tx, "y": ty}}


def fixed(x, y):
    return link(x, y, x, y)


def P(x, y):
    return {"x": x, "y": y}


def cases():
    out = []

    # Three links: the spline is their affine map (no bending).
    three = [link(0.0, 0.0, 10.0, 5.0), link(100.0, 0.0, 110.5, 4.0), link(0.0, 100.0, 9.0, 106.0)]
    out.append(("Üç bağ: yalnız afin", three, [P(0.0, 0.0), P(50.0, 50.0), P(-20.0, 140.0), P(1000.0, -500.0)]))

    # A 4×4 grid of a sheet: the border fixed, the four inner crosses moved by centimetres.
    grid = []
    for i in range(4):
        for j in range(4):
            x, y = 1000.0 + 100.0 * i, 2000.0 + 100.0 * j
            if i in (1, 2) and j in (1, 2):
                grid.append(link(x, y, x + 0.03 * (i - 1.5) * 2, y - 0.02 * (j - 1.5) * 2 + 0.01))
            else:
                grid.append(fixed(x, y))
    out.append(
        (
            "Pafta ızgarası 4×4: kenarı sabit, içteki dört kesişim santimetrelerle kayar",
            grid,
            [P(1100.0, 2100.0), P(1150.0, 2150.0), P(1125.5, 2210.25), P(1000.0, 2000.0), P(1350.0, 2050.0), P(800.0, 2400.0)],
        )
    )

    # Scattered links at national coordinates: a Helmert-like trend plus centimetre noise.
    rnd = random.Random(156158)
    E, N = 487_000.0, 4_420_000.0
    scale, turn = 1.00012, 0.0004
    scattered = []
    for _ in range(12):
        x = round(E + rnd.uniform(-300, 300), 3)
        y = round(N + rnd.uniform(-300, 300), 3)
        dx, dy = x - E, y - N
        tx = E + 25.0 + scale * (math.cos(turn) * dx - math.sin(turn) * dy) + rnd.uniform(-0.03, 0.03)
        ty = N - 12.0 + scale * (math.sin(turn) * dx + math.cos(turn) * dy) + rnd.uniform(-0.03, 0.03)
        scattered.append(link(x, y, round(tx, 4), round(ty, 4)))
    out.append(
        (
            "Dağınık 12 bağ, TM koordinatları: Helmert eğilimi ve santimetrelik bozulma",
            scattered,
            [P(E, N), P(E + 123.456, N - 78.9), P(E - 250.0, N + 260.0), P(E + 2000.0, N + 1500.0), dict(scattered[3]["from"])],
        )
    )

    # A local bump: one link moves, eight fixed points around it hold the rest.
    bump = [fixed(500.0 + 50.0 * math.cos(k * math.pi / 4), 500.0 + 50.0 * math.sin(k * math.pi / 4)) for k in range(8)]
    bump.append(link(500.0, 500.0, 500.08, 499.95))
    out.append(
        (
            "Yerel bozulma: tek bağ kayar, çevresindeki sekiz sabit nokta gerisini tutar",
            bump,
            [P(500.0, 500.0), P(510.0, 505.0), P(530.0, 500.0), P(560.0, 500.0), P(500.0, 700.0)],
        )
    )

    # Every link fixed: nothing moves.
    still = [fixed(0.0, 0.0), fixed(30.0, 0.0), fixed(0.0, 40.0), fixed(25.0, 35.0)]
    out.append(("Hepsi sabit: hiçbir şey kıpırdamaz", still, [P(10.0, 10.0), P(-500.0, 900.0)]))

    out.append(("İki bağ: az", [link(0.0, 0.0, 1.0, 1.0), link(5.0, 0.0, 6.0, 1.0)], [P(0.0, 0.0)]))
    out.append(
        (
            "Aynı kaynaktan iki bağ",
            [link(0.0, 0.0, 1.0, 1.0), link(5.0, 0.0, 6.0, 1.0), link(0.0, 0.0, 0.5, 0.5), link(0.0, 5.0, 0.0, 6.0)],
            [P(0.0, 0.0)],
        )
    )
    out.append(
        (
            "Kaynaklar bir doğru üstünde",
            [link(0.0, 0.0, 0.0, 0.1), link(10.0, 10.0, 10.0, 10.1), link(20.0, 20.0, 20.0, 20.1), link(-7.5, -7.5, -7.5, -7.4)],
            [P(0.0, 0.0)],
        )
    )
    # A thin strip of links along a road at TM coordinates: still a sheet (only an exact line is refused).
    out.append(
        (
            "Yol boyunca ince şerit, TM koordinatları: yine çözülür",
            [
                link(E, N, E + 0.01, N),
                link(E + 100.0, N + 100.0, E + 100.0, N + 100.02),
                link(E + 200.0, N + 203.5, E + 200.0, N + 203.5),
                link(E + 300.0, N + 300.0, E + 300.02, N + 300.0),
            ],
            [P(E + 50.0, N + 50.0), P(E + 150.0, N + 151.0), P(E + 250.0, N + 260.0)],
        )
    )
    return out


def build():
    out = []
    for name, links, probes in cases():
        sheet = solve(links)
        if "error" in sheet:
            expected = {"error": sheet["error"]}
        else:
            images, jacs = [], []
            for p in probes:
                image, jac = evaluate(sheet, p)
                images.append(image)
                jacs.append(jac)
            expected = {"map": images, "jacobian": jacs}
        out.append({"name": name, "links": links, "probes": probes, "expected": expected})
    return {
        "format": "kentos.fit-rubber",
        "version": 1,
        "description": "Kauçuk levha'nın çözümü (docs/adr/0158 §2): bağların ince plaka eğrisi; örneklerin görüntüsü ve haritanın türevi (sütunları doğuya ve kuzeye bir metrenin görüntüsü), ya da çözümsüzlüğün nedeni. scripts/fixtures/rubber_cases.py mpmath ile 50 basamakta yazar.",
        "tolerance": {"metres": 1e-9, "jacobian": 1e-9},
        "cases": out,
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="compare with the written file instead of writing it")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT} güncel değil: python3 scripts/fixtures/rubber_cases.py", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT}: güncel")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f"{OUT}: {len(json.loads(text)['cases'])} durum")


if __name__ == "__main__":
    main()
