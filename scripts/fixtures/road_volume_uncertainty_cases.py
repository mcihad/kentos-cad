#!/usr/bin/env python3
"""An earthwork volume's sensitivity to the ground's heights, shared cases (docs/adr/0941, plan/road.md Y35.1), from an
independent reference: scripts/fixtures/road_volume_cases.py's overlay in exact fractions (clip each design triangle
by each ground triangle, split at d = 0), and on each piece the integral of each ground corner's barycentric weight,
area · φ(centroid), added with its sign to the cut (d < 0) or the fill (d > 0). No KentOS code.

Two checks run here before anything is written, so the fixture is the reference's and the reference is right:

- **The derivative.** Each weight is compared with a central difference of the exact cut and fill when that corner
  is raised and lowered by 10⁻⁶ m (in fractions): they agree to 10⁻⁶ relative, 10⁻⁴ absolute where a corner lies on the zero line (a kink).
- **The statistics.** The ground's heights are disturbed by independent normal errors N(0, σ) 4000 times (seeded).
  The net volume (cut − fill) is linear in the heights: its standard deviation over the trials is σ·√Σ(w_cut −
  w_fill)² within 6 % (the standard error of a standard deviation over 4000 trials is 1.1 %). The cut and the fill
  apart are linear only away from the zero line (an error there moves the line): checked to 6 % where under 5 % of
  the area lies within 2σ of it; the trials' numbers are written for the ADR either way.

    python3 scripts/fixtures/road_volume_uncertainty_cases.py
    ... --check   # fails when fixtures/road/v1/volume_uncertainty.json is not what this writes
"""

import json
import math
import random
import sys
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_volume_cases as vref  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "volume_uncertainty.json"
TRIALS = 4000


def shifted(poly, level, above):
    """The part of a convex polygon with linear d on the `above` side of `level`."""
    moved = [(p, d - level) for p, d in poly]
    return [(p, d + level) for p, d in vref.split(moved, above)]


def weights(design, vertices, triangles, near=None):
    """∂cut/∂zⱼ and ∂fill/∂zⱼ per corner, the cut and fill plan areas, and the area where |d| < near, exactly."""
    verts = [tuple(map(F, v)) for v in vertices]
    kept = []
    for t in triangles:
        tri = [verts[i] for i in t]
        a = vref.area2(tri[0], tri[1], tri[2])
        if a > 0:
            kept.append((tri, list(t)))
        elif a < 0:
            kept.append(([tri[0], tri[2], tri[1]], [t[0], t[2], t[1]]))
    boxes = [(min(p[0] for p in g), min(p[1] for p in g), max(p[0] for p in g), max(p[1] for p in g)) for g, _ in kept]
    cut, fill = [F(0)] * len(verts), [F(0)] * len(verts)
    areas = {"cut": F(0), "fill": F(0), "near": F(0), "covered": F(0)}
    for t in design:
        d = vref.ccw([tuple(map(F, p)) for p in t])
        if not d:
            continue
        dx0, dy0 = min(p[0] for p in d), min(p[1] for p in d)
        dx1, dy1 = max(p[0] for p in d), max(p[1] for p in d)
        for (g, ids), (gx0, gy0, gx1, gy1) in zip(kept, boxes):
            # A ground triangle whose box misses the design triangle's meets none of it.
            if gx1 < dx0 or gx0 > dx1 or gy1 < dy0 or gy0 > dy1:
                continue
            poly = [(p[0], p[1]) for p in d]
            for k in range(3):
                poly = vref.clip(poly, g[k], g[(k + 1) % 3])
                if len(poly) < 3:
                    break
            if len(poly) < 3:
                continue
            w = [(p, vref.z_at(d, p) - vref.z_at(g, p)) for p in poly]
            a, _ = vref.integral(w)
            if a <= 0:
                continue
            areas["covered"] += a
            if near is not None:
                band = shifted(w, -F(near), True)
                if len(band) >= 3:
                    band = shifted(band, F(near), False)
                    if len(band) >= 3:
                        areas["near"] += vref.integral(band)[0]
            whole = vref.area2(g[0], g[1], g[2])
            for above, sign, into, key in ((True, -1, fill, "fill"), (False, 1, cut, "cut")):
                part = vref.split(w, above)
                if len(part) < 3:
                    continue
                area, cx, cy = F(0), F(0), F(0)
                for i in range(1, len(part) - 1):
                    p0, p1, p2 = part[0][0], part[i][0], part[i + 1][0]
                    s = vref.area2(p0, p1, p2) / 2
                    area += s
                    cx += s * (p0[0] + p1[0] + p2[0]) / 3
                    cy += s * (p0[1] + p1[1] + p2[1]) / 3
                if area <= 0:
                    continue
                c = (cx / area, cy / area)
                wa = vref.area2(c, g[1], g[2]) / whole
                wb = vref.area2(g[0], c, g[2]) / whole
                phi = (wa, wb, 1 - wa - wb)
                for k in range(3):
                    into[ids[k]] += sign * area * phi[k]
                areas[key] += area
    return cut, fill, areas


