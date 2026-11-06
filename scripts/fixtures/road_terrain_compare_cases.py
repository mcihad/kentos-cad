#!/usr/bin/env python3
"""The shared cases of `road.terrains.compare` (plan/road.md Y18.3, docs/adr/0916 §3; two surfaces' difference).

The drawing is road_terrain_analysis_cases.py's with two more lattices: a cut over the plane (a coarser lattice
shifted half off it) and a pad over the terrace. Every lattice is z = g(x) + h(y), so each square is one plane; laying
two lattices over each other gives rectangles (between the union of their x and of their y lines) each inside one
square of each, where the difference d = z_comparison − z_base is linear: its fill and cut are the fan integral
Σ A·(d₀ + d₁ + d₂)/3 of the rectangle's parts where d ≥ 0 and d ≤ 0; a range's area the rectangle clipped by d ≥ lo and
d ≤ hi. The extremes are at rectangle corners (the smallest x, then y, among equals; the place given only where it is
the one). The comparison's area off the base is uncovered.

No KentOS code; the ranges are the cases' own.

    python3 scripts/fixtures/road_terrain_compare_cases.py [--check]
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_drainage_cases as drainage  # noqa: E402
import road_terrain_analysis_cases as analysis  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
W = 1e-6
ANY = "$any"
UID = "0192f5f0-7c3e-7d4a-9b1e-{:012d}"
UNCHANGED = analysis.UNCHANGED
FX, TX = drainage.FX, analysis.TX



# ADR 1520: surfaces are a Harita's and a Yol's; another project is refused.
NOT_SURFACE = drainage.failed("not_a_road_project", "Arazi yüzeyi Harita ve Yol projelerinde olur (KentOS Harita, KentOS Yol); bu çizim ikisinden biri değil.")

def cut_z(x, y):
    # Over x 6002.5–6012.5, y 0–10 every 2.5: the plane cut down towards x′ = 7.5 and tilted the other way in y.
    xr = x - FX
    return 119.9 + 0.1 * xr - 0.15 * max(2.5 - abs(xr - 7.5), 0.0) - 0.01 * y


def pad_z(x, y):
    return analysis.terrace_z(x, y) + 0.2 - 0.05 * abs(x - TX - 5.0)


EXTRA = [("kazi", cut_z, [FX + 2.5 + 2.5 * i for i in range(5)], [2.5 * j for j in range(5)]),
         ("set", pad_z, [TX + i for i in range(11)], [float(j) for j in range(11)])]
LATTICES = {**analysis.LATTICES, "t-8": EXTRA[0], "t-9": EXTRA[1]}
NAMES = {**analysis.NAMES, "t-8": "Kazı", "t-9": "Set"}


def setup(work="road"):
    s = analysis.setup(work)
    base = {"type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}
    for lid, name in (("kazi", "Kazı"), ("set", "Set")):
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
        s["settings"]["road"]["surfaces"] += [{"id": "t-8", "name": "Kazı", "points": {"layers": ["kazi"]}},
                                              {"id": "t-9", "name": "Set", "points": {"layers": ["set"]}}]
    return s


def area2(a, b, c):
    return (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])


def split(poly, level, above):
    out = []
    for i in range(len(poly)):
        p, q = poly[i], poly[(i + 1) % len(poly)]
        dp, dq = p[1] - level, q[1] - level
        if (dp >= 0) if above else (dp <= 0):
            out.append(p)
        if (dp > 0 and dq < 0) or (dp < 0 and dq > 0):
            t = dp / (dp - dq)
            out.append(([p[0][0] + (q[0][0] - p[0][0]) * t, p[0][1] + (q[0][1] - p[0][1]) * t], level))
    return out


def integral(poly):
    a = v = 0.0
    for i in range(1, len(poly) - 1):
        s = area2(poly[0][0], poly[i][0], poly[i + 1][0]) / 2.0
        a += s
        v += s * (poly[0][1] + poly[i][1] + poly[i + 1][1]) / 3.0
    return a, v


def best(items, larger):
    if not items:
        return None
    v = max(d for d, _ in items) if larger else min(d for d, _ in items)
    near = sorted(p for d, p in items if abs(d - v) <= 1e-9)
    return {"difference": v, "at": near[0] if len(near) == 1 else ANY}


def answer(inp):
    bid = next(i for i, n in NAMES.items() if inp["base"] in (i, n))
    cid = next(i for i, n in NAMES.items() if inp["comparison"] in (i, n))
    _, fb, bxs, bys = LATTICES[bid]
    _, fc, cxs, cys = LATTICES[cid]
    xs = sorted({x for x in bxs + cxs if cxs[0] <= x <= cxs[-1]})
    ys = sorted({y for y in bys + cys if cys[0] <= y <= cys[-1]})
    breaks = inp.get("ranges")
    out = {"base": bid, "comparison": cid, "cut": 0.0, "fill": 0.0, "net": 0.0, "cutArea": 0.0, "fillArea": 0.0,
           "covered": 0.0, "uncovered": 0.0}
    bands = [[breaks[i], breaks[i + 1], 0.0] for i in range(len(breaks) - 1)] if breaks else []
    outside = 0.0
    corners = []
    for j in range(len(ys) - 1):
        for i in range(len(xs) - 1):
            x0, x1, y0, y1 = xs[i], xs[i + 1], ys[j], ys[j + 1]
            a = (x1 - x0) * (y1 - y0)
            if not (bxs[0] <= x0 and x1 <= bxs[-1] and bys[0] <= y0 and y1 <= bys[-1]):
                out["uncovered"] += a
                continue
            out["covered"] += a
            poly = [([x, y], fc(x, y) - fb(x, y)) for x, y in ((x0, y0), (x1, y0), (x1, y1), (x0, y1))]
            corners += [(d, p) for p, d in poly]
            fa, fv = integral(split(poly, 0.0, True))
            ca, cv = integral(split(poly, 0.0, False))
            out["fill"] += fv
            out["fillArea"] += fa
            out["cut"] -= cv
            out["cutArea"] += ca
            inside = 0.0
            for b in bands:
                low = split(poly, b[0], True)
                part = split(low, b[1], False) if len(low) >= 3 else []
                if len(part) >= 3:
                    pa, _ = integral(part)
                    if pa > 0:
                        b[2] += pa
                        inside += pa
            outside += max(a - inside, 0.0)
    out["net"] = out["fill"] - out["cut"]
    for key, larger in (("least", False), ("largest", True)):
        e = best(corners, larger)
        if e:
            out[key] = e
    if breaks:
        out["bands"] = [{"from": b[0], "to": b[1], "area": b[2]} for b in bands]
        out["outsideArea"] = outside
    warns = []
    if out["uncovered"] > 0:
        warns.append({"code": "compare_uncovered", "message": f"Karşılaştırma yüzeyinin {out['uncovered']:.2f} m²'si "
                      "tabanın dışında; o kısım hacme girmedi.", "path": "comparison"})
    return out, warns


def cases():
    out = []

    def q(name, inp, result=None, setup_=None):
        if result is None:
            o, w = answer(inp)
            result = {"status": "completed", "output": o, "warnings": w}
        c = {"name": name, "steps": [{"op": "execute", "input": inp, "result": result, "expect": UNCHANGED,
                                      "within": W}]}
        if setup_:
            c["setup"] = setup_
        out.append(c)

    q("Kazı düzlemin üstünde, yarısı dışında", {"base": "Düz", "comparison": "Kazı"})
    q("Kazı aralıklarıyla", {"base": "Düz", "comparison": "Kazı", "ranges": [-0.5, -0.2, 0.0, 0.2, 0.5]})
    q("Set terasın üstünde", {"base": "Teras", "comparison": "Set"})
    q("Set aralıklarıyla", {"base": "Teras", "comparison": "Set", "ranges": [-0.1, 0.0, 0.1, 0.2]})
    q("Ters yönde kazı ve dolgu yer değiştirir", {"base": "Set", "comparison": "Teras", "ranges": [-0.2, 0.0, 0.2]})
    q("Düzlem kazının üstünde", {"base": "Kazı", "comparison": "Düz"})
    q("Kimlikle", {"base": "t-6", "comparison": "t-9"})
    q("Örtüşmeyen yüzeyler", {"base": "Vadi", "comparison": "Düz"})
    q("Aralıkların dışı", {"base": "Teras", "comparison": "Set", "ranges": [0.15, 0.18]})
    q("Tek aralık bütün farkı alır", {"base": "Düz", "comparison": "Kazı", "ranges": [-5.0, 5.0]})
    q("Terasın sırtla farkı", {"base": "Teras", "comparison": "Sırt"})
    q("Çukurun düzle farkı", {"base": "Düz", "comparison": "Çukur"})
    q("Teras ile set, ince aralıklar", {"base": "Teras", "comparison": "Set",
                                        "ranges": [-0.05, 0.0, 0.05, 0.1, 0.15, 0.2, 0.25]})
    q("Ters yönde ince aralıklar", {"base": "Set", "comparison": "Teras", "ranges": [-0.25, -0.1, 0.0, 0.1]})
    # Refusals.
    bad = "Fark aralıklarının sınırları en az iki, sonlu ve artan sayılar olmalı."
    q("Tek sınır", {"base": "Düz", "comparison": "Kazı", "ranges": [0.0]},
      analysis.failed("invalid_analysis", bad, "ranges"))
    q("Artmayan sınırlar", {"base": "Düz", "comparison": "Kazı", "ranges": [0.0, 0.0]},
      analysis.failed("invalid_analysis", bad, "ranges"))
    q("Azalan sınırlar", {"base": "Düz", "comparison": "Kazı", "ranges": [1.0, -1.0]},
      analysis.failed("invalid_analysis", bad, "ranges"))
    q("Aynı yüzey", {"base": "Düz", "comparison": "t-5"},
      analysis.failed("invalid_analysis", "Taban ile karşılaştırma aynı yüzey; iki ayrı yüzey verin.", "comparison"))
    q("Olmayan taban", {"base": "Tasarım", "comparison": "Düz"},
      analysis.failed("unknown_terrain", "“Tasarım” kimlikli ya da adlı yüzey projede yok.", "base"))
    q("Olmayan karşılaştırma", {"base": "Düz", "comparison": "Tasarım"},
      analysis.failed("unknown_terrain", "“Tasarım” kimlikli ya da adlı yüzey projede yok.", "comparison"))
    q("Kurulamayan yüzey", {"base": "Düz", "comparison": "Kurulamaz"},
      {"status": "failed", "error": {"code": "terrain_unbuildable", "message": ANY}})
    q("Yol projesi değil", {"base": "Düz", "comparison": "Kazı"}, NOT_SURFACE, setup_="$map")
    return out


def main():
    found = cases()
    for c in found:
        if c.get("setup") == "$map":
            c["setup"] = setup(work="architecture")
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.terrains.compare", "commandVersion": 1,
           "title": "Yüzeyleri karşılaştır: kazı, dolgu, fark aralıkları",
           "note": "scripts/fixtures/road_terrain_compare_cases.py'den; iki kafesin dikdörtgenlerle üst üste konuşu, her "
                   "dikdörtgende fark doğrusal, ADR 0916'nın kuralıyla, KentOS kodu olmadan.",
           "setup": setup(), "cases": found}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    path = OUT / "road.terrains.compare.json"
    if "--check" in sys.argv:
        if not path.exists() or path.read_text() != text:
            sys.exit("güncel değil: road.terrains.compare.json")
    else:
        path.write_text(text)
    print(f"road.terrains.compare.json: {len(found)} durum")


if __name__ == "__main__":
    main()
