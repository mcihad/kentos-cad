#!/usr/bin/env python3
"""The shared cases of `road.terrains.analysis` (plan/road.md Y18.2, docs/adr/0916; a surface's analysis).

The drawing is road_drainage_cases.py's (its valley, pit and plane lattices) with two more lattices of this script: a
terrace with flat squares and a ridge. Every lattice is z = g(x) + h(y), so the four corners of each square lie on one
plane and every answer is the squares', whichever diagonal the triangulation takes:

- the plan and 3D area: Σ dx·dy and Σ dx·dy·√(1 + |∇z|²);
- an elevation range: each square (a planar quadrilateral) clipped by the levels z ≥ lo and z ≤ hi, its shoelace area;
- slope (percent, |∇z|·100) and aspect (degrees clockwise from north towards the fall, atan2(−gx, −gy)) are the
  square's; a flat square has no aspect; two triangles a square;
- roughness: at a node, 1 − |Σ (π/2)·n| / Σ π/2 over the squares round it (n each square's unit normal), the mean over
  the nodes and the largest (the place given only where it is the one largest);
- a raster's slope: heights at the cell centres of the lattice's box (half a cell in), Horn's 3 × 3 differences, against
  the slope of the square the centre is in, where a centre and its eight neighbours are on the surface.

No KentOS code; the breaks and the cells are the cases' own.

    python3 scripts/fixtures/road_terrain_analysis_cases.py [--check]
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_drainage_cases as drainage  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
W = 1e-6
ANY = "$any"
UID = "0192f5e0-7c3e-7d4a-9b1e-{:012d}"
UNCHANGED = {"canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}



# ADR 1520: surfaces are a Harita's and a Yol's; another project is refused.
NOT_SURFACE = drainage.failed("not_a_road_project", "Arazi yüzeyi Harita ve Yol projelerinde olur (KentOS Harita, KentOS Yol); bu çizim ikisinden biri değil.")

def completed(output):
    return {"status": "completed", "output": output, "warnings": []}


def failed(code, message, path=None):
    e = {"code": code, "message": message}
    if path:
        e["path"] = path
    return {"status": "failed", "error": e}


# ── The lattices ─────────────────────────────────────────────────────────────────────────────────────────────────
TX = 8000.0


def terrace_z(x, y):
    # Rises 0.3 a metre east up to x' = 4, flat to 7, then 0.1 more; falls 0.04 a metre north up to y = 5, flat after.
    xr = x - TX
    g = 0.3 * min(xr, 4.0) + 0.1 * max(xr - 7.0, 0.0)
    h = -0.04 * min(y, 5.0)
    return 30.0 + g + h


RX = 9000.0


def ridge_z(x, y):
    return 40.0 - 0.5 * abs(x - RX - 6.0) + 0.1 * y


EXTRA = [("teras", terrace_z, [TX + i for i in range(11)], [float(j) for j in range(11)]),
         ("sirt", ridge_z, [RX + i for i in range(13)], [float(j) for j in range(9)])]
LATTICES = {"t-3": drainage.LATTICES[0], "t-4": drainage.LATTICES[1], "t-5": drainage.LATTICES[2],
            "t-6": EXTRA[0], "t-7": EXTRA[1]}
NAMES = {"t-3": "Vadi", "t-4": "Çukur", "t-5": "Düz", "t-6": "Teras", "t-7": "Sırt"}


def setup(work="road"):
    s = drainage.setup(work)
    base = {"type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}
    for lid, name in (("teras", "Teras"), ("sirt", "Sırt")):
        s["layers"].append({"id": lid, "name": name, **copy.deepcopy(base)})
    k = max(e["id"] for e in s["entities"])
    for layer, f, xs, ys in EXTRA:
        for y in ys:
            for x in xs:
                k += 1
                s["entities"].append({"kind": "point", "id": k, "layerId": layer, "attrs": {}, "p": {"x": x, "y": y},
                                      "z": f(x, y)})
                s["uids"].append(UID.format(k))
    if work == "road":
        s["settings"]["road"]["surfaces"] += [{"id": "t-6", "name": "Teras", "points": {"layers": ["teras"]}},
                                              {"id": "t-7", "name": "Sırt", "points": {"layers": ["sirt"]}}]
    return s


# ── The answers ──────────────────────────────────────────────────────────────────────────────────────────────────
def squares(sid):
    _, f, xs, ys = LATTICES[sid]
    for j in range(len(ys) - 1):
        for i in range(len(xs) - 1):
            x0, x1, y0, y1 = xs[i], xs[i + 1], ys[j], ys[j + 1]
            gx = (f(x1, y0) - f(x0, y0)) / (x1 - x0)
            gy = (f(x0, y1) - f(x0, y0)) / (y1 - y0)
            poly = [[x0, y0, f(x0, y0)], [x1, y0, f(x1, y0)], [x1, y1, f(x1, y1)], [x0, y1, f(x0, y1)]]
            yield poly, gx, gy, (x1 - x0) * (y1 - y0)


def shoelace(p):
    return sum(p[i][0] * p[(i + 1) % len(p)][1] - p[(i + 1) % len(p)][0] * p[i][1] for i in range(len(p))) / 2.0


def clip(poly, level, above):
    out = []
    n = len(poly)
    for i in range(n):
        a, b = poly[i], poly[(i + 1) % n]
        ia = a[2] >= level if above else a[2] <= level
        ib = b[2] >= level if above else b[2] <= level
        if ia:
            out.append(a)
        if ia != ib:
            t = (level - a[2]) / (b[2] - a[2])
            out.append([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1]), level])
    return out


def range_of(breaks, v):
    if v < breaks[0] or v > breaks[-1]:
        return None
    if v == breaks[-1]:
        return len(breaks) - 2
    return max(i for i in range(len(breaks) - 1) if breaks[i] <= v)


def bands(sid, measure, breaks):
    out = {"bands": [{"from": breaks[i], "to": breaks[i + 1], "area": 0.0, "surfaceArea": 0.0, "triangles": 0}
                     for i in range(len(breaks) - 1)], "outsideArea": 0.0, "outsideTriangles": 0, "flatArea": 0.0,
           "flatTriangles": 0}
    for poly, gx, gy, plan in squares(sid):
        k = math.sqrt(1.0 + gx * gx + gy * gy)
        if measure == "elevation":
            inside = 0.0
            for b in out["bands"]:
                low = clip(poly, b["from"], True)
                part = clip(low, b["to"], False) if len(low) >= 3 else []
                if len(part) < 3:
                    continue
                a = shoelace(part)
                if a <= 0:
                    continue
                b["area"] += a
                b["surfaceArea"] += a * k
                inside += a
            out["outsideArea"] += max(plan - inside, 0.0)
            continue
        if measure == "slope":
            v = math.hypot(gx, gy) * 100.0
        elif gx == 0.0 and gy == 0.0:
            out["flatArea"] += plan
            out["flatTriangles"] += 2
            continue
        else:
            v = math.degrees(math.atan2(-gx, -gy)) % 360.0
        r = range_of(breaks, v)
        if r is None:
            out["outsideArea"] += plan
            out["outsideTriangles"] += 2
        else:
            b = out["bands"][r]
            b["area"] += plan
            b["surfaceArea"] += plan * k
            b["triangles"] += 2
    return out


def unique_best(items):
    """(value, place) of the largest, the place `$any` where another is within 1e-9 of it."""
    best = max(v for v, _ in items)
    near = [p for v, p in items if abs(v - best) <= 1e-9]
    return best, (near[0] if len(near) == 1 else ANY)


def roughness(sid):
    _, f, xs, ys = LATTICES[sid]
    acc = {}
    for j in range(len(ys) - 1):
        for i in range(len(xs) - 1):
            x0, x1, y0, y1 = xs[i], xs[i + 1], ys[j], ys[j + 1]
            gx = (f(x1, y0) - f(x0, y0)) / (x1 - x0)
            gy = (f(x0, y1) - f(x0, y0)) / (y1 - y0)
            ln = math.sqrt(gx * gx + gy * gy + 1.0)
            n = (-gx / ln, -gy / ln, 1.0 / ln)
            for c in ((i, j), (i + 1, j), (i, j + 1), (i + 1, j + 1)):
                s = acc.setdefault(c, [0.0, 0.0, 0.0, 0.0])
                for d in range(3):
                    s[d] += math.pi / 2 * n[d]
                s[3] += math.pi / 2
    items = []
    for (i, j), s in acc.items():
        r = max(0.0, 1.0 - math.sqrt(s[0] ** 2 + s[1] ** 2 + s[2] ** 2) / s[3])
        items.append((r, [xs[i], ys[j]]))
    mean = sum(v for v, _ in items) / len(items)
    best, at = unique_best(items)
    return {"mean": mean, "max": best, "at": at}


def grid(sid, cell):
    _, f, xs, ys = LATTICES[sid]
    x0, y0 = xs[0], ys[0]
    nx, ny = math.floor((xs[-1] - x0) / cell), math.floor((ys[-1] - y0) / cell)

    def centre(i, j):
        return [x0 + cell / 2 + i * cell, y0 + cell / 2 + j * cell]

    def facet(p):
        i = max(k for k in range(len(xs) - 1) if xs[k] <= p[0])
        j = max(k for k in range(len(ys) - 1) if ys[k] <= p[1])
        gx = (f(xs[i + 1], ys[j]) - f(xs[i], ys[j])) / (xs[i + 1] - xs[i])
        gy = (f(xs[i], ys[j + 1]) - f(xs[i], ys[j])) / (ys[j + 1] - ys[j])
        return math.hypot(gx, gy) * 100.0

    diffs = []
    for j in range(1, ny - 1):
        for i in range(1, nx - 1):
            w = [[f(*centre(i + di - 1, j + dj - 1)) for di in range(3)] for dj in range(3)]
            gx = ((w[0][2] + 2 * w[1][2] + w[2][2]) - (w[0][0] + 2 * w[1][0] + w[2][0])) / (8 * cell)
            gy = ((w[2][0] + 2 * w[2][1] + w[2][2]) - (w[0][0] + 2 * w[0][1] + w[0][2])) / (8 * cell)
            p = centre(i, j)
            diffs.append((math.hypot(gx, gy) * 100.0 - facet(p), p))
    out = {"cell": cell, "cells": len(diffs), "mean": 0.0, "meanAbsolute": 0.0, "maxAbsolute": 0.0}
    if diffs:
        out["mean"] = sum(d for d, _ in diffs) / len(diffs)
        out["meanAbsolute"] = sum(abs(d) for d, _ in diffs) / len(diffs)
        best, at = unique_best([(abs(d), p) for d, p in diffs])
        out["maxAbsolute"] = best
        out["at"] = at
    return out


def answer(inp):
    sid = next(i for i, n in NAMES.items() if inp["surface"] in (i, n))
    plan = surf = 0.0
    for _, gx, gy, a in squares(sid):
        plan += a
        surf += a * math.sqrt(1.0 + gx * gx + gy * gy)
    out = {"surface": sid, "area": plan, "surfaceArea": surf}
    for key, measure in (("elevations", "elevation"), ("slopes", "slope"), ("aspects", "aspect")):
        if key in inp:
            out[key] = bands(sid, measure, inp[key])
            if inp.get("pieces"):
                for b in out[key]["bands"]:
                    if b["area"] > 0:
                        b["parts"] = ANY
    if inp.get("roughness"):
        out["roughness"] = roughness(sid)
    if "grid" in inp:
        out["grid"] = grid(sid, inp["grid"])
    return out


def cases():
    out = []

    def q(name, inp, result=None, setup_=None):
        c = {"name": name, "steps": [{"op": "execute", "input": inp,
                                      "result": result if result is not None else completed(answer(inp)),
                                      "expect": UNCHANGED, "within": W}]}
        if setup_:
            c["setup"] = setup_
        out.append(c)

    q("Düzlemin kot aralıkları", {"surface": "Düz", "elevations": [119.0, 120.3, 120.65, 121.0]})
    q("Düzlemin eğimi ve bakısı", {"surface": "Düz", "slopes": [0.0, 5.0, 15.0], "aspects": [0.0, 200.0, 260.0, 360.0]})
    q("Vadinin kot aralıkları", {"surface": "Vadi", "elevations": [100.0, 130.0, 160.0, 175.5, 210.0, 250.0]})
    q("Vadinin eğimi ve iki yamacının bakısı", {"surface": "Vadi", "slopes": [10.0, 20.0, 25.0],
                                                "aspects": [0.0, 90.0, 180.0, 270.0, 360.0]})
    q("Çukurun kot aralıkları", {"surface": "Çukur", "elevations": [50.0, 51.5, 53.25, 55.0, 60.0]})
    q("Çukurun dört yüzü dört bakı", {"surface": "Çukur", "aspects": [10.0, 100.0, 190.0, 280.0, 350.0]})
    q("Terasın düzleri bakısız", {"surface": "Teras", "aspects": [0.0, 30.0, 200.0, 280.0, 360.0]})
    q("Terasın eğim aralıkları sıfırla", {"surface": "Teras", "slopes": [0.0, 1.0, 12.0, 35.0]})
    q("Aralık dışında kalan", {"surface": "Teras", "slopes": [2.0, 11.0], "elevations": [30.5, 31.0]})
    q("Sırtın pürüzlülüğü", {"surface": "Sırt", "roughness": True})
    q("Düzlem pürüzsüz", {"surface": "Düz", "roughness": True})
    q("Vadinin pürüzlülüğü", {"surface": "t-3", "roughness": True})
    q("Düzlemde raster aynı eğimi verir", {"surface": "Düz", "grid": 0.7})
    q("Sırtta raster eğimi düşürür", {"surface": "Sırt", "grid": 0.3})
    q("Vadide raster", {"surface": "Vadi", "grid": 30.0})
    q("Terasta raster", {"surface": "Teras", "grid": 0.45})
    q("Hepsi birden", {"surface": "Vadi", "elevations": [100.0, 175.0, 250.0], "slopes": [0.0, 30.0],
                       "aspects": [0.0, 180.0, 360.0], "roughness": True, "grid": 45.0})
    q("Parçalarıyla", {"surface": "Düz", "slopes": [0.0, 15.0], "elevations": [120.0, 120.5, 121.5], "pieces": True})
    q("Kimlikle", {"surface": "t-4", "slopes": [100.0, 200.0]})
    # Refusals.
    q("Bir şey sorulmadı", {"surface": "Düz"},
      failed("invalid_analysis", "Bir şey sorun: elevations, slopes, aspects, roughness ya da grid."))
    q("Tek sınır", {"surface": "Düz", "slopes": [5.0]},
      failed("invalid_analysis", "Aralık sınırları en az iki, sonlu ve artan sayılar olmalı.", "slopes"))
    q("Artmayan sınırlar", {"surface": "Düz", "aspects": [10.0, 10.0, 20.0]},
      failed("invalid_analysis", "Aralık sınırları en az iki, sonlu ve artan sayılar olmalı.", "aspects"))
    q("Azalan kot sınırları", {"surface": "Düz", "elevations": [121.0, 120.0]},
      failed("invalid_analysis", "Aralık sınırları en az iki, sonlu ve artan sayılar olmalı.", "elevations"))
    q("Hücre sıfır", {"surface": "Düz", "grid": 0.0}, failed("invalid_analysis", "Izgaranın hücresi sıfırdan büyük olmalı.",
                                                            "grid"))
    q("Hücre eksi", {"surface": "Düz", "grid": -1.0}, failed("invalid_analysis", "Izgaranın hücresi sıfırdan büyük olmalı.",
                                                            "grid"))
    q("Çok hücre", {"surface": "Vadi", "grid": 0.1},
      failed("invalid_analysis", "Izgara bu hücreyle 4000000 hücreden çok tutuyor; hücreyi büyütün.", "grid"))
    q("Olmayan yüzey", {"surface": "Tasarım", "slopes": [0.0, 1.0]},
      failed("unknown_terrain", "“Tasarım” kimlikli ya da adlı yüzey projede yok.", "surface"))
    q("Kurulamayan yüzey", {"surface": "Kurulamaz", "slopes": [0.0, 1.0]},
      {"status": "failed", "error": {"code": "terrain_unbuildable", "message": ANY}})
    q("Yol projesi değil", {"surface": "Düz", "slopes": [0.0, 1.0]}, NOT_SURFACE, setup_="$map")
    return out


def main():
    found = cases()
    for c in found:
        if c.get("setup") == "$map":
            c["setup"] = setup(work="architecture")
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.terrains.analysis", "commandVersion": 1,
           "title": "Yüzeyi analiz et: kot, eğim, bakı, pürüzlülük, raster",
           "note": "scripts/fixtures/road_terrain_analysis_cases.py'den; z = g(x) + h(y) kafeslerinde (her kare düzlem) "
                   "karelerle, ADR 0916'nın kuralıyla, KentOS kodu olmadan.",
           "setup": setup(), "cases": found}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    path = OUT / "road.terrains.analysis.json"
    if "--check" in sys.argv:
        if not path.exists() or path.read_text() != text:
            sys.exit("güncel değil: road.terrains.analysis.json")
    else:
        path.write_text(text)
    print(f"road.terrains.analysis.json: {len(found)} durum")


if __name__ == "__main__":
    main()