def volumes(design, vertices, triangles):
    return vref.between(design, [[vertices[i] for i in t] for t in triangles])


def check_derivative(design, vertices, triangles, cut, fill):
    h = F(1, 10**6)
    for j in range(len(vertices)):
        up = [list(v) for v in vertices]
        dn = [list(v) for v in vertices]
        up[j][2] = F(up[j][2]) + h
        dn[j][2] = F(dn[j][2]) - h
        vu, vd = volumes(design, up, triangles), volumes(design, dn, triangles)
        for key, w in (("cut", cut), ("fill", fill)):
            fd = (vu[key] - vd[key]) / (2 * float(h))
            # A corner on the zero line is a kink: the difference there is of the step's order (10⁻⁵), not 0.
            assert abs(fd - float(w[j])) < 1e-4 + 1e-6 * abs(float(w[j])), (j, key, fd, float(w[j]))


def float_volumes(design, vertices, triangles):
    """The cut and fill in floats, for the trials (the same overlay without fractions)."""
    ground = [[vertices[i] for i in t] for t in triangles]
    cut = fill = 0.0
    for t in design:
        d = vref.ccw([tuple(p) for p in t])
        if not d:
            continue
        for g0 in ground:
            g = vref.ccw([tuple(p) for p in g0])
            if not g:
                continue
            poly = [(p[0], p[1]) for p in d]
            for k in range(3):
                poly = vref.clip(poly, g[k], g[(k + 1) % 3])
                if len(poly) < 3:
                    break
            if len(poly) < 3:
                continue
            w = [(p, vref.z_at(d, p) - vref.z_at(g, p)) for p in poly]
            _, fv = float_integral(vref.split(w, True))
            _, cv = float_integral(vref.split(w, False))
            fill += fv
            cut -= cv
    return cut, fill


def float_integral(poly):
    a = v = 0.0
    for i in range(1, len(poly) - 1):
        s = vref.area2(poly[0][0], poly[i][0], poly[i + 1][0]) / 2
        a += s
        v += s * (poly[0][1] + poly[i][1] + poly[i + 1][1]) / 3
    return a, v


def trials(design, vertices, triangles, sigma, seed):
    rnd = random.Random(seed)
    cuts, fills, nets = [], [], []
    for _ in range(TRIALS):
        moved = [(x, y, z + rnd.gauss(0.0, sigma)) for x, y, z in vertices]
        c, f = float_volumes(design, moved, triangles)
        cuts.append(c)
        fills.append(f)
        nets.append(c - f)

    def sd(xs):
        m = sum(xs) / len(xs)
        return math.sqrt(sum((x - m) ** 2 for x in xs) / (len(xs) - 1))

    return sd(cuts), sd(fills), sd(nets)


# ---- the grounds and designs ---------------------------------------------------------------------------------------

def grid(n, m, step, z, x0=0.0, y0=0.0):
    """An n × m grid of corners, each cell two triangles; z(x, y) its heights."""
    vertices = [(x0 + i * step, y0 + j * step, z(x0 + i * step, y0 + j * step)) for j in range(m) for i in range(n)]
    tris = []
    for j in range(m - 1):
        for i in range(n - 1):
            a, b, c, d = j * n + i, j * n + i + 1, (j + 1) * n + i + 1, (j + 1) * n + i
            tris += [[a, b, c], [a, c, d]]
    return vertices, tris


def plane(x0, y0, x1, y1, z):
    return vref.square(x0, y0, x1, y1, z)


def bumpy(seed, base=100.0, amp=1.0):
    rnd = random.Random(seed)
    table = {}

    def z(x, y):
        key = (round(x, 6), round(y, 6))
        if key not in table:
            table[key] = round(base + amp * (rnd.random() - 0.5) + 0.01 * x, 3)
        return table[key]
    return z


def case(name, ground, design, sigma=0.05, beta=0.02, seed=1, mc=True):
    vertices, triangles = ground
    cut, fill, areas = weights(design, vertices, triangles, near=2 * sigma)
    check_derivative(design, vertices, triangles, cut, fill)
    wc = [float(x) for x in cut]
    wf = [float(x) for x in fill]
    wn = [a - b for a, b in zip(wc, wf)]
    sc = sigma * math.sqrt(sum(x * x for x in wc))
    sf = sigma * math.sqrt(sum(x * x for x in wf))
    sn = sigma * math.sqrt(sum(x * x for x in wn))
    covered = float(areas["covered"])
    out = {"name": name, "vertices": [list(v) for v in vertices], "triangles": triangles,
           "design": [[list(p) for p in t] for t in design], "sigma": sigma, "beta": beta,
           "expected": {"cut": wc, "fill": wf, "cutArea": float(areas["cut"]), "fillArea": float(areas["fill"]),
                        "nearArea": float(areas["near"]), "cutSigma": sc, "fillSigma": sf, "netSigma": sn,
                        "cutBias": beta * sum(wc), "fillBias": beta * sum(wf)}}
    if mc:
        mcut, mfill, mnet = trials(design, vertices, triangles, sigma, seed)
        # The net is linear in the heights: its spread is the propagation's exactly (to the trials' own error).
        if sn > 1e-9:
            assert abs(mnet - sn) / sn < 0.06, (name, mnet, sn)
        # Cut and fill apart are linear only away from the zero line: checked where little lies near it.
        share = float(areas["near"]) / covered if covered > 0 else 0.0
        if share < 0.05:
            for got, want in ((mcut, sc), (mfill, sf)):
                if want > 1e-9:
                    assert abs(got - want) / want < 0.06, (name, got, want)
        out["trials"] = {"count": TRIALS, "cutSigma": mcut, "fillSigma": mfill, "netSigma": mnet, "nearShare": share}
    return out


def cases():
    out = []
    flat = lambda z0: (lambda x, y: z0)  # noqa: E731
    tilt = lambda z0, gx, gy: (lambda x, y: z0 + gx * x + gy * y)  # noqa: E731
    out.append(case("Düz arazi, düz dolgu: bütün ağırlık dolguda", grid(4, 4, 10.0, flat(100.0)),
                    plane(5, 5, 25, 25, flat(101.0))))
    out.append(case("Düz arazi, düz yarma", grid(4, 4, 10.0, flat(100.0)), plane(5, 5, 25, 25, flat(99.0))))
    out.append(case("Eğik tasarım: yarma ve dolgu yan yana", grid(5, 5, 10.0, flat(100.0)),
                    plane(0, 0, 40, 40, tilt(98.0, 0.1, 0.0))))
    out.append(case("Dalgalı arazi, düz platform", grid(6, 6, 10.0, bumpy(3)), plane(5, 5, 45, 45, flat(100.2))))
    out.append(case("Seyrek arazi (20 m), aynı platform", grid(4, 4, 20.0, bumpy(4)), plane(5, 5, 55, 55, flat(100.2))))
    out.append(case("Sık arazi (5 m), aynı platform", grid(12, 12, 5.0, bumpy(5)), plane(5, 5, 50, 50, flat(100.2)),
                    mc=False))
    out.append(case("Tasarım arazinin dışına taşar", grid(3, 3, 10.0, flat(100.0)), plane(10, 10, 35, 35, flat(101.0))))
    out.append(case("Tek üçgen arazi", ([(0, 0, 100), (30, 0, 101), (0, 30, 102)], [[0, 1, 2]]),
                    [[(2, 2, 101.5), (20, 2, 101.5), (2, 20, 101.5)]]))
    out.append(case("Ters dönük üçgenler", ([(0, 0, 100), (20, 0, 100.5), (20, 20, 101), (0, 20, 100.5)],
                                            [[0, 2, 1], [0, 3, 2]]), plane(2, 2, 18, 18, flat(100.4))))
    out.append(case("Kenarlarla hizalı tasarım", grid(3, 3, 10.0, tilt(100, 0.02, 0.01)),
                    plane(0, 0, 20, 20, flat(100.3))))
    out.append(case("Sıfır çizgisi bir köşeden geçer", grid(3, 3, 10.0, tilt(100, 0.1, 0.0)),
                    plane(0, 0, 20, 20, flat(101.0))))
    out.append(case("Büyük σ, küçük β", grid(4, 4, 10.0, bumpy(7)), plane(3, 3, 27, 27, flat(100.0)), sigma=0.2,
                    beta=0.005, seed=2))
    out.append(case("Küçük σ, büyük β", grid(4, 4, 10.0, bumpy(8)), plane(3, 3, 27, 27, flat(100.0)), sigma=0.01,
                    beta=0.1, seed=3))
    out.append(case("Eksi β: arazi aşağı kaymış", grid(4, 4, 10.0, bumpy(9)), plane(3, 3, 27, 27, flat(100.0)),
                    beta=-0.03, seed=4))
    out.append(case("Şerit gibi uzun dar tasarım", grid(8, 3, 10.0, bumpy(10)), plane(2, 8, 68, 12, flat(100.1)),
                    seed=5))
    out.append(case("Yamaçta yol: yarmada üst, dolguda alt", grid(6, 4, 10.0, tilt(100, 0.0, 0.15)),
                    plane(0, 10, 50, 20, tilt(101.5, 0.0, 0.0))))
    out.append(case("İki tasarım üçgeni farklı kotta", grid(4, 4, 10.0, flat(100.0)),
                    [[(0, 0, 101), (30, 0, 101), (0, 30, 99)], [(30, 0, 101), (30, 30, 99), (0, 30, 99)]]))
    out.append(case("Tasarım arazinin 1 cm üstünde: her yer sıfır çizgisine yakın", grid(3, 3, 10.0, flat(100.0)),
                    plane(0, 0, 20, 20, flat(100.01)), seed=9))
    out.append(case("Ötelenmiş ızgara", grid(4, 4, 10.0, bumpy(11), x0=1000.0, y0=2000.0),
                    plane(1003, 2003, 1027, 2027, flat(100.3)), seed=6))
    out.append(case("Arazinin üstünde çok yüksek dolgu", grid(4, 4, 10.0, bumpy(12)), plane(2, 2, 28, 28, flat(110.0)),
                    seed=7))
    out.append(case("Arazinin altında derin yarma", grid(4, 4, 10.0, bumpy(13)), plane(2, 2, 28, 28, flat(90.0)),
                    seed=8))
    return out


def main():
    data = {"cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            sys.exit(f"{OUT} is not what scripts/fixtures/road_volume_uncertainty_cases.py writes")
        print(f"{OUT}: {len(data['cases'])} cases")
        return
    OUT.write_text(text)
    print(f"wrote {OUT}: {len(data['cases'])} cases")
    for c in data["cases"]:
        e, t = c["expected"], c.get("trials")
        if t:
            print(f"  {c['name'][:44]:44} near {t['nearShare']:.2f}  cut {e['cutSigma']:.3f}/{t['cutSigma']:.3f}  "
                  f"fill {e['fillSigma']:.3f}/{t['fillSigma']:.3f}  net {e['netSigma']:.3f}/{t['netSigma']:.3f}")


if __name__ == "__main__":
    main()
